//! A deliberately small executable-snapshot subset of the CREXB draft.
//! Hashes check integrity, not publisher authenticity. Programs are not sandboxed.
use crate::{
    history::Baseline,
    paths,
    secure_archive::{self, Archive, ArchiveFile, Limits},
};
use anyhow::{bail, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::{Read, Write},
    path::Path,
};

const MANIFEST: &str = "META-INF/crexe-bundle.json";
const INVENTORY: &str = "META-INF/artifact-manifest.json";
const CHECKSUMS: &str = "META-INF/checksums.sha256";
const SNAPSHOT: &str = "CREXE-ORIGINAL-RECIPE.json";
const ORIGINAL: &str = "CREXE-ORIGINAL-RECIPE.bin";
const MAX_BYTES: usize = 256 * 1024 * 1024;

pub(crate) struct OriginalRecipe {
    pub kind: String,
    pub bytes: Vec<u8>,
}
impl OriginalRecipe {
    pub fn new(kind: &str, bytes: Vec<u8>) -> Result<Self> {
        original_contract(kind, &bytes)?;
        if kind == "crexp-package" {
            let archive = secure_archive::from_bytes(bytes.clone(), &Limits::recipe())?;
            if archive.files.contains_key(MANIFEST) {
                bail!("Original CREXP snapshot cannot itself be a bundle");
            }
            crate::crexp::load(&archive)?;
        }
        Ok(Self {
            kind: kind.into(),
            bytes,
        })
    }
}

fn original_contract(kind: &str, bytes: &[u8]) -> Result<()> {
    let limit = match kind {
        "crexp-package" => 8 * 1024 * 1024,
        "crexe-text" | "crexe-yaml" | "crexe-markdown" => 1024 * 1024,
        _ => bail!("Unsupported original recipe kind"),
    };
    if bytes.is_empty() || bytes.len() > limit {
        bail!("Empty or oversized original recipe snapshot");
    }
    if kind == "crexp-package" {
        if !secure_archive::has_zip_magic(bytes) {
            bail!("Original CREXP requires ZIP content");
        }
    } else {
        std::str::from_utf8(bytes).context("Original text recipe must be UTF-8")?;
        if secure_archive::has_zip_magic(bytes) {
            bail!("ZIP recipe requires crexp-package kind");
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginalMetadata {
    schema: u32,
    kind: String,
    path: String,
    sha256: String,
}

pub(crate) fn write_original(original: &OriginalRecipe, revision: &Path) -> Result<()> {
    for name in [SNAPSHOT, ORIGINAL] {
        paths::reject_links(&revision.join(name))?;
    }
    fs::write(revision.join(ORIGINAL), &original.bytes)?;
    fs::write(
        revision.join(SNAPSHOT),
        serde_json::to_vec_pretty(&OriginalMetadata {
            schema: 1,
            kind: original.kind.clone(),
            path: ORIGINAL.into(),
            sha256: digest(&original.bytes),
        })?,
    )?;
    Ok(())
}

fn original_from_revision(files: &BTreeMap<String, Vec<u8>>) -> Result<OriginalRecipe> {
    let descriptor = files.get(SNAPSHOT).context("This accepted generation predates original-recipe snapshots. Generate explicitly once before creating a bundle.")?;
    let metadata: OriginalMetadata = serde_json::from_slice(descriptor)?;
    let bytes = files
        .get(ORIGINAL)
        .context("Accepted original recipe is missing")?;
    if metadata.schema != 1 || metadata.path != ORIGINAL || metadata.sha256 != digest(bytes) {
        bail!("Accepted original recipe snapshot is modified");
    }
    OriginalRecipe::new(&metadata.kind, bytes.clone())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    format: String,
    schema_version: u32,
    bundle: Bundle,
    recipe: Recipe,
    generation: Value,
    targets: Vec<Target>,
    validation: Validation,
    integrity: Integrity,
    behavior: Behavior,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    publisher: Option<Publisher>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cache: Option<CacheMetadata>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Bundle {
    id: String,
    name: String,
    version: String,
    build_id: String,
    created_at: String,
    kind: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Publisher {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(default)]
    signature_required: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CacheMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    content_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    recipe_key: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Recipe {
    kind: String,
    path: String,
    sha256: String,
    resolved_path: String,
    generation_plan_path: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    id: String,
    os: String,
    architecture: String,
    payload_root: String,
    launch_manifest: String,
    runtime_manifest: String,
    runtime_model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    abi: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Validation {
    status: String,
    build: String,
    tests: String,
    application_start: String,
    acceptance: String,
    accepted_at: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Integrity {
    algorithm: String,
    #[serde(rename = "checksumsPath")]
    checksums: String,
    #[serde(rename = "artifactManifestPath")]
    artifact_manifest: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Behavior {
    allow_regeneration: bool,
    on_incompatible_target: String,
    on_missing_runtime: String,
    on_integrity_failure: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Launch {
    schema_version: u32,
    entrypoint: String,
    arguments: Vec<String>,
    working_directory: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Runtime {
    schema_version: u32,
    model: String,
    profile: String,
    requirement: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    schema: u32,
    files: BTreeMap<String, Artifact>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    sha256: String,
    size: u64,
    executable: bool,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn insert(
    files: &mut BTreeMap<String, ArchiveFile>,
    name: &str,
    bytes: Vec<u8>,
    executable: bool,
) -> Result<()> {
    paths::relative_file(name)?;
    if !name.is_ascii() || name.contains('\\') {
        bail!("CREXB MVP requires portable ASCII archive paths: {name}");
    }
    if files
        .insert(name.into(), ArchiveFile { bytes, executable })
        .is_some()
    {
        bail!("Duplicate bundle path: {name}");
    }
    Ok(())
}
fn insert_json(
    files: &mut BTreeMap<String, ArchiveFile>,
    name: &str,
    value: &impl Serialize,
) -> Result<()> {
    insert(files, name, serde_json::to_vec_pretty(value)?, false)
}

/// Read exactly the accepted cache inventory, rather than arbitrary leftover files.
fn accepted_files(accepted: &Baseline) -> Result<BTreeMap<String, Vec<u8>>> {
    if !accepted.cached() {
        bail!("Last accepted generation has missing/modified compiled artifacts; regenerate explicitly before bundling");
    }
    let pointer = accepted
        .revision
        .parent()
        .context("Missing cache root")?
        .join("current.json");
    let value: Value = serde_json::from_slice(&fs::read(pointer)?)?;
    let expected: BTreeMap<String, String> = serde_json::from_value(value["files"].clone())?;
    let mut actual = Vec::new();
    collect_files(&accepted.revision, &accepted.revision, &mut actual)?;
    actual.sort();
    if actual != expected.keys().cloned().collect::<Vec<_>>() {
        bail!("Accepted cache contains missing or unlisted files; restore or regenerate explicitly before bundling");
    }
    let mut result = BTreeMap::new();
    let mut total = 0usize;
    for (name, hash) in expected {
        let file = accepted.revision.join(paths::relative_file(&name)?);
        paths::reject_links(&file)?;
        let size = fs::metadata(&file)?.len();
        if size > 128 * 1024 * 1024 {
            bail!("Accepted file exceeds CREXB MVP size limit: {name}");
        }
        total = total
            .checked_add(size as usize)
            .context("Bundle too large")?;
        if total > MAX_BYTES || result.len() >= 1024 {
            bail!("Accepted revision exceeds CREXB MVP limits");
        }
        let mut bytes = Vec::new();
        fs::File::open(&file)?
            .take(size + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 != size {
            bail!("Accepted artifact size changed while creating bundle: {name}");
        }
        if digest(&bytes) != hash {
            bail!("Accepted artifact changed while creating bundle: {name}");
        }
        result.insert(name, bytes);
    }
    for (name, expected) in &accepted.metadata.sources {
        if result
            .get(name)
            .is_none_or(|bytes| digest(bytes) != *expected)
        {
            bail!("Accepted source metadata mismatch: {name}");
        }
    }
    Ok(result)
}

fn profile_contract(profile: &str, os: &str) -> Result<(&'static str, &'static str)> {
    match (profile, os) {
        ("dotnet-console", "windows" | "linux" | "macos") => Ok(("framework-dependent", ".NET 8 runtime")),
        ("dotnet-winforms", "windows") => Ok(("framework-dependent", ".NET 8 Windows Desktop runtime")),
        ("cpp-win32", "windows") => Ok(("external-runtime", "Windows system libraries")),
        ("objc-cocoa", "macos") => Ok(("external-runtime", "macOS Cocoa framework")),
        ("c-gtk", "linux") => Ok(("external-runtime", "GTK 3 shared libraries")),
        _ => bail!("Profile {profile} on {os} is outside the CREXB MVP (OpenGL/custom commands are roadmap)"),
    }
}
fn architecture(arch: &str) -> Result<&'static str> {
    match arch {
        "x86_64" | "x64" => Ok("x64"),
        "aarch64" | "arm64" => Ok("arm64"),
        _ => bail!("CREXB MVP supports only x64 and arm64"),
    }
}
fn native_name(os: &str) -> &'static str {
    if os == "windows" {
        "CrexeApp.exe"
    } else {
        "CrexeApp"
    }
}

pub(crate) fn create(recipe: &Path, accepted: &Baseline, output: &Path) -> Result<()> {
    let entry = crate::cache::Entry {
        root: accepted
            .revision
            .parent()
            .context("Missing accepted cache root")?
            .into(),
        fingerprint: accepted
            .revision
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into(),
    };
    let _guard = entry.lock()?;
    let revision = accepted_files(accepted)?;
    let original = original_from_revision(&revision)?;
    let project = crate::project::Project::load(&accepted.revision)?;
    let (runtime_model, requirement) = profile_contract(&project.profile, &project.os)?;
    let arch = architecture(&project.arch)?;
    if project.os != std::env::consts::OS || arch != architecture(std::env::consts::ARCH)? {
        bail!("Bundle creation supports only the current host target");
    }
    if project.profile != accepted.metadata.profile {
        bail!("Accepted profile metadata mismatch");
    }
    let target_id = format!("{}-{arch}", project.os);
    let payload_root = format!("payload/{target_id}/app");
    let executable = native_name(&project.os);
    let from = if project.profile.starts_with("dotnet-") {
        format!("build/{executable}")
    } else {
        executable.into()
    };
    native_binary(
        revision
            .get(&from)
            .context("Accepted native entrypoint is missing")?,
        &project.os,
        arch,
    )?;
    let mut files = BTreeMap::new();
    for (name, bytes) in &revision {
        if project.profile.starts_with("dotnet-") {
            if let Some(relative) = name.strip_prefix("build/") {
                insert(
                    &mut files,
                    &format!("{payload_root}/{relative}"),
                    bytes.clone(),
                    relative == executable,
                )?;
            }
        } else if name == executable {
            insert(
                &mut files,
                &format!("{payload_root}/{executable}"),
                bytes.clone(),
                true,
            )?;
        }
    }
    for name in accepted.metadata.sources.keys() {
        insert(
            &mut files,
            &format!("source/{name}"),
            revision
                .get(name)
                .context("Accepted source missing")?
                .clone(),
            false,
        )?;
    }
    let recipe_path = if original.kind == "crexp-package" {
        "recipe/original.crexp"
    } else {
        "recipe/original.crexe"
    };
    let recipe_hash = digest(&original.bytes);
    insert(&mut files, recipe_path, original.bytes, false)?;
    insert(
        &mut files,
        "recipe/resolved.yaml",
        serde_yaml::to_string(&accepted.metadata.spec)?.into_bytes(),
        false,
    )?;
    insert_json(
        &mut files,
        "recipe/generation-plan.json",
        &json!({"schemaVersion":1,"profile":project.profile,"generationId":accepted.metadata.request}),
    )?;
    insert(
        &mut files,
        "LICENSE-GENERATED",
        include_bytes!("../LICENSE-GENERATED").to_vec(),
        false,
    )?;
    insert(&mut files, "tests/README.md", b"# Accepted tests\n\nThe accepted engine build passed its --crexe-self-test command. Test implementations are preserved in source/. Application startup and human acceptance were not verified by bundle creation. Opening the bundle does not re-run tests.\n".to_vec(), false)?;
    insert_json(
        &mut files,
        "evidence/acceptance.json",
        &json!({"schemaVersion":1,"generationId":accepted.metadata.request,
        "build":"passed","tests":"passed","applicationStart":"not-verified","acceptance":"not-required",
        "sourceHashes":accepted.metadata.sources,"engineVersion":env!("CARGO_PKG_VERSION")}),
    )?;
    let launch_path = format!("payload/{target_id}/launch.json");
    let runtime_path = format!("payload/{target_id}/runtime-manifest.json");
    insert_json(
        &mut files,
        &launch_path,
        &Launch {
            schema_version: 1,
            entrypoint: format!("{payload_root}/{executable}"),
            arguments: vec![],
            working_directory: "data".into(),
        },
    )?;
    insert_json(
        &mut files,
        &runtime_path,
        &Runtime {
            schema_version: 1,
            model: runtime_model.into(),
            profile: project.profile.clone(),
            requirement: requirement.into(),
        },
    )?;
    let build_id = payload_digest(&files, &payload_root);
    // Stable on this recipe path across revisions; never disclose its local path.
    let mut identity = recipe.canonicalize()?.to_string_lossy().into_owned();
    if cfg!(windows) {
        identity = identity.to_lowercase();
    }
    let now = chrono::Utc::now().to_rfc3339();
    let cache_pointer: Value = serde_json::from_slice(&fs::read(entry.root.join("current.json"))?)?;
    let accepted_at = cache_pointer["generated_at"]
        .as_str()
        .context("Accepted cache timestamp missing")?
        .to_owned();
    let manifest = Manifest {
        format: "crexe.bundle".into(),
        schema_version: 1,
        bundle: Bundle {
            id: format!("org.crexe.local.{}", &digest(identity.as_bytes())[..24]),
            name: project.name,
            version: format!("0.0.0+{}", &build_id[..12]),
            build_id: format!("sha256:{build_id}"),
            created_at: now.clone(),
            kind: "executable-snapshot".into(),
        },
        recipe: Recipe {
            kind: original.kind,
            path: recipe_path.into(),
            sha256: recipe_hash,
            resolved_path: "recipe/resolved.yaml".into(),
            generation_plan_path: "recipe/generation-plan.json".into(),
        },
        generation: json!({"generationId":accepted.metadata.request,"engineVersion":env!("CARGO_PKG_VERSION"),"resolver":{"id":"crexe-v1-accepted-cache","version":"1"}}),
        targets: vec![Target {
            id: target_id,
            os: project.os,
            architecture: arch.into(),
            payload_root,
            launch_manifest: launch_path,
            runtime_manifest: runtime_path,
            runtime_model: runtime_model.into(),
            abi: None,
        }],
        validation: Validation {
            status: "accepted".into(),
            build: "passed".into(),
            tests: "passed".into(),
            application_start: "not-verified".into(),
            acceptance: "not-required".into(),
            accepted_at,
        },
        integrity: Integrity {
            algorithm: "sha256".into(),
            checksums: CHECKSUMS.into(),
            artifact_manifest: INVENTORY.into(),
        },
        behavior: Behavior {
            allow_regeneration: true,
            on_incompatible_target: "ask-to-regenerate".into(),
            on_missing_runtime: "block".into(),
            on_integrity_failure: "block".into(),
        },
        publisher: None,
        cache: None,
    };
    insert_json(&mut files, MANIFEST, &manifest)?;
    seal(&mut files)?;
    write_zip(output, &files)?;
    println!(
        "Created unsigned CREXB from last accepted generation: {}",
        output.display()
    );
    Ok(())
}

fn payload_digest(files: &BTreeMap<String, ArchiveFile>, root: &str) -> String {
    let mut hash = Sha256::new();
    for (name, file) in files
        .iter()
        .filter(|(name, _)| name.starts_with(&format!("{root}/")))
    {
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update([u8::from(file.executable)]);
        hash.update((file.bytes.len() as u64).to_le_bytes());
        hash.update(&file.bytes);
    }
    format!("{:x}", hash.finalize())
}
fn seal(files: &mut BTreeMap<String, ArchiveFile>) -> Result<()> {
    let inventory = Inventory {
        schema: 1,
        files: files
            .iter()
            .map(|(name, file)| {
                (
                    name.clone(),
                    Artifact {
                        sha256: digest(&file.bytes),
                        size: file.bytes.len() as u64,
                        executable: file.executable,
                    },
                )
            })
            .collect(),
    };
    insert_json(files, INVENTORY, &inventory)?;
    let sums: String = files
        .iter()
        .map(|(name, file)| format!("{}  {name}\n", digest(&file.bytes)))
        .collect();
    insert(files, CHECKSUMS, sums.into_bytes(), false)
}
fn write_zip(output: &Path, files: &BTreeMap<String, ArchiveFile>) -> Result<()> {
    paths::reject_links(output)?;
    if output.exists() {
        bail!(
            "Bundle output already exists; choose another path: {}",
            output.display()
        );
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut writer = zip::ZipWriter::new(staged.as_file_mut());
        // Store entries: predictable resource bounds and no compression-ratio surprises.
        for name in std::iter::once(MANIFEST).chain(
            files
                .keys()
                .map(String::as_str)
                .filter(|name| *name != MANIFEST),
        ) {
            let file = &files[name];
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)
                .unix_permissions(if file.executable { 0o755 } else { 0o644 });
            writer.start_file(name, options)?;
            writer.write_all(&file.bytes)?;
        }
        writer.finish()?;
    }
    staged.as_file().sync_all()?;
    // Re-read with the exact consumer before exposing a result.
    checked(staged.path())?;
    staged.persist_noclobber(output).map_err(|e| e.error)?;
    Ok(())
}

struct Checked {
    archive: Archive,
    manifest: Manifest,
    launch: Launch,
}
fn json_file<T: serde::de::DeserializeOwned>(archive: &Archive, name: &str) -> Result<T> {
    let file = archive
        .files
        .get(name)
        .with_context(|| format!("Required bundle file missing: {name}"))?;
    if file.bytes.len() > 2 * 1024 * 1024 {
        bail!("Bundle metadata too large: {name}");
    }
    serde_json::from_slice(&file.bytes).with_context(|| format!("Invalid bundle metadata: {name}"))
}
fn checked(path: &Path) -> Result<Checked> {
    checked_archive(secure_archive::read(path, &Limits::bundle())?)
}
fn checked_archive(archive: Archive) -> Result<Checked> {
    if archive.files.contains_key("crexp.yaml") {
        bail!("Ambiguous CREXB/CREXP markers");
    }
    let manifest: Manifest = json_file(&archive, MANIFEST)?;
    if manifest.format != "crexe.bundle"
        || manifest.schema_version != 1
        || manifest.bundle.kind != "executable-snapshot"
        || manifest.bundle.id.is_empty()
        || manifest.bundle.id.len() > 200
        || manifest.bundle.name.is_empty()
        || manifest.bundle.version.is_empty()
        || chrono::DateTime::parse_from_rfc3339(&manifest.bundle.created_at).is_err()
        || manifest.targets.len() != 1
    {
        bail!("Unsupported CREXB manifest; MVP requires one executable-snapshot target");
    }
    if manifest
        .publisher
        .as_ref()
        .is_some_and(|p| p.signature_required)
        || archive.files.keys().any(|name| {
            name.to_ascii_lowercase()
                .starts_with("meta-inf/signatures/")
        })
    {
        bail!("CREXB signature verification is not implemented; signed/required-signature bundles are unsupported");
    }
    if manifest.integrity.algorithm != "sha256"
        || manifest.integrity.checksums != CHECKSUMS
        || manifest.integrity.artifact_manifest != INVENTORY
        || manifest.behavior.on_integrity_failure != "block"
        || !matches!(
            manifest.behavior.on_incompatible_target.as_str(),
            "ask-to-regenerate" | "block"
        )
        || manifest.behavior.on_missing_runtime != "block"
    {
        bail!("Unsupported CREXB integrity or behavior policy");
    }
    if manifest.validation.status != "accepted"
        || manifest.validation.build != "passed"
        || manifest.validation.tests != "passed"
        || !matches!(
            manifest.validation.application_start.as_str(),
            "passed" | "not-verified"
        )
        || !matches!(
            manifest.validation.acceptance.as_str(),
            "passed" | "partial" | "not-required"
        )
        || chrono::DateTime::parse_from_rfc3339(&manifest.validation.accepted_at).is_err()
    {
        bail!("Bundle does not declare an accepted, tested build");
    }
    for key in ["generationId", "engineVersion"] {
        if manifest.generation[key].as_str().is_none_or(str::is_empty) {
            bail!("Missing generation provenance: {key}");
        }
    }
    for key in ["recipeCompilerVersion", "toolchainFingerprint"] {
        if manifest
            .generation
            .get(key)
            .is_some_and(|value| !value.is_string())
        {
            bail!("Invalid generation provenance type: {key}");
        }
    }
    if manifest
        .generation
        .get("provider")
        .is_some_and(|value| !value.is_object())
    {
        bail!("Generation provider provenance must be an object");
    }
    for key in ["id", "version"] {
        if manifest.generation["resolver"][key]
            .as_str()
            .is_none_or(str::is_empty)
        {
            bail!("Missing resolver provenance");
        }
    }
    verify_inventory(&archive)?;
    let target = &manifest.targets[0];
    let arch = architecture(&target.architecture)?;
    if target.architecture != arch
        || target.id != format!("{}-{arch}", target.os)
        || target.payload_root != format!("payload/{}/app", target.id)
        || target.launch_manifest != format!("payload/{}/launch.json", target.id)
        || target.runtime_manifest != format!("payload/{}/runtime-manifest.json", target.id)
        || target.abi.is_some()
    {
        bail!("Unsupported CREXB target layout");
    }
    let runtime: Runtime = json_file(&archive, &target.runtime_manifest)?;
    let (model, requirement) = profile_contract(&runtime.profile, &target.os)?;
    if runtime.schema_version != 1
        || runtime.model != model
        || target.runtime_model != model
        || runtime.requirement != requirement
    {
        bail!("Unsupported CREXB runtime contract");
    }
    let launch: Launch = json_file(&archive, &target.launch_manifest)?;
    if launch.schema_version != 1
        || launch.entrypoint != format!("{}/{}", target.payload_root, native_name(&target.os))
        || !launch.arguments.is_empty()
        || launch.working_directory != "data"
    {
        bail!("Unsupported CREXB launch contract; only the contained native app is permitted");
    }
    let binary = archive
        .files
        .get(&launch.entrypoint)
        .context("Bundle entrypoint missing")?;
    if !binary.executable {
        bail!("Bundle entrypoint executable flag missing");
    }
    native_binary(&binary.bytes, &target.os, arch)?;
    if archive
        .files
        .iter()
        .any(|(name, file)| file.executable && name != &launch.entrypoint)
    {
        bail!("Unexpected executable flag outside the declared native entrypoint");
    }
    if archive.files.keys().any(|name| {
        name.starts_with("payload/") && !name.starts_with(&format!("payload/{}/", target.id))
    }) {
        bail!("Unexpected payload target");
    }
    if manifest.bundle.build_id
        != format!(
            "sha256:{}",
            payload_digest(&archive.files, &target.payload_root)
        )
    {
        bail!("Bundle payload content hash mismatch");
    }
    let expected_recipe = if manifest.recipe.kind == "crexp-package" {
        "recipe/original.crexp"
    } else {
        "recipe/original.crexe"
    };
    if manifest.recipe.path != expected_recipe
        || manifest.recipe.resolved_path != "recipe/resolved.yaml"
        || manifest.recipe.generation_plan_path != "recipe/generation-plan.json"
    {
        bail!("Unsupported recipe layout");
    }
    let recipe = archive
        .files
        .get(&manifest.recipe.path)
        .context("Original recipe missing")?;
    if digest(&recipe.bytes) != manifest.recipe.sha256 {
        bail!("Original recipe hash mismatch");
    }
    // The enclosed recipe is inert on open. Check its contract and hash, without
    // interpreting SDD documents; explicit regeneration performs full validation.
    original_contract(&manifest.recipe.kind, &recipe.bytes)?;
    for name in [
        &manifest.recipe.resolved_path,
        &manifest.recipe.generation_plan_path,
    ] {
        if !archive.files.contains_key(name) {
            bail!("Missing resolved recipe or generation plan");
        }
    }
    if runtime.profile.starts_with("dotnet-") {
        for file in [
            "CrexeApp.dll",
            "CrexeApp.deps.json",
            "CrexeApp.runtimeconfig.json",
        ] {
            if !archive
                .files
                .contains_key(&format!("{}/{file}", target.payload_root))
            {
                bail!("Incomplete .NET apphost payload: missing {file}");
            }
        }
    }
    Ok(Checked {
        archive,
        manifest,
        launch,
    })
}
fn verify_inventory(archive: &Archive) -> Result<()> {
    let inventory: Inventory = json_file(archive, INVENTORY)?;
    if inventory.schema != 1 || inventory.files.len() + 2 != archive.files.len() {
        bail!("Bundle artifact inventory does not cover every file");
    }
    for (name, expected) in inventory.files {
        if matches!(name.as_str(), INVENTORY | CHECKSUMS) {
            bail!("Self-referential integrity manifest");
        }
        let file = archive
            .files
            .get(&name)
            .context("Inventory artifact missing")?;
        if expected.sha256 != digest(&file.bytes)
            || expected.size != file.bytes.len() as u64
            || expected.executable != file.executable
        {
            bail!("Bundle artifact integrity failed: {name}");
        }
    }
    let expected: String = archive
        .files
        .iter()
        .filter(|(name, _)| name.as_str() != CHECKSUMS)
        .map(|(name, file)| format!("{}  {name}\n", digest(&file.bytes)))
        .collect();
    if archive
        .files
        .get(CHECKSUMS)
        .context("Checksums missing")?
        .bytes
        != expected.as_bytes()
    {
        bail!("Bundle checksums do not cover the exact archive");
    }
    Ok(())
}

/// Accept a native target executable, never a shell script or a renamed interpreter command.
pub(crate) fn native_binary(bytes: &[u8], os: &str, arch: &str) -> Result<()> {
    let machine = match os {
        "windows" if bytes.len() >= 64 && bytes.starts_with(b"MZ") => {
            let offset = u32::from_le_bytes(bytes[60..64].try_into().unwrap()) as usize;
            if offset > bytes.len().saturating_sub(24)
                || bytes.get(offset..offset + 4) != Some(b"PE\0\0")
            {
                bail!("Invalid PE entrypoint");
            }
            let flags = u16::from_le_bytes(bytes[offset + 22..offset + 24].try_into().unwrap());
            if flags & 0x2000 != 0 {
                bail!("A DLL cannot be a bundle entrypoint");
            }
            match u16::from_le_bytes(bytes[offset + 4..offset + 6].try_into().unwrap()) {
                0x8664 => "x64",
                0xaa64 => "arm64",
                _ => "unsupported",
            }
        }
        "linux"
            if bytes.len() >= 64
                && bytes.starts_with(b"\x7fELF")
                && bytes[4] == 2
                && bytes[5] == 1 =>
        {
            let kind = u16::from_le_bytes(bytes[16..18].try_into().unwrap());
            if !matches!(kind, 2 | 3) {
                bail!("ELF entrypoint must be an executable");
            }
            match u16::from_le_bytes(bytes[18..20].try_into().unwrap()) {
                62 => "x64",
                183 => "arm64",
                _ => "unsupported",
            }
        }
        "macos" if bytes.len() >= 32 && bytes.starts_with(&[0xcf, 0xfa, 0xed, 0xfe]) => {
            if u32::from_le_bytes(bytes[12..16].try_into().unwrap()) != 2 {
                bail!("Mach-O entrypoint must be executable");
            }
            match u32::from_le_bytes(bytes[4..8].try_into().unwrap()) {
                0x01000007 => "x64",
                0x0100000c => "arm64",
                _ => "unsupported",
            }
        }
        _ => bail!("Bundle entrypoint is not a supported native executable for {os}-{arch}"),
    };
    if machine != arch {
        bail!("Bundle native architecture mismatch: declared {arch}, binary {machine}");
    }
    Ok(())
}

pub(crate) fn inspect(file: &Path) -> Result<Value> {
    let checked = checked(file)?;
    Ok(
        json!({"manifest":checked.manifest,"integrity":"verified","authenticity":"unsigned/unverified",
        "compatible":compatible(&checked.manifest.targets[0]),"archiveSha256":checked.archive.raw_sha256,
        "files":checked.archive.files.keys().collect::<Vec<_>>(),"mvp":true}),
    )
}
pub(crate) fn verify(file: &Path) -> Result<()> {
    let checked = checked(file)?;
    println!(
        "CREXB integrity verified (unsigned; publisher authenticity is not verified): {}",
        checked.archive.raw_sha256
    );
    Ok(())
}
fn compatible(target: &Target) -> bool {
    target.os == std::env::consts::OS
        && architecture(std::env::consts::ARCH).is_ok_and(|arch| arch == target.architecture)
}
pub(crate) fn extract_recipe(file: &Path, output: &Path) -> Result<()> {
    let checked = checked(file)?;
    paths::reject_links(output)?;
    if output.exists() {
        bail!("Recipe output already exists: {}", output.display());
    }
    let expected = if checked.manifest.recipe.kind == "crexp-package" {
        "crexp"
    } else {
        "crexe"
    };
    if output
        .extension()
        .and_then(|e| e.to_str())
        .is_none_or(|e| !e.eq_ignore_ascii_case(expected))
    {
        bail!("Use a .{expected} output path for the original recipe");
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    staged.write_all(&checked.archive.files[&checked.manifest.recipe.path].bytes)?;
    staged.as_file().sync_all()?;
    staged.persist_noclobber(output).map_err(|e| e.error)?;
    println!(
        "Original recipe extracted; generation remains an explicit action: {}",
        output.display()
    );
    Ok(())
}

pub(crate) fn run(file: &Path, cli: &crate::Cli) -> Result<()> {
    let _progress = crate::presentation::Progress::start(cli.ui)?;
    crate::presentation::phase("Verificando bundle CREXB…");
    let checked = checked(file)?;
    let target = &checked.manifest.targets[0];
    if !compatible(target) {
        bail!("CREXB target {} is incompatible with {}-{}. No AI or compiler was called. Use bundle extract-recipe and explicitly generate for this computer.",target.id,std::env::consts::OS,architecture(std::env::consts::ARCH).unwrap_or("unsupported"));
    }
    if cli.no_run {
        println!("CREXB verified; --no-run skips launching");
        return Ok(());
    }
    let cache = crate::default_cache_root()?
        .join("bundles")
        .join("objects")
        .join("sha256");
    paths::reject_links(&cache)?;
    fs::create_dir_all(&cache)?;
    let hash = &checked.archive.raw_sha256;
    let lock_path = cache.join(format!("{hash}.lock"));
    paths::reject_links(&lock_path)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)?;
    lock.try_lock_exclusive()
        .context("This CREXB is already opening/running; retry after it exits")?;
    let object = cache.join(hash);
    hydrate(&checked.archive, &object)?;
    let app = object.join(&target.payload_root);
    let data_root = crate::installation::root()?
        .join("apps")
        .join(digest(checked.manifest.bundle.id.as_bytes()));
    let data = data_root.join("data");
    let config = data_root.join("config");
    let state = data_root.join("state");
    for path in [&data, &config, &state] {
        paths::reject_links(path)?;
        fs::create_dir_all(path)?;
    }
    let environment: BTreeMap<String, OsString> = [
        ("CREXE_APP_DIR", app.as_os_str().to_owned()),
        ("CREXE_DATA_DIR", data.as_os_str().to_owned()),
        ("CREXE_CONFIG_DIR", config.as_os_str().to_owned()),
        ("CREXE_STATE_DIR", state.as_os_str().to_owned()),
        ("CREXE_CACHE_DIR", object.as_os_str().to_owned()),
        (
            "CREXE_PACKAGE_ID",
            checked.manifest.bundle.id.clone().into(),
        ),
        (
            "CREXE_BUNDLE_ID",
            checked.manifest.bundle.build_id.clone().into(),
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v))
    .collect();
    let executable = object.join(&checked.launch.entrypoint);
    println!(
        "Opening unsigned CREXB; no AI, build or SDK is used. Required runtime: {}",
        target.runtime_model
    );
    crate::presentation::finish();
    crate::executor::run_with_env(&[executable.to_string_lossy().into_owned()],&data,None,false,&[],&environment)
        .context("CREXB app failed to run. Install the runtime declared by bundle inspect if missing; regeneration is never automatic.")
}

fn hydrate(archive: &Archive, object: &Path) -> Result<()> {
    paths::reject_links(object)?;
    if object.exists() {
        verify_hydrated(archive,object).context("CREXB cache is incomplete or modified; it was blocked without running. Remove this object directory explicitly and retry.")?;
        return Ok(());
    }
    let parent = object.parent().context("Missing bundle cache root")?;
    let stage = tempfile::Builder::new()
        .prefix("staging-")
        .tempdir_in(parent)?;
    for (name, file) in &archive.files {
        let target = stage.path().join(paths::relative_file(name)?);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, &file.bytes)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                &target,
                fs::Permissions::from_mode(if file.executable { 0o755 } else { 0o644 }),
            )?;
        }
    }
    fs::write(stage.path().join(".complete"), &archive.raw_sha256)?;
    verify_hydrated(archive, stage.path())?;
    fs::rename(stage.path(), object)?;
    Ok(())
}
fn verify_hydrated(archive: &Archive, object: &Path) -> Result<()> {
    paths::reject_links(&object.join(".complete"))?;
    if fs::read(object.join(".complete"))? != archive.raw_sha256.as_bytes() {
        bail!("Bundle cache completion marker mismatch");
    }
    let mut actual = Vec::new();
    collect_files(object, object, &mut actual)?;
    actual.sort();
    let mut expected: Vec<String> = archive
        .files
        .keys()
        .cloned()
        .chain(std::iter::once(".complete".into()))
        .collect();
    expected.sort();
    if actual != expected {
        bail!("Bundle cache contains missing or unexpected files");
    }
    for (name, file) in &archive.files {
        let target = object.join(paths::relative_file(name)?);
        paths::reject_links(&target)?;
        if fs::metadata(&target)?.len() != file.bytes.len() as u64
            || crate::cache::hash(&target)? != digest(&file.bytes)
        {
            bail!("Bundle cache artifact modified: {name}");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if (fs::metadata(&target)?.permissions().mode() & 0o111 != 0) != file.executable {
                bail!("Bundle cache executable permissions changed: {name}");
            }
        }
    }
    Ok(())
}
fn collect_files(root: &Path, directory: &Path, files: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        let meta = fs::symlink_metadata(&path)?;
        if paths::is_link(&meta) {
            bail!("Link in bundle cache");
        }
        if meta.is_dir() {
            if path.strip_prefix(root)?.components().count() > 16 {
                bail!("Unexpected bundle cache depth");
            }
            collect_files(root, &path, files)?;
        } else if meta.is_file() {
            if files.len() > 1024 {
                bail!("Unexpected bundle cache files");
            }
            files.push(
                path.strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        } else {
            bail!("Unsupported file type in bundle cache");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Cursor, path::PathBuf};

    fn binary(os: &str, arch: &str) -> Vec<u8> {
        let mut bytes = vec![0; 128];
        match os {
            "windows" => {
                bytes[..2].copy_from_slice(b"MZ");
                bytes[60..64].copy_from_slice(&64u32.to_le_bytes());
                bytes[64..68].copy_from_slice(b"PE\0\0");
                bytes[68..70].copy_from_slice(
                    &(if arch == "x64" { 0x8664u16 } else { 0xaa64u16 }).to_le_bytes(),
                );
            }
            "linux" => {
                bytes[..4].copy_from_slice(b"\x7fELF");
                bytes[4] = 2;
                bytes[5] = 1;
                bytes[16..18].copy_from_slice(&2u16.to_le_bytes());
                bytes[18..20]
                    .copy_from_slice(&(if arch == "x64" { 62u16 } else { 183u16 }).to_le_bytes());
            }
            "macos" => {
                bytes[..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
                bytes[4..8].copy_from_slice(
                    &(if arch == "x64" {
                        0x01000007u32
                    } else {
                        0x0100000cu32
                    })
                    .to_le_bytes(),
                );
                bytes[12..16].copy_from_slice(&2u32.to_le_bytes());
            }
            _ => panic!("Unsupported test host"),
        }
        bytes
    }

    fn fixture(root: &Path, snapshot: bool) -> (PathBuf, Baseline) {
        let recipe = root.join("original.crexe");
        fs::write(&recipe, "Original accepted intent").unwrap();
        let workspace = root.join("workspace");
        fs::create_dir(&workspace).unwrap();
        let os = std::env::consts::OS;
        let profile = match os {
            "windows" => "cpp-win32",
            "linux" => "c-gtk",
            _ => "objc-cocoa",
        };
        fs::write(workspace.join("main.cpp"), "int main() { return 0; }").unwrap();
        fs::write(workspace.join(crate::project::MANIFEST),serde_json::to_vec(&json!({
            "schema":1,"name":"Fixture","os":os,"arch":std::env::consts::ARCH,"profile":profile,
            "sources":["main.cpp"],"commands":{"build":[["compiler","main.cpp"]],"run":[[native_name(os)]],"test":[[native_name(os),"--crexe-self-test"]]}
        })).unwrap()).unwrap();
        fs::write(
            workspace.join(native_name(os)),
            binary(os, architecture(std::env::consts::ARCH).unwrap()),
        )
        .unwrap();
        if snapshot {
            write_original(
                &OriginalRecipe::new("crexe-text", fs::read(&recipe).unwrap()).unwrap(),
                &workspace,
            )
            .unwrap();
        }
        let mut metadata = crate::history::Metadata::new(
            "fixture-accepted".into(),
            None,
            "Original accepted intent".into(),
            serde_yaml::from_str("meta: {name: Fixture}").unwrap(),
            json!({}),
        );
        metadata.write(&workspace).unwrap();
        let entry = crate::cache::Entry {
            root: root.join("cache").join("fingerprint"),
            fingerprint: "fingerprint".into(),
        };
        let _guard = entry.lock().unwrap();
        let revision = entry.publish(&workspace).unwrap();
        (recipe, Baseline { revision, metadata })
    }

    fn valid_archive(root: &Path) -> Archive {
        let (recipe, baseline) = fixture(root, true);
        let output = root.join("app.crexb");
        create(&recipe, &baseline, &output).unwrap();
        checked(&output).unwrap().archive
    }
    fn reseal(archive: &mut Archive) {
        archive.files.remove(INVENTORY);
        archive.files.remove(CHECKSUMS);
        seal(&mut archive.files).unwrap();
    }
    fn mutate_manifest(archive: &mut Archive, f: impl FnOnce(&mut Value)) {
        let mut manifest: Value = serde_json::from_slice(&archive.files[MANIFEST].bytes).unwrap();
        f(&mut manifest);
        archive.files.get_mut(MANIFEST).unwrap().bytes =
            serde_json::to_vec_pretty(&manifest).unwrap();
        reseal(archive);
    }

    #[test]
    fn export_preserves_the_accepted_recipe_and_source_not_the_current_file_or_attempt() {
        let temp = tempfile::tempdir().unwrap();
        let (recipe, baseline) = fixture(temp.path(), true);
        fs::write(&recipe, "Edited but not accepted").unwrap();
        let output = temp.path().join("app.crexb");
        create(&recipe, &baseline, &output).unwrap();
        let checked = checked(&output).unwrap();
        assert_eq!(
            checked.archive.files["recipe/original.crexe"].bytes,
            b"Original accepted intent"
        );
        assert!(checked.archive.files.contains_key("source/main.cpp"));
        assert!(checked.archive.files.contains_key("tests/README.md"));
        assert!(checked
            .archive
            .files
            .contains_key("evidence/acceptance.json"));
        assert_eq!(
            checked.manifest.validation.application_start,
            "not-verified"
        );
        assert!(create(&recipe, &baseline, &output).is_err());
    }

    #[test]
    fn old_or_corrupted_accepted_generations_cannot_be_exported() {
        let temp = tempfile::tempdir().unwrap();
        let (recipe, baseline) = fixture(temp.path(), false);
        let output = temp.path().join("old.crexb");
        assert!(create(&recipe, &baseline, &output)
            .unwrap_err()
            .to_string()
            .contains("predates"));
        assert!(!output.exists());
        let temp = tempfile::tempdir().unwrap();
        let (recipe, baseline) = fixture(temp.path(), true);
        fs::write(
            baseline.revision.join(native_name(std::env::consts::OS)),
            b"tamper",
        )
        .unwrap();
        assert!(create(&recipe, &baseline, &temp.path().join("bad.crexb")).is_err());
        let temp = tempfile::tempdir().unwrap();
        let (recipe, baseline) = fixture(temp.path(), true);
        fs::write(
            baseline.revision.join("unlisted.dll"),
            "unexpected cached code",
        )
        .unwrap();
        assert!(create(&recipe, &baseline, &temp.path().join("extra.crexb"))
            .unwrap_err()
            .to_string()
            .contains("unlisted"));
    }

    #[test]
    fn exported_manifest_matches_published_field_contract() {
        // Check serialization against the supplied draft, not a second list of
        // Rust field names. This deliberately tests fields/constants, not a full
        // JSON Schema implementation.
        fn check(value: &Value, schema: &Value) {
            if let Some(expected) = schema.get("const") {
                assert_eq!(value, expected);
            }
            if let Some(allowed) = schema.get("enum") {
                assert!(allowed.as_array().unwrap().contains(value));
            }
            if let Some(object) = value.as_object() {
                if let Some(required) = schema["required"].as_array() {
                    for name in required {
                        assert!(
                            object.contains_key(name.as_str().unwrap()),
                            "Missing field: {name}"
                        );
                    }
                }
                if let Some(properties) = schema["properties"].as_object() {
                    for (name, value) in object {
                        if let Some(property) = properties.get(name) {
                            check(value, property);
                        } else {
                            assert_ne!(
                                schema.get("additionalProperties"),
                                Some(&Value::Bool(false)),
                                "Unknown field: {name}"
                            );
                        }
                    }
                }
            }
            if let (Some(values), Some(items)) = (value.as_array(), schema.get("items")) {
                for value in values {
                    check(value, items);
                }
            }
        }
        let schema: Value = serde_json::from_str(include_str!(
            "../docs/formatos/originais/crexb-manifest-v1.schema.json"
        ))
        .unwrap();
        let temp = tempfile::tempdir().unwrap();
        let archive = valid_archive(temp.path());
        let manifest: Value = serde_json::from_slice(&archive.files[MANIFEST].bytes).unwrap();
        check(&manifest, &schema);
    }

    #[test]
    fn tampered_payload_or_unlisted_files_fail_integrity() {
        let temp = tempfile::tempdir().unwrap();
        let mut archive = valid_archive(temp.path());
        archive
            .files
            .get_mut("source/main.cpp")
            .unwrap()
            .bytes
            .push(b'x');
        assert!(checked_archive(archive).is_err());
        let temp = tempfile::tempdir().unwrap();
        let mut archive = valid_archive(temp.path());
        insert(
            &mut archive.files,
            "unexpected.txt",
            b"extra".to_vec(),
            false,
        )
        .unwrap();
        assert!(checked_archive(archive).is_err());
    }

    #[test]
    fn optional_metadata_preserves_the_published_types() {
        for invalid in [
            json!({"cache":"invalid"}),
            json!({"cache":{"unknown":1}}),
            json!({"generation":{"provider":"invalid"}}),
            json!({"generation":{"recipeCompilerVersion":42}}),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let mut archive = valid_archive(temp.path());
            mutate_manifest(&mut archive, |manifest| {
                for (key, value) in invalid.as_object().unwrap() {
                    if key == "generation" {
                        for (key, value) in value.as_object().unwrap() {
                            manifest["generation"][key] = value.clone();
                        }
                    } else {
                        manifest[key] = value.clone();
                    }
                }
            });
            assert!(checked_archive(archive).is_err());
        }
        let temp = tempfile::tempdir().unwrap();
        let mut archive = valid_archive(temp.path());
        mutate_manifest(&mut archive, |manifest| {
            manifest["publisher"] = json!({"name":"Example"})
        });
        let manifest = serde_json::to_value(checked_archive(archive).unwrap().manifest).unwrap();
        assert!(manifest["publisher"].get("id").is_none());
    }

    #[test]
    fn signatures_ambiguous_markers_and_arbitrary_launch_commands_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let mut archive = valid_archive(temp.path());
        mutate_manifest(&mut archive, |manifest| {
            manifest["publisher"] = json!({"name":"claimed","signatureRequired":true})
        });
        assert!(checked_archive(archive)
            .unwrap_err_text()
            .contains("signature"));
        let temp = tempfile::tempdir().unwrap();
        let mut archive = valid_archive(temp.path());
        insert(
            &mut archive.files,
            "META-INF/signatures/signer.sig",
            b"fake".to_vec(),
            false,
        )
        .unwrap();
        reseal(&mut archive);
        assert!(checked_archive(archive).is_err());
        let temp = tempfile::tempdir().unwrap();
        let mut archive = valid_archive(temp.path());
        insert(
            &mut archive.files,
            "crexp.yaml",
            b"format: crexe.package".to_vec(),
            false,
        )
        .unwrap();
        reseal(&mut archive);
        assert!(checked_archive(archive).is_err());
        let temp = tempfile::tempdir().unwrap();
        let mut archive = valid_archive(temp.path());
        let launch = archive
            .files
            .iter_mut()
            .find(|(name, _)| name.ends_with("/launch.json"))
            .unwrap()
            .1;
        let mut value: Value = serde_json::from_slice(&launch.bytes).unwrap();
        value["entrypoint"] = json!("powershell");
        value["arguments"] = json!(["-EncodedCommand", "ignored"]);
        launch.bytes = serde_json::to_vec(&value).unwrap();
        reseal(&mut archive);
        assert!(checked_archive(archive).is_err());
    }

    #[test]
    fn native_headers_and_architectures_are_checked() {
        for os in ["windows", "linux", "macos"] {
            for arch in ["x64", "arm64"] {
                native_binary(&binary(os, arch), os, arch).unwrap();
            }
            assert!(native_binary(&binary(os, "x64"), os, "arm64").is_err());
            assert!(native_binary(b"#!/bin/sh\necho bypass", os, "x64").is_err());
        }
    }

    #[test]
    fn hydrate_rechecks_every_file_and_keeps_mutable_data_outside() {
        let temp = tempfile::tempdir().unwrap();
        let archive = valid_archive(temp.path());
        let object = temp.path().join("object");
        let data = temp.path().join("data");
        fs::create_dir(&data).unwrap();
        fs::write(data.join("document.txt"), "user data").unwrap();
        hydrate(&archive, &object).unwrap();
        hydrate(&archive, &object).unwrap();
        fs::write(object.join("source/main.cpp"), "cache tamper").unwrap();
        assert!(hydrate(&archive, &object).is_err());
        assert_eq!(
            fs::read_to_string(data.join("document.txt")).unwrap(),
            "user data"
        );
    }

    #[test]
    fn hydrate_blocks_incomplete_and_extra_cache_content() {
        let temp = tempfile::tempdir().unwrap();
        let archive = valid_archive(temp.path());
        let incomplete = temp.path().join("incomplete");
        fs::create_dir(&incomplete).unwrap();
        assert!(hydrate(&archive, &incomplete).is_err());
        let object = temp.path().join("complete");
        hydrate(&archive, &object).unwrap();
        fs::write(object.join("extra.dll"), "injected library").unwrap();
        assert!(hydrate(&archive, &object).is_err());
    }

    #[test]
    fn extract_is_exact_and_never_overwrites_a_recipe() {
        let temp = tempfile::tempdir().unwrap();
        valid_archive(temp.path());
        let output = temp.path().join("extracted.crexe");
        extract_recipe(&temp.path().join("app.crexb"), &output).unwrap();
        assert_eq!(fs::read(&output).unwrap(), b"Original accepted intent");
        assert!(extract_recipe(&temp.path().join("app.crexb"), &output).is_err());
        assert!(
            extract_recipe(&temp.path().join("app.crexb"), &temp.path().join("bad.exe")).is_err()
        );
    }

    #[test]
    fn original_crexp_zip_bytes_are_kept_verbatim() {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("sdd/01-requirements.md", options).unwrap();
        zip.write_all("# Intenção\nUm aplicativo de teste.\n".as_bytes())
            .unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        let original = OriginalRecipe::new("crexp-package", bytes.clone()).unwrap();
        assert_eq!(original.bytes, bytes);
        assert!(OriginalRecipe::new("crexe-text", bytes).is_err());
    }

    trait ErrorText {
        fn unwrap_err_text(self) -> String;
    }
    impl<T> ErrorText for Result<T> {
        fn unwrap_err_text(self) -> String {
            match self {
                Ok(_) => panic!("expected failure"),
                Err(e) => e.to_string(),
            }
        }
    }
}
