use anyhow::{bail, Context, Result};
use std::{io, path::Path};
use winreg::{enums::*, RegKey};

const PROG_ID: &str = "CREXE.Intent.v1";
const EXTENSION: &str = ".crexe";
const PREVIOUS: &str = "PreviousDefault";

#[link(name = "shell32")]
unsafe extern "system" {
    fn SHChangeNotify(
        event: i32,
        flags: u32,
        item1: *const std::ffi::c_void,
        item2: *const std::ffi::c_void,
    );
}

fn notify_shell() {
    // SHCNE_ASSOCCHANGED, SHCNF_IDLIST; no item pointers for this event.
    unsafe {
        SHChangeNotify(0x08000000, 0, std::ptr::null(), std::ptr::null());
    }
}

fn open_command(exe: &Path) -> Result<String> {
    let text = exe
        .to_str()
        .context("Windows association requires a Unicode executable path")?;
    if !exe.is_absolute() || text.contains(['"', '\r', '\n']) {
        bail!("Invalid association path");
    }
    Ok(format!("\"{text}\" exec --ui \"%1\""))
}

fn string(key: &RegKey, name: &str) -> Result<Option<String>> {
    match key.get_value(name) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn associate(exe: &Path) -> Result<()> {
    let (classes, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey("Software\\Classes")?;
    register(&classes, exe)?;
    notify_shell();
    Ok(())
}

fn register(classes: &RegKey, exe: &Path) -> Result<()> {
    let command = open_command(exe)?;
    let (app, _) = classes.create_subkey(PROG_ID)?;
    let (open, _) = app.create_subkey("shell\\open\\command")?;
    if let Some(existing) = string(&open, "")? {
        if existing != command && existing != format!("\"{}\" \"%1\"", exe.display()) {
            bail!("Another CREXE installation owns this association; unregister it first");
        }
    }
    let (extension, _) = classes.create_subkey(EXTENSION)?;
    let current = string(&extension, "")?;
    // Save once, never replace the original default on repeated registration.
    if string(&app, PREVIOUS)?.is_none() {
        app.set_value(
            PREVIOUS,
            &current.as_deref().filter(|v| *v != PROG_ID).unwrap_or(""),
        )?;
    }
    app.set_value("", &"Creative Executable")?;
    open.set_value("", &command)?;
    let (with, _) = extension.create_subkey("OpenWithProgids")?;
    with.set_raw_value(
        PROG_ID,
        &winreg::RegValue {
            bytes: Vec::new(),
            vtype: REG_NONE,
        },
    )?;
    extension.set_value("", &PROG_ID)?;
    Ok(())
}

pub(super) fn unassociate(exe: &Path) -> Result<()> {
    let (classes, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey("Software\\Classes")?;
    unregister(&classes, exe)?;
    notify_shell();
    Ok(())
}

fn unregister(classes: &RegKey, exe: &Path) -> Result<()> {
    let app = match classes.open_subkey_with_flags(PROG_ID, KEY_READ | KEY_WRITE) {
        Ok(app) => app,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let command = app.open_subkey("shell\\open\\command")?;
    if string(&command, "")?.as_deref() != Some(open_command(exe)?.as_str()) {
        bail!("Association belongs to another installation; nothing was removed");
    }
    if let Ok(extension) = classes.open_subkey_with_flags(EXTENSION, KEY_READ | KEY_WRITE) {
        if string(&extension, "")?.as_deref() == Some(PROG_ID) {
            match string(&app, PREVIOUS)?.filter(|v| !v.is_empty()) {
                Some(previous) => extension.set_value("", &previous)?,
                None => {
                    extension.delete_value("")?;
                }
            }
        }
        if let Ok(with) = extension.open_subkey_with_flags("OpenWithProgids", KEY_WRITE) {
            if let Err(error) = with.delete_value(PROG_ID) {
                if error.kind() != io::ErrorKind::NotFound {
                    return Err(error.into());
                }
            }
        }
    }
    // Only our verified ProgID; never delete .crexe or Explorer's UserChoice.
    classes.delete_subkey_all(PROG_ID)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_other_default_and_repeated_registration() {
        let temp = tempfile::tempdir().unwrap();
        let id = temp.path().file_name().unwrap().to_str().unwrap();
        let key_path = format!("Software\\CREXE.Tests\\{id}");
        let user = RegKey::predef(HKEY_CURRENT_USER);
        let (classes, _) = user.create_subkey(&key_path).unwrap();
        let (ext, _) = classes.create_subkey(EXTENSION).unwrap();
        ext.set_value("", &"Previous.App").unwrap();
        let exe = temp.path().join("usuário com espaço\\crexe.exe");
        register(&classes, &exe).unwrap();
        register(&classes, &exe).unwrap();
        assert_eq!(string(&ext, "").unwrap().as_deref(), Some(PROG_ID));
        unregister(&classes, &exe).unwrap();
        assert_eq!(string(&ext, "").unwrap().as_deref(), Some("Previous.App"));
        register(&classes, &exe).unwrap();
        assert!(unregister(&classes, &temp.path().join("another.exe")).is_err());
        ext.set_value("", &"Other.App").unwrap();
        unregister(&classes, &exe).unwrap();
        unregister(&classes, &exe).unwrap();
        assert_eq!(string(&ext, "").unwrap().as_deref(), Some("Other.App"));
        user.delete_subkey_all(key_path).unwrap();
    }
}
