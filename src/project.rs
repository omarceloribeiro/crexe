//! Persisted operations shared by CLI and exported scripts. They never call an LLM.
use super::{
    config::Settings, get_build_steps, get_cmd, get_path, paths, render_template, GeneratedFile,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

pub(crate) const MANIFEST: &str = "CREXE-PROJECT.json";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Project {
    schema: u32,
    pub name: String,
    pub os: String,
    pub arch: String,
    pub profile: String,
    pub sources: Vec<String>,
    pub commands: BTreeMap<String, Vec<Vec<String>>>,
}

impl Project {
    pub fn from_spec(
        spec: &Value,
        target: &str,
        ctx: &BTreeMap<String, String>,
        files: &[GeneratedFile],
    ) -> Result<Self> {
        let mut commands = BTreeMap::from([
            ("build".into(), get_build_steps(spec, target)?),
            (
                "run".into(),
                vec![get_cmd(spec, &["targets", target, "run", "cmd"])?],
            ),
        ]);
        for operation in ["test", "publish"] {
            if let Some(steps) = get_path(spec, &["targets", target, operation, "steps"])
                .and_then(Value::as_sequence)
            {
                let steps = steps
                    .iter()
                    .map(|step| super::value_to_cmd(get_path(step, &["cmd"]).unwrap_or(step)))
                    .collect::<Result<Vec<_>>>()?;
                commands.insert(operation.into(), steps);
            }
        }
        let mut sources: Vec<_> = files
            .iter()
            .map(|file| render_template(&file.path, spec, ctx).replace('\\', "/"))
            .collect();
        let dotnet = get_path(spec, &["engine_project"]).is_some();
        if dotnet {
            sources.retain(|source| !source.eq_ignore_ascii_case("CrexeApp.csproj"));
            sources.push("CrexeApp.csproj".into());
            commands.entry("publish".into()).or_insert_with(|| {
                vec![vec![
                    "dotnet",
                    "publish",
                    "CrexeApp.csproj",
                    "-c",
                    "Release",
                    "-o",
                    "publish",
                    "--self-contained",
                    "false",
                ]
                .into_iter()
                .map(String::from)
                .collect()]
            });
        }
        if get_path(spec, &["engine_profile"]).and_then(Value::as_str) == Some("c-gtk") {
            sources.push("CrexeBuild.mk".into());
        }
        for steps in commands.values_mut() {
            for cmd in steps {
                for argument in cmd {
                    *argument = render_template(argument, spec, ctx);
                }
            }
        }
        let plan = Self {
            schema: 1,
            name: get_path(spec, &["meta", "name"])
                .and_then(Value::as_str)
                .unwrap_or("CREXE app")
                .to_owned(),
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            profile: get_path(spec, &["engine_profile"])
                .and_then(Value::as_str)
                .unwrap_or("legacy")
                .into(),
            sources,
            commands,
        };
        plan.validate()?;
        Ok(plan)
    }

    fn validate(&self) -> Result<()> {
        if self.schema != 1 {
            bail!("Unsupported project manifest version");
        }
        if self.sources.is_empty() || self.sources.len() > 129 {
            bail!("Invalid project source count");
        }
        let mut unique = BTreeSet::new();
        for source in &self.sources {
            let relative = paths::relative_file(source)?;
            let normalized = relative.to_string_lossy().replace('\\', "/");
            if reserved(&normalized) || !unique.insert(normalized.to_lowercase()) {
                bail!("Reserved or duplicate project source: {source}");
            }
        }
        for operation in ["build", "run"] {
            if self.commands.get(operation).is_none_or(Vec::is_empty) {
                bail!("Project has no {operation} operation");
            }
        }
        if self.commands["run"].len() != 1 {
            bail!("Project run requires one command");
        }
        for (operation, steps) in &self.commands {
            if !["build", "run", "test", "publish"].contains(&operation.as_str())
                || steps.is_empty()
                || steps.len() > 32
            {
                bail!("Invalid project operation: {operation}");
            }
            for cmd in steps {
                if cmd.is_empty()
                    || cmd[0].is_empty()
                    || cmd.len() > 128
                    || cmd
                        .iter()
                        .any(|arg| arg.len() > 32768 || arg.contains('\0'))
                {
                    bail!("Invalid command in project manifest");
                }
            }
        }
        Ok(())
    }

    pub fn load(root: &Path) -> Result<Self> {
        let file = root.join(MANIFEST);
        paths::reject_links(&file)?;
        if fs::metadata(&file)
            .context(
                "Project manifest missing; use a generated project/ZIP from the current engine",
            )?
            .len()
            > 1024 * 1024
        {
            bail!("Project manifest exceeds 1 MiB");
        }
        let plan: Self =
            serde_json::from_slice(&fs::read(file)?).context("Invalid project manifest")?;
        plan.validate()?;
        Ok(plan)
    }

    pub fn write(&self, root: &Path) -> Result<()> {
        fs::write(root.join(MANIFEST), serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }

    fn compatible(&self) -> Result<()> {
        if self.os != std::env::consts::OS || self.arch != std::env::consts::ARCH {
            bail!("Project targets {}/{}, but this engine is {}/{}; cross-compilation is not implicit", self.os, self.arch, std::env::consts::OS, std::env::consts::ARCH);
        }
        Ok(())
    }

    fn preflight(&self, operations: &[&str], settings: &Settings) -> Result<()> {
        self.compatible()?;
        for operation in operations {
            let steps = self
                .commands
                .get(*operation)
                .with_context(|| format!("Project does not define {operation}"))?;
            if *operation != "run" {
                for cmd in steps {
                    if !(*operation == "test" && cmd[0] == self.commands["run"][0][0]) {
                        settings.execution.authorize(&cmd[0])?;
                    }
                }
            }
        }
        Ok(())
    }

    fn execute(&self, operation: &str, root: &Path, settings: &Settings) -> Result<()> {
        for cmd in self
            .commands
            .get(operation)
            .with_context(|| format!("Project does not define {operation}"))?
        {
            super::run_command(
                &Value::Null,
                cmd,
                root,
                operation,
                &self.os,
                &BTreeMap::new(),
                settings,
            )?;
        }
        Ok(())
    }

    fn copy_sources(&self, root: &Path, destination: &Path) -> Result<()> {
        let mut total = 0u64;
        for source in &self.sources {
            let relative = paths::relative_file(source)?;
            let from = root.join(&relative);
            paths::reject_links(&from)?;
            let metadata = fs::metadata(&from)?;
            total = total
                .checked_add(metadata.len())
                .context("Source size overflow")?;
            if !metadata.is_file() || metadata.len() > 4 * 1024 * 1024 || total > 20 * 1024 * 1024 {
                bail!("Project sources exceed limits");
            }
            let to = destination.join(relative);
            fs::create_dir_all(to.parent().unwrap())?;
            fs::copy(from, to)?;
        }
        Ok(())
    }
}

pub(crate) fn reserved(path: &str) -> bool {
    [
        MANIFEST,
        "source.zip",
        "CREXE-README.md",
        "build.bat",
        "run.bat",
        "test.bat",
        "publish.bat",
        "build.sh",
        "run.sh",
        "test.sh",
        "publish.sh",
    ]
    .iter()
    .any(|name| path.eq_ignore_ascii_case(name))
}

pub(crate) fn operate(
    operation: &str,
    root: &Path,
    output: Option<&Path>,
    settings: &Settings,
) -> Result<()> {
    let root = root.canonicalize().context("Project directory not found")?;
    let plan = Project::load(&root)?;
    if operation == "run" {
        plan.preflight(&["run"], settings)?;
        return plan.execute("run", &root, settings);
    }
    let destination = output.map(std::path::absolute).transpose()?;
    if let Some(path) = &destination {
        paths::reject_links(path)?;
        if path.exists() {
            bail!("Output already exists; choose a new path");
        }
    }
    if operation != "export" {
        let mut operations = vec!["build"];
        if operation == "test" || plan.commands.contains_key("test") {
            operations.push("test");
        }
        if operation == "publish" {
            operations.push("publish");
        }
        plan.preflight(&operations, settings)?;
    }
    let temporary = tempfile::Builder::new()
        .prefix("crexe-project-")
        .tempdir()?;
    let workspace = temporary.path().canonicalize()?;
    let result = (|| -> Result<()> {
        plan.copy_sources(&root, &workspace)?;
        if operation != "export" {
            if plan.profile != "legacy" {
                super::profiles::preflight(
                    &serde_yaml::to_value(serde_json::json!({"engine_profile": plan.profile}))?,
                    &workspace,
                    settings,
                )?;
            }
            plan.execute("build", &workspace, settings)?;
            if operation == "test" || plan.commands.contains_key("test") {
                plan.execute("test", &workspace, settings)?;
            }
            super::resolve_run_command(plan.commands["run"][0].clone(), &workspace)?;
            if operation == "publish" {
                plan.execute("publish", &workspace, settings)?;
            }
        }
        super::package::bundle(&plan, &workspace)?;
        match operation {
            "export" => super::package::export(&workspace, destination.as_deref())?,
            "build" | "publish" => {
                let destination = destination
                    .as_ref()
                    .context("This operation requires --output")?;
                publish_directory(&workspace, destination)?;
                println!("Project {operation} output: {}", destination.display());
            }
            "test" => println!("Project tests passed; no provider requests were made"),
            _ => bail!("Unknown project operation"),
        }
        Ok(())
    })();
    if let Err(error) = result {
        let saved = temporary.keep();
        return Err(error).with_context(|| {
            format!(
                "Project operation failed. Diagnostic workspace: {}",
                saved.display()
            )
        });
    }
    Ok(())
}

fn publish_directory(source: &Path, destination: &Path) -> Result<()> {
    let parent = destination.parent().context("Invalid output path")?;
    fs::create_dir_all(parent)?;
    let stage = tempfile::Builder::new()
        .prefix("crexe-output-")
        .tempdir_in(parent)?;
    super::cache::copy_tree(source, source, stage.path(), &mut BTreeMap::new(), &mut 0)?;
    if destination.exists() {
        bail!("Output appeared during build; it was preserved");
    }
    fs::rename(stage.path(), destination)?;
    Ok(())
}
