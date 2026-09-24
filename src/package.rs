use super::{paths, project::Project, GeneratedFile};
use anyhow::{bail, Context, Result};
use serde_yaml::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::Path,
};

const ARCHIVE: &str = "source.zip";

pub(crate) fn create(
    spec: &Value,
    target: &str,
    ctx: &BTreeMap<String, String>,
    root: &Path,
    files: &[GeneratedFile],
) -> Result<()> {
    let plan = Project::from_spec(spec, target, ctx, files)?;
    bundle(&plan, root)
}

pub(crate) fn bundle(plan: &Project, root: &Path) -> Result<()> {
    let mut sources: BTreeSet<_> = plan
        .sources
        .iter()
        .map(|source| paths::relative_file(source))
        .collect::<Result<_>>()?;
    paths::reject_links(&root.join(super::project::MANIFEST))?;
    plan.write(root)?;
    sources.insert(super::project::MANIFEST.into());
    let windows = plan.os == "windows";
    for (name, steps) in &plan.commands {
        let filename = format!("{name}.{}", if windows { "bat" } else { "sh" });
        let mut script = if windows {
            "@echo off\r\nsetlocal DisableDelayedExpansion\r\ncd /d \"%~dp0\"\r\n".to_owned()
        } else {
            "#!/bin/sh\nset -eu\ncd -- \"$(dirname -- \"$0\")\"\n".to_owned()
        };
        for args in steps {
            let quoted: Result<Vec<_>> = args.iter().map(|arg| quote(arg, windows)).collect();
            if windows {
                script.push_str("call ");
            }
            script.push_str(&quoted?.join(" "));
            script.push_str(if windows {
                "\r\nif errorlevel 1 exit /b %errorlevel%\r\n"
            } else {
                "\n"
            });
        }
        paths::reject_links(&root.join(&filename))?;
        fs::write(root.join(&filename), script)?;
        sources.insert(filename.into());
    }
    let instructions = format!("# {}\n\nGenerated with Creative Executable. Target: {} / {}; profile: {}.\n\nThis ZIP contains sources and scripts, not installed compilers or provider credentials.\n\n1. Install the toolchain indicated by the project/build script (.NET 8 SDK for dotnet profiles).\n2. Run build.{} from this directory.\n3. Run run.{} to open the app.\n\nAvailable operations: {}. They are recorded in CREXE-PROJECT.json; scripts and CREXE use the same command lists. The .NET publish is framework-dependent and does not bundle the runtime.\n\nGenerated tests are not independent assurance of correctness. Review the code. Builds run on the local host; the working directory is not security isolation.\n", plan.name, plan.os, plan.arch, plan.profile, if windows { "bat" } else { "sh" }, if windows { "bat" } else { "sh" }, plan.commands.keys().cloned().collect::<Vec<_>>().join(", "));
    paths::reject_links(&root.join("CREXE-README.md"))?;
    fs::write(root.join("CREXE-README.md"), instructions)?;
    sources.insert("CREXE-README.md".into());
    paths::reject_links(&root.join(ARCHIVE))?;
    let archive = fs::File::create(root.join(ARCHIVE))?;
    let mut zip = zip::ZipWriter::new(archive);
    for relative in sources {
        let file = root.join(&relative);
        paths::reject_links(&file)?;
        let mode = if relative.extension().is_some_and(|ext| ext == "sh") {
            0o755
        } else {
            0o644
        };
        zip.start_file(
            relative.to_string_lossy().replace('\\', "/"),
            zip::write::SimpleFileOptions::default().unix_permissions(mode),
        )?;
        std::io::copy(&mut fs::File::open(file)?, &mut zip)?;
    }
    zip.finish()?.sync_all()?;
    Ok(())
}

fn quote(arg: &str, windows: bool) -> Result<String> {
    if arg.chars().any(|c| c == '\n' || c == '\r' || c == '\0') {
        bail!("Multiline command argument cannot be exported as a script");
    }
    if windows {
        if arg.contains('"') || arg.contains('%') {
            bail!("Command argument containing quotes or percent cannot be safely exported to BAT; use a project script");
        }
        Ok(format!("\"{arg}\""))
    } else {
        Ok(format!("'{}'", arg.replace('\'', "'\"'\"'")))
    }
}

pub(crate) fn export(revision: &Path, destination: Option<&Path>) -> Result<()> {
    let Some(destination) = destination else {
        return Ok(());
    };
    let destination = std::path::absolute(destination)?;
    paths::reject_links(&destination)?;
    let parent = destination.parent().context("Invalid export path")?;
    fs::create_dir_all(parent)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    std::io::copy(
        &mut fs::File::open(revision.join(ARCHIVE))
            .context("No source archive in this revision; rebuild it")?,
        staged.as_file_mut(),
    )?;
    staged.flush()?;
    staged
        .persist_noclobber(&destination)
        .map_err(|error| error.error)
        .context("Export destination already exists or is not writable; select a new filename")?;
    println!("Source ZIP: {}", destination.display());
    Ok(())
}
