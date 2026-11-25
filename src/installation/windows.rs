use anyhow::{bail, Context, Result};
use std::{io, path::Path};
use winreg::{enums::*, RegKey};

const PROG_ID: &str = "CREXE.Intent.v1";
const EXTENSION: &str = ".crexe";
const PREVIOUS: &str = "PreviousDefault";
const GENERATE_VERB: &str = "*\\shell\\CREXE.Generate";

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

fn executable_text(exe: &Path) -> Result<&str> {
    let text = exe
        .to_str()
        .context("Windows association requires a Unicode executable path")?;
    if !exe.is_absolute() || text.contains(['"', '\r', '\n']) {
        bail!("Invalid association path");
    }
    Ok(text)
}

fn open_command(exe: &Path) -> Result<String> {
    Ok(format!("\"{}\" exec --ui \"%1\"", executable_text(exe)?))
}

fn generate_command(exe: &Path) -> Result<String> {
    Ok(format!(
        "\"{}\" open-generating --ui -- \"%1\"",
        executable_text(exe)?
    ))
}

fn string(key: &RegKey, name: &str) -> Result<Option<String>> {
    match key.get_value(name) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn optional_key(parent: &RegKey, path: &str) -> Result<Option<RegKey>> {
    match parent.open_subkey_with_flags(path, KEY_READ | KEY_WRITE) {
        Ok(key) => Ok(Some(key)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn check_registration_owner(classes: &RegKey, path: &str, accepted: &[String]) -> Result<()> {
    if let Some(key) = optional_key(classes, path)? {
        if let Some(existing) = string(&key, "")? {
            if !accepted.contains(&existing) {
                bail!(
                    "Another CREXE installation owns this association or menu; unregister it first"
                );
            }
        }
    }
    Ok(())
}

pub(super) fn associate(exe: &Path) -> Result<()> {
    let (classes, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey("Software\\Classes")?;
    register(&classes, exe)?;
    notify_shell();
    Ok(())
}

fn register(classes: &RegKey, exe: &Path) -> Result<()> {
    let command = open_command(exe)?;
    let generate = generate_command(exe)?;
    // Check both owners before writing either integration. A second installation
    // must not leave the first one's .crexe handler or generic menu half-replaced.
    check_registration_owner(
        classes,
        &format!("{PROG_ID}\\shell\\open\\command"),
        &[command.clone(), format!("\"{}\" \"%1\"", exe.display())],
    )?;
    check_registration_owner(
        classes,
        &format!("{GENERATE_VERB}\\command"),
        std::slice::from_ref(&generate),
    )?;
    let (app, _) = classes.create_subkey(PROG_ID)?;
    let (open, _) = app.create_subkey("shell\\open\\command")?;
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
    let (edit, _) = app.create_subkey("shell\\edit")?;
    edit.set_value("", &"Editar")?;
    let (edit_command, _) = edit.create_subkey("command")?;
    edit_command.set_value("", &format!("\"{}\" edit \"%1\"", exe.display()))?;
    let (with, _) = extension.create_subkey("OpenWithProgids")?;
    with.set_raw_value(
        PROG_ID,
        &winreg::RegValue {
            bytes: Vec::new(),
            vtype: REG_NONE,
        },
    )?;
    extension.set_value("", &PROG_ID)?;
    let (generate_verb, _) = classes.create_subkey(GENERATE_VERB)?;
    generate_verb.set_value("", &"Gerar com CREXE")?;
    // A document is supplied explicitly, never made the default application for
    // arbitrary formats. Hide this action for multi-selection in the first v1.
    generate_verb.set_value("NeverDefault", &"")?;
    generate_verb.set_value("MultiSelectModel", &"Single")?;
    let (generate_key, _) = generate_verb.create_subkey("command")?;
    generate_key.set_value("", &generate)?;
    Ok(())
}

pub(super) fn unassociate(exe: &Path) -> Result<()> {
    let (classes, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey("Software\\Classes")?;
    unregister(&classes, exe)?;
    notify_shell();
    Ok(())
}

fn unregister(classes: &RegKey, exe: &Path) -> Result<()> {
    let app = optional_key(classes, PROG_ID)?;
    let generate = optional_key(classes, GENERATE_VERB)?;
    // Missing one integration must not prevent removing the other. Verify every
    // present owner first so refusal never partially removes an installation.
    if let Some(app) = &app {
        let command = app.open_subkey("shell\\open\\command")?;
        if string(&command, "")?.as_deref() != Some(open_command(exe)?.as_str()) {
            bail!("Association belongs to another installation; nothing was removed");
        }
    }
    if let Some(generate) = &generate {
        let command = generate.open_subkey("command")?;
        if string(&command, "")?.as_deref() != Some(generate_command(exe)?.as_str()) {
            bail!("Generate menu belongs to another installation; nothing was removed");
        }
    }
    if let Some(app) = app {
        if let Some(extension) = optional_key(classes, EXTENSION)? {
            if string(&extension, "")?.as_deref() == Some(PROG_ID) {
                match string(&app, PREVIOUS)?.filter(|v| !v.is_empty()) {
                    Some(previous) => extension.set_value("", &previous)?,
                    None => {
                        extension.delete_value("")?;
                    }
                }
            }
            if let Some(with) = optional_key(&extension, "OpenWithProgids")? {
                if let Err(error) = with.delete_value(PROG_ID) {
                    if error.kind() != io::ErrorKind::NotFound {
                        return Err(error.into());
                    }
                }
            }
        }
        // Only our verified ProgID; never delete .crexe or Explorer's UserChoice.
        classes.delete_subkey_all(PROG_ID)?;
    }
    if generate.is_some() {
        // Do not remove the shared *\\shell key or any neighbouring verb.
        classes.delete_subkey_all(GENERATE_VERB)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RegistryFixture {
        user: RegKey,
        key_path: String,
        classes: RegKey,
        _temp: tempfile::TempDir,
        exe: std::path::PathBuf,
    }

    impl RegistryFixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let id = temp.path().file_name().unwrap().to_str().unwrap();
            // No test changes Software\Classes or the user's actual associations.
            let key_path = format!("Software\\CREXE.Tests\\{id}");
            let user = RegKey::predef(HKEY_CURRENT_USER);
            let (classes, _) = user.create_subkey(&key_path).unwrap();
            let exe = temp.path().join("usuário com espaço & sinais\\crexe.exe");
            Self {
                user,
                key_path,
                classes,
                _temp: temp,
                exe,
            }
        }
    }

    impl Drop for RegistryFixture {
        fn drop(&mut self) {
            let _ = self.user.delete_subkey_all(&self.key_path);
        }
    }

    #[test]
    fn generated_reader_menu_preserves_file_defaults_and_other_verbs() {
        let fixture = RegistryFixture::new();
        let classes = &fixture.classes;
        let (markdown, _) = classes.create_subkey(".md").unwrap();
        markdown.set_value("", &"Existing.Markdown").unwrap();
        let (shell, _) = classes.create_subkey("*\\shell").unwrap();
        shell.set_value("", &"Existing.Action").unwrap();
        let (other, _) = shell.create_subkey("Existing.Action\\command").unwrap();
        other
            .set_value("", &"leave this command untouched")
            .unwrap();
        register(classes, &fixture.exe).unwrap();
        register(classes, &fixture.exe).unwrap();
        let menu = classes.open_subkey(GENERATE_VERB).unwrap();
        assert_eq!(
            string(&menu, "").unwrap().as_deref(),
            Some("Gerar com CREXE")
        );
        assert_eq!(
            string(&menu, "MultiSelectModel").unwrap().as_deref(),
            Some("Single")
        );
        assert_eq!(string(&menu, "NeverDefault").unwrap().as_deref(), Some(""));
        assert_eq!(
            string(&menu.open_subkey("command").unwrap(), "").unwrap(),
            Some(format!(
                "\"{}\" open-generating --ui -- \"%1\"",
                fixture.exe.display()
            ))
        );
        assert_eq!(
            string(&markdown, "").unwrap().as_deref(),
            Some("Existing.Markdown")
        );
        assert_eq!(
            string(&shell, "").unwrap().as_deref(),
            Some("Existing.Action")
        );
        unregister(classes, &fixture.exe).unwrap();
        unregister(classes, &fixture.exe).unwrap();
        assert!(optional_key(classes, GENERATE_VERB).unwrap().is_none());
        assert_eq!(
            string(&markdown, "").unwrap().as_deref(),
            Some("Existing.Markdown")
        );
        assert_eq!(
            string(&other, "").unwrap().as_deref(),
            Some("leave this command untouched")
        );
    }

    #[test]
    fn foreign_menu_refuses_registration_before_any_association_change() {
        let fixture = RegistryFixture::new();
        let (extension, _) = fixture.classes.create_subkey(EXTENSION).unwrap();
        extension.set_value("", &"Previous.App").unwrap();
        let (command, _) = fixture
            .classes
            .create_subkey(format!("{GENERATE_VERB}\\command"))
            .unwrap();
        command.set_value("", &"another installation").unwrap();
        assert!(register(&fixture.classes, &fixture.exe).is_err());
        assert!(optional_key(&fixture.classes, PROG_ID).unwrap().is_none());
        assert_eq!(
            string(&extension, "").unwrap().as_deref(),
            Some("Previous.App")
        );
        assert_eq!(
            string(&command, "").unwrap().as_deref(),
            Some("another installation")
        );
    }

    #[test]
    fn foreign_association_refuses_registration_before_menu_creation() {
        let fixture = RegistryFixture::new();
        let (command, _) = fixture
            .classes
            .create_subkey(format!("{PROG_ID}\\shell\\open\\command"))
            .unwrap();
        command.set_value("", &"another installation").unwrap();
        assert!(register(&fixture.classes, &fixture.exe).is_err());
        assert!(optional_key(&fixture.classes, GENERATE_VERB)
            .unwrap()
            .is_none());
        assert!(optional_key(&fixture.classes, EXTENSION).unwrap().is_none());
    }

    #[test]
    fn removal_checks_both_owners_and_can_remove_an_orphaned_menu() {
        let fixture = RegistryFixture::new();
        register(&fixture.classes, &fixture.exe).unwrap();
        let command = fixture
            .classes
            .open_subkey_with_flags(format!("{GENERATE_VERB}\\command"), KEY_READ | KEY_WRITE)
            .unwrap();
        command.set_value("", &"another installation").unwrap();
        assert!(unregister(&fixture.classes, &fixture.exe).is_err());
        assert!(optional_key(&fixture.classes, PROG_ID).unwrap().is_some());
        assert_eq!(
            string(&command, "").unwrap().as_deref(),
            Some("another installation")
        );
        command
            .set_value("", &generate_command(&fixture.exe).unwrap())
            .unwrap();
        fixture.classes.delete_subkey_all(PROG_ID).unwrap();
        unregister(&fixture.classes, &fixture.exe).unwrap();
        assert!(optional_key(&fixture.classes, GENERATE_VERB)
            .unwrap()
            .is_none());
    }

    #[test]
    fn shell_commands_reject_unquotable_paths() {
        for path in [
            "relative.exe",
            "C:\\bad\"path\\crexe.exe",
            "C:\\bad\npath\\crexe.exe",
            "C:\\bad\rpath\\crexe.exe",
        ] {
            assert!(open_command(Path::new(path)).is_err());
            assert!(generate_command(Path::new(path)).is_err());
        }
    }

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
        let edit = classes
            .open_subkey(format!("{PROG_ID}\\shell\\edit\\command"))
            .unwrap();
        assert_eq!(
            string(&edit, "").unwrap(),
            Some(format!("\"{}\" edit \"%1\"", exe.display()))
        );
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
