use anyhow::{bail, Context, Result};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

#[cfg(windows)]
mod windows;

pub(crate) fn root() -> Result<PathBuf> {
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

pub(crate) fn install() -> Result<()> {
    let source = env::current_exe()?;
    let destination = executable()?;
    install_file(&source, &destination)?;
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
        "Associated .crexe with {}. Windows may require choosing CREXE as the default app.",
        exe.display()
    );
    #[cfg(windows)]
    Ok(())
}

pub(crate) fn unassociate() -> Result<()> {
    #[cfg(windows)]
    {
        windows::unassociate(&executable()?)?;
        println!("Removed CREXE-owned association (other default apps preserved)");
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
