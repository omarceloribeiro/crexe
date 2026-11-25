//! Text document state and atomic persistence, independent of the editor UI.
use anyhow::{bail, Context, Result};
use fs2::FileExt;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) struct Document {
    pub path: PathBuf,
    pub text: String,
    baseline: Vec<u8>,
    original_text: String,
    bom: bool,
    crlf: bool,
}

impl Document {
    pub fn load(path: &Path) -> Result<Self> {
        super::paths::reject_links(path)?;
        let path = path
            .canonicalize()
            .context("Não foi possível localizar o arquivo")?;
        let raw = super::format::read(&path)?;
        let bom = raw.starts_with('\u{feff}');
        let content = raw.strip_prefix('\u{feff}').unwrap_or(&raw);
        let crlf = content.contains("\r\n");
        let text = content.replace("\r\n", "\n");
        Ok(Self {
            path,
            original_text: text.clone(),
            text,
            baseline: raw.into_bytes(),
            bom,
            crlf,
        })
    }

    pub fn dirty(&self) -> bool {
        self.text != self.original_text
    }

    pub fn reload(&mut self) -> Result<()> {
        *self = Self::load(&self.path)?;
        Ok(())
    }

    pub fn save(&mut self) -> Result<()> {
        self.save_with_lock(&super::installation::root()?.join("editor-locks"))
    }

    fn save_with_lock(&mut self, lock_root: &Path) -> Result<()> {
        super::paths::reject_links(&self.path)?;
        super::paths::reject_links(lock_root)?;
        fs::create_dir_all(lock_root)?;
        let mut identity = self.path.to_string_lossy().into_owned();
        if cfg!(windows) {
            identity = identity.to_lowercase();
        }
        let lock_path = lock_root.join(format!("{:x}.lock", Sha256::digest(identity)));
        super::paths::reject_links(&lock_path)?;
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .context("Não foi possível abrir o arquivo para salvar")?;
        file.try_lock_exclusive()
            .context("O arquivo está sendo salvo em outra janela")?;
        if super::format::read(&self.path)?.as_bytes() != self.baseline {
            bail!("O arquivo mudou fora desta janela. Seu texto foi preservado; use Recarregar para ler a versão do disco.");
        }
        if !self.dirty() {
            return Ok(());
        }
        let normalized = self.text.replace("\r\n", "\n");
        let text = if self.crlf {
            normalized.replace('\n', "\r\n")
        } else {
            normalized.clone()
        };
        let bytes = format!("{}{text}", if self.bom { "\u{feff}" } else { "" }).into_bytes();
        if bytes.len() > super::format::MAX_DOCUMENT as usize {
            bail!("O documento deve ter no máximo 1 MiB. Seu texto foi preservado.");
        }
        let mut staged =
            tempfile::NamedTempFile::new_in(self.path.parent().context("Caminho inválido")?)?;
        let permissions = fs::metadata(&self.path)?.permissions();
        if permissions.readonly() {
            bail!("O arquivo é somente leitura. Seu texto foi preservado.");
        }
        staged.as_file().set_permissions(permissions)?;
        staged.write_all(&bytes)?;
        staged.as_file().sync_all()?;
        if super::format::read(&self.path)?.as_bytes() != self.baseline {
            bail!("O arquivo mudou durante o salvamento. Seu texto foi preservado.");
        }
        staged
            .persist(&self.path)
            .map_err(|e| e.error)
            .context("Não foi possível salvar; seu texto foi preservado")?;
        self.baseline = bytes;
        self.text = normalized;
        self.original_text = self.text.clone();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_preserves_encoding_newlines_and_allows_empty_or_invalid_documents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("edição com espaços.crexe");
        for initial in ["texto\n", "\u{feff}ação\r\n", "", "\u{feff}"] {
            fs::write(&path, initial).unwrap();
            let mut doc = Document::load(&path).unwrap();
            doc.save_with_lock(dir.path()).unwrap();
            assert_eq!(fs::read_to_string(&path).unwrap(), initial);
            doc.text = "---\r\nnão é YAML\r\n".into();
            doc.save_with_lock(dir.path()).unwrap();
            let saved = fs::read_to_string(&path).unwrap();
            assert_eq!(
                saved.starts_with('\u{feff}'),
                initial.starts_with('\u{feff}')
            );
            assert_eq!(saved.contains("\r\n"), initial.contains("\r\n"));
            assert!(!saved.contains("\r\r\n"));
            assert!(!doc.dirty());
            doc.text.clear();
            doc.save_with_lock(dir.path()).unwrap();
            assert!(super::super::format::is_empty(
                &fs::read_to_string(&path).unwrap()
            ));
        }
    }
    #[test]
    fn conflict_and_size_failures_keep_disk_and_buffer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.crexe");
        fs::write(&path, "old").unwrap();
        let mut doc = Document::load(&path).unwrap();
        doc.text = "mine".into();
        fs::write(&path, "theirs").unwrap();
        assert!(doc.save_with_lock(dir.path()).is_err());
        assert_eq!(doc.text, "mine");
        assert_eq!(fs::read_to_string(&path).unwrap(), "theirs");
        doc.reload().unwrap();
        doc.text = "x".repeat(super::super::format::MAX_DOCUMENT as usize);
        doc.save_with_lock(dir.path()).unwrap();
        doc.text.push('x');
        assert!(doc.save_with_lock(dir.path()).is_err());
        assert_eq!(
            fs::metadata(&path).unwrap().len(),
            super::super::format::MAX_DOCUMENT
        );
        fs::remove_file(&path).unwrap();
        assert!(doc.save_with_lock(dir.path()).is_err());
        assert!(doc.dirty());
    }
}
