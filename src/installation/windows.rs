use anyhow::{bail, Context, Result};
use fs2::FileExt;
use std::{fs, io, path::Path};
use winreg::{enums::*, RegKey};

#[path = "reader_defaults.rs"]
mod reader_defaults;

const PROG_ID: &str = "CREXE.Intent.v1";
const EXTENSION: &str = ".crexe";
const PREVIOUS: &str = "PreviousDefault";
const GENERATE_VERB: &str = "*\\shell\\CREXE.Generate";
const KEEP_VERB: &str = "*\\shell\\CREXE.GenerateAndKeep";
const ASSOCIATIONS: [(&str, &str, &str, bool); 3] = [
    (EXTENSION, PROG_ID, "Creative Executable", true),
    (
        ".crexp",
        "CREXE.Package.v1",
        "Creative Executable Package",
        false,
    ),
    (
        ".crexb",
        "CREXE.Bundle.v1",
        "Creative Executable Bundle",
        false,
    ),
];

// Registry identities are shared by every installation for this Windows user.
// CREXE_HOME must not relocate this lock: two separate installations must not
// both pass ownership checks and then interleave their journaled writes.
fn registry_lock() -> Result<fs::File> {
    let local = std::env::var_os("LOCALAPPDATA")
        .context("LOCALAPPDATA não está disponível para proteger o registro do CREXE")?;
    lock_registry_at(&Path::new(&local).join("CREXE"))
}

fn lock_registry_at(directory: &Path) -> Result<fs::File> {
    if !directory.is_absolute() {
        bail!("O diretório do lock de registro deve ser absoluto");
    }
    crate::paths::reject_links(directory)?;
    fs::create_dir_all(directory)?;
    let path = directory.join("registry.lock");
    crate::paths::reject_links(&path)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .context("Não foi possível abrir o lock de registro do CREXE")?;
    lock.try_lock_exclusive().context(
        "Outra operação está atualizando a integração do CREXE com o Windows. Tente novamente após concluir.",
    )?;
    Ok(lock)
}

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

fn executable_text(exe: &Path) -> Result<String> {
    use std::path::{Component, Prefix};
    let text = exe
        .to_str()
        .context("Windows association requires a Unicode executable path")?;
    if text.contains('"') || text.chars().any(char::is_control) {
        bail!("Invalid association path");
    }
    // current_exe may be verbatim while CREXE_HOME contains forward slashes.
    // Use one lexical representation for commands and ownership; do not require
    // the executable to exist (registration fixtures and removals need this).
    let text = text.replace('/', "\\");
    let text = if text
        .get(..8)
        .is_some_and(|value| value.eq_ignore_ascii_case("\\\\?\\UNC\\"))
    {
        format!("\\\\{}", &text[8..])
    } else if let Some(drive) = text.strip_prefix("\\\\?\\") {
        drive.to_owned()
    } else {
        text
    };
    let path = Path::new(&text);
    if !path.is_absolute() {
        bail!("Invalid association path");
    }
    let mut prefix = None;
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Prefix(value) => {
                prefix = Some(match value.kind() {
                    Prefix::Disk(drive) => format!("{}:", drive.to_ascii_uppercase() as char),
                    Prefix::UNC(server, share) => format!(
                        "\\\\{}\\{}",
                        server.to_str().context("Invalid UNC server")?,
                        share.to_str().context("Invalid UNC share")?
                    ),
                    _ => bail!("Unsupported Windows executable namespace"),
                });
            }
            Component::RootDir | Component::CurDir => {}
            Component::ParentDir => {
                parts.pop();
            }
            Component::Normal(part) => {
                parts.push(part.to_str().context("Invalid executable path")?)
            }
        }
    }
    if parts.is_empty() {
        bail!("Association path must name an executable");
    }
    Ok(format!(
        "{}\\{}",
        prefix.context("Missing Windows path prefix")?,
        parts.join("\\")
    ))
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

fn keep_command(exe: &Path) -> Result<String> {
    Ok(format!(
        "\"{}\" open-generating --keep-as-default --ui -- \"%1\"",
        executable_text(exe)?
    ))
}

fn same_executable(left: &str, right: &str) -> bool {
    match (
        executable_text(Path::new(left)),
        executable_text(Path::new(right)),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn same_command(left: &str, right: &str) -> bool {
    let split = |value: &str| -> Option<(String, String)> {
        let quoted = value.strip_prefix('"')?;
        let (executable, arguments) = quoted.split_once('"')?;
        Some((
            executable_text(Path::new(executable)).ok()?,
            arguments.to_owned(),
        ))
    };
    match (split(left), split(right)) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

pub(super) fn register_reader(exe: &Path, extension: &str) -> Result<()> {
    let _lock = registry_lock()?;
    reader_defaults::register_reader(exe, extension)
}

pub(super) fn validate_reader_registration(exe: &Path, extension: &str) -> Result<()> {
    let _lock = registry_lock()?;
    reader_defaults::validate_reader_registration(exe, extension)
}

pub(super) fn unregister_reader(exe: &Path, extension: &str) -> Result<()> {
    let _lock = registry_lock()?;
    reader_defaults::unregister_reader(exe, extension)
}

pub(super) fn choose_reader_default(extension: &str) -> Result<()> {
    reader_defaults::choose_reader_default(extension)
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
            if !accepted
                .iter()
                .any(|candidate| same_command(&existing, candidate))
            {
                bail!(
                    "Another CREXE installation owns this association or menu; unregister it first"
                );
            }
        }
    }
    Ok(())
}

pub(super) fn associate(exe: &Path) -> Result<()> {
    let _lock = registry_lock()?;
    let (classes, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey("Software\\Classes")?;
    register(&classes, exe)?;
    notify_shell();
    Ok(())
}

fn register(classes: &RegKey, exe: &Path) -> Result<()> {
    let command = open_command(exe)?;
    let generate = generate_command(exe)?;
    let keep = keep_command(exe)?;
    // Verify every handler and the menu before changing any integration. A second
    // installation must not leave a mixture of different owners behind.
    for (_, prog_id, _, _) in ASSOCIATIONS {
        let mut accepted = vec![command.clone()];
        if prog_id == PROG_ID {
            accepted.push(format!("\"{}\" \"%1\"", executable_text(exe)?));
        }
        check_registration_owner(
            classes,
            &format!("{prog_id}\\shell\\open\\command"),
            &accepted,
        )?;
    }
    check_registration_owner(
        classes,
        &format!("{GENERATE_VERB}\\command"),
        std::slice::from_ref(&generate),
    )?;
    check_registration_owner(
        classes,
        &format!("{KEEP_VERB}\\command"),
        std::slice::from_ref(&keep),
    )?;
    for (extension_name, prog_id, label, editable) in ASSOCIATIONS {
        let (app, _) = classes.create_subkey(prog_id)?;
        let (open, _) = app.create_subkey("shell\\open\\command")?;
        let (extension, _) = classes.create_subkey(extension_name)?;
        let current = string(&extension, "")?;
        // Store each extension's original default under its own ProgID, once.
        if string(&app, PREVIOUS)?.is_none() {
            app.set_value(
                PREVIOUS,
                &current.as_deref().filter(|v| *v != prog_id).unwrap_or(""),
            )?;
        }
        app.set_value("", &label)?;
        open.set_value("", &command)?;
        if editable {
            let (edit, _) = app.create_subkey("shell\\edit")?;
            edit.set_value("", &"Editar")?;
            let (edit_command, _) = edit.create_subkey("command")?;
            edit_command.set_value("", &format!("\"{}\" edit \"%1\"", executable_text(exe)?))?;
        }
        let (with, _) = extension.create_subkey("OpenWithProgids")?;
        with.set_raw_value(
            prog_id,
            &winreg::RegValue {
                bytes: Vec::new(),
                vtype: REG_NONE,
            },
        )?;
        extension.set_value("", &prog_id)?;
    }
    let (generate_verb, _) = classes.create_subkey(GENERATE_VERB)?;
    generate_verb.set_value("", &"Gerar com CREXE")?;
    // A document is supplied explicitly, never made the default application for
    // arbitrary formats. Hide this action for multi-selection in the first v1.
    generate_verb.set_value("NeverDefault", &"")?;
    generate_verb.set_value("MultiSelectModel", &"Single")?;
    let (generate_key, _) = generate_verb.create_subkey("command")?;
    generate_key.set_value("", &generate)?;
    let (keep_verb, _) = classes.create_subkey(KEEP_VERB)?;
    keep_verb.set_value("", &"Gerar com CREXE e Manter como Programa Padrão")?;
    keep_verb.set_value("NeverDefault", &"")?;
    keep_verb.set_value("MultiSelectModel", &"Single")?;
    keep_verb.create_subkey("command")?.0.set_value("", &keep)?;
    Ok(())
}

pub(super) fn unassociate(exe: &Path) -> Result<()> {
    let _lock = registry_lock()?;
    let (classes, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey("Software\\Classes")?;
    // Preflight both groups before changing either integration.
    check_unregister(&classes, exe)?;
    reader_defaults::unregister_all_readers(exe)?;
    unregister(&classes, exe)?;
    notify_shell();
    Ok(())
}

fn check_unregister(classes: &RegKey, exe: &Path) -> Result<()> {
    let expected_open = open_command(exe)?;
    let expected_generate = generate_command(exe)?;
    let expected_keep = keep_command(exe)?;
    let apps = ASSOCIATIONS
        .iter()
        .map(|(_, prog_id, _, _)| optional_key(classes, prog_id))
        .collect::<Result<Vec<_>>>()?;
    let generate = optional_key(classes, GENERATE_VERB)?;
    let keep = optional_key(classes, KEEP_VERB)?;
    // Missing one integration must not prevent removing the others. Verify every
    // present owner first so refusal never partially removes an installation.
    for app in apps.iter().flatten() {
        let command = app.open_subkey("shell\\open\\command")?;
        if !string(&command, "")?
            .as_deref()
            .is_some_and(|value| same_command(value, &expected_open))
        {
            bail!("Association belongs to another installation; nothing was removed");
        }
    }
    if let Some(generate) = &generate {
        let command = generate.open_subkey("command")?;
        if !string(&command, "")?
            .as_deref()
            .is_some_and(|value| same_command(value, &expected_generate))
        {
            bail!("Generate menu belongs to another installation; nothing was removed");
        }
    }
    if let Some(keep) = &keep {
        let command = keep.open_subkey("command")?;
        if !string(&command, "")?
            .as_deref()
            .is_some_and(|value| same_command(value, &expected_keep))
        {
            bail!("Keep-reader menu belongs to another installation; nothing was removed");
        }
    }
    Ok(())
}

fn unregister(classes: &RegKey, exe: &Path) -> Result<()> {
    check_unregister(classes, exe)?;
    let apps = ASSOCIATIONS
        .iter()
        .map(|(_, prog_id, _, _)| optional_key(classes, prog_id))
        .collect::<Result<Vec<_>>>()?;
    for ((extension_name, prog_id, _, _), app) in ASSOCIATIONS.iter().zip(apps) {
        let Some(app) = app else {
            continue;
        };
        if let Some(extension) = optional_key(classes, extension_name)? {
            if string(&extension, "")?.as_deref() == Some(*prog_id) {
                match string(&app, PREVIOUS)?.filter(|v| !v.is_empty()) {
                    Some(previous) => extension.set_value("", &previous)?,
                    None => {
                        extension.delete_value("")?;
                    }
                }
            }
            if let Some(with) = optional_key(&extension, "OpenWithProgids")? {
                if let Err(error) = with.delete_value(prog_id) {
                    if error.kind() != io::ErrorKind::NotFound {
                        return Err(error.into());
                    }
                }
            }
        }
        // Only our verified ProgID; never delete extension keys or UserChoice.
        classes.delete_subkey_all(prog_id)?;
    }
    if optional_key(classes, GENERATE_VERB)?.is_some() {
        // Do not remove the shared *\\shell key or any neighbouring verb.
        classes.delete_subkey_all(GENERATE_VERB)?;
    }
    if optional_key(classes, KEEP_VERB)?.is_some() {
        classes.delete_subkey_all(KEEP_VERB)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_lock_serializes_handles_and_releases_on_drop() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("CREXE");
        let first = lock_registry_at(&directory).unwrap();
        let error = lock_registry_at(&directory).unwrap_err();
        assert!(error.to_string().contains("Outra operação"));
        drop(first);
        let second = lock_registry_at(&directory).unwrap();
        assert!(directory.join("registry.lock").is_file());
        drop(second);
        assert!(lock_registry_at(&directory).is_ok());
    }

    #[test]
    fn registry_lock_refuses_a_relative_directory() {
        assert!(lock_registry_at(Path::new("relative-crexe-registry")).is_err());
    }

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
        let keep = classes.open_subkey(KEEP_VERB).unwrap();
        assert_eq!(
            string(&keep, "").unwrap().as_deref(),
            Some("Gerar com CREXE e Manter como Programa Padrão")
        );
        assert_eq!(
            string(&keep, "MultiSelectModel").unwrap().as_deref(),
            Some("Single")
        );
        assert_eq!(string(&keep, "NeverDefault").unwrap().as_deref(), Some(""));
        assert_eq!(
            string(&keep.open_subkey("command").unwrap(), "").unwrap(),
            Some(keep_command(&fixture.exe).unwrap())
        );
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
        assert!(optional_key(classes, KEEP_VERB).unwrap().is_none());
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
    fn foreign_keep_menu_blocks_registration_and_removal_without_partial_changes() {
        let fixture = RegistryFixture::new();
        let (command, _) = fixture
            .classes
            .create_subkey(format!("{KEEP_VERB}\\command"))
            .unwrap();
        command.set_value("", &"another installation").unwrap();
        assert!(register(&fixture.classes, &fixture.exe).is_err());
        assert!(optional_key(&fixture.classes, GENERATE_VERB)
            .unwrap()
            .is_none());
        assert!(optional_key(&fixture.classes, PROG_ID).unwrap().is_none());
        command
            .set_value("", &keep_command(&fixture.exe).unwrap())
            .unwrap();
        register(&fixture.classes, &fixture.exe).unwrap();
        command.set_value("", &"another installation").unwrap();
        assert!(unregister(&fixture.classes, &fixture.exe).is_err());
        assert!(optional_key(&fixture.classes, GENERATE_VERB)
            .unwrap()
            .is_some());
        assert!(optional_key(&fixture.classes, PROG_ID).unwrap().is_some());
        command
            .set_value("", &keep_command(&fixture.exe).unwrap())
            .unwrap();
        unregister(&fixture.classes, &fixture.exe).unwrap();
        assert!(optional_key(&fixture.classes, KEEP_VERB).unwrap().is_none());
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
            assert!(keep_command(Path::new(path)).is_err());
        }
    }

    #[test]
    fn commands_normalize_separators_and_verbatim_prefix_without_touching_disk() {
        let expected = r"C:\Users\Nome com espaço\CREXE\releases\v1\crexe.exe";
        for path in [
            r"C:/Users/Nome com espaço/CREXE/releases/v1/crexe.exe",
            r"c:\Users\Nome com espaço\CREXE\releases/v1\crexe.exe",
            r"\\?\C:\Users\Nome com espaço\CREXE\releases\v1\crexe.exe",
            r"C:\Users\Nome com espaço\CREXE\unused\..\releases\.\v1\crexe.exe",
        ] {
            assert_eq!(executable_text(Path::new(path)).unwrap(), expected);
        }
        assert_eq!(
            executable_text(Path::new(r"\\?\UNC\server\share\crexe.exe")).unwrap(),
            r"\\server\share\crexe.exe"
        );
    }

    #[test]
    fn association_registration_and_removal_accept_equivalent_path_spellings() {
        let fixture = RegistryFixture::new();
        let forward = std::path::PathBuf::from(fixture.exe.to_str().unwrap().replace('\\', "/"));
        register(&fixture.classes, &forward).unwrap();
        register(&fixture.classes, &fixture.exe).unwrap();
        unregister(&fixture.classes, &fixture.exe).unwrap();
        assert!(optional_key(&fixture.classes, KEEP_VERB).unwrap().is_none());
        assert!(optional_key(&fixture.classes, PROG_ID).unwrap().is_none());
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

    #[test]
    fn package_and_bundle_handlers_restore_their_own_defaults_without_edit_verbs() {
        let fixture = RegistryFixture::new();
        for (extension, _, _, _) in ASSOCIATIONS {
            let (key, _) = fixture.classes.create_subkey(extension).unwrap();
            key.set_value("", &format!("Previous{extension}")).unwrap();
            let (user_choice, _) = key.create_subkey("UserChoice").unwrap();
            user_choice
                .set_value("ProgId", &"User.Selected.App")
                .unwrap();
            let (other, _) = key.create_subkey("OpenWithProgids").unwrap();
            other.set_value("Other.App", &"").unwrap();
        }
        register(&fixture.classes, &fixture.exe).unwrap();
        register(&fixture.classes, &fixture.exe).unwrap();
        for (extension, prog_id, _, editable) in ASSOCIATIONS {
            let key = fixture.classes.open_subkey(extension).unwrap();
            assert_eq!(string(&key, "").unwrap().as_deref(), Some(prog_id));
            let app = fixture.classes.open_subkey(prog_id).unwrap();
            assert_eq!(
                string(&app, PREVIOUS).unwrap(),
                Some(format!("Previous{extension}"))
            );
            assert_eq!(
                string(&app.open_subkey("shell\\open\\command").unwrap(), "").unwrap(),
                Some(open_command(&fixture.exe).unwrap())
            );
            assert_eq!(
                optional_key(&app, "shell\\edit").unwrap().is_some(),
                editable
            );
        }
        unregister(&fixture.classes, &fixture.exe).unwrap();
        for (extension, prog_id, _, _) in ASSOCIATIONS {
            let key = fixture.classes.open_subkey(extension).unwrap();
            assert_eq!(
                string(&key, "").unwrap(),
                Some(format!("Previous{extension}"))
            );
            assert_eq!(
                string(&key.open_subkey("UserChoice").unwrap(), "ProgId")
                    .unwrap()
                    .as_deref(),
                Some("User.Selected.App")
            );
            let with = key.open_subkey("OpenWithProgids").unwrap();
            assert!(with.get_raw_value(prog_id).is_err());
            assert_eq!(string(&with, "Other.App").unwrap().as_deref(), Some(""));
            assert!(optional_key(&fixture.classes, prog_id).unwrap().is_none());
        }
    }

    #[test]
    fn foreign_package_or_bundle_owner_prevents_any_registration_or_removal() {
        for (_, foreign_id, _, _) in ASSOCIATIONS.into_iter().skip(1) {
            let fixture = RegistryFixture::new();
            let (command, _) = fixture
                .classes
                .create_subkey(format!("{foreign_id}\\shell\\open\\command"))
                .unwrap();
            command.set_value("", &"another installation").unwrap();
            assert!(register(&fixture.classes, &fixture.exe).is_err());
            for (extension, _, _, _) in ASSOCIATIONS {
                assert!(optional_key(&fixture.classes, extension).unwrap().is_none());
            }
            assert!(optional_key(&fixture.classes, PROG_ID).unwrap().is_none());
            assert!(optional_key(&fixture.classes, GENERATE_VERB)
                .unwrap()
                .is_none());

            command
                .set_value("", &open_command(&fixture.exe).unwrap())
                .unwrap();
            register(&fixture.classes, &fixture.exe).unwrap();
            command.set_value("", &"another installation").unwrap();
            assert!(unregister(&fixture.classes, &fixture.exe).is_err());
            for (extension, prog_id, _, _) in ASSOCIATIONS {
                assert!(optional_key(&fixture.classes, prog_id).unwrap().is_some());
                assert_eq!(
                    string(&fixture.classes.open_subkey(extension).unwrap(), "")
                        .unwrap()
                        .as_deref(),
                    Some(prog_id)
                );
            }
            assert!(optional_key(&fixture.classes, GENERATE_VERB)
                .unwrap()
                .is_some());
        }
    }

    #[test]
    fn removing_package_handlers_preserves_new_user_defaults() {
        let fixture = RegistryFixture::new();
        register(&fixture.classes, &fixture.exe).unwrap();
        for (extension, _, _, _) in ASSOCIATIONS {
            let key = fixture
                .classes
                .open_subkey_with_flags(extension, KEY_READ | KEY_WRITE)
                .unwrap();
            key.set_value("", &"Changed.After.Install").unwrap();
        }
        unregister(&fixture.classes, &fixture.exe).unwrap();
        unregister(&fixture.classes, &fixture.exe).unwrap();
        for (extension, _, _, _) in ASSOCIATIONS {
            assert_eq!(
                string(&fixture.classes.open_subkey(extension).unwrap(), "")
                    .unwrap()
                    .as_deref(),
                Some("Changed.After.Install")
            );
        }
    }
}
