use anyhow::{bail, Context, Result};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

#[cfg(windows)]
mod windows;

pub(crate) fn root() -> Result<PathBuf> {
    if env::var_os("CREXE_HOME").is_none() {
        if let Some(root) = installed_root(&env::current_exe()?) {
            return Ok(root);
        }
    }
    let root = if let Some(root) = env::var_os("CREXE_HOME") {
        PathBuf::from(root)
    } else {
        #[cfg(windows)]
        {
            PathBuf::from(env::var_os("LOCALAPPDATA").context("LOCALAPPDATA is not set")?)
                .join("CREXE")
        }
        #[cfg(target_os = "macos")]
        {
            PathBuf::from(env::var_os("HOME").context("HOME is not set")?)
                .join("Library/Application Support/CREXE")
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            let base = match env::var_os("XDG_DATA_HOME") {
                Some(value) => PathBuf::from(value),
                None => PathBuf::from(env::var_os("HOME").context("HOME is not set")?)
                    .join(".local/share"),
            };
            base.join("crexe")
        }
    };
    if !root.is_absolute() {
        bail!(
            "Installation root must be an absolute path: {}",
            root.display()
        );
    }
    Ok(root)
}

pub(crate) fn executable() -> Result<PathBuf> {
    Ok(root()?
        .join("releases")
        .join("v1")
        .join(if cfg!(windows) { "crexe.exe" } else { "crexe" }))
}

/// Shell invocations do not inherit a custom CREXE_HOME from the installing shell.
/// An installed launcher can recover its maintained-reader root from its own path.
pub(crate) fn reader_root() -> Result<PathBuf> {
    root()
}

fn installed_root(executable: &Path) -> Option<PathBuf> {
    let filename = if cfg!(windows) { "crexe.exe" } else { "crexe" };
    if executable.file_name()? != filename {
        return None;
    }
    let version = executable.parent()?;
    if version.file_name()? != "v1" {
        return None;
    }
    let releases = version.parent()?;
    if releases.file_name()? != "releases" {
        return None;
    }
    Some(releases.parent()?.to_owned())
}

pub(crate) fn reader_engine() -> Result<PathBuf> {
    let executable =
        reader_root()?
            .join("releases/v1")
            .join(if cfg!(windows) { "crexe.exe" } else { "crexe" });
    super::paths::reject_links(&executable)?;
    if !executable.is_file() {
        bail!("Instale esta versão antes de manter um leitor: crexe install e crexe associate.");
    }
    use sha2::{Digest, Sha256};
    if Sha256::digest(fs::read(&executable)?) != Sha256::digest(fs::read(env::current_exe()?)?) {
        bail!("A engine instalada é diferente desta versão. Execute crexe install e crexe associate antes de manter um leitor.");
    }
    Ok(executable)
}

pub(crate) fn register_reader(extension: &str) -> Result<()> {
    #[cfg(windows)]
    {
        windows::register_reader(&reader_engine()?, extension)
    }
    #[cfg(not(windows))]
    {
        let _ = extension;
        bail!("Programas padrão mantidos pelo CREXE estão disponíveis somente no Windows.")
    }
}

pub(crate) fn validate_reader_registration(extension: &str) -> Result<()> {
    let executable = reader_engine()?;
    #[cfg(windows)]
    {
        windows::validate_reader_registration(&executable, extension)
    }
    #[cfg(not(windows))]
    {
        let _ = (executable, extension);
        bail!("Programas padrão mantidos pelo CREXE estão disponíveis somente no Windows.")
    }
}

pub(crate) fn unregister_reader(extension: &str) -> Result<()> {
    #[cfg(windows)]
    {
        windows::unregister_reader(&reader_engine()?, extension)
    }
    #[cfg(not(windows))]
    {
        let _ = extension;
        bail!("Programas padrão mantidos pelo CREXE estão disponíveis somente no Windows.")
    }
}

pub(crate) fn choose_reader_default(extension: &str) -> Result<()> {
    #[cfg(windows)]
    {
        windows::choose_reader_default(extension)
    }
    #[cfg(not(windows))]
    {
        let _ = extension;
        bail!("Programas padrão mantidos pelo CREXE estão disponíveis somente no Windows.")
    }
}

pub(crate) fn install() -> Result<()> {
    let source = env::current_exe()?;
    let destination = executable()?;
    install_file(&source, &destination)?;
    let config = super::config::default_path()?;
    super::config_editor::initialize(&config)?;
    println!("Configuration preserved/initialized: {}", config.display());
    println!(
        "Installed CREXE {}: {}",
        env!("CARGO_PKG_VERSION"),
        destination.display()
    );
    Ok(())
}

fn install_file(source: &Path, destination: &Path) -> Result<()> {
    super::paths::reject_links(destination)?;
    let parent = destination
        .parent()
        .context("Missing installation directory")?;
    fs::create_dir_all(parent)?;
    if destination.exists() && source.canonicalize()? == destination.canonicalize()? {
        return Ok(());
    }
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    io::copy(&mut fs::File::open(source)?, staged.as_file_mut())?;
    staged
        .as_file()
        .set_permissions(fs::metadata(source)?.permissions())?;
    staged.as_file().sync_all()?;
    staged.persist(destination).map_err(|error| error.error)
        .with_context(|| format!("Cannot replace {}. Close running instances and retry; existing installation was preserved.", destination.display()))?;
    Ok(())
}

pub(crate) fn associate() -> Result<()> {
    let exe = executable()?;
    if !exe.is_file() {
        bail!(
            "Installed engine not found: {}. Run crexe install first.",
            exe.display()
        );
    }
    super::paths::reject_links(&exe)?;
    #[cfg(windows)]
    {
        windows::associate(&exe)?;
    }
    #[cfg(not(windows))]
    {
        bail!("Desktop association is currently implemented only on Windows");
    }
    #[cfg(windows)]
    println!(
        "Associated .crexe, .crexp and .crexb with {} and added Gerar com CREXE for individual files. Windows may require choosing CREXE as the default app for these formats; other file defaults are unchanged.",
        exe.display()
    );
    #[cfg(windows)]
    Ok(())
}

pub(crate) fn unassociate() -> Result<()> {
    #[cfg(windows)]
    {
        windows::unassociate(&executable()?)?;
        println!("Removed CREXE-owned association and Gerar com CREXE menu (other default apps preserved)");
        Ok(())
    }
    #[cfg(not(windows))]
    {
        bail!("Desktop association is currently implemented only on Windows");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shell_launch_recovers_only_the_installed_directory_shape() {
        let temp = tempfile::tempdir().unwrap();
        let filename = if cfg!(windows) { "crexe.exe" } else { "crexe" };
        assert_eq!(
            installed_root(&temp.path().join("releases/v1").join(filename)),
            Some(temp.path().into())
        );
        assert_eq!(
            installed_root(&temp.path().join("target/debug").join(filename)),
            None
        );
        assert_eq!(
            installed_root(&temp.path().join("releases/v1/another-program")),
            None
        );
    }
    #[test]
    fn installation_replaces_bytes_and_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("perfil com espaços/releases/v1/crexe");
        fs::write(&source, b"first").unwrap();
        install_file(&source, &target).unwrap();
        fs::write(&source, b"second").unwrap();
        install_file(&source, &target).unwrap();
        install_file(&target, &target).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"second");
    }
}
