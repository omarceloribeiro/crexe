//! Transactional configuration editing shared by the desktop UI and tests.
use super::{
    config::{self, Provider},
    credentials::{self, Vault},
};
use anyhow::{bail, Context, Result};
use fs2::FileExt;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use toml_edit::{value, DocumentMut, Item};

#[derive(Clone)]
pub(crate) struct Editor {
    pub path: PathBuf,
    baseline: Option<Vec<u8>>,
    document: DocumentMut,
    pub profile: String,
    pub provider: Provider,
}

#[derive(Clone, Default)]
pub(crate) enum KeyChange {
    #[default]
    Keep,
    Replace(String),
    Remove,
}

impl Editor {
    pub fn load(path: &Path) -> Result<Self> {
        super::paths::reject_links(path)?;
        let path = config::absolute_path(path)?;
        let baseline = read_optional(&path)?;
        let text = baseline
            .as_deref()
            .unwrap_or(include_bytes!("../config.example.toml"));
        let text = std::str::from_utf8(text).context("A configuração deve usar UTF-8")?;
        let parsed = config::parse(text)?;
        let document = text.parse().map_err(|_| anyhow::anyhow!("TOML inválido"))?;
        Ok(Self {
            path,
            baseline,
            document,
            profile: parsed.default_provider.clone(),
            provider: parsed.providers[&parsed.default_provider].clone(),
        })
    }

    pub fn profiles(&self) -> Vec<String> {
        let mut names: Vec<_> = self.document["providers"]
            .as_table()
            .map(|t| t.iter().map(|(n, _)| n.to_string()).collect())
            .unwrap_or_default();
        let defaults = config::parse(include_str!("../config.example.toml"))
            .expect("bundled configuration is valid");
        for name in defaults
            .providers
            .keys()
            .chain(std::iter::once(&self.profile))
        {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
        names
    }

    pub fn select(&mut self, profile: &str) -> Result<()> {
        let config = config::parse(&self.document.to_string())?;
        let defaults = config::parse(include_str!("../config.example.toml"))?;
        self.provider = config
            .providers
            .get(profile)
            .or_else(|| defaults.providers.get(profile))
            .context("Perfil inexistente")?
            .clone();
        self.profile = profile.to_string();
        Ok(())
    }

    pub fn import(&mut self, path: &Path) -> Result<()> {
        let text = fs::read_to_string(path).context("Não foi possível ler o TOML para importar")?;
        let imported = config::parse(&text)?;
        // Import only the selected provider. No execution policy or foreign vault reference.
        let existing = config::parse(&self.document.to_string())?;
        self.profile = imported.default_provider;
        self.provider = imported.providers[&self.profile].clone();
        self.provider.credential = existing
            .providers
            .get(&self.profile)
            .and_then(|p| p.credential.clone());
        Ok(())
    }

    fn apply_provider(&mut self) -> Result<()> {
        let serialized = toml::to_string(&self.provider)?;
        let table: DocumentMut = serialized
            .parse()
            .map_err(|_| anyhow::anyhow!("Cannot serialize provider"))?;
        if !self.document["providers"]
            .as_table()
            .is_some_and(|t| t.contains_key(&self.profile))
        {
            self.document["providers"][&self.profile] = Item::Table(toml_edit::Table::new());
        }
        let target = self.document["providers"][&self.profile]
            .as_table_mut()
            .context("Tabela de provider inválida")?;
        for key in ["api_key_env", "thinking", "credential"] {
            if !table.contains_key(key) {
                target.remove(key);
            }
        }
        for (key, item) in table.iter() {
            let mut item = item.clone();
            // Keep comments/formatting attached to existing scalar fields.
            if let (Some(old), Some(new)) = (
                target.get(key).and_then(Item::as_value),
                item.as_value_mut(),
            ) {
                *new.decor_mut() = old.decor().clone();
            }
            target.insert(key, item);
        }
        self.document["default_provider"] = value(&self.profile);
        Ok(())
    }

    pub fn save(self, change: KeyChange) -> Result<(Self, String)> {
        self.save_with(change, &credentials::Native, |staged, path| {
            staged.persist(path).map_err(|e| e.error).context(
                "Não foi possível substituir a configuração; o arquivo anterior foi preservado",
            )?;
            Ok(())
        })
    }

    fn save_with(
        mut self,
        change: KeyChange,
        vault: &dyn Vault,
        publish: impl FnOnce(tempfile::NamedTempFile, &Path) -> Result<()>,
    ) -> Result<(Self, String)> {
        self.provider.validate()?;
        super::paths::reject_links(&self.path)?;
        let parent = self
            .path
            .parent()
            .context("Diretório de configuração inválido")?
            .to_path_buf();
        fs::create_dir_all(&parent)?;
        let _lock = lock(&self.path)?;
        if read_optional(&self.path)? != self.baseline {
            bail!("A configuração foi alterada por outra janela ou programa. Use Recarregar antes de salvar.");
        }
        // Only delete a previous credential owned by this exact destination/profile.
        let old = self
            .baseline
            .as_deref()
            .and_then(|b| std::str::from_utf8(b).ok())
            .and_then(|s| config::parse(s).ok())
            .and_then(|c| c.providers.get(&self.profile).cloned())
            .and_then(|p| {
                p.credential
                    .filter(|r| r.validate(&self.path, &self.profile, &p.base_url).is_ok())
            });
        let new_key = match change {
            KeyChange::Keep => {
                if let Some(reference) = &self.provider.credential {
                    reference.validate(&self.path, &self.profile, &self.provider.base_url)?;
                }
                None
            }
            KeyChange::Remove => {
                self.provider.credential = None;
                None
            }
            KeyChange::Replace(key) => {
                if key.trim().is_empty() || key.len() > 4096 {
                    bail!("Informe uma chave válida (até 4096 caracteres).");
                }
                self.provider.credential = Some(credentials::Reference::new(
                    &self.path,
                    &self.profile,
                    &self.provider.base_url,
                )?);
                Some(key)
            }
        };
        self.apply_provider()?;
        let text = self.document.to_string();
        config::parse(&text)?;
        let mut staged = tempfile::NamedTempFile::new_in(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            staged
                .as_file()
                .set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        staged.write_all(text.as_bytes())?;
        staged.as_file().sync_all()?;
        if let Some(key) = &new_key {
            vault.set(&self.provider.credential.as_ref().unwrap().id, key)?;
        }
        if publish(staged, &self.path).is_err() {
            let cleanup = if new_key.is_some() {
                vault.delete(&self.provider.credential.as_ref().unwrap().id)
            } else {
                Ok(())
            };
            if cleanup.is_err() {
                bail!("Falha ao salvar. A configuração anterior foi preservada; uma entrada CREXE temporária ficou no cofre.");
            }
            bail!(
                "Não foi possível salvar. A configuração e a chave anteriores foram preservadas."
            );
        }
        let mut message = "Configuração salva. Será usada na próxima execução.".to_string();
        if let Some(old) = old {
            if self.provider.credential.as_ref().map(|r| &r.id) != Some(&old.id)
                && vault.delete(&old.id).is_err()
            {
                message.push_str(" A antiga entrada CREXE não pôde ser removida do cofre; ela não será mais usada.");
            }
        }
        self.baseline = Some(text.into_bytes());
        Ok((self, message))
    }

    pub fn models(&self, change: &KeyChange) -> Result<Vec<String>> {
        self.provider.validate()?;
        let key = match change {
            KeyChange::Replace(key) if !key.trim().is_empty() => Some(key.clone()),
            KeyChange::Replace(_) => bail!("Informe uma chave antes de testar."),
            KeyChange::Remove => None,
            KeyChange::Keep => {
                config::Settings::draft(&self.path, &self.profile, &self.provider).api_key()?
            }
        };
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let endpoint = if self.provider.kind == config::Kind::Ollama {
            "api/tags"
        } else {
            "models"
        };
        let mut request = client.get(format!(
            "{}/{endpoint}",
            self.provider.base_url.trim_end_matches('/')
        ));
        if let Some(key) = key {
            request = request.bearer_auth(key);
        }
        let response = request.send().map_err(|error| if error.is_timeout() {
            anyhow::anyhow!("O provider excedeu o tempo de conexão. Tente novamente mais tarde.")
        } else {
            anyhow::anyhow!("Não foi possível conectar ao provider. Verifique o endereço e se o serviço está disponível.")
        })?;
        if !response.status().is_success() {
            bail!(self.provider.kind.http_error(response.status()));
        }
        let mut data = Vec::new();
        response
            .take(1_048_577)
            .read_to_end(&mut data)
            .context("Resposta de modelos incompleta")?;
        if data.len() > 1_048_576 {
            bail!("Lista de modelos muito grande");
        }
        let json: serde_json::Value = serde_json::from_slice(&data)
            .map_err(|_| anyhow::anyhow!("Resposta de modelos inválida"))?;
        let (list, name) = if self.provider.kind == config::Kind::Ollama {
            ("models", "name")
        } else {
            ("data", "id")
        };
        let mut models: Vec<_> = json[list]
            .as_array()
            .context("Lista de modelos ausente")?
            .iter()
            .filter_map(|item| item[name].as_str())
            .filter(|s| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control))
            .map(str::to_string)
            .collect();
        models.sort();
        models.dedup();
        Ok(models)
    }
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).context("Não foi possível ler a configuração"),
    }
}
fn lock(path: &Path) -> Result<fs::File> {
    let path = path.with_extension("toml.lock");
    super::paths::reject_links(&path)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    lock.try_lock_exclusive()
        .context("Outra janela está salvando a configuração. Tente novamente.")?;
    Ok(lock)
}

pub(crate) fn initialize(path: &Path) -> Result<()> {
    super::paths::reject_links(path)?;
    fs::create_dir_all(path.parent().context("Invalid configuration directory")?)?;
    let _lock = lock(path)?;
    if path.exists() {
        return Ok(());
    }
    let mut staged = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    staged.write_all(include_bytes!("../config.example.toml"))?;
    staged.as_file().sync_all()?;
    staged.persist_noclobber(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use credentials::testing::Memory;

    fn publish(file: tempfile::NamedTempFile, path: &Path) -> Result<()> {
        file.persist(path).map_err(|e| e.error)?;
        Ok(())
    }

    #[test]
    fn legacy_configuration_offers_deepseek_without_mutating_until_save() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let old = "version=1\ndefault_provider='custom'\n[providers.custom]\nkind='ollama'\nbase_url='http://localhost:11434'\nmodel='my-local-model'\n";
        fs::write(&path, old).unwrap();
        let mut editor = Editor::load(&path).unwrap();
        assert!(editor.profiles().contains(&"deepseek".into()));
        editor.select("deepseek").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), old);
        assert_eq!(config::parse(old).unwrap().default_provider, "custom");
        let vault = Memory::default();
        let (editor, _) = editor
            .save_with(
                KeyChange::Replace("synthetic-deepseek".into()),
                &vault,
                publish,
            )
            .unwrap();
        assert_eq!(editor.profile, "deepseek");
        let persisted = fs::read_to_string(&path).unwrap();
        assert!(!persisted.contains("synthetic-deepseek"));
        let parsed = config::parse(&persisted).unwrap();
        assert_eq!(parsed.providers["custom"].model, "my-local-model");
        assert_eq!(parsed.providers["deepseek"].kind.label(), "DeepSeek");
        let loaded = Editor::load(&path).unwrap();
        assert_eq!(loaded.profile, "deepseek");
        assert_eq!(
            credentials::resolve(
                loaded.provider.credential.as_ref().unwrap(),
                &path,
                "deepseek",
                &loaded.provider.base_url,
                &vault
            )
            .unwrap(),
            "synthetic-deepseek"
        );
    }

    #[test]
    fn imports_selected_provider_preserves_policy_comments_and_reinstallation() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        initialize(&path).unwrap();
        let original = fs::read_to_string(&path).unwrap().replace(
            "allow_shells = false",
            "# locally reviewed policy\nallow_shells = false",
        );
        fs::write(&path, &original).unwrap();
        let imported = temp.path().join("example.toml");
        fs::write(
            &imported,
            original
                .replace(
                    "default_provider = \"local\"",
                    "default_provider = \"openai\"",
                )
                .replace("allow_shells = false", "allow_shells = true")
                .replace("gpt-4o-mini", "imported-model"),
        )
        .unwrap();
        let mut editor = Editor::load(&path).unwrap();
        editor.import(&imported).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            original,
            "import is review-only"
        );
        let (editor, _) = editor
            .save_with(KeyChange::Keep, &Memory::default(), publish)
            .unwrap();
        let bytes = fs::read(&path).unwrap();
        initialize(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        let settings = config::Settings::load(Some(&path), None, None, None).unwrap();
        assert_eq!(settings.profile, "openai");
        assert_eq!(settings.provider.model, "imported-model");
        assert!(!settings.execution.allow_shells);
        assert!(fs::read_to_string(&path)
            .unwrap()
            .contains("# locally reviewed policy"));
        assert_eq!(editor.profiles().len(), 3);
    }

    #[test]
    fn secret_rotation_is_transactional_and_never_written_to_toml() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let vault = Memory::default();
        let mut editor = Editor::load(&path).unwrap();
        editor.select("openai").unwrap();
        let (editor, _) = editor
            .save_with(
                KeyChange::Replace("synthetic-first".into()),
                &vault,
                publish,
            )
            .unwrap();
        let identity = config::Settings::load(Some(&path), None, None, None)
            .unwrap()
            .identity()
            .unwrap();
        let before = fs::read(&path).unwrap();
        let old = editor.provider.credential.as_ref().unwrap().id.clone();
        vault.unavailable.set(true);
        assert!(editor
            .clone()
            .save_with(
                KeyChange::Replace("synthetic-second".into()),
                &vault,
                publish
            )
            .is_err());
        vault.unavailable.set(false);
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(vault.entries.borrow().len(), 1);
        assert!(editor
            .clone()
            .save_with(
                KeyChange::Replace("synthetic-second".into()),
                &vault,
                |_, _| bail!("injected write failure")
            )
            .is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(vault.entries.borrow().len(), 1);
        assert_eq!(vault.get(&old).unwrap(), "synthetic-first");
        let (editor, _) = editor
            .save_with(
                KeyChange::Replace("synthetic-second".into()),
                &vault,
                publish,
            )
            .unwrap();
        assert_eq!(vault.entries.borrow().len(), 1);
        assert!(vault.get(&old).is_err());
        assert_eq!(
            identity,
            config::Settings::load(Some(&path), None, None, None)
                .unwrap()
                .identity()
                .unwrap()
        );
        assert!(!fs::read_to_string(&path).unwrap().contains("synthetic-"));
        editor
            .save_with(KeyChange::Remove, &vault, publish)
            .unwrap();
        assert!(vault.entries.borrow().is_empty());
    }

    #[test]
    fn conflicts_and_changed_endpoint_do_not_overwrite_previous_key() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let vault = Memory::default();
        let mut editor = Editor::load(&path).unwrap();
        editor.select("openai").unwrap();
        let (editor, _) = editor
            .save_with(KeyChange::Replace("synthetic-key".into()), &vault, publish)
            .unwrap();
        let mut stale = editor.clone();
        stale.provider.base_url = "https://another.example.invalid/v1".into();
        assert!(stale.save_with(KeyChange::Keep, &vault, publish).is_err());
        let bytes = fs::read(&path).unwrap();
        fs::write(&path, [bytes.as_slice(), b"\n# external edit\n"].concat()).unwrap();
        assert!(editor
            .save_with(KeyChange::Replace("synthetic-new".into()), &vault, publish)
            .is_err());
        assert!(fs::read_to_string(&path).unwrap().contains("external edit"));
        assert_eq!(vault.entries.borrow().len(), 1);
    }

    #[test]
    fn model_lookup_is_a_bounded_get_without_generation_and_omits_error_bodies() {
        use std::net::TcpListener;
        for (kind, body, expected_path, status) in [
            (
                config::Kind::Ollama,
                r#"{"models":[{"name":"local-model"}]}"#,
                "/api/tags",
                200,
            ),
            (
                config::Kind::Openai,
                r#"{"data":[{"id":"remote-model"}]}"#,
                "/models",
                200,
            ),
            (
                config::Kind::Openai,
                "do-not-show-remote-body",
                "/models",
                401,
            ),
            (
                config::Kind::Deepseek,
                r#"{"data":[{"id":"deepseek-flash","unknown_field":true},{"id":"deepseek-flash"}]}"#,
                "/models",
                200,
            ),
            (
                config::Kind::Deepseek,
                "do-not-show-remote-body",
                "/models",
                401,
            ),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let worker = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut request = [0; 8192];
                let n = stream.read(&mut request).unwrap();
                let request = std::str::from_utf8(&request[..n]).unwrap();
                assert!(request.starts_with(&format!("GET {expected_path} HTTP/1.1")));
                assert!(request
                    .to_ascii_lowercase()
                    .contains("authorization: bearer synthetic-test-only"));
                write!(stream, "HTTP/1.1 {status} Result\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            });
            let temp = tempfile::tempdir().unwrap();
            let mut editor = Editor::load(&temp.path().join("config.toml")).unwrap();
            editor.provider.kind = kind;
            editor.provider.base_url = url;
            let result = editor.models(&KeyChange::Replace("synthetic-test-only".into()));
            worker.join().unwrap();
            if status == 200 {
                assert_eq!(result.unwrap().len(), 1);
            } else {
                let message = result.unwrap_err().to_string();
                assert!(message.contains("401"));
                assert!(!message.contains("do-not-show"));
            }
        }
    }
}
