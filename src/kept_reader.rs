//! A kept reader is a local, immutable accepted payload, independent of generation.
//! The document is only a native argument to the final application, never a prompt.
use crate::{cache, executor, history, installation, paths, presentation, project};
use anyhow::{bail, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const CONTRACT: u32 = 2;
const MAX_FILE: usize = 128 * 1024 * 1024;
const MAX_TOTAL: usize = 256 * 1024 * 1024;
const MAX_FILES: usize = 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct KeptReader {
    schema: u32,
    pub extension: String,
    pub object_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u32,
    reader_contract: u32,
    extension: String,
    os: String,
    arch: String,
    profile: String,
    entrypoint: String,
    secret_names: Vec<String>,
    files: BTreeMap<String, String>,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn normalized(extension: &str) -> Result<String> {
    let extension = extension
        .strip_prefix('.')
        .unwrap_or(extension)
        .to_lowercase();
    if extension.is_empty()
        || extension.len() > 64
        || !extension
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
    {
        bail!("O programa padrão requer uma extensão de documento válida.");
    }
    if [
        "crexe",
        "crexp",
        "crexb",
        "exe",
        "com",
        "dll",
        "bat",
        "cmd",
        "ps1",
        "psm1",
        "psd1",
        "vbs",
        "vbe",
        "js",
        "jse",
        "wsf",
        "wsh",
        "msi",
        "msp",
        "mst",
        "scr",
        "pif",
        "lnk",
        "url",
        "reg",
        "cpl",
        "hta",
        "appref-ms",
        "scf",
        "application",
        "appx",
        "msix",
        "appxbundle",
        "msixbundle",
        "sys",
        "drv",
        "ocx",
        "jar",
        "py",
        "pyw",
        "sh",
        "bash",
        "zsh",
        "fish",
        "desktop",
        "webloc",
        "workflow",
        "command",
    ]
    .contains(&extension.as_str())
    {
        bail!("Sem suporte para manter como programa padrão de .{extension}: executáveis, scripts, atalhos e formatos CREXE não são documentos deste fluxo.");
    }
    Ok(extension)
}

pub(crate) fn document_extension(file: &Path) -> Result<String> {
    normalized(
        file.extension()
            .and_then(|value| value.to_str())
            .context("Sem suporte: o documento precisa de uma extensão válida.")?,
    )
}

fn root() -> Result<PathBuf> {
    Ok(installation::reader_root()?.join("reader-apps"))
}

fn index(root: &Path, extension: &str) -> PathBuf {
    root.join("extensions")
        .join(format!("{}.json", hash(extension.as_bytes())))
}

fn lock(root: &Path, extension: &str) -> Result<fs::File> {
    let directory = root.join("extensions");
    paths::reject_links(&directory)?;
    fs::create_dir_all(&directory)?;
    let file = directory.join(format!("{}.lock", hash(extension.as_bytes())));
    paths::reject_links(&file)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(file)?;
    lock.try_lock_exclusive()
        .context("Outro processo está preparando este leitor. Tente novamente após concluir.")?;
    Ok(lock)
}

fn read(path: &Path, limit: usize) -> Result<Vec<u8>> {
    paths::reject_links(path)?;
    let source = fs::File::open(path)?;
    let metadata = source.metadata()?;
    if !metadata.is_file() || metadata.len() > limit as u64 {
        bail!(
            "Arquivo de leitor inválido ou grande demais: {}",
            path.display()
        );
    }
    let mut bytes = Vec::new();
    source.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        bail!("Arquivo de leitor excedeu o limite");
    }
    Ok(bytes)
}

fn atomic_json(file: &Path, value: &impl Serialize) -> Result<()> {
    paths::reject_links(file)?;
    let directory = file.parent().context("Missing reader directory")?;
    fs::create_dir_all(directory)?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.write_all(&serde_json::to_vec_pretty(value)?)?;
    temporary.as_file().sync_all()?;
    temporary.persist(file).map_err(|error| error.error)?;
    Ok(())
}

fn native_name() -> &'static str {
    if cfg!(windows) {
        "CrexeApp.exe"
    } else {
        "CrexeApp"
    }
}

fn binary_arch() -> Result<&'static str> {
    match std::env::consts::ARCH {
        "x86_64" => Ok("x64"),
        "aarch64" => Ok("arm64"),
        _ => bail!("Arquitetura sem suporte para leitores mantidos"),
    }
}

fn supported_profile(profile: &str, os: &str) -> bool {
    matches!(
        (profile, os),
        ("dotnet-console", "windows" | "linux" | "macos")
            | ("dotnet-winforms" | "cpp-win32", "windows")
            | ("objc-cocoa", "macos")
            | ("c-gtk", "linux")
    )
}

fn validate_manifest(manifest: &Manifest, extension: &str) -> Result<()> {
    if manifest.schema != 1
        || manifest.reader_contract != CONTRACT
        || manifest.extension != extension
        || normalized(&manifest.extension)? != extension
        || manifest.os != std::env::consts::OS
        || manifest.arch != std::env::consts::ARCH
        || !supported_profile(&manifest.profile, &manifest.os)
        || manifest.entrypoint != format!("app/{}", native_name())
        || manifest.files.is_empty()
        || manifest.files.len() > MAX_FILES
        || manifest.secret_names.len() > 128
        || manifest.secret_names.iter().any(|name| {
            name.is_empty() || name.len() > 256 || name.chars().any(|c| c.is_control() || c == '=')
        })
    {
        bail!(
            "Leitor mantido incompatível ou manifesto inválido; prepare novamente explicitamente."
        );
    }
    let mut names = BTreeSet::new();
    for (name, digest) in &manifest.files {
        paths::relative_file(name)?;
        if name.contains('\\')
            || !["app/", "source/", "recipe/"]
                .iter()
                .any(|prefix| name.starts_with(prefix))
            || !names.insert(name.to_lowercase())
            || !valid_hash(digest)
        {
            bail!("Inventário de leitor inválido");
        }
    }
    if !manifest.files.contains_key(&manifest.entrypoint) {
        bail!("Leitor sem executável");
    }
    if manifest.profile.starts_with("dotnet-") {
        for name in [
            "CrexeApp.dll",
            "CrexeApp.deps.json",
            "CrexeApp.runtimeconfig.json",
        ] {
            if !manifest.files.contains_key(&format!("app/{name}")) {
                bail!("Payload .NET incompleto: {name}");
            }
        }
    }
    Ok(())
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn collect(root: &Path, directory: &Path, names: &mut Vec<String>) -> Result<()> {
    paths::reject_links(directory)?;
    for entry in fs::read_dir(directory)? {
        let file = entry?.path();
        paths::reject_links(&file)?;
        let relative = file.strip_prefix(root)?;
        if relative.components().count() > 16 || names.len() > MAX_FILES {
            bail!("Inventário de leitor excede limites");
        }
        let metadata = fs::symlink_metadata(&file)?;
        if metadata.is_dir() {
            collect(root, &file, names)?;
        } else if metadata.is_file() {
            names.push(relative.to_string_lossy().replace('\\', "/"));
        } else {
            bail!("Leitor contém arquivo especial");
        }
    }
    Ok(())
}

fn verify(root: &Path, kept: &KeptReader) -> Result<(PathBuf, Manifest)> {
    if kept.schema != 1
        || !valid_hash(&kept.object_id)
        || normalized(&kept.extension)? != kept.extension
    {
        bail!("Índice de leitor inválido");
    }
    let object = root.join("objects").join(&kept.object_id);
    let bytes = read(&object.join("manifest.json"), 1024 * 1024)?;
    if hash(&bytes) != kept.object_id {
        bail!("Manifesto do leitor foi alterado");
    }
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    validate_manifest(&manifest, &kept.extension)?;
    let mut actual = Vec::new();
    collect(&object, &object, &mut actual)?;
    actual.sort();
    let mut expected: Vec<_> = manifest
        .files
        .keys()
        .cloned()
        .chain(std::iter::once("manifest.json".into()))
        .collect();
    expected.sort();
    if actual != expected {
        bail!("Leitor mantido contém arquivos ausentes ou não listados");
    }
    let mut total = 0usize;
    for (name, digest) in &manifest.files {
        let file = object.join(paths::relative_file(name)?);
        let bytes = read(&file, MAX_FILE)?;
        total = total
            .checked_add(bytes.len())
            .context("Reader size overflow")?;
        if total > MAX_TOTAL || hash(&bytes) != *digest {
            bail!("Leitor mantido foi alterado; prepare novamente explicitamente.");
        }
        if name == &manifest.entrypoint {
            crate::crexb::native_binary(&bytes, &manifest.os, binary_arch()?)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if fs::metadata(file)?.permissions().mode() & 0o111 == 0 {
                    bail!("Leitor perdeu permissão de execução");
                }
            }
        }
    }
    Ok((object, manifest))
}

fn current_at(root: &Path, extension: &str) -> Result<Option<KeptReader>> {
    let extension = normalized(extension)?;
    let file = index(root, &extension);
    paths::reject_links(&file)?;
    if !file.exists() {
        return Ok(None);
    }
    let kept: KeptReader = serde_json::from_slice(&read(&file, 4096)?)?;
    if kept.extension != extension {
        bail!("Índice corresponde a outra extensão");
    }
    verify(root, &kept).with_context(|| format!(
        "Leitor mantido inválido. Use crexe reader-defaults forget --extension {extension}. Para reutilizar uma geração idêntica, feche o leitor e remova explicitamente o objeto {}; depois prepare novamente.",
        root.join("objects").join(&kept.object_id).display()))?;
    Ok(Some(kept))
}

pub(crate) fn ensure_registered(
    extension: &str,
    register: &mut dyn FnMut(&KeptReader) -> Result<()>,
) -> Result<bool> {
    let root = root()?;
    ensure_registered_at(&root, extension, register)
}

fn ensure_registered_at(
    root: &Path,
    extension: &str,
    register: &mut dyn FnMut(&KeptReader) -> Result<()>,
) -> Result<bool> {
    let extension = normalized(extension)?;
    let _lock = lock(root, &extension)?;
    if let Some(kept) = current_at(root, &extension)? {
        register(&kept)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub(crate) fn keep(
    extension: &str,
    revision: &Path,
    secret_names: &[String],
    register: &mut dyn FnMut(&KeptReader) -> Result<()>,
    rollback: &mut dyn FnMut() -> Result<()>,
) -> Result<KeptReader> {
    keep_at(
        &root()?,
        extension,
        revision,
        secret_names,
        register,
        rollback,
    )
}

fn keep_at(
    root: &Path,
    extension: &str,
    revision: &Path,
    secret_names: &[String],
    register: &mut dyn FnMut(&KeptReader) -> Result<()>,
    rollback: &mut dyn FnMut() -> Result<()>,
) -> Result<KeptReader> {
    let extension = normalized(extension)?;
    let _lock = lock(root, &extension)?;
    let had_previous = current_at(root, &extension)?.is_some();
    let parent = revision.parent().context("Missing accepted cache root")?;
    let entry = cache::Entry {
        root: parent.into(),
        fingerprint: parent
            .file_name()
            .context("Missing cache identity")?
            .to_string_lossy()
            .into(),
    };
    let _accepted_lock = entry.lock()?;
    if entry.current().as_deref() != Some(revision) {
        bail!("Somente a revisão aceita e íntegra pode ser mantida");
    }
    let pointer: serde_json::Value =
        serde_json::from_slice(&read(&parent.join("current.json"), 2 * 1024 * 1024)?)?;
    let accepted: BTreeMap<String, String> = serde_json::from_value(pointer["files"].clone())?;
    if accepted.len() > MAX_FILES {
        bail!("Revisão aceita excede limite de arquivos");
    }
    let project = project::Project::load(revision)?;
    let metadata: history::Metadata =
        serde_json::from_slice(&read(&revision.join(history::METADATA), 2 * 1024 * 1024)?)?;
    if project.name != "CREXEReader"
        || project.profile != metadata.profile
        || !metadata.prompt.contains("ENGINE READER CONTRACT v2")
        || metadata.sources.is_empty()
        || metadata
            .sources
            .iter()
            .any(|(name, hash)| accepted.get(name) != Some(hash))
    {
        bail!("A revisão não é um leitor aceito do contrato atual");
    }
    let mut files = BTreeMap::<String, Vec<u8>>::new();
    let mut total = 0usize;
    for (name, expected) in &accepted {
        let payload = if project.profile.starts_with("dotnet-") {
            name.strip_prefix("build/")
        } else {
            None
        };
        let destination = if let Some(relative) = payload {
            Some(format!("app/{relative}"))
        } else if !project.profile.starts_with("dotnet-") && name == native_name() {
            Some(format!("app/{name}"))
        } else if metadata.sources.contains_key(name) {
            Some(format!("source/{name}"))
        } else if name == "CREXE-ORIGINAL-RECIPE.bin" {
            Some("recipe/original.crexe".into())
        } else {
            None
        };
        let Some(destination) = destination else {
            continue;
        };
        let bytes = read(&revision.join(paths::relative_file(name)?), MAX_FILE)?;
        total = total
            .checked_add(bytes.len())
            .context("Reader size overflow")?;
        if total > MAX_TOTAL || hash(&bytes) != *expected {
            bail!("A revisão mudou durante a preservação do leitor");
        }
        if let Some(source_hash) = metadata.sources.get(name) {
            if source_hash != expected {
                bail!("Inventário dos fontes aceitos diverge do cache");
            }
        }
        files.insert(destination, bytes);
    }
    files.insert(
        "recipe/resolved.yaml".into(),
        serde_yaml::to_string(&metadata.spec)?.into_bytes(),
    );
    let mut secrets = secret_names.to_vec();
    secrets.sort();
    secrets.dedup();
    let manifest = Manifest {
        schema: 1,
        reader_contract: CONTRACT,
        extension: extension.clone(),
        os: project.os,
        arch: project.arch,
        profile: project.profile,
        entrypoint: format!("app/{}", native_name()),
        secret_names: secrets,
        files: files
            .iter()
            .map(|(name, bytes)| (name.clone(), hash(bytes)))
            .collect(),
    };
    validate_manifest(&manifest, &extension)?;
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    let kept = KeptReader {
        schema: 1,
        extension,
        object_id: hash(&bytes),
    };
    let objects = root.join("objects");
    paths::reject_links(&objects)?;
    fs::create_dir_all(&objects)?;
    let object = objects.join(&kept.object_id);
    paths::reject_links(&object)?;
    if !object.exists() {
        let stage = tempfile::Builder::new()
            .prefix("staging-")
            .tempdir_in(&objects)?;
        for (name, contents) in &files {
            let target = stage.path().join(paths::relative_file(name)?);
            fs::create_dir_all(target.parent().unwrap())?;
            fs::write(&target, contents)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(
                    &target,
                    fs::Permissions::from_mode(if name == &manifest.entrypoint {
                        0o700
                    } else {
                        0o600
                    }),
                )?;
            }
        }
        fs::write(stage.path().join("manifest.json"), &bytes)?;
        // Validate before publishing; the temporary directory is never an index target.
        crate::crexb::native_binary(&files[&manifest.entrypoint], &manifest.os, binary_arch()?)?;
        fs::rename(stage.path(), &object)?;
    }
    verify(root, &kept)?;
    register(&kept)?;
    if let Err(error) = atomic_json(&index(root, &kept.extension), &kept) {
        if !had_previous {
            if let Err(rollback_error) = rollback() {
                return Err(error).context(format!("O índice não foi publicado e a remoção do candidato falhou: {rollback_error:#}"));
            }
        }
        return Err(error).context(
            "O índice do programa mantido não foi publicado; a revisão anterior foi preservada.",
        );
    }
    Ok(kept)
}

pub(crate) fn open(
    document: &Path,
    expected_extension: &str,
    ui: bool,
    ready: &mut dyn FnMut(),
) -> Result<()> {
    let extension = document_extension(document)?;
    if extension != normalized(expected_extension)? {
        bail!("Este leitor foi registrado para outra extensão");
    }
    let document = document
        .canonicalize()
        .context("Não foi possível localizar o documento")?;
    if !fs::metadata(&document)?.is_file() {
        bail!("O leitor recebe um arquivo regular por vez");
    }
    let _progress = presentation::Progress::start(ui)?;
    presentation::phase("Abrindo com programa mantido — sem IA ou compilação…");
    let root = root()?;
    let kept = current_at(&root, &extension)?.context("Nenhum programa mantido para esta extensão. Use Gerar com CREXE e Manter como Programa Padrão explicitamente.")?;
    let (object, manifest) = verify(&root, &kept)?;
    let cwd = root.join("state").join(hash(extension.as_bytes()));
    paths::reject_links(&cwd)?;
    fs::create_dir_all(&cwd)?;
    let executable = object.join(paths::relative_file(&manifest.entrypoint)?);
    let executable = executable
        .to_str()
        .context("Caminho do leitor não é Unicode")?
        .to_owned();
    let mut started = || {
        presentation::finish();
        ready();
    };
    executor::launch_with_args(&[executable], &[document.into_os_string()], &cwd, None,
        &manifest.secret_names, &mut started)
        .context("Não foi possível abrir o leitor mantido. Verifique o runtime (.NET 8 para leitores C#). Nenhuma geração ou compilação foi iniciada.")
}

pub(crate) fn list() -> Result<()> {
    let root = root()?;
    let directory = root.join("extensions");
    paths::reject_links(&directory)?;
    let mut result = Vec::new();
    if directory.exists() {
        for entry in fs::read_dir(directory)? {
            let file = entry?.path();
            if file.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let kept: KeptReader = serde_json::from_slice(&read(&file, 4096)?)?;
            if index(&root, &kept.extension) != file {
                bail!("Índice de leitor em caminho inválido");
            }
            verify(&root, &kept)?;
            result.push(kept);
        }
    }
    result.sort_by(|a, b| a.extension.cmp(&b.extension));
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub(crate) fn forget(extension: &str, unregister: &mut dyn FnMut() -> Result<()>) -> Result<()> {
    forget_at(&root()?, extension, unregister)
}

fn forget_at(
    root: &Path,
    extension: &str,
    unregister: &mut dyn FnMut() -> Result<()>,
) -> Result<()> {
    forget_inner(root, extension, unregister, &|from, to| {
        fs::rename(from, to)
    })
}

fn forget_inner(
    root: &Path,
    extension: &str,
    unregister: &mut dyn FnMut() -> Result<()>,
    move_index: &dyn Fn(&Path, &Path) -> std::io::Result<()>,
) -> Result<()> {
    let extension = normalized(extension)?;
    let _lock = lock(root, &extension)?;
    let file = index(root, &extension);
    paths::reject_links(&file)?;
    // Move the pointer before touching the registry. A locked/read-only index
    // must not leave a formerly working default app without its registration.
    let pending = if file.exists() {
        if !fs::metadata(&file)?.is_file() {
            bail!("O índice do leitor não é um arquivo regular; nenhum registro foi removido.");
        }
        let directory = tempfile::Builder::new()
            .prefix("forget-")
            .tempdir_in(file.parent().unwrap())?;
        move_index(&file, &directory.path().join("index.json")).context(
            "Não foi possível preparar a remoção do índice; nenhum registro foi removido",
        )?;
        Some(directory)
    } else {
        None
    };
    if let Err(error) = unregister() {
        if let Some(directory) = pending {
            if let Err(restore) = fs::rename(directory.path().join("index.json"), &file) {
                // Never let TempDir cleanup destroy the only remaining pointer.
                let preserved = directory.keep();
                return Err(error).context(format!("O registro não foi removido e o índice não pôde ser restaurado ({restore}). Cópia preservada em {}", preserved.display()));
            }
        }
        return Err(error);
    }
    if let Some(directory) = pending {
        if fs::remove_file(directory.path().join("index.json")).is_err() {
            // The live index and registration are gone. An inert tombstone can
            // remain if Windows still holds a handle; never restore a stale app.
            let preserved = directory.keep();
            eprintln!(
                "Leitor removido; índice antigo inativo preservado em {}",
                preserved.display()
            );
        }
    }
    // Objects deliberately survive: an already-running reader may still use them.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keep_test(
        root: &Path,
        extension: &str,
        revision: &Path,
        secret_names: &[String],
        register: &mut dyn FnMut(&KeptReader) -> Result<()>,
    ) -> Result<KeptReader> {
        keep_at(
            root,
            extension,
            revision,
            secret_names,
            register,
            &mut || Ok(()),
        )
    }

    fn binary() -> Vec<u8> {
        let mut bytes = vec![0; 128];
        if cfg!(windows) {
            bytes[..2].copy_from_slice(b"MZ");
            bytes[60..64].copy_from_slice(&64u32.to_le_bytes());
            bytes[64..68].copy_from_slice(b"PE\0\0");
            let machine: u16 = if cfg!(target_arch = "aarch64") {
                0xaa64
            } else {
                0x8664
            };
            bytes[68..70].copy_from_slice(&machine.to_le_bytes());
        } else if cfg!(target_os = "macos") {
            bytes[..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
            let machine: u32 = if cfg!(target_arch = "aarch64") {
                0x0100000c
            } else {
                0x01000007
            };
            bytes[4..8].copy_from_slice(&machine.to_le_bytes());
            bytes[12..16].copy_from_slice(&2u32.to_le_bytes());
        } else {
            bytes[..4].copy_from_slice(b"\x7fELF");
            bytes[4] = 2;
            bytes[5] = 1;
            bytes[16..18].copy_from_slice(&2u16.to_le_bytes());
            let machine: u16 = if cfg!(target_arch = "aarch64") {
                183
            } else {
                62
            };
            bytes[18..20].copy_from_slice(&machine.to_le_bytes());
        }
        bytes
    }

    fn accepted(directory: &Path, dotnet: bool, marker: u8) -> PathBuf {
        fs::create_dir_all(directory).unwrap();
        let work = tempfile::tempdir_in(directory).unwrap();
        let profile = if dotnet {
            "dotnet-console"
        } else if cfg!(windows) {
            "cpp-win32"
        } else if cfg!(target_os = "macos") {
            "objc-cocoa"
        } else {
            "c-gtk"
        };
        let source = if dotnet { "Program.cs" } else { "main.c" };
        fs::write(work.path().join(source), "test fixture source").unwrap();
        let project = json!({"schema":1,"name":"CREXEReader", "os":std::env::consts::OS,
            "arch":std::env::consts::ARCH,"profile":profile,"sources":[source],
            "commands":{"build":[["fixture-compiler"]],"run":[["fixture-run"]]}});
        fs::write(
            work.path().join(project::MANIFEST),
            serde_json::to_vec(&project).unwrap(),
        )
        .unwrap();
        let app_dir = if dotnet {
            work.path().join("build")
        } else {
            work.path().to_owned()
        };
        fs::create_dir_all(&app_dir).unwrap();
        let mut native = binary();
        native[110] = marker;
        fs::write(app_dir.join(native_name()), native).unwrap();
        if dotnet {
            for name in [
                "CrexeApp.dll",
                "CrexeApp.deps.json",
                "CrexeApp.runtimeconfig.json",
            ] {
                fs::write(app_dir.join(name), "synthetic-runtime-fixture").unwrap();
            }
        }
        let spec = serde_yaml::from_str("meta:\n  name: CREXEReader\n").unwrap();
        let mut metadata = history::Metadata::new(
            "request-fixture".into(),
            None,
            "ENGINE READER CONTRACT v2".into(),
            spec,
            json!({"provider":"synthetic"}),
        );
        metadata.write(work.path()).unwrap();
        fs::write(
            work.path().join("CREXE-ORIGINAL-RECIPE.bin"),
            "synthetic recipe",
        )
        .unwrap();
        let root = directory.join("cache-entry");
        let entry = cache::Entry {
            root,
            fingerprint: "cache-entry".into(),
        };
        let _guard = entry.lock().unwrap();
        entry.publish(work.path()).unwrap()
    }

    #[test]
    fn extensions_are_normalized_without_inspecting_document_metadata() {
        assert_eq!(
            document_extension(Path::new("/does/not/exist/ação.TXT")).unwrap(),
            "txt"
        );
        assert_eq!(normalized(".EML").unwrap(), "eml");
        for name in [
            "empty",
            "reader.CREXB",
            "script.PS1",
            "run.exe",
            "link.lnk",
            "script.py",
            "a.bad extension",
            "a.x:y",
        ] {
            assert!(document_extension(Path::new(name)).is_err(), "{name}");
        }
    }

    #[test]
    fn keeps_accepted_payload_outside_cache_and_deduplicates_without_documents() {
        let temp = tempfile::tempdir().unwrap();
        let revision = accepted(&temp.path().join("generation"), false, 1);
        let root = temp.path().join("reader-apps");
        let mut registered = Vec::new();
        let first = keep_test(
            &root,
            "txt",
            &revision,
            &["MY_PROVIDER_KEY".into()],
            &mut |kept| {
                assert!(current_at(&root, "txt")?.is_none());
                registered.push(kept.extension.clone());
                Ok(())
            },
        )
        .unwrap();
        let second = keep_test(
            &root,
            "txt",
            &revision,
            &["MY_PROVIDER_KEY".into()],
            &mut |_| Ok(()),
        )
        .unwrap();
        assert_eq!(first.object_id, second.object_id);
        assert_eq!(registered, ["txt"]);
        fs::remove_dir_all(temp.path().join("generation")).unwrap();
        let kept = current_at(&root, "txt").unwrap().unwrap();
        let (object, manifest) = verify(&root, &kept).unwrap();
        assert!(object.join("app").join(native_name()).is_file());
        assert_eq!(manifest.secret_names, ["MY_PROVIDER_KEY"]);
        assert!(object.join("recipe/original.crexe").is_file());
        assert!(object.join("source").is_dir());
        assert!(current_at(&root, "eml").unwrap().is_none());
    }

    #[test]
    fn registration_failure_and_bad_revision_preserve_previous_reader() {
        let temp = tempfile::tempdir().unwrap();
        let generation = temp.path().join("generation");
        let revision = accepted(&generation, false, 1);
        let root = temp.path().join("reader-apps");
        let first = keep_test(&root, "txt", &revision, &[], &mut |_| Ok(())).unwrap();
        let new_revision = accepted(&generation, false, 2);
        assert!(keep_test(&root, "txt", &new_revision, &[], &mut |_| bail!(
            "registry unavailable"
        ))
        .is_err());
        assert_eq!(
            current_at(&root, "txt").unwrap().unwrap().object_id,
            first.object_id
        );
        fs::write(new_revision.join(native_name()), "modified").unwrap();
        assert!(keep_test(&root, "txt", &new_revision, &[], &mut |_| panic!(
            "must not register invalid payload"
        ))
        .is_err());
        assert_eq!(
            current_at(&root, "txt").unwrap().unwrap().object_id,
            first.object_id
        );
    }

    #[test]
    fn net_payload_includes_runtime_files_and_rejects_missing_or_modified_files() {
        let temp = tempfile::tempdir().unwrap();
        let revision = accepted(&temp.path().join("generation"), true, 1);
        let root = temp.path().join("reader-apps");
        let kept = keep_test(&root, "txt", &revision, &[], &mut |_| Ok(())).unwrap();
        let (object, manifest) = verify(&root, &kept).unwrap();
        assert_eq!(
            manifest
                .files
                .keys()
                .filter(|name| name.starts_with("app/"))
                .count(),
            4
        );
        fs::write(object.join("app/CrexeApp.dll"), "corrupt").unwrap();
        assert!(current_at(&root, "txt").is_err());
        fs::remove_file(object.join("app/CrexeApp.dll")).unwrap();
        assert!(current_at(&root, "txt").is_err());
    }

    #[test]
    fn manifest_index_and_unlisted_payload_are_checked_without_fallback_generation() {
        let temp = tempfile::tempdir().unwrap();
        let revision = accepted(&temp.path().join("generation"), false, 1);
        let root = temp.path().join("reader-apps");
        let kept = keep_test(&root, "txt", &revision, &[], &mut |_| Ok(())).unwrap();
        let object = root.join("objects").join(&kept.object_id);
        fs::write(object.join("app/unlisted.txt"), "unexpected").unwrap();
        assert!(current_at(&root, "txt").is_err());
        fs::remove_file(object.join("app/unlisted.txt")).unwrap();
        let manifest = fs::read(object.join("manifest.json")).unwrap();
        fs::write(object.join("manifest.json"), "{}").unwrap();
        assert!(current_at(&root, "txt").is_err());
        fs::write(object.join("manifest.json"), manifest).unwrap();
        let mut bad = kept.clone();
        bad.extension = "eml".into();
        atomic_json(&index(&root, "txt"), &bad).unwrap();
        assert!(current_at(&root, "txt").is_err());
    }

    #[test]
    fn locks_cover_promotion_only_and_forgetting_preserves_running_payloads() {
        let temp = tempfile::tempdir().unwrap();
        let revision = accepted(&temp.path().join("generation"), false, 1);
        let root = temp.path().join("reader-apps");
        let kept = keep_test(&root, "txt", &revision, &[], &mut |_| {
            assert!(lock(&root, "txt").is_err());
            Ok(())
        })
        .unwrap();
        let guard = lock(&root, "txt").unwrap();
        assert!(current_at(&root, "txt").unwrap().is_some());
        drop(guard);
        forget_at(&root, "txt", &mut || Ok(())).unwrap();
        forget_at(&root, "txt", &mut || Ok(())).unwrap();
        assert!(current_at(&root, "txt").unwrap().is_none());
        assert!(root.join("objects").join(kept.object_id).is_dir());
    }

    #[test]
    fn registration_reuse_forget_and_index_failure_stay_under_the_same_lock() {
        let temp = tempfile::tempdir().unwrap();
        let revision = accepted(&temp.path().join("generation"), false, 1);
        let root = temp.path().join("reader-apps");
        assert!(!ensure_registered_at(&root, "txt", &mut |_| panic!("no candidate yet")).unwrap());
        let kept = keep_test(&root, "txt", &revision, &[], &mut |_| Ok(())).unwrap();
        assert!(ensure_registered_at(&root, "txt", &mut |_| {
            assert!(lock(&root, "txt").is_err());
            Ok(())
        })
        .unwrap());
        assert!(forget_at(&root, "txt", &mut || {
            assert!(lock(&root, "txt").is_err());
            bail!("registration still owned")
        })
        .is_err());
        assert_eq!(
            current_at(&root, "txt").unwrap().unwrap().object_id,
            kept.object_id
        );
        forget_at(&root, "txt", &mut || Ok(())).unwrap();
        let mut rolled_back = false;
        let failed = keep_at(
            &root,
            "txt",
            &revision,
            &[],
            &mut |_| {
                // Simulate a filesystem failure after the external registration succeeds.
                fs::create_dir(index(&root, "txt"))?;
                Ok(())
            },
            &mut || {
                assert!(lock(&root, "txt").is_err());
                rolled_back = true;
                fs::remove_dir(index(&root, "txt"))?;
                Ok(())
            },
        );
        assert!(failed.is_err());
        assert!(rolled_back);
        assert!(current_at(&root, "txt").unwrap().is_none());
    }

    #[test]
    fn forget_move_failure_preserves_index_without_unregistering() {
        let temp = tempfile::tempdir().unwrap();
        let revision = accepted(&temp.path().join("generation"), false, 1);
        let root = temp.path().join("reader-apps");
        let kept = keep_test(&root, "txt", &revision, &[], &mut |_| Ok(())).unwrap();
        let result = forget_inner(
            &root,
            "txt",
            &mut || panic!("must not unregister when moving the index fails"),
            &|_, _| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "locked index",
                ))
            },
        );
        assert!(result.is_err());
        assert_eq!(
            current_at(&root, "txt").unwrap().unwrap().object_id,
            kept.object_id
        );
        let before = fs::read(index(&root, "txt")).unwrap();
        assert!(forget_at(&root, "txt", &mut || {
            assert!(!index(&root, "txt").exists());
            bail!("registry operation failed")
        })
        .is_err());
        assert_eq!(fs::read(index(&root, "txt")).unwrap(), before);
        assert_eq!(
            current_at(&root, "txt").unwrap().unwrap().object_id,
            kept.object_id
        );
    }

    #[test]
    fn rejects_incompatible_contracts_platforms_and_paths_even_with_a_rehashed_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let revision = accepted(&temp.path().join("generation"), false, 1);
        let root = temp.path().join("reader-apps");
        let kept = keep_test(&root, "txt", &revision, &[], &mut |_| Ok(())).unwrap();
        let (_, original) = verify(&root, &kept).unwrap();
        let value = serde_json::to_value(original).unwrap();
        for (field, replacement) in [
            ("reader_contract", json!(999)),
            ("os", json!("other-os")),
            ("arch", json!("other-arch")),
            ("entrypoint", json!("../../other.exe")),
            ("profile", json!("custom")),
            ("extension", json!("exe")),
        ] {
            let mut altered = value.clone();
            altered[field] = replacement;
            let manifest: Manifest = serde_json::from_value(altered).unwrap();
            assert!(
                validate_manifest(&manifest, "txt").is_err(),
                "accepted {field}"
            );
        }
        for path in [
            "app/../outside",
            "app/C:stream",
            "app/CON",
            "source\\bypass",
        ] {
            let mut altered = value.clone();
            altered["files"][path] = json!("a".repeat(64));
            let manifest: Manifest = serde_json::from_value(altered).unwrap();
            assert!(
                validate_manifest(&manifest, "txt").is_err(),
                "accepted {path}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn kept_payload_links_are_rejected_before_launch() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let revision = accepted(&temp.path().join("generation"), false, 1);
        let root = temp.path().join("reader-apps");
        let kept = keep_test(&root, "txt", &revision, &[], &mut |_| Ok(())).unwrap();
        let object = root.join("objects").join(&kept.object_id);
        let app = object.join("app").join(native_name());
        let outside = temp.path().join("outside");
        fs::rename(&app, &outside).unwrap();
        symlink(&outside, &app).unwrap();
        assert!(current_at(&root, "txt").is_err());
    }
}
