//! Immutable revisions + an atomically replaced current pointer.
use anyhow::{bail, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) struct Entry {
    pub root: PathBuf,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize)]
struct Current {
    schema: u32,
    fingerprint: String,
    revision: String,
    generated_at: String,
    files: BTreeMap<String, String>,
}

impl Entry {
    pub fn lock(&self) -> Result<fs::File> {
        super::paths::reject_links(&self.root)?;
        fs::create_dir_all(&self.root)?;
        let lock_path = self.root.join(".lock");
        super::paths::reject_links(&lock_path)?;
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        file.try_lock_exclusive().context(
            "Another CREXE generation is using this cache entry; retry when it finishes",
        )?;
        Ok(file)
    }

    pub fn current(&self) -> Option<PathBuf> {
        self.checked_current().ok().flatten()
    }

    fn checked_current(&self) -> Result<Option<PathBuf>> {
        let pointer = self.root.join("current.json");
        super::paths::reject_links(&pointer)?;
        if !pointer.exists() {
            return Ok(None);
        }
        if fs::metadata(&pointer)?.len() > 2 * 1024 * 1024 {
            bail!("Invalid cache manifest size");
        }
        let manifest: Current = serde_json::from_slice(&fs::read(pointer)?)?;
        if manifest.schema != 3
            || manifest.fingerprint != self.fingerprint
            || manifest.files.is_empty()
            || !manifest.revision.starts_with("revision-")
            || !manifest
                .revision
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            bail!("Invalid cache manifest");
        }
        let revision = self.root.join(&manifest.revision);
        super::paths::reject_links(&revision)?;
        for (relative, expected) in &manifest.files {
            let path = revision.join(super::paths::relative_file(relative)?);
            super::paths::reject_links(&path)?;
            if !path.is_file() || hash(&path)? != *expected {
                bail!("Cached artifact missing or modified");
            }
        }
        Ok(Some(revision))
    }

    pub fn publish(&self, workspace: &Path) -> Result<PathBuf> {
        let stage = tempfile::Builder::new()
            .prefix("revision-")
            .tempdir_in(&self.root)?;
        let mut files = BTreeMap::new();
        let mut total = 0u64;
        copy_tree(workspace, workspace, stage.path(), &mut files, &mut total)?;
        if files.is_empty() {
            bail!("Cannot publish an empty project");
        }
        let revision = stage
            .path()
            .file_name()
            .context("Missing revision name")?
            .to_string_lossy()
            .to_string();
        let manifest = Current {
            schema: 3,
            fingerprint: self.fingerprint.clone(),
            revision,
            generated_at: chrono::Utc::now().to_rfc3339(),
            files,
        };
        let mut pointer = tempfile::NamedTempFile::new_in(&self.root)?;
        pointer.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
        pointer.as_file().sync_all()?;
        super::paths::reject_links(&self.root.join("current.json"))?;
        // Preserve the completed revision before exposing its pointer. A crash before
        // persist leaves an unreferenced revision, never a pointer to missing files.
        let revision_path = stage.keep();
        pointer
            .persist(self.root.join("current.json"))
            .map_err(|error| error.error)?;
        Ok(revision_path)
    }
}

pub(crate) fn copy_tree(
    root: &Path,
    directory: &Path,
    destination: &Path,
    files: &mut BTreeMap<String, String>,
    total: &mut u64,
) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        let metadata = fs::symlink_metadata(&path)?;
        if super::paths::is_link(&metadata) {
            bail!("Project contains a link: {}", path.display());
        }
        let relative = path.strip_prefix(root)?;
        let relative_text = relative.to_string_lossy().replace('\\', "/");
        super::paths::relative_file(&relative_text)?;
        let target = destination.join(relative);
        if metadata.is_dir() {
            fs::create_dir(&target)?;
            copy_tree(root, &path, destination, files, total)?;
        } else if metadata.is_file() {
            *total = total
                .checked_add(metadata.len())
                .context("Project too large")?;
            if *total > 2 * 1024 * 1024 * 1024 || files.len() >= 4096 {
                bail!("Project exceeds cache limits");
            }
            fs::copy(&path, &target)?;
            files.insert(relative_text, hash(&target)?);
        } else {
            bail!("Unsupported project file: {}", path.display());
        }
    }
    Ok(())
}

fn hash(path: &Path) -> Result<String> {
    let mut source = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let size = source.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        hash.update(&buffer[..size]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publishing_is_atomic_and_revisions_survive_temporary_cleanup() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let entry = Entry {
            root: root.join("cache"),
            fingerprint: "fixture".into(),
        };
        let guard = entry.lock().unwrap();
        assert!(entry.lock().is_err());
        let work = tempfile::tempdir().unwrap();
        fs::write(work.path().join("program"), "one").unwrap();
        let first = entry.publish(work.path()).unwrap();
        fs::write(work.path().join("program"), "two").unwrap();
        let second = entry.publish(work.path()).unwrap();
        drop(work);
        assert_eq!(fs::read_to_string(first.join("program")).unwrap(), "one");
        assert_eq!(fs::read_to_string(second.join("program")).unwrap(), "two");
        assert_eq!(entry.current().unwrap(), second);
        fs::write(second.join("program"), "corrupted").unwrap();
        assert!(entry.current().is_none());
        fs::write(entry.root.join("current.json"), "invalid JSON").unwrap();
        assert!(entry.current().is_none());
        drop(guard);
        assert!(entry.lock().is_ok());
    }
}
