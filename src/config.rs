//! Trusted, local provider settings. Recipes never select credential destinations.
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Provider {
    pub kind: Kind,
    pub base_url: String,
    pub model: String,
    pub api_key_env: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential: Option<super::credentials::Reference>,
    #[serde(default = "timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "tokens")]
    pub max_output_tokens: u32,
    #[serde(default = "context")]
    pub context_tokens: u32,
    #[serde(default)]
    pub thinking: Option<bool>,
    #[serde(default = "keep_alive")]
    pub keep_alive: String,
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Kind {
    Ollama,
    Openai,
    Deepseek,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ollama => "Ollama",
            Self::Openai => "OpenAI compatível",
            Self::Deepseek => "DeepSeek",
        }
    }

    pub fn http_error(self, status: reqwest::StatusCode) -> String {
        let advice = match status.as_u16() {
            401 => "Credencial inválida; confira a API key.",
            402 => "Saldo insuficiente no provider.",
            400 | 422 => "Parâmetros inválidos; confira o modelo e as opções de geração.",
            429 => "Limite de requisições atingido; tente novamente mais tarde.",
            500 | 503 => "Serviço indisponível ou sobrecarregado; tente novamente mais tarde.",
            _ => "Confira o endereço, o modelo e as credenciais.",
        };
        format!("{} retornou HTTP {}. {advice} Nenhuma tentativa ou troca de provider automática foi feita.", self.label(), status.as_u16())
    }
}

fn timeout() -> u64 {
    300
}
fn tokens() -> u32 {
    6000
}
fn context() -> u32 {
    8192
}
fn keep_alive() -> String {
    "2m".into()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileConfig {
    pub version: u32,
    pub default_provider: String,
    pub providers: BTreeMap<String, Provider>,
    #[serde(default)]
    pub execution: super::policy::Policy,
}

#[derive(Clone)]
pub(crate) struct Settings {
    pub profile: String,
    pub provider: Provider,
    pub secret_names: Vec<String>,
    pub execution: super::policy::Policy,
    secrets: BTreeMap<String, String>,
    pub path: PathBuf,
    pub from_file: bool,
}

impl Settings {
    pub fn draft(path: &Path, profile: &str, provider: &Provider) -> Self {
        Self {
            path: path.into(),
            profile: profile.into(),
            provider: provider.clone(),
            secret_names: Vec::new(),
            secrets: BTreeMap::new(),
            execution: Default::default(),
            from_file: path.is_file(),
        }
    }
    pub fn load(
        path: Option<&Path>,
        profile: Option<&str>,
        model: Option<&str>,
        env_file: Option<&Path>,
    ) -> Result<Self> {
        let explicit = path.is_some();
        let path = path
            .map(PathBuf::from)
            .map(Ok)
            .unwrap_or_else(default_path)?;
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if !explicit && error.kind() == std::io::ErrorKind::NotFound => {
                include_str!("../config.example.toml").into()
            }
            Err(error) => return Err(error).context("Cannot read local configuration"),
        };
        let from_file = path.is_file();
        let path = absolute_path(&path)?;
        // Do not print a TOML parser error: it may contain a line with a secret.
        let config: FileConfig = toml::from_str(&contents).map_err(|_| anyhow::anyhow!("Invalid local configuration; compare with config.example.toml (unknown fields are rejected)"))?;
        if config.version != 1 {
            bail!("Unsupported local configuration version");
        }
        let profile = profile.unwrap_or(&config.default_provider).to_owned();
        let mut provider = config
            .providers
            .get(&profile)
            .context("Unknown provider profile")?
            .clone();
        if let Some(model) = model {
            provider.model = model.to_owned();
        }
        provider.validate()?;
        config.execution.validate()?;
        let mut secret_names = vec![
            "OPENAI_API_KEY".into(),
            "crexe_openai_api_key_env".into(),
            "DEEPSEEK_API_KEY".into(),
            "crexe_deepseek_api_key".into(),
        ];
        secret_names.extend(
            config
                .providers
                .values()
                .filter_map(|p| p.api_key_env.clone()),
        );
        let mut secrets = BTreeMap::new();
        if let Some(path) = env_file {
            let entries = dotenvy::from_path_iter(path)
                .map_err(|_| anyhow::anyhow!("Cannot read --env-file"))?;
            for entry in entries {
                let (key, value) = entry.map_err(|_| {
                    anyhow::anyhow!("Invalid --env-file; expected NAME=value lines")
                })?;
                secret_names.push(key.clone());
                secrets.insert(key, value);
            }
        }
        Ok(Self {
            profile,
            provider,
            secret_names,
            execution: config.execution,
            secrets,
            path,
            from_file,
        })
    }

    pub fn api_key(&self) -> Result<Option<String>> {
        self.api_key_with(&super::credentials::Native)
    }

    fn api_key_with(&self, vault: &dyn super::credentials::Vault) -> Result<Option<String>> {
        if let Some(reference) = &self.provider.credential {
            if let Some(key) = self
                .provider
                .api_key_env
                .as_ref()
                .and_then(|name| self.secrets.get(name))
            {
                if key.trim().is_empty() {
                    bail!("Empty credential in --env-file");
                }
                return Ok(Some(key.clone()));
            }
            return super::credentials::resolve(
                reference,
                &self.path,
                &self.profile,
                &self.provider.base_url,
                vault,
            )
            .map(Some);
        }
        let Some(name) = &self.provider.api_key_env else {
            return Ok(None);
        };
        let value = env::var(name)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| self.secrets.get(name).cloned())
            .or_else(|| user_variable(name));
        let value = value.filter(|v| !v.trim().is_empty()).with_context(|| format!("Missing credential variable {name}; set it in the environment or pass --env-file explicitly"))?;
        Ok(Some(value))
    }

    pub fn identity(&self) -> Result<Vec<u8>> {
        let mut provider = self.provider.clone();
        provider.credential = None;
        if provider.kind == Kind::Deepseek {
            // Credential location is not a generation parameter. Preserve legacy identities.
            provider.api_key_env = None;
            provider.thinking = Some(provider.thinking.unwrap_or(false));
        }
        Ok(serde_json::to_vec(
            &serde_json::json!({"provider": provider, "execution": self.execution}),
        )?)
    }
}

impl Provider {
    pub fn validate(&self) -> Result<()> {
        let url = reqwest::Url::parse(&self.base_url).context("Invalid provider URL")?;
        let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if !(url.scheme() == "https" || (url.scheme() == "http" && local))
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            bail!("Provider URL must use HTTPS (HTTP allowed only on loopback), without credentials, query or fragment");
        }
        if self.model.trim().is_empty() || self.model.len() > 256 {
            bail!("Invalid provider model");
        }
        if !(1..=1800).contains(&self.timeout_seconds)
            || !(128..=65536).contains(&self.max_output_tokens)
            || !(512..=131072).contains(&self.context_tokens)
        {
            bail!("Provider resource limits are outside supported bounds");
        }
        if self.kind == Kind::Ollama && self.max_output_tokens >= self.context_tokens {
            bail!(
                "Ollama context_tokens must exceed max_output_tokens and leave room for the prompt"
            );
        }
        if let Some(name) = &self.api_key_env {
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                bail!("Invalid credential variable name");
            }
        }
        Ok(())
    }
}

pub(crate) fn parse(contents: &str) -> Result<FileConfig> {
    let config: FileConfig = toml::from_str(contents).map_err(|_| {
        anyhow::anyhow!("Configuração TOML inválida; campos desconhecidos não são aceitos.")
    })?;
    if config.version != 1 || !config.providers.contains_key(&config.default_provider) {
        bail!("Versão ou provider padrão inválido na configuração.");
    }
    for provider in config.providers.values() {
        provider.validate()?;
    }
    config.execution.validate()?;
    Ok(config)
}

pub(crate) fn absolute_path(path: &Path) -> Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let mut ancestor = absolute.as_path();
    let mut missing = Vec::new();
    while !ancestor.exists() {
        missing.push(
            ancestor
                .file_name()
                .context("Invalid configuration path")?
                .to_owned(),
        );
        ancestor = ancestor
            .parent()
            .context("Invalid configuration directory")?;
    }
    let mut resolved = ancestor.canonicalize()?;
    for part in missing.into_iter().rev() {
        resolved.push(part);
    }
    Ok(resolved)
}

#[cfg(windows)]
fn user_variable(name: &str) -> Option<String> {
    use winreg::{
        enums::{HKEY_CURRENT_USER, KEY_READ},
        RegKey,
    };
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags("Environment", KEY_READ)
        .ok()?
        .get_value(name)
        .ok()
}
#[cfg(not(windows))]
fn user_variable(_: &str) -> Option<String> {
    None
}

pub(crate) fn default_path() -> Result<PathBuf> {
    if env::var_os("CREXE_HOME").is_some() {
        return Ok(super::installation::root()?.join("config.toml"));
    }
    let base = if cfg!(windows) {
        PathBuf::from(env::var_os("APPDATA").context("APPDATA is not set")?).join("CREXE")
    } else if cfg!(target_os = "macos") {
        super::installation::root()?
    } else {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or(
                PathBuf::from(env::var_os("HOME").context("HOME is not set")?).join(".config"),
            )
            .join("crexe")
    };
    Ok(base.join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deepseek_identity_ignores_credentials_and_normalizes_default_thinking() {
        let temp = tempfile::tempdir().unwrap();
        let mut settings = Settings::draft(
            &temp.path().join("config.toml"),
            "deepseek",
            &parse(include_str!("../config.example.toml"))
                .unwrap()
                .providers["deepseek"],
        );
        let original = settings.identity().unwrap();
        settings.provider.thinking = None;
        settings.provider.api_key_env = Some("ANOTHER_SECRET_NAME".into());
        settings.provider.credential = Some(
            super::super::credentials::Reference::new(
                &settings.path,
                "deepseek",
                &settings.provider.base_url,
            )
            .unwrap(),
        );
        assert_eq!(original, settings.identity().unwrap());
        settings.provider.thinking = Some(true);
        assert_ne!(original, settings.identity().unwrap());
        settings.provider.thinking = Some(false);
        settings.provider.model = "another-model".into();
        assert_ne!(original, settings.identity().unwrap());
    }
    #[test]
    fn vault_resolution_is_bound_and_explicit_env_file_overrides_it() {
        use super::super::credentials::{testing::Memory, Reference, Vault};
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        fs::write(&path, include_str!("../config.example.toml")).unwrap();
        let mut settings = Settings::load(Some(&path), Some("openai"), None, None).unwrap();
        // PATH exists but must never replace the saved key. No process-global env mutation.
        settings.provider.api_key_env = Some("PATH".into());
        let legacy_identity = settings.identity().unwrap();
        let reference = Reference::new(&path, "openai", &settings.provider.base_url).unwrap();
        let vault = Memory::default();
        vault.set(&reference.id, "synthetic-saved").unwrap();
        settings.provider.credential = Some(reference);
        assert_eq!(settings.identity().unwrap(), legacy_identity);
        assert_eq!(
            settings.api_key_with(&vault).unwrap().as_deref(),
            Some("synthetic-saved")
        );
        vault.unavailable.set(true);
        assert!(settings.api_key_with(&vault).is_err());
        vault.unavailable.set(false);
        settings
            .secrets
            .insert("PATH".into(), "synthetic-explicit-override".into());
        assert_eq!(
            settings.api_key_with(&vault).unwrap().as_deref(),
            Some("synthetic-explicit-override")
        );
        settings.secrets.clear();
        settings.provider.base_url = "https://different.example.invalid/v1".into();
        assert!(settings.api_key_with(&vault).is_err());
        settings.provider.base_url = "https://api.openai.com/v1".into();
        settings.path = temp.path().join("copied.toml");
        assert!(settings.api_key_with(&vault).is_err());
        settings.path = path;
        settings.profile = "other".into();
        assert!(settings.api_key_with(&vault).is_err());
    }
    #[test]
    fn explicit_env_file_does_not_mutate_process_or_identity() {
        let temp = tempfile::tempdir().unwrap();
        let config = temp.path().join("config.toml");
        let env_file = temp.path().join(".env");
        fs::write(
            &config,
            include_str!("../config.example.toml")
                .replace("crexe_openai_api_key_env", "CREXE_TEST_SCOPED_SECRET"),
        )
        .unwrap();
        fs::write(&env_file, "CREXE_TEST_SCOPED_SECRET=synthetic-only").unwrap();
        let settings =
            Settings::load(Some(&config), Some("openai"), None, Some(&env_file)).unwrap();
        assert_eq!(
            settings.api_key().unwrap().as_deref(),
            Some("synthetic-only")
        );
        assert!(env::var_os("CREXE_TEST_SCOPED_SECRET").is_none());
        assert!(!String::from_utf8(settings.identity().unwrap())
            .unwrap()
            .contains("synthetic-only"));
    }
    #[test]
    fn endpoints_and_configuration_are_strict() {
        let config: FileConfig = toml::from_str(include_str!("../config.example.toml")).unwrap();
        let mut provider = config.providers["openai"].clone();
        for url in [
            "http://example.org",
            "https://user:password@example.org",
            "https://example.org?key=secret",
        ] {
            provider.base_url = url.into();
            assert!(provider.validate().is_err());
        }
        assert!(toml::from_str::<FileConfig>("version=1\nsecret='oops'").is_err());
    }
}
