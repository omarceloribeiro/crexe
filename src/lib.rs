use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_yaml::{Mapping, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

mod cache;
mod config;
mod config_editor;
mod configure;
mod credentials;
mod environment;
mod executor;
mod format;
mod generator;
mod installation;
mod package;
mod paths;
mod policy;
mod preparation;
mod presentation;
mod profiles;
mod project;

const RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(name = "crexe", version)]
#[command(about = "CREXE v1 Rust executor", long_about = None)]
struct Cli {
    /// Trusted local provider settings (never loaded from a recipe)
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long, global = true)]
    provider: Option<String>,
    #[arg(long, global = true)]
    model: Option<String>,
    /// Explicit credential file; never searched in the recipe's directory
    #[arg(long, global = true)]
    env_file: Option<PathBuf>,
    /// Intent profile (for plaintext/Markdown)
    #[arg(long, global = true)]
    profile: Option<String>,
    /// Build and validate without launching the final application
    #[arg(long, global = true)]
    no_run: bool,
    /// Export the complete source ZIP to a new file
    #[arg(long, global = true)]
    export: Option<PathBuf>,
    /// Maximum compiler/test repair proposals after the initial generation
    #[arg(long, global = true, default_value_t = 2, value_parser = clap::value_parser!(u8).range(0..=2))]
    max_repairs: u8,
    /// Simple desktop progress window (used by the file association)
    #[arg(long, global = true)]
    ui: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Open native provider/model/API-key settings; import only stages changes for review
    Configure {
        #[arg(long)]
        import: Option<PathBuf>,
    },
    /// Generate + build + run a .crexe file
    Exec {
        /// Path to .crexe file
        file: PathBuf,
        /// Override inputs: KEY=VALUE
        #[arg(long = "set")]
        set: Vec<String>,
        /// Ignore cache and regenerate/rebuild
        #[arg(long = "rebuild", default_value_t = false)]
        rebuild: bool,
        /// Override the local cache directory (never taken from the recipe)
        #[arg(long)]
        cache_dir: Option<PathBuf>,
    },
    /// Install this engine at the stable per-user releases/v1 path
    Install,
    /// Associate .crexe with the installed engine for the current user
    Associate,
    /// Remove only associations owned by the installed CREXE engine
    Unassociate,
    /// Read format and resolve the local profile without generation or execution
    Inspect { file: PathBuf },
    /// Show local configuration, host architecture and tools without reading secrets
    Doctor {
        /// Probe the selected profile's SDK locally; no provider or generation call
        #[arg(long)]
        check: bool,
    },
    /// Build a generated project without using a model; write a new project directory
    Build {
        project: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Build and test a generated project without using a model
    Test { project: PathBuf },
    /// Run the existing artifact described by a generated project's manifest
    Run { project: PathBuf },
    /// Build, test and publish a project into a new directory; no deploy or model call
    Publish {
        project: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Export current project sources and scripts into a new ZIP without generation
    Export {
        project: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Debug, Clone)]
struct RuntimeVars {
    os: String,
    arch: String,
    exe_ext: String,
    user_home: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct GeneratedOutput {
    files: Vec<GeneratedFile>,
    #[allow(dead_code)]
    hints: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct GeneratedFile {
    path: String,
    content: String,
}

pub fn run() -> Result<()> {
    executor::install_cancel_handler()?;
    let mut args: Vec<std::ffi::OsString> = env::args_os().collect();
    if args.len() == 1 {
        args.push("configure".into());
    }
    if args
        .get(1)
        .is_some_and(|arg| looks_like_crexe_file(Path::new(arg)))
    {
        args.insert(1, "exec".into());
    }
    let cli = Cli::parse_from(args);

    match &cli.command {
        Commands::Configure { import } => {
            configure::open(cli.config.as_deref(), import.as_deref())?
        }
        Commands::Exec {
            file,
            set,
            rebuild,
            cache_dir,
        } => {
            let overrides = parse_set_args(set)?;
            if let Err(error) = exec_crexe(file, overrides, *rebuild, cache_dir.as_deref(), &cli) {
                if cli.ui && !executor::cancelled() {
                    presentation::show_error(&truncate(&format!("{error:#}"), 2000));
                }
                return Err(error);
            }
        }
        Commands::Install => installation::install()?,
        Commands::Associate => installation::associate()?,
        Commands::Unassociate => installation::unassociate()?,
        Commands::Inspect { file } => inspect(file, &cli)?,
        Commands::Doctor { check } => doctor(&cli, *check)?,
        Commands::Build { project, output } => {
            project_operation("build", project, Some(output), &cli)?
        }
        Commands::Test { project } => project_operation("test", project, None, &cli)?,
        Commands::Run { project } => project_operation("run", project, None, &cli)?,
        Commands::Publish { project, output } => {
            project_operation("publish", project, Some(output), &cli)?
        }
        Commands::Export { project, output } => {
            project_operation("export", project, Some(output), &cli)?
        }
    }

    Ok(())
}

fn project_operation(operation: &str, root: &Path, output: Option<&Path>, cli: &Cli) -> Result<()> {
    let settings = config::Settings::load(
        cli.config.as_deref(),
        cli.provider.as_deref(),
        cli.model.as_deref(),
        None,
    )?;
    project::operate(operation, root, output, &settings)
}

fn doctor(cli: &Cli, check: bool) -> Result<()> {
    let settings = config::Settings::load(
        cli.config.as_deref(),
        cli.provider.as_deref(),
        cli.model.as_deref(),
        None,
    )?;
    println!(
        "CREXE {} — {} / {} (engine architecture)",
        RUNTIME_VERSION,
        env::consts::OS,
        env::consts::ARCH
    );
    let architecture = environment::architecture();
    println!(
        "Host architecture: {} ({})",
        architecture.host.as_deref().unwrap_or("unknown"),
        architecture.source
    );
    println!("Installation: {}", installation::executable()?.display());
    println!("Configuration: {}", settings.path.display());
    println!(
        "Configuration source: {}",
        if settings.from_file {
            "active file"
        } else {
            "built-in defaults (no active file yet)"
        }
    );
    println!(
        "Provider: {}; model: {}",
        settings.profile, settings.provider.model
    );
    println!("Workspace: temporary directory on this host; no process isolation");
    let profile = cli
        .profile
        .as_deref()
        .unwrap_or_else(|| profiles::default_profile());
    let spec = profiles::resolve("doctor", "SDK verification only", Some(profile))?;
    let requirements = profiles::requirements(profile)?;
    println!(
        "Intent profile: {profile}; language: {}",
        requirements.language
    );
    println!("Required SDK: {}", requirements.sdk);
    for tool in ["dotnet", "g++", "clang++", "gcc", "make", "pkg-config"] {
        match executor::resolve_tool(tool) {
            Ok(path) => println!("{tool}: {}", path.display()),
            Err(_) => println!("{tool}: not found in PATH"),
        }
    }
    if check {
        let workspace = tempfile::Builder::new().prefix("crexe-doctor-").tempdir()?;
        paths::reject_links(workspace.path())?;
        profiles::preflight(&spec, workspace.path(), &settings)
            .with_context(|| format!("SDK verification failed for {profile}"))?;
        println!("SDK verification passed for {profile}; no application was generated or built.");
    } else {
        println!(
            "SDK compatibility: not checked; use doctor --check (runs local SDK queries only)."
        );
    }
    Ok(())
}

fn inspect(file: &Path, cli: &Cli) -> Result<()> {
    let raw = format::read(file)?;
    let (format, name, spec) = match format::parse(&raw, file)? {
        format::Document::Yaml(spec) => (
            "yaml",
            get_path(&spec, &["meta", "name"])
                .and_then(Value::as_str)
                .unwrap_or("unnamed")
                .to_string(),
            spec,
        ),
        format::Document::Intent {
            name,
            prompt,
            profile,
            format,
        } => {
            let spec = profiles::resolve(
                &name,
                &prompt,
                cli.profile.as_deref().or(profile.as_deref()),
            )?;
            (format, name, spec)
        }
    };
    let settings = config::Settings::load(
        cli.config.as_deref(),
        cli.provider.as_deref(),
        cli.model.as_deref(),
        None,
    )?;
    let mut context = resolve_inputs(&spec, BTreeMap::new())?;
    let runtime = detect_runtime_vars();
    context.insert("OS".into(), runtime.os);
    context.insert("ARCH".into(), runtime.arch);
    context.insert("EXE_EXT".into(), runtime.exe_ext);
    context.insert("USER_HOME".into(), runtime.user_home);
    let target = select_target(&spec, &context)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "format": format, "name": name, "target": target,
            "engine_profile": get_path(&spec, &["engine_profile"]),
            "os": env::consts::OS, "engine_architecture": env::consts::ARCH,
            "architecture": environment::architecture(),
            "requirements": get_path(&spec, &["engine_profile"]).and_then(Value::as_str).map(profiles::requirements).transpose()?,
            "provider": settings.profile, "model": settings.provider.model,
            "build": get_build_steps(&spec, &target)?, "run": get_cmd(&spec, &["targets", &target, "run", "cmd"])?,
            "execution": "temporary host workspace (not security isolation)"
        }))?
    );
    Ok(())
}

fn looks_like_crexe_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("crexe"))
        .unwrap_or(false)
}

fn exec_crexe(
    file: &Path,
    overrides: BTreeMap<String, String>,
    rebuild: bool,
    cache_dir: Option<&Path>,
    cli: &Cli,
) -> Result<()> {
    let crexe_path = file
        .canonicalize()
        .with_context(|| format!("CREXE not found: {}", file.display()))?;
    let raw = format::read(&crexe_path)?;
    let spec = match format::parse(&raw, &crexe_path)? {
        format::Document::Yaml(spec) => {
            if get_path(&spec, &["engine_project"]).is_some()
                || get_path(&spec, &["engine_profile"]).is_some()
            {
                bail!("Reserved engine fields in YAML");
            }
            spec
        }
        format::Document::Intent {
            name,
            prompt,
            profile,
            format,
        } => {
            println!("Format: {format}; app: {name}");
            profiles::resolve(
                &name,
                &prompt,
                cli.profile.as_deref().or(profile.as_deref()),
            )?
        }
    };
    let mut settings = config::Settings::load(
        cli.config.as_deref(),
        cli.provider.as_deref(),
        cli.model.as_deref(),
        cli.env_file.as_deref(),
    )?;
    if let Some(seconds) =
        get_path(&spec, &["policies", "timeoutSeconds", "generate"]).and_then(Value::as_u64)
    {
        settings.provider.timeout_seconds = settings.provider.timeout_seconds.min(seconds);
    }
    if get_path(&spec, &["generator"]).is_some() {
        println!("Legacy generator settings are ignored; provider and credentials come from local configuration.");
    }

    let inputs = resolve_inputs(&spec, overrides)?;
    let runtime = detect_runtime_vars();

    let mut ctx = BTreeMap::new();
    for (k, v) in &inputs {
        ctx.insert(k.clone(), v.clone());
    }
    ctx.insert("OS".to_string(), runtime.os.clone());
    ctx.insert("ARCH".to_string(), runtime.arch.clone());
    ctx.insert("EXE_EXT".to_string(), runtime.exe_ext.clone());
    ctx.insert("USER_HOME".to_string(), runtime.user_home.clone());

    let target = select_target(&spec, &ctx)?;
    println!(
        "Target: {} (OS={}, ARCH={})",
        target, runtime.os, runtime.arch
    );

    let build_steps = get_build_steps(&spec, &target)?;
    let run_cmd = get_cmd(&spec, &["targets", &target, "run", "cmd"])?;
    // Catch missing template values before contacting a provider.
    let mut all_commands = build_steps.clone();
    all_commands.push(run_cmd.clone());
    for operation in ["test", "publish"] {
        if let Some(steps) =
            get_path(&spec, &["targets", &target, operation, "steps"]).and_then(Value::as_sequence)
        {
            for step in steps {
                all_commands.push(value_to_cmd(get_path(step, &["cmd"]).unwrap_or(step))?);
            }
        }
    }
    for cmd in &all_commands {
        for argument in cmd {
            check_template(&render_template(argument, &spec, &ctx))?;
        }
    }

    let mut identity = raw.as_bytes().to_vec();
    identity.extend(serde_yaml::to_string(&spec)?.as_bytes());
    identity.extend(settings.identity()?);
    let mut request_identity = identity.clone();
    request_identity.extend(serde_json::to_vec(&json!({"inputs": inputs, "rebuild": rebuild,
        "no_run": cli.no_run, "export": cli.export, "repairs": cli.max_repairs, "cache_dir": cache_dir}))?);
    let Some(mut preparation) = preparation::Session::acquire(&crexe_path, &request_identity)?
    else {
        return Ok(());
    };
    let _progress = presentation::Progress::start(cli.ui)?;
    let mut ready = || {
        presentation::finish();
        preparation.release();
    };
    let cache = get_cache_state(&crexe_path, &identity, &spec, &inputs, cache_dir)?;
    let lock = cache.lock()?;
    if !rebuild {
        if let Some(revision) = cache.current() {
            println!("Cache hit: skipping generate/build");
            drop(lock);
            package::export(&revision, cli.export.as_deref())?;
            if !cli.no_run {
                run_command(
                    &spec,
                    &run_cmd,
                    &revision,
                    "run",
                    &runtime.os,
                    &ctx,
                    &settings,
                    Some(&mut ready),
                )?;
            } else {
                ready();
            }
            return Ok(());
        }
    }
    let workspace = tempfile::Builder::new().prefix("crexe-work-").tempdir()?;
    let workspace_path = workspace.path().canonicalize()?;
    println!("Workspace: {}", workspace_path.display());
    let mut generation = generator::Session::new(&settings);
    let result = (|| -> Result<PathBuf> {
        for cmd in &build_steps {
            let command = render_template(&cmd[0], &spec, &ctx);
            executor::resolve_tool(&command)?;
            settings.execution.authorize(&command)?;
            check_allowlist(&spec, &[command], &runtime.os, "build", &workspace_path)?;
        }
        profiles::preflight(&spec, &workspace_path, &settings)?;
        let (system_prompt, user_prompt) = build_prompt(&spec, &target, &ctx)?;
        presentation::phase("Gerando seu programa…");
        let mut generated = generation.generate(&system_prompt, &user_prompt)?;
        for attempt in 0..=cli.max_repairs {
            let attempt_path = workspace_path.join(format!("attempt-{}", attempt + 1));
            fs::create_dir(&attempt_path)?;
            apply_generated_files(&attempt_path, &generated.files, &ctx)?;
            profiles::scaffold(&spec, &attempt_path)?;
            let attempt_spec = profiles::materialize(&spec, &attempt_path, &generated.files, &ctx)?;
            presentation::phase("Compilando seu programa…");
            let built = (|| -> Result<()> {
                for cmd in &get_build_steps(&attempt_spec, &target)? {
                    run_command(
                        &attempt_spec,
                        cmd,
                        &attempt_path,
                        "build",
                        &runtime.os,
                        &ctx,
                        &settings,
                        None,
                    )?;
                }
                if let Some(steps) = get_path(&spec, &["targets", &target, "test", "steps"])
                    .and_then(Value::as_sequence)
                {
                    presentation::phase("Verificando seu programa…");
                    for step in steps {
                        let cmd = value_to_cmd(get_path(step, &["cmd"]).unwrap_or(step))?;
                        run_command(
                            &spec,
                            &cmd,
                            &attempt_path,
                            "test",
                            &runtime.os,
                            &ctx,
                            &settings,
                            None,
                        )?;
                    }
                }
                let rendered_run: Vec<String> = run_cmd
                    .iter()
                    .map(|s| render_template(s, &spec, &ctx))
                    .collect();
                resolve_run_command(rendered_run, &attempt_path)?;
                Ok(())
            })();
            match built {
                Ok(()) => {
                    presentation::phase("Preparando os arquivos…");
                    package::create(
                        &attempt_spec,
                        &target,
                        &ctx,
                        &attempt_path,
                        &generated.files,
                    )?;
                    return cache.publish(&attempt_path);
                }
                Err(error) => {
                    let diagnostics = format!("{error:#}");
                    fs::write(
                        workspace_path.join(format!("attempt-{}.log", attempt + 1)),
                        &diagnostics,
                    )?;
                    if attempt == cli.max_repairs || executor::cancelled() {
                        return Err(error);
                    }
                    println!(
                        "Build/test failed; requesting correction {}/{}",
                        attempt + 1,
                        cli.max_repairs
                    );
                    let repair = format!("Original request:\n{user_prompt}\nCurrent complete source files:\n{}\nCompiler/test diagnostics:\n{}\nCorrect the actual failure. Return the FULL corrected files[] snapshot, not a patch. Preserve requested behavior and tests. Do not change the engine-owned build commands or install global tools.", serde_json::to_string(&generated)?, truncate(&diagnostics, 24000));
                    presentation::phase("Corrigindo erros de compilação…");
                    generated = generation.generate(&system_prompt, &repair)?;
                }
            }
        }
        unreachable!()
    })();
    let revision = match result {
        Ok(revision) => revision,
        Err(error) => {
            let preserved = workspace.keep();
            return Err(error).with_context(|| {
                format!(
                    "Generation failed; previous cache preserved. Diagnostic workspace: {}",
                    preserved.display()
                )
            });
        }
    };
    drop(workspace);
    drop(lock);
    println!("Validated revision: {}", revision.display());
    package::export(&revision, cli.export.as_deref())?;
    if !cli.no_run {
        run_command(
            &spec,
            &run_cmd,
            &revision,
            "run",
            &runtime.os,
            &ctx,
            &settings,
            Some(&mut ready),
        )?;
    } else {
        ready();
    }
    Ok(())
}

fn parse_set_args(items: &[String]) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for item in items {
        let Some((k, v)) = item.split_once('=') else {
            bail!("--set must be KEY=VALUE, got: {}", item);
        };
        if k.trim().is_empty() {
            bail!("--set requires a nonempty key");
        }
        if ["OS", "ARCH", "EXE_EXT", "USER_HOME"].contains(&k.trim()) {
            bail!("Cannot override runtime variable: {}", k.trim());
        }
        map.insert(k.trim().to_string(), v.to_string());
    }
    Ok(map)
}

fn validate_spec(spec: &Value) -> Result<()> {
    if !spec.is_mapping() {
        bail!("Expected a YAML recipe mapping");
    }
    let version = get_path(spec, &["version"]).context("Missing recipe version")?;
    if version.as_f64() != Some(1.0) && version.as_str() != Some("1.0") {
        bail!("Unsupported recipe version; expected 1.0");
    }
    for field in [
        "meta",
        "inputs",
        "workspace",
        "policies",
        "selectors",
        "prompt_core",
        "prompt",
        "targets",
    ] {
        if let Some(value) = get_path(spec, &[field]) {
            if !value.is_mapping() {
                bail!("Recipe field '{field}' must be a mapping");
            }
        }
    }
    if let Some(policies) = get_mapping(spec, &["policies"]) {
        for key in policies.keys() {
            if !matches!(
                key.as_str(),
                Some("allowNetwork" | "timeoutSeconds" | "commandAllowlist")
            ) {
                bail!("Unsupported recipe policy; only allowNetwork, timeoutSeconds and commandAllowlist are implemented");
            }
        }
    }
    if let Some(network) = get_path(spec, &["policies", "allowNetwork"]) {
        if !network.is_bool() {
            bail!("allowNetwork must be a boolean; string values do not enforce isolation");
        }
    }
    if let Some(timeouts) = get_path(spec, &["policies", "timeoutSeconds"]) {
        for (phase, seconds) in timeouts
            .as_mapping()
            .context("timeoutSeconds must be a mapping")?
        {
            if !matches!(
                phase.as_str(),
                Some("generate" | "build" | "test" | "publish" | "run")
            ) || seconds.as_u64().is_none_or(|n| n == 0 || n > 604800)
            {
                bail!("Timeouts require supported phase names and integer seconds between 1 and 604800");
            }
        }
    }
    if let Some(allowlist) = get_path(spec, &["policies", "commandAllowlist"]) {
        for (os, tools) in allowlist
            .as_mapping()
            .context("commandAllowlist must be a mapping")?
        {
            if os.as_str().is_none() {
                bail!("commandAllowlist OS keys must be strings");
            }
            let tools = tools
                .as_sequence()
                .context("commandAllowlist entries must be lists")?;
            if tools.len() > 64
                || tools
                    .iter()
                    .any(|tool| tool.as_str().is_none_or(str::is_empty))
            {
                bail!("commandAllowlist entries must contain at most 64 nonempty tool names");
            }
        }
    }
    if let Some(targets) = get_mapping(spec, &["targets"]) {
        for (name, target) in targets {
            let name = name.as_str().context("Target names must be strings")?;
            if !target.is_mapping() {
                bail!("Target '{name}' must be a mapping");
            }
            for operation in ["build", "test", "publish"] {
                if let Some(value) = get_path(target, &[operation]) {
                    let steps = get_path(value, &["steps"])
                        .and_then(Value::as_sequence)
                        .context("Build/test/publish require a steps list")?;
                    if steps.is_empty() || steps.len() > 32 {
                        bail!("Operations require between 1 and 32 steps");
                    }
                    for step in steps {
                        value_to_cmd(get_path(step, &["cmd"]).unwrap_or(step))?;
                    }
                }
            }
            if let Some(run) = get_path(target, &["run"]) {
                value_to_cmd(get_path(run, &["cmd"]).context("run requires cmd")?)?;
            }
        }
    }
    if get_path(spec, &["policies", "allowNetwork"]).and_then(Value::as_bool) == Some(false) {
        bail!(
            "allowNetwork: false requires network isolation, unavailable in the local-host engine"
        );
    }
    for (name, expected) in [
        ("manifest", "crexe.manifest.json"),
        ("build_ok", ".build_ok"),
    ] {
        if let Some(value) = get_path(spec, &["workspace", "cache", "markers", name]) {
            if value.as_str() != Some(expected) {
                bail!("Custom cache markers are not supported: {name}");
            }
        }
    }
    if let Some(layout) = get_path(spec, &["workspace", "cache", "layout"]) {
        if layout.as_str() != Some("{{fingerprint}}") {
            bail!("Custom cache layout is not supported");
        }
    }
    Ok(())
}

fn detect_runtime_vars() -> RuntimeVars {
    let os = match env::consts::OS {
        "windows" => "windows",
        "macos" => "macos",
        "linux" => "linux",
        other => other,
    }
    .to_string();

    let exe_ext = if os == "windows" { ".exe" } else { "" }.to_string();
    let user_home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());

    RuntimeVars {
        os,
        arch: env::consts::ARCH.to_string(),
        exe_ext,
        user_home,
    }
}

fn resolve_inputs(
    spec: &Value,
    overrides: BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    if let Some(inputs) = get_mapping(spec, &["inputs"]) {
        for (key, definition) in inputs {
            let key = key.as_str().context("Input keys must be strings")?;
            if ["OS", "ARCH", "EXE_EXT", "USER_HOME"].contains(&key) {
                bail!("Input cannot override runtime fact {key}");
            }
            let kind = get_path(definition, &["type"])
                .and_then(Value::as_str)
                .unwrap_or("string");
            let default = get_path(definition, &["default"]).unwrap_or(&Value::Null);
            let default = match default {
                Value::String(value) => value.clone(),
                Value::Number(value) => value.to_string(),
                Value::Bool(value) => value.to_string(),
                Value::Null if kind == "string" => String::new(),
                _ => bail!("Invalid default value for input {key}"),
            };
            let value = overrides.get(key).cloned().unwrap_or(default);
            let valid = match kind {
                "string" => true,
                "integer" => value.parse::<i64>().is_ok(),
                "number" => value.parse::<f64>().is_ok_and(f64::is_finite),
                "boolean" => matches!(value.as_str(), "true" | "false"),
                _ => bail!("Unsupported input type for {key}: {kind}"),
            };
            if !valid {
                bail!("Invalid {kind} input: {key}");
            }
            out.insert(key.to_string(), value);
        }
    }
    if let Some(unknown) = overrides.keys().find(|key| !out.contains_key(*key)) {
        bail!("Unknown input override: {unknown}");
    }
    Ok(out)
}

fn select_target(spec: &Value, ctx: &BTreeMap<String, String>) -> Result<String> {
    let fallback = get_path(spec, &["selectors", "target", "fallback"])
        .and_then(Value::as_str)
        .unwrap_or(env::consts::OS)
        .to_string();

    let Some(rules) =
        get_path(spec, &["selectors", "target", "rules"]).and_then(Value::as_sequence)
    else {
        return Ok(fallback);
    };

    for rule in rules {
        let when = get_path(rule, &["when"])
            .and_then(Value::as_str)
            .unwrap_or("");
        let use_target = get_path(rule, &["use"]).and_then(Value::as_str);
        if let Some(use_target) = use_target {
            if eval_condition(when, ctx)? {
                return Ok(use_target.to_string());
            }
        }
    }

    Ok(fallback)
}

fn eval_condition(expr: &str, ctx: &BTreeMap<String, String>) -> Result<bool> {
    // Supports only: {{KEY}} == 'value' and {{KEY}} != 'value'
    let re =
        Regex::new(r#"^\s*\{\{([A-Za-z_][A-Za-z0-9_]*)\}\}\s*(==|!=)\s*'([^']*)'\s*$"#).unwrap();
    let Some(caps) = re.captures(expr) else {
        bail!("Invalid target selector: {expr}");
    };
    let key = caps.get(1).unwrap().as_str();
    let op = caps.get(2).unwrap().as_str();
    let expected = caps.get(3).unwrap().as_str();
    let actual = ctx
        .get(key)
        .context("Unknown variable in target selector")?
        .as_str();
    Ok(match op {
        "==" => actual == expected,
        "!=" => actual != expected,
        _ => false,
    })
}

fn build_prompt(
    spec: &Value,
    target: &str,
    ctx: &BTreeMap<String, String>,
) -> Result<(String, String)> {
    let system_raw = get_path(spec, &["prompt", "system"])
        .and_then(Value::as_str)
        .unwrap_or("");

    let user_raw = get_path(spec, &["prompt", "user_by_target", target])
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Missing prompt.user_by_target.{}", target))?;

    let system = render_template(system_raw, spec, ctx);
    let user = render_template(&render_template(user_raw, spec, ctx), spec, ctx);
    check_template(&system)?;
    check_template(&user)?;
    Ok((system, user))
}

fn check_template(rendered: &str) -> Result<()> {
    if Regex::new(r"\{\{[A-Za-z_][A-Za-z0-9_.]*\}\}")?.is_match(rendered) {
        bail!("Unresolved template variable; check inputs and recipe field names");
    }
    Ok(())
}

fn render_template(text: &str, spec: &Value, ctx: &BTreeMap<String, String>) -> String {
    let re = Regex::new(r#"\{\{([A-Za-z_][A-Za-z0-9_\.]*?)\}\}"#).unwrap();
    re.replace_all(text, |caps: &regex::Captures| {
        let key = caps.get(1).unwrap().as_str();
        if let Some(v) = ctx.get(key) {
            return v.clone();
        }
        if key.contains('.') {
            let parts: Vec<&str> = key.split('.').collect();
            if let Some(v) = get_path(spec, &parts).and_then(Value::as_str) {
                return v.to_string();
            }
        }
        caps.get(0).unwrap().as_str().to_string()
    })
    .to_string()
}

fn extract_json(text: &str) -> String {
    let trimmed = text.trim();
    let re = Regex::new(r#"(?is)```(?:json)?\s*(.*?)\s*```"#).unwrap();
    if let Some(caps) = re.captures(trimmed) {
        return caps.get(1).unwrap().as_str().trim().to_string();
    }
    trimmed.to_string()
}

fn apply_generated_files(
    workspace: &Path,
    files: &[GeneratedFile],
    ctx: &BTreeMap<String, String>,
) -> Result<()> {
    if files.is_empty() || files.len() > 128 {
        bail!("Expected between 1 and 128 generated files");
    }
    let mut destinations = std::collections::BTreeSet::new();
    let mut validated = Vec::new();
    let mut total = 0usize;
    // Validate the whole response before creating anything, including the final file.
    for file in files {
        let lower = file.content.to_ascii_lowercase();
        if lower.contains("windowssandbox")
            || lower.contains("containers-disposableclientvm")
            || file.path.to_ascii_lowercase().ends_with(".wsb")
        {
            bail!("Windows Sandbox is prohibited; generated content was rejected");
        }
        let rendered = render_template(&file.path, &Value::Null, ctx);
        if project::reserved(&rendered.replace('\\', "/")) {
            bail!("Generated file targets engine-owned project metadata/scripts: {rendered}");
        }
        let relative = paths::relative_file(&rendered)?;
        let key = relative.to_string_lossy().to_lowercase();
        if !destinations.insert(key) {
            bail!("Duplicate generated path: {rendered}");
        }
        total = total
            .checked_add(file.content.len())
            .context("Generated output too large")?;
        if file.content.len() > 4 * 1024 * 1024 || total > 16 * 1024 * 1024 {
            bail!("Generated output too large");
        }
        let destination = workspace.join(&relative);
        paths::reject_links(&destination)?;
        if destination.is_dir() {
            bail!("Generated file targets a directory: {rendered}");
        }
        validated.push((relative, file));
    }
    for (relative, _) in &validated {
        if relative
            .ancestors()
            .skip(1)
            .any(|parent| destinations.contains(&parent.to_string_lossy().to_lowercase()))
        {
            bail!(
                "Generated file is also used as a directory: {}",
                relative.display()
            );
        }
    }
    fs::create_dir_all(workspace)?;
    for (relative, file) in validated {
        let out_path = workspace.join(relative);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        paths::reject_links(&out_path)?;
        fs::write(&out_path, &file.content)?;
        println!(
            "Wrote: {}",
            out_path
                .strip_prefix(workspace)
                .unwrap_or(&out_path)
                .display()
        );
    }

    Ok(())
}

fn get_build_steps(spec: &Value, target: &str) -> Result<Vec<Vec<String>>> {
    let seq = get_path(spec, &["targets", target, "build", "steps"])
        .and_then(Value::as_sequence)
        .ok_or_else(|| anyhow!("Missing targets.{}.build.steps", target))?;

    let mut out = Vec::new();
    if seq.is_empty() {
        bail!("Build must contain at least one step");
    }
    for step in seq {
        let cmd_val = if let Some(v) = get_path(step, &["cmd"]) {
            v
        } else {
            step
        };
        out.push(value_to_cmd(cmd_val)?);
    }
    Ok(out)
}

fn get_cmd(spec: &Value, path: &[&str]) -> Result<Vec<String>> {
    let v = get_path(spec, path).ok_or_else(|| anyhow!("Missing command at {}", path.join(".")))?;
    value_to_cmd(v)
}

fn value_to_cmd(v: &Value) -> Result<Vec<String>> {
    let seq = v.as_sequence().ok_or_else(|| {
        anyhow!("Command must be an argument list, e.g. [\"tool\", \"argument with spaces\"]")
    })?;
    if seq.is_empty() {
        bail!("Empty command");
    }
    let mut out = Vec::new();
    for item in seq {
        let s = item
            .as_str()
            .ok_or_else(|| anyhow!("Command list items must be strings"))?;
        out.push(s.to_string());
    }
    if out[0].is_empty() {
        bail!("Empty executable name");
    }
    Ok(out)
}

fn is_inside(root: &Path, child: &Path) -> bool {
    let Ok(root) = fs::canonicalize(root) else {
        return false;
    };

    let Ok(child) = fs::canonicalize(child) else {
        return false;
    };

    child.starts_with(root)
}

fn normalize_generated_command_path(cmd0: &str, cwd: &Path) -> PathBuf {
    let p = Path::new(cmd0);

    if p.is_absolute() {
        return p.to_path_buf();
    }

    cwd.join(p)
}

fn is_generated_run_target(cwd: &Path, cmd0: &str) -> bool {
    let candidate = normalize_generated_command_path(cmd0, cwd);

    candidate.is_file() && is_inside(cwd, &candidate) && paths::reject_links(&candidate).is_ok()
}

#[allow(clippy::too_many_arguments)]
fn run_command(
    spec: &Value,
    cmd_raw: &[String],
    cwd: &Path,
    phase: &str,
    os_name: &str,
    ctx: &BTreeMap<String, String>,
    settings: &config::Settings,
    ready: Option<&mut dyn FnMut()>,
) -> Result<()> {
    let mut cmd: Vec<String> = cmd_raw
        .iter()
        .map(|s| render_template(s, spec, ctx))
        .collect();

    if cmd.is_empty() {
        bail!("Empty command");
    }

    if phase == "run" || (phase == "test" && is_generated_run_target(cwd, &cmd[0])) {
        cmd = resolve_run_command(cmd, cwd)?;
    }

    if !(matches!(phase, "run" | "test")
        && is_generated_run_target(cwd, &cmd[0])
        && !policy::requires_shell(&cmd[0]))
    {
        settings.execution.authorize(&cmd[0])?;
    }
    check_allowlist(spec, &cmd, os_name, phase, cwd)?;

    let printable = cmd.join(" ");
    println!("$ (cwd={}) {}", cwd.display(), printable);

    let timeout = settings.execution.timeout(
        phase,
        get_path(spec, &["policies", "timeoutSeconds", phase]).and_then(Value::as_u64),
    );
    if let Some(ready) = ready {
        executor::launch(&cmd, cwd, timeout, &settings.secret_names, ready)
    } else {
        executor::run(&cmd, cwd, timeout, phase != "run", &settings.secret_names)
    }
}

fn resolve_run_command(mut cmd: Vec<String>, cwd: &Path) -> Result<Vec<String>> {
    if cmd.is_empty() {
        bail!("Empty command");
    }

    let first = Path::new(&cmd[0]);
    let first_name = first
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    // Do not rewrite shell commands.
    if ["cmd", "powershell", "sh", "bash"].contains(&first_name.as_str()) {
        return Ok(cmd);
    }

    let expected = normalize_generated_command_path(&cmd[0], cwd);

    if expected.is_file() {
        if !is_inside(cwd, &expected) {
            bail!(
                "Run target is outside the workspace: {}",
                expected.display()
            );
        }
        paths::reject_links(&expected)?;
        cmd[0] = expected.canonicalize()?.to_string_lossy().into_owned();
        return Ok(cmd);
    }
    bail!(
        "Expected run artifact is missing: {}. Build the project first, or use exec --rebuild with the recipe.",
        expected.display()
    )
}

fn check_allowlist(
    spec: &Value,
    cmd: &[String],
    os_name: &str,
    phase: &str,
    cwd: &Path,
) -> Result<()> {
    if matches!(phase, "run" | "test") && is_generated_run_target(cwd, &cmd[0]) {
        return Ok(());
    }
    let Some(list) =
        get_path(spec, &["policies", "commandAllowlist", os_name]).and_then(Value::as_sequence)
    else {
        return Ok(());
    };

    let allowed: Vec<String> = list
        .iter()
        .filter_map(Value::as_str)
        .map(|s| s.to_ascii_lowercase())
        .collect();

    if allowed.is_empty() {
        bail!("Command not allowed by recipe policy: empty allowlist for {os_name}");
    }

    let exe = Path::new(&cmd[0])
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&cmd[0])
        .to_ascii_lowercase();

    if !allowed.iter().any(|a| a == &exe) {
        bail!("Command not allowed by policy: {}", cmd[0]);
    }

    Ok(())
}

fn get_cache_state(
    crexe_path: &Path,
    crexe_bytes: &[u8],
    spec: &Value,
    inputs: &BTreeMap<String, String>,
    cache_dir: Option<&Path>,
) -> Result<cache::Entry> {
    let fp = compute_fingerprint(crexe_bytes, spec, inputs)?;

    let root = match cache_dir {
        Some(path) => path.to_path_buf(),
        None => default_cache_root()?,
    };
    let root = std::path::absolute(root)?;
    paths::reject_links(&root)?;
    let workspace = root.join(&fp);

    let _ = crexe_path;

    Ok(cache::Entry {
        root: workspace,
        fingerprint: fp,
    })
}

fn compute_fingerprint(
    crexe_bytes: &[u8],
    spec: &Value,
    inputs: &BTreeMap<String, String>,
) -> Result<String> {
    let mut crexe_hasher = Sha256::new();
    crexe_hasher.update(crexe_bytes);
    let crexe_sha = format!("{:x}", crexe_hasher.finalize());

    let generator = get_path(spec, &["generator"]).unwrap_or(&Value::Null);
    let payload = json!({
        "runtimeVersion": RUNTIME_VERSION,
        "cacheSchema": 3,
        "os": env::consts::OS,
        "arch": env::consts::ARCH,
        "crexeSha256": crexe_sha,
        "inputs": inputs,
        "generator": {
            "provider": get_path(generator, &["provider"]).and_then(Value::as_str),
            "baseUrl": get_path(generator, &["baseUrl"]).and_then(Value::as_str),
            "model": get_path(generator, &["model"]).and_then(Value::as_str),
            "temperature": get_path(generator, &["temperature"]).and_then(Value::as_f64),
            "maxOutputTokens": get_path(generator, &["maxOutputTokens"]).and_then(Value::as_i64),
        }
    });

    let bytes = serde_json::to_vec(&payload)?;
    let mut h = Sha256::new();
    h.update(bytes);
    Ok(format!("{:x}", h.finalize()))
}

fn default_cache_root() -> Result<PathBuf> {
    if env::var_os("CREXE_HOME").is_some() {
        return Ok(installation::root()?.join("cache"));
    }
    Ok(match env::consts::OS {
        "windows" => env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .context("LOCALAPPDATA is not set")?
            .join("CREXE")
            .join("cache"),
        "macos" => PathBuf::from(env::var("HOME").context("HOME is not set")?)
            .join("Library")
            .join("Caches")
            .join("CREXE"),
        _ => env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|_| env::var("HOME").map(|home| PathBuf::from(home).join(".cache")))
            .context("Neither XDG_CACHE_HOME nor HOME is set")?
            .join("crexe"),
    })
}

fn get_path<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cur = v;
    for p in path {
        let key = Value::String((*p).to_string());
        cur = cur.as_mapping()?.get(&key)?;
    }
    Some(cur)
}

fn get_mapping<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Mapping> {
    get_path(v, path)?.as_mapping()
}

fn truncate(s: &str, max: usize) -> String {
    let prefix: String = s.chars().take(max).collect();
    if prefix.len() < s.len() {
        format!("{prefix}...")
    } else {
        prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, content: &str) -> GeneratedFile {
        GeneratedFile {
            path: path.into(),
            content: content.into(),
        }
    }

    #[test]
    fn original_yaml_still_parses_and_resolves_all_targets() {
        let spec: Value =
            serde_yaml::from_str(include_str!("../tests/fixtures/calculator_baseline.crexe"))
                .unwrap();
        validate_spec(&spec).unwrap();
        for os in ["windows", "linux", "macos"] {
            let mut ctx = resolve_inputs(&spec, BTreeMap::new()).unwrap();
            ctx.insert("OS".into(), os.into());
            assert_eq!(select_target(&spec, &ctx).unwrap(), os);
            assert!(!get_build_steps(&spec, os).unwrap().is_empty());
        }
    }

    #[test]
    fn invalid_later_path_does_not_write_earlier_file() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("work");
        assert!(apply_generated_files(
            &workspace,
            &[file("valid.txt", "hello"), file("../escape", "bad")],
            &BTreeMap::new()
        )
        .is_err());
        assert!(!workspace.exists());
        assert!(!temp.path().join("escape").exists());
    }

    #[test]
    fn duplicate_and_file_directory_collisions_are_rejected_before_write() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("work");
        for files in [
            vec![file("src/A", "a"), file("src/a", "b")],
            vec![file("src", "a"), file("src/a", "b")],
        ] {
            assert!(apply_generated_files(&workspace, &files, &BTreeMap::new()).is_err());
            assert!(!workspace.exists());
        }
    }

    #[test]
    fn writes_multiple_files_with_unicode_paths() {
        let temp = tempfile::tempdir().unwrap();
        apply_generated_files(
            temp.path(),
            &[file("src/ação.rs", "one"), file("README.md", "two")],
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(temp.path().join("src/ação.rs")).unwrap(),
            "one"
        );
    }

    #[test]
    fn run_path_is_absolute_and_no_unrelated_fallback_is_used() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("build")).unwrap();
        fs::write(temp.path().join("build/app.exe"), "fixture").unwrap();
        let cmd =
            resolve_run_command(vec!["build/app.exe".into(), "a b".into()], temp.path()).unwrap();
        assert!(Path::new(&cmd[0]).is_absolute());
        assert_eq!(cmd[1], "a b");
        assert!(resolve_run_command(vec!["build/missing.exe".into()], temp.path()).is_err());
    }

    #[test]
    fn run_cannot_escape_via_build_prefix() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("work");
        fs::create_dir_all(workspace.join("build")).unwrap();
        fs::write(temp.path().join("outside.exe"), "fixture").unwrap();
        assert!(!is_generated_run_target(
            &workspace,
            "build/../../outside.exe"
        ));
        assert!(resolve_run_command(vec!["build/../../outside.exe".into()], &workspace).is_err());
    }

    #[test]
    fn diagnostics_preserve_utf8_boundaries() {
        assert_eq!(truncate("ação 🦀", 6), "ação 🦀");
        assert_eq!(truncate("ação 🦀", 2), "aç...");
    }

    #[test]
    fn invalid_schema_and_security_claims_are_rejected() {
        for text in [
            "hello",
            "{}",
            "version: 2",
            "version: 1.0\npolicies:\n  allowNetwork: false",
            "version: 1.0\npolicies:\n  allowNetwork: 'false'",
            "version: 1.0\npolicies:\n  isolation: true",
            "version: 1.0\npolicies:\n  timeoutSeconds: {build: nope}",
            "version: 1.0\npolicies:\n  commandAllowlist: {windows: dotnet}",
            "version: 1.0\ntargets:\n  native:\n    test: {steps: nope}",
            "version: 1.0\nworkspace:\n  cache:\n    layout: '../outside'",
        ] {
            assert!(validate_spec(&serde_yaml::from_str::<Value>(text).unwrap()).is_err());
        }
        assert!(eval_condition("invalid", &BTreeMap::new()).is_err());
        assert!(value_to_cmd(&Value::String("tool argument".into())).is_err());
        assert!(value_to_cmd(&Value::Sequence(Vec::new())).is_err());
    }

    #[test]
    fn typed_inputs_and_unresolved_templates_fail_before_generation() {
        let spec: Value = serde_yaml::from_str("version: 1.0\ninputs:\n  count:\n    type: integer\n    default: 2\n  flag:\n    type: boolean\n    default: true").unwrap();
        let inputs = resolve_inputs(&spec, BTreeMap::new()).unwrap();
        assert_eq!(inputs["count"], "2");
        assert_eq!(inputs["flag"], "true");
        assert!(resolve_inputs(&spec, BTreeMap::from([("count".into(), "two".into())])).is_err());
        assert!(resolve_inputs(&spec, BTreeMap::from([("unknown".into(), "x".into())])).is_err());
        assert!(check_template("src/{{MissingName}}.cs").is_err());
    }

    #[test]
    fn cache_root_is_controlled_by_cli_and_not_recipe() {
        let temp = tempfile::tempdir().unwrap();
        let spec: Value = serde_yaml::from_str("version: 1.0\nworkspace:\n  cache:\n    root:\n      windows: C:/unexpected\n      linux: /unexpected").unwrap();
        let cache = get_cache_state(
            Path::new("a.crexe"),
            b"fixture",
            &spec,
            &BTreeMap::new(),
            Some(temp.path()),
        )
        .unwrap();
        assert!(cache.root.starts_with(temp.path()));
    }

    #[cfg(unix)]
    #[test]
    fn generated_files_reject_final_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("work");
        fs::create_dir(&workspace).unwrap();
        let sentinel = temp.path().join("sentinel");
        fs::write(&sentinel, "unchanged").unwrap();
        std::os::unix::fs::symlink(&sentinel, workspace.join("file")).unwrap();
        assert!(
            apply_generated_files(&workspace, &[file("file", "changed")], &BTreeMap::new())
                .is_err()
        );
        assert_eq!(fs::read_to_string(sentinel).unwrap(), "unchanged");
    }
}
