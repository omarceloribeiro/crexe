//! Native user vault. Only opaque, destination-bound references enter configuration.
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Arc};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reference {
    pub id: String,
    pub binding: String,
}

impl Reference {
    pub fn new(path: &Path, profile: &str, endpoint: &str) -> Result<Self> {
        let mut random = [0; 32];
        getrandom::fill(&mut random).map_err(|_| anyhow::anyhow!("Cannot create credential ID"))?;
        Ok(Self {
            id: format!("{:x}", Sha256::digest(random)),
            binding: binding(path, profile, endpoint)?,
        })
    }
    pub fn validate(&self, path: &Path, profile: &str, endpoint: &str) -> Result<()> {
        if self.id.len() != 64
            || !self.id.bytes().all(|c| c.is_ascii_hexdigit())
            || self.binding != binding(path, profile, endpoint)?
        {
            bail!("A chave salva pertence a outra configuração ou endereço. Informe uma nova chave em Configurações.");
        }
        Ok(())
    }
}

fn binding(path: &Path, profile: &str, endpoint: &str) -> Result<String> {
    let path = super::config::absolute_path(path)?;
    let mut path = path.to_string_lossy().into_owned();
    if cfg!(windows) {
        path.make_ascii_lowercase();
    }
    let endpoint = reqwest::Url::parse(endpoint)?.to_string();
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(
            path,
            profile,
            endpoint.trim_end_matches('/')
        ))?)
    ))
}

pub(crate) trait Vault {
    fn get(&self, id: &str) -> Result<String>;
    fn set(&self, id: &str, key: &str) -> Result<()>;
    fn delete(&self, id: &str) -> Result<()>;
}

pub(crate) struct Native;
impl Native {
    fn entry(id: &str) -> Result<keyring_core::Entry> {
        let store: Arc<keyring_core::CredentialStore> = {
            #[cfg(windows)]
            { windows_native_keyring_store::Store::new() }
            #[cfg(target_os = "macos")]
            { apple_native_keyring_store::keychain::Store::new() }
            #[cfg(target_os = "linux")]
            { zbus_secret_service_keyring_store::Store::new() }
        }.map_err(|_| anyhow::anyhow!("Cofre do sistema indisponível. Desbloqueie o cofre da sua sessão e tente novamente; a chave não será salva em texto puro."))?;
        #[cfg(windows)]
        let modifiers = Some(std::collections::HashMap::from([("persistence", "Local")]));
        #[cfg(not(windows))]
        let modifiers: Option<std::collections::HashMap<&str, &str>> = None;
        store
            .build("org.crexe.credentials.v1", id, modifiers.as_ref())
            .map_err(|_| {
                anyhow::anyhow!("Não foi possível acessar a entrada CREXE no cofre do sistema.")
            })
    }
}
impl Vault for Native {
    fn get(&self, id: &str) -> Result<String> {
        let key = Self::entry(id)?.get_password()
            .map_err(|_| anyhow::anyhow!("Chave salva ausente ou cofre bloqueado. Abra Configurações para corrigir; nenhuma outra chave foi usada."))?;
        if key.trim().is_empty() {
            bail!("Chave salva vazia; configure uma nova chave.");
        }
        Ok(key)
    }
    fn set(&self, id: &str, key: &str) -> Result<()> {
        Self::entry(id)?.set_password(key).map_err(|_| {
            anyhow::anyhow!(
                "Não foi possível salvar a chave no cofre. A configuração anterior foi preservada."
            )
        })
    }
    fn delete(&self, id: &str) -> Result<()> {
        match Self::entry(id)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(_) => bail!("Não foi possível remover a antiga entrada CREXE do cofre."),
        }
    }
}

pub(crate) fn resolve(
    reference: &Reference,
    path: &Path,
    profile: &str,
    endpoint: &str,
    vault: &dyn Vault,
) -> Result<String> {
    reference.validate(path, profile, endpoint)?;
    vault
        .get(&reference.id)
        .context("Credencial da configuração ativa")
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        collections::BTreeMap,
    };
    #[derive(Default)]
    pub struct Memory {
        pub entries: RefCell<BTreeMap<String, String>>,
        pub unavailable: Cell<bool>,
    }
    impl Vault for Memory {
        fn get(&self, id: &str) -> Result<String> {
            if self.unavailable.get() {
                bail!("vault unavailable");
            }
            self.entries
                .borrow()
                .get(id)
                .cloned()
                .context("entry missing")
        }
        fn set(&self, id: &str, key: &str) -> Result<()> {
            if self.unavailable.get() {
                bail!("vault unavailable");
            }
            self.entries.borrow_mut().insert(id.into(), key.into());
            Ok(())
        }
        fn delete(&self, id: &str) -> Result<()> {
            if self.unavailable.get() {
                bail!("vault unavailable");
            }
            self.entries.borrow_mut().remove(id);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires an unlocked native user vault; creates and deletes only a synthetic CREXE entry"]
    fn native_vault_roundtrip_synthetic_only() {
        let temp = tempfile::tempdir().unwrap();
        let reference = Reference::new(
            &temp.path().join("config.toml"),
            "synthetic-test",
            "https://example.invalid",
        )
        .unwrap();
        struct Cleanup(String);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = Native.delete(&self.0);
            }
        }
        let _cleanup = Cleanup(reference.id.clone());
        Native
            .set(&reference.id, "CREXE-synthetic-test-first")
            .unwrap();
        assert_eq!(
            Native.get(&reference.id).unwrap(),
            "CREXE-synthetic-test-first"
        );
        Native
            .set(&reference.id, "CREXE-synthetic-test-second")
            .unwrap();
        assert_eq!(
            Native.get(&reference.id).unwrap(),
            "CREXE-synthetic-test-second"
        );
        Native.delete(&reference.id).unwrap();
        assert!(Native.get(&reference.id).is_err());
    }
}
