//! Per-document lineage points only to immutable, approved source snapshots.
use crate::{cache, paths, project::Project, GeneratedFile};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) const METADATA: &str = "CREXE-GENERATION.json";

pub(crate) struct History {
    root: PathBuf,
    index: PathBuf,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pointer {
    schema: u32,
    revision: String,
    metadata_hash: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Metadata {
    schema: u32,
    pub request: String,
    pub parent: Option<String>,
    pub prompt: String,
    pub spec: Value,
    pub profile: String,
    pub parameters: serde_json::Value,
    pub change_kind: Option<String>,
    pub reason: Option<String>,
    pub sources: BTreeMap<String, String>,
}

pub(crate) struct Baseline {
    pub revision: PathBuf,
    pub metadata: Metadata,
}

impl History {
    pub fn new(root: &Path, document: &Path) -> Result<Self> {
        let mut path = document.canonicalize()?.to_string_lossy().into_owned();
        if cfg!(windows) {
            path = path.to_lowercase();
        }
        let id = format!(
            "{:x}",
            Sha256::digest(format!(
                "{}:{}:{path}",
                std::env::consts::OS,
                std::env::consts::ARCH
            ))
        );
        Ok(Self {
            root: root.into(),
            index: root.join(".documents").join(format!("{id}.json")),
        })
    }

    pub fn load(&self) -> Result<Option<Baseline>> {
        paths::reject_links(&self.index)?;
        if !self.index.exists() {
            return Ok(None);
        }
        let pointer: Pointer = serde_json::from_slice(&bounded_read(&self.index, 4096)?)?;
        if pointer.schema != 1 {
            bail!("Unsupported document history");
        }
        let relative = paths::relative_file(&pointer.revision)?;
        if relative.components().count() != 2 {
            bail!("Invalid history revision");
        }
        let revision = self.root.join(relative);
        let metadata_path = revision.join(METADATA);
        paths::reject_links(&metadata_path)?;
        let bytes = bounded_read(&metadata_path, 2 * 1024 * 1024)?;
        if digest(&bytes) != pointer.metadata_hash {
            bail!("Baseline metadata missing or modified; use --fresh to start without it");
        }
        let metadata: Metadata = serde_json::from_slice(&bytes)?;
        if metadata.schema != 1 || metadata.sources.is_empty() || metadata.sources.len() > 132 {
            bail!("Invalid baseline source inventory");
        }
        let baseline = Baseline { revision, metadata };
        // Validate only source files, independently of compiled artifacts.
        baseline.files()?;
        Ok(Some(baseline))
    }

    pub fn publish(&self, revision: &Path) -> Result<()> {
        let relative = revision
            .strip_prefix(&self.root)?
            .to_string_lossy()
            .replace('\\', "/");
        let pointer = Pointer {
            schema: 1,
            revision: relative,
            metadata_hash: cache::hash(&revision.join(METADATA))?,
        };
        paths::reject_links(&self.index)?;
        let parent = self.index.parent().context("Invalid history directory")?;
        fs::create_dir_all(parent)?;
        let mut staged = tempfile::NamedTempFile::new_in(parent)?;
        staged.write_all(&serde_json::to_vec_pretty(&pointer)?)?;
        staged.as_file().sync_all()?;
        staged.persist(&self.index).map_err(|e| e.error)?;
        Ok(())
    }
}

impl Baseline {
    pub fn id(&self) -> String {
        format!(
            "{}/{}",
            self.revision
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy(),
            self.revision.file_name().unwrap().to_string_lossy()
        )
    }
    pub fn files(&self) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();
        let mut total = 0;
        for (relative, hash) in &self.metadata.sources {
            let path = self.revision.join(paths::relative_file(relative)?);
            paths::reject_links(&path)?;
            let bytes = bounded_read(&path, 4 * 1024 * 1024)?;
            total += bytes.len();
            if total > 16 * 1024 * 1024 || digest(&bytes) != *hash {
                bail!("Baseline source missing, modified or too large; use --fresh to start without it");
            }
            if relative != crate::project::MANIFEST {
                files.push(GeneratedFile {
                    path: relative.clone(),
                    content: String::from_utf8(bytes).context("Baseline source must be UTF-8")?,
                });
            }
        }
        Ok(files)
    }
    pub fn cached(&self) -> bool {
        let entry = cache::Entry {
            root: self.revision.parent().unwrap().into(),
            fingerprint: self
                .revision
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        };
        entry.current().as_ref() == Some(&self.revision)
    }
}

impl Metadata {
    pub fn new(
        request: String,
        parent: Option<String>,
        prompt: String,
        spec: Value,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            schema: 1,
            request,
            parent,
            prompt,
            spec,
            profile: String::new(),
            parameters,
            change_kind: None,
            reason: None,
            sources: BTreeMap::new(),
        }
    }
    pub fn write(&mut self, root: &Path) -> Result<()> {
        let project = Project::load(root)?;
        self.profile = project.profile;
        self.sources.clear();
        let mut total = 0;
        for source in project
            .sources
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(crate::project::MANIFEST))
        {
            if excluded(source) {
                bail!("Compiled or temporary file in source inventory: {source}");
            }
            let path = root.join(paths::relative_file(source)?);
            paths::reject_links(&path)?;
            let bytes = bounded_read(&path, 4 * 1024 * 1024)?;
            total += bytes.len();
            if total > 16 * 1024 * 1024 {
                bail!("Baseline source inventory exceeds 16 MiB");
            }
            std::str::from_utf8(&bytes).context("Baseline sources must use UTF-8")?;
            self.sources.insert(source.to_owned(), digest(&bytes));
        }
        fs::write(root.join(METADATA), serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}

fn excluded(path: &str) -> bool {
    let path = path.replace('\\', "/").to_lowercase();
    path.split('/').any(|part| {
        matches!(
            part,
            "bin" | "obj" | "build" | "target" | "node_modules" | ".git"
        )
    }) || [
        "exe", "dll", "pdb", "o", "obj", "so", "dylib", "a", "lib", "zip", "log", "tmp",
    ]
    .contains(&path.rsplit('.').next().unwrap_or(""))
}

fn bounded_read(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let file = fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        bail!("Expected a regular source file");
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        bail!("Baseline file exceeds limit");
    }
    Ok(bytes)
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
