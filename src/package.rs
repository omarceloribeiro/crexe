use super::{paths, project::Project, GeneratedFile};
use anyhow::{bail, Context, Result};
use serde_yaml::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::Path,
};

const ARCHIVE: &str = "source.zip";
const OUTPUT_LICENSE_FILE: &str = "CREXE-OUTPUT-LICENSE.txt";
const OUTPUT_LICENSE: &str = include_str!("../LICENSE-GENERATED");
// Exact historical MIT-0 policy from ba2e508, before the engine's MIT migration.
// Its references to the former GPL engine license must not block old exports.
const HISTORICAL_OUTPUT_LICENSE_SHA256: &str =
    "fbca894ec6e0602c502ae31b69a5c3ad8f08795debaf6fee7417d78fc9985732";
const HISTORICAL_OUTPUT_LICENSE_LF_BYTES: usize = 5191;

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
    if sources.iter().any(|source| {
        source
            .to_string_lossy()
            .eq_ignore_ascii_case(OUTPUT_LICENSE_FILE)
    }) {
        bail!("{OUTPUT_LICENSE_FILE} is reserved for the CREXE output-materials policy");
    }
    write_output_license(root)?;
    sources.insert(OUTPUT_LICENSE_FILE.into());
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
            let quoted: Result<Vec<_>> = args
                .iter()
                .enumerate()
                .map(|(index, arg)| {
                    if !windows
                        && index == 0
                        && arg == &plan.commands["run"][0][0]
                        && !Path::new(arg).is_absolute()
                    {
                        quote(&format!("./{arg}"), false)
                    } else {
                        quote(arg, windows)
                    }
                })
                .collect();
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
    let instructions = format!("# {}\n\nGenerated with Creative Executable. Target: {} / {}; profile: {}.\n\nThis ZIP contains sources and scripts, not installed compilers or provider credentials.\n\n1. Install the toolchain indicated by the project/build script (.NET 8 SDK for dotnet profiles).\n2. Run build.{} from this directory.\n3. Run run.{} to open the app.\n\nAvailable operations: {}. They are recorded in CREXE-PROJECT.json; scripts and CREXE use the same command lists. The .NET publish is framework-dependent and does not bundle the runtime.\n\n## Licensing\n\nYou choose your application's license, including a proprietary or commercial license. CREXE does not require you to disclose application source code or credit CREXE. CREXE-supplied output materials are covered by the scoped MIT-0 permission in CREXE-OUTPUT-LICENSE.txt; that permission does not assign a license to your whole application. MIT-0 does not require retaining this notice or attributing CREXE. The engine implementation is separately licensed under MIT; its notice conditions do not transfer to the designated MIT-0 output materials. Recipes, dependencies, assets and provider terms may have their own conditions. Preserve and review any applicable third-party notices. CREXE does not create or replace your application's LICENSE file.\n\nGenerated tests are not independent assurance of correctness. Review the code. Builds run on the local host; the working directory is not security isolation.\n", plan.name, plan.os, plan.arch, plan.profile, if windows { "bat" } else { "sh" }, if windows { "bat" } else { "sh" }, plan.commands.keys().cloned().collect::<Vec<_>>().join(", "));
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

fn write_output_license(root: &Path) -> Result<()> {
    let notice = root.join(OUTPUT_LICENSE_FILE);
    paths::reject_links(&notice)?;
    if notice.try_exists()?
        && (fs::metadata(&notice)?.len() > output_license_size_limit()
            || !recognized_output_license(&fs::read(&notice)?))
    {
        bail!("{OUTPUT_LICENSE_FILE} already contains different material; preserve or rename it before exporting");
    }
    fs::write(notice, OUTPUT_LICENSE)?;
    Ok(())
}

fn output_license_size_limit() -> u64 {
    (OUTPUT_LICENSE.len().max(HISTORICAL_OUTPUT_LICENSE_LF_BYTES) * 2) as u64
}

fn recognized_output_license(content: &[u8]) -> bool {
    // Git checkouts can use CRLF; exporting on another OS still recognizes the
    // same policy and writes the current build's exact text into the new ZIP.
    let Ok(text) = std::str::from_utf8(content) else {
        return false;
    };
    let normalized = text.replace("\r\n", "\n");
    normalized == OUTPUT_LICENSE.replace("\r\n", "\n")
        || format!("{:x}", Sha256::digest(normalized.as_bytes()))
            == HISTORICAL_OUTPUT_LICENSE_SHA256
}

// Cached revisions can predate the output policy. Upgrade only the requested
// ZIP, streaming existing entries without extracting or changing the cache.
fn export_with_output_license(source: &Path, output: &mut fs::File) -> Result<()> {
    paths::reject_links(source)?;
    let mut archive = zip::ZipArchive::new(
        fs::File::open(source).context("No source archive in this revision; rebuild it")?,
    )?;
    let mut zip = zip::ZipWriter::new(output);
    let mut names = BTreeSet::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let relative = paths::relative_file(entry.name().trim_end_matches('/'))?;
        let normalized = relative.to_string_lossy().replace('\\', "/");
        if normalized.contains('/')
            && normalized
                .split('/')
                .next()
                .is_some_and(|part| part.eq_ignore_ascii_case(OUTPUT_LICENSE_FILE))
        {
            bail!("{OUTPUT_LICENSE_FILE} is a reserved file, not a source directory");
        }
        if !names.insert(normalized.to_lowercase()) {
            bail!("Duplicate source archive path: {}", entry.name());
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!(
                "Links are not accepted in source archives: {}",
                entry.name()
            );
        }
        if normalized.eq_ignore_ascii_case(OUTPUT_LICENSE_FILE) {
            if entry.is_dir() || entry.size() > output_license_size_limit() {
                bail!("{OUTPUT_LICENSE_FILE} already contains different material; the cached archive was preserved");
            }
            let mut existing = Vec::new();
            entry.read_to_end(&mut existing)?;
            if !recognized_output_license(&existing) {
                bail!("{OUTPUT_LICENSE_FILE} already contains different material; the cached archive was preserved");
            }
        } else {
            zip.raw_copy_file(entry)?;
        }
    }
    zip.start_file(
        OUTPUT_LICENSE_FILE,
        zip::write::SimpleFileOptions::default().unix_permissions(0o644),
    )?;
    zip.write_all(OUTPUT_LICENSE.as_bytes())?;
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
    if destination.try_exists()? {
        bail!("Export destination already exists; select a new filename");
    }
    let parent = destination.parent().context("Invalid export path")?;
    fs::create_dir_all(parent)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    export_with_output_license(&revision.join(ARCHIVE), staged.as_file_mut())?;
    staged.flush()?;
    staged
        .persist_noclobber(&destination)
        .map_err(|error| error.error)
        .context("Export destination already exists or is not writable; select a new filename")?;
    println!("Source ZIP: {}", destination.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Historical document copied byte-for-byte from ba2e508:LICENSE-GENERATED.
    // Its GPL references describe that old engine version, not current licensing.
    const HISTORICAL_OUTPUT_POLICY: &str =
        include_str!("../tests/fixtures/output-license-ba2e508.txt");

    fn historical_policy_variants() -> [String; 2] {
        let lf = HISTORICAL_OUTPUT_POLICY.replace("\r\n", "\n");
        assert_eq!(lf.len(), HISTORICAL_OUTPUT_LICENSE_LF_BYTES);
        assert_eq!(
            format!("{:x}", Sha256::digest(lf.as_bytes())),
            HISTORICAL_OUTPUT_LICENSE_SHA256
        );
        assert_ne!(lf, OUTPUT_LICENSE.replace("\r\n", "\n"));
        [lf.clone(), lf.replace('\n', "\r\n")]
    }

    fn project(root: &Path, user_license: Option<&[u8]>) -> Project {
        fs::write(root.join("Program.cs"), "// user's application source\n").unwrap();
        let mut sources = vec!["Program.cs"];
        if let Some(license) = user_license {
            fs::write(root.join("LICENSE"), license).unwrap();
            sources.push("LICENSE");
        }
        fs::write(
            root.join(super::super::project::MANIFEST),
            serde_json::to_vec(&serde_json::json!({
                "schema": 1, "name": "License fixture", "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH, "profile": "legacy", "sources": sources,
                "commands": {"build": [["unused-compiler"]], "run": [["unused-app"]]}
            }))
            .unwrap(),
        )
        .unwrap();
        Project::load(root).unwrap()
    }

    fn archive_files(path: &Path) -> BTreeMap<String, Vec<u8>> {
        let mut zip = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
        let mut files = BTreeMap::new();
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).unwrap();
            let name = entry.name().to_owned();
            let mut content = Vec::new();
            entry.read_to_end(&mut content).unwrap();
            assert!(files.insert(name, content).is_none());
        }
        files
    }

    fn legacy_archive(root: &Path, files: &[(&str, &[u8])]) {
        let mut zip = zip::ZipWriter::new(fs::File::create(root.join(ARCHIVE)).unwrap());
        for (name, content) in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(content).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn bundle_preserves_the_application_license_and_does_not_create_one() {
        for user_license in [
            None,
            Some(b"My proprietary application license\r\n".as_slice()),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let plan = project(temp.path(), user_license);
            let original_manifest =
                fs::read(temp.path().join(super::super::project::MANIFEST)).unwrap();
            bundle(&plan, temp.path()).unwrap();
            // A second bundle of an already exported project is supported.
            let reloaded = Project::load(temp.path()).unwrap();
            bundle(&reloaded, temp.path()).unwrap();
            let files = archive_files(&temp.path().join(ARCHIVE));
            assert_eq!(files[OUTPUT_LICENSE_FILE], OUTPUT_LICENSE.as_bytes());
            assert_eq!(files.get("LICENSE").map(Vec::as_slice), user_license);
            assert_eq!(temp.path().join("LICENSE").exists(), user_license.is_some());
            if let Some(expected) = user_license {
                assert_eq!(fs::read(temp.path().join("LICENSE")).unwrap(), expected);
            }
            let current: serde_json::Value =
                serde_json::from_slice(&files[super::super::project::MANIFEST]).unwrap();
            let original: serde_json::Value = serde_json::from_slice(&original_manifest).unwrap();
            assert_eq!(current, original);
            let readme = String::from_utf8(files["CREXE-README.md"].clone()).unwrap();
            assert!(readme.contains("You choose your application's license"));
            assert!(readme.contains(
                "does not require you to disclose application source code or credit CREXE"
            ));
            assert!(readme.contains("their own conditions"));
        }
    }

    #[test]
    fn cached_legacy_export_adds_policy_without_changing_cache_or_user_files() {
        let temp = tempfile::tempdir().unwrap();
        let sources = [
            (
                "Program.cs",
                b"// unchanged application source\n".as_slice(),
            ),
            ("LICENSE", b"Copyright Example. Proprietary.\r\n".as_slice()),
            ("assets/pixel.bin", b"\x00\xff\x01".as_slice()),
            ("CREXE-README.md", b"An older exported guide.\n".as_slice()),
        ];
        legacy_archive(temp.path(), &sources);
        let cached = fs::read(temp.path().join(ARCHIVE)).unwrap();
        let destination = temp.path().join("first-export.zip");
        export(temp.path(), Some(&destination)).unwrap();
        assert_eq!(fs::read(temp.path().join(ARCHIVE)).unwrap(), cached);
        assert!(!temp.path().join(OUTPUT_LICENSE_FILE).exists());
        let files = archive_files(&destination);
        assert_eq!(files.len(), sources.len() + 1);
        assert_eq!(files[OUTPUT_LICENSE_FILE], OUTPUT_LICENSE.as_bytes());
        for (name, content) in sources {
            assert_eq!(files[name], content);
        }
        // Existing destinations and their bytes are never replaced.
        let exported = fs::read(&destination).unwrap();
        assert!(export(temp.path(), Some(&destination)).is_err());
        assert_eq!(fs::read(&destination).unwrap(), exported);
        // Re-exporting an updated ZIP does not duplicate its notice.
        let another = tempfile::tempdir().unwrap();
        fs::write(another.path().join(ARCHIVE), &exported).unwrap();
        let second = another.path().join("second-export.zip");
        export(another.path(), Some(&second)).unwrap();
        assert_eq!(archive_files(&second), files);
        assert_eq!(fs::read(another.path().join(ARCHIVE)).unwrap(), exported);
    }

    #[test]
    fn export_recognizes_the_same_notice_across_checkout_line_endings() {
        let temp = tempfile::tempdir().unwrap();
        let crlf = OUTPUT_LICENSE.replace("\r\n", "\n").replace('\n', "\r\n");
        legacy_archive(temp.path(), &[(OUTPUT_LICENSE_FILE, crlf.as_bytes())]);
        let output = temp.path().join("export.zip");
        export(temp.path(), Some(&output)).unwrap();
        assert_eq!(
            archive_files(&output)[OUTPUT_LICENSE_FILE],
            OUTPUT_LICENSE.as_bytes()
        );
    }

    #[test]
    fn bundle_migrates_the_known_historical_notice_and_preserves_user_license() {
        let user_license = b"My proprietary license remains unchanged.\r\n";
        for historical in historical_policy_variants() {
            let temp = tempfile::tempdir().unwrap();
            let plan = project(temp.path(), Some(user_license));
            fs::write(temp.path().join(OUTPUT_LICENSE_FILE), historical).unwrap();
            bundle(&plan, temp.path()).unwrap();
            assert_eq!(
                fs::read(temp.path().join(OUTPUT_LICENSE_FILE)).unwrap(),
                OUTPUT_LICENSE.as_bytes()
            );
            assert_eq!(fs::read(temp.path().join("LICENSE")).unwrap(), user_license);
            let files = archive_files(&temp.path().join(ARCHIVE));
            assert_eq!(files[OUTPUT_LICENSE_FILE], OUTPUT_LICENSE.as_bytes());
            assert_eq!(files["LICENSE"], user_license);
            assert_eq!(files["Program.cs"], b"// user's application source\n");
            assert!(String::from_utf8(files["CREXE-README.md"].clone())
                .unwrap()
                .contains("MIT-0 does not require retaining this notice"));
        }
    }

    #[test]
    fn cached_historical_notice_migrates_without_changing_revision_or_destination() {
        let user_license = b"Copyright Example. All rights reserved.\r\n";
        for historical in historical_policy_variants() {
            let cache = tempfile::tempdir().unwrap();
            legacy_archive(
                cache.path(),
                &[
                    (OUTPUT_LICENSE_FILE, historical.as_bytes()),
                    ("LICENSE", user_license),
                    ("Program.cs", b"// cached source remains unchanged\n"),
                ],
            );
            fs::write(
                cache.path().join("CREXE-GENERATION.json"),
                b"historical revision metadata",
            )
            .unwrap();
            let original_archive = fs::read(cache.path().join(ARCHIVE)).unwrap();
            let delivery = tempfile::tempdir().unwrap();
            let destination = delivery.path().join("migrated.zip");
            export(cache.path(), Some(&destination)).unwrap();
            let files = archive_files(&destination);
            assert_eq!(files.len(), 3);
            assert_eq!(files[OUTPUT_LICENSE_FILE], OUTPUT_LICENSE.as_bytes());
            assert_eq!(files["LICENSE"], user_license);
            assert_eq!(files["Program.cs"], b"// cached source remains unchanged\n");
            assert_eq!(
                fs::read(cache.path().join(ARCHIVE)).unwrap(),
                original_archive
            );
            assert_eq!(
                fs::read(cache.path().join("CREXE-GENERATION.json")).unwrap(),
                b"historical revision metadata"
            );
            assert_eq!(fs::read_dir(cache.path()).unwrap().count(), 2);
            let original_export = fs::read(&destination).unwrap();
            assert!(export(cache.path(), Some(&destination)).is_err());
            assert_eq!(fs::read(&destination).unwrap(), original_export);
            assert_eq!(
                fs::read(cache.path().join(ARCHIVE)).unwrap(),
                original_archive
            );
        }
    }

    #[test]
    fn a_modified_historical_notice_is_not_mistaken_for_engine_owned_material() {
        let historical = HISTORICAL_OUTPUT_POLICY.replace("\r\n", "\n");
        for changed in [
            format!("{historical}\nAdditional user-owned conditions.\n"),
            historical.replace("CREXE", "Another project"),
            historical.replace("GNU GPL v3", "custom conditions"),
        ] {
            assert!(!recognized_output_license(changed.as_bytes()));
            let temp = tempfile::tempdir().unwrap();
            let plan = project(temp.path(), None);
            fs::write(temp.path().join(OUTPUT_LICENSE_FILE), &changed).unwrap();
            assert!(bundle(&plan, temp.path()).is_err());
            assert_eq!(
                fs::read_to_string(temp.path().join(OUTPUT_LICENSE_FILE)).unwrap(),
                changed
            );
            legacy_archive(temp.path(), &[(OUTPUT_LICENSE_FILE, changed.as_bytes())]);
            let original = fs::read(temp.path().join(ARCHIVE)).unwrap();
            let output = temp.path().join("export.zip");
            assert!(export(temp.path(), Some(&output)).is_err());
            assert!(!output.exists());
            assert_eq!(fs::read(temp.path().join(ARCHIVE)).unwrap(), original);
        }
    }

    #[test]
    fn export_rejects_unsafe_entries_and_policy_collisions_without_touching_cache() {
        for entry in [
            "../outside.txt",
            "/outside.txt",
            "CREXE-OUTPUT-LICENSE.txt",
            "crexe-output-license.txt",
            "CREXE-OUTPUT-LICENSE.txt/child",
        ] {
            let temp = tempfile::tempdir().unwrap();
            legacy_archive(
                temp.path(),
                &[(entry, b"User material must not be overwritten")],
            );
            let original = fs::read(temp.path().join(ARCHIVE)).unwrap();
            let output = temp.path().join("export.zip");
            assert!(
                export(temp.path(), Some(&output)).is_err(),
                "accepted {entry}"
            );
            assert!(!output.exists());
            assert_eq!(fs::read(temp.path().join(ARCHIVE)).unwrap(), original);
        }
        let temp = tempfile::tempdir().unwrap();
        let mut zip = zip::ZipWriter::new(fs::File::create(temp.path().join(ARCHIVE)).unwrap());
        zip.add_symlink(
            "link",
            "../outside.txt",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.finish().unwrap();
        let output = temp.path().join("export.zip");
        assert!(export(temp.path(), Some(&output)).is_err());
        assert!(!output.exists());
    }

    #[test]
    fn bundle_preserves_a_conflicting_reserved_file() {
        let temp = tempfile::tempdir().unwrap();
        let plan = project(temp.path(), None);
        fs::write(temp.path().join(OUTPUT_LICENSE_FILE), "user-owned material").unwrap();
        assert!(bundle(&plan, temp.path()).is_err());
        assert_eq!(
            fs::read_to_string(temp.path().join(OUTPUT_LICENSE_FILE)).unwrap(),
            "user-owned material"
        );
        assert!(!temp.path().join(ARCHIVE).exists());
    }
}
