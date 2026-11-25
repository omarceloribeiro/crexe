//! Per-user candidates for the Windows default-app UI. Never writes UserChoice
//! or the default value of a document extension. The user makes that choice.
use super::{executable_text, notify_shell, optional_key, same_command, same_executable, string};
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{ffi::c_void, io, path::Path};
use winreg::{enums::*, types::ToRegValue, RegKey, RegValue};

const APPS: &str = "Software\\CREXE\\ReaderApps";
const REGISTERED: &str = "Software\\RegisteredApplications";
const OWNER: &str = "CREXEOwner";

struct Registration {
    extension: String,
    prog_id: String,
    app_path: String,
    classes_path: String,
    capabilities: String,
    with_path: String,
    name: String,
    exe: String,
    command: String,
}

impl Registration {
    fn new(exe: &Path, extension: &str) -> Result<Self> {
        validate_extension(extension)?;
        let exe = executable_text(exe)?;
        let hash = format!("{:x}", Sha256::digest(extension.as_bytes()));
        // ProgIDs are limited to 39 characters by the Shell naming guidance.
        let prog_id = format!("CREXE.Reader.{}.v1", &hash[..23]);
        let app_path = format!("{APPS}\\{hash}");
        Ok(Self {
            extension: format!(".{extension}"),
            classes_path: format!("Software\\Classes\\{prog_id}"),
            capabilities: format!("{app_path}\\Capabilities"),
            with_path: format!("Software\\Classes\\.{extension}\\OpenWithProgids"),
            name: format!("CREXE — Leitor .{extension}"),
            command: format!("\"{exe}\" open-kept-reader --extension={extension} --ui -- \"%1\""),
            prog_id,
            app_path,
            exe,
        })
    }

    fn values(&self) -> Vec<Mutation> {
        vec![
            Mutation::text(&self.app_path, OWNER, &self.exe),
            Mutation::text(&self.app_path, "Extension", &self.extension),
            Mutation::text(&self.classes_path, OWNER, &self.exe),
            Mutation::text(&self.classes_path, "", &self.name),
            Mutation::text(&self.classes_path, "FriendlyTypeName", &self.name),
            Mutation::text(&format!("{}\\shell\\open\\command", self.classes_path), "", &self.command),
            Mutation::text(&format!("{}\\Application", self.classes_path), "ApplicationName", &self.name),
            Mutation::text(&self.capabilities, "ApplicationName", &self.name),
            Mutation::text(&self.capabilities, "ApplicationDescription", &format!("Leitor local gerado com CREXE para {}. A escolha de padrão é feita pelo usuário no Windows.", self.extension)),
            Mutation::text(&format!("{}\\FileAssociations", self.capabilities), &self.extension, &self.prog_id),
            Mutation { path: self.with_path.clone(), name: self.prog_id.clone(), value: Some(RegValue { bytes: Vec::new(), vtype: REG_NONE }) },
            Mutation::text(REGISTERED, &self.name, &self.capabilities),
        ]
    }

    fn check_owner(&self, user: &RegKey) -> Result<()> {
        for path in [&self.app_path, &self.classes_path] {
            if let Some(key) = optional_key(user, path)? {
                if !string(&key, OWNER)?
                    .as_deref()
                    .is_some_and(|owner| same_executable(owner, &self.exe))
                {
                    bail!("Outro programa ou instalação possui o registro deste leitor; nada foi alterado.");
                }
            }
        }
        if let Some(key) = optional_key(
            user,
            &format!("{}\\shell\\open\\command", self.classes_path),
        )? {
            if string(&key, "")?
                .as_deref()
                .is_some_and(|value| !same_command(value, &self.command))
            {
                bail!("O comando registrado do leitor pertence a outra instalação.");
            }
        }
        if let Some(key) = optional_key(user, &self.app_path)? {
            if string(&key, "Extension")?
                .as_deref()
                .is_some_and(|value| value != self.extension)
            {
                bail!("A extensão do registro do leitor não corresponde ao identificador.");
            }
        }
        if let Some(key) = optional_key(user, REGISTERED)? {
            if string(&key, &self.name)?
                .as_deref()
                .is_some_and(|value| value != self.capabilities)
            {
                bail!("Outro aplicativo utiliza o nome registrado deste leitor.");
            }
        }
        if let Some(key) = optional_key(user, &self.with_path)? {
            if let Some(value) = raw(&key, &self.prog_id)? {
                if value.vtype != REG_NONE || !value.bytes.is_empty() {
                    bail!("O candidato OpenWith do leitor foi alterado por outro programa.");
                }
            }
        }
        Ok(())
    }
}

fn validate_extension(extension: &str) -> Result<()> {
    if extension.is_empty()
        || extension.len() > 64
        || extension.to_lowercase() != extension
        || !extension
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
    {
        bail!("Extensão inválida para registrar o leitor.");
    }
    Ok(())
}

struct Mutation {
    path: String,
    name: String,
    value: Option<RegValue>,
}

impl Mutation {
    fn text(path: &str, name: &str, value: &str) -> Self {
        Self {
            path: path.into(),
            name: name.into(),
            value: Some(value.to_reg_value()),
        }
    }
}

fn raw(key: &RegKey, name: &str) -> Result<Option<RegValue>> {
    match key.get_raw_value(name) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn set_raw(key: &RegKey, name: &str, value: &Option<RegValue>) -> Result<()> {
    match value {
        Some(value) => key.set_raw_value(name, value)?,
        None => match key.delete_value(name) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        },
    }
    Ok(())
}

// A small write journal allows an I/O failure to restore every value already
// changed. Owner records also let the next invocation recognize interrupted
// registrations after a process crash, without guessing at shared defaults.
fn apply(user: &RegKey, changes: &[Mutation], fail_after: Option<usize>) -> Result<()> {
    let mut undo: Vec<Mutation> = Vec::new();
    let mut created: Vec<String> = Vec::new();
    let result = (|| {
        for (index, change) in changes.iter().enumerate() {
            if fail_after == Some(index) {
                bail!("Injected registry write failure");
            }
            let mut prefix = String::new();
            for part in change.path.split('\\') {
                if !prefix.is_empty() {
                    prefix.push('\\');
                }
                prefix.push_str(part);
                let (_, disposition) = user.create_subkey(&prefix)?;
                if disposition == REG_CREATED_NEW_KEY {
                    created.push(prefix.clone());
                }
            }
            let key = user.open_subkey_with_flags(&change.path, KEY_READ | KEY_WRITE)?;
            undo.push(Mutation {
                path: change.path.clone(),
                name: change.name.clone(),
                value: raw(&key, &change.name)?,
            });
            set_raw(&key, &change.name, &change.value)?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        let rollback = (|| -> Result<()> {
            for change in undo.iter().rev() {
                let key = user.open_subkey_with_flags(&change.path, KEY_READ | KEY_WRITE)?;
                set_raw(&key, &change.name, &change.value)?;
            }
            Ok(())
        })();
        for path in created.iter().rev() {
            remove_empty(user, path);
        }
        rollback.context(
            "Falha ao restaurar o registro; tente registrar ou remover novamente o leitor",
        )?;
        return Err(error);
    }
    Ok(())
}

fn remove_empty(user: &RegKey, path: &str) {
    // RegDeleteKey would also remove values. Check that this key is truly empty;
    // never remove a neighbour/foreign value or an extension's default.
    if let Ok(Some(key)) = optional_key(user, path) {
        if key.enum_keys().next().is_none() && key.enum_values().next().is_none() {
            drop(key);
            let _ = user.delete_subkey(path);
        }
    }
}

fn register_at(
    user: &RegKey,
    exe: &Path,
    extension: &str,
    fail_after: Option<usize>,
) -> Result<()> {
    let registration = Registration::new(exe, extension)?;
    registration.check_owner(user)?;
    apply(user, &registration.values(), fail_after)
}

fn unregister_at(user: &RegKey, exe: &Path, extension: &str) -> Result<()> {
    let registration = Registration::new(exe, extension)?;
    registration.check_owner(user)?;
    // An absent candidate must not create empty shared registry branches.
    let changes: Vec<_> = registration
        .values()
        .into_iter()
        .rev()
        .filter_map(|change| match optional_key(user, &change.path) {
            Ok(Some(_)) => Some(Ok(Mutation {
                value: None,
                ..change
            })),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<_>>()?;
    apply(user, &changes, None)?;
    let mut paths: Vec<_> = registration
        .values()
        .into_iter()
        .map(|change| change.path)
        .collect();
    paths.push(format!("{}\\shell\\open", registration.classes_path));
    paths.push(format!("{}\\shell", registration.classes_path));
    paths.sort_by_key(|path| std::cmp::Reverse(path.len()));
    paths.dedup();
    for path in paths {
        remove_empty(user, &path);
    }
    Ok(())
}

pub(super) fn validate_reader_registration(exe: &Path, extension: &str) -> Result<()> {
    Registration::new(exe, extension)?.check_owner(&RegKey::predef(HKEY_CURRENT_USER))
}

pub(super) fn register_reader(exe: &Path, extension: &str) -> Result<()> {
    register_at(&RegKey::predef(HKEY_CURRENT_USER), exe, extension, None)?;
    notify_shell();
    Ok(())
}

pub(super) fn unregister_reader(exe: &Path, extension: &str) -> Result<()> {
    unregister_at(&RegKey::predef(HKEY_CURRENT_USER), exe, extension)?;
    notify_shell();
    Ok(())
}

fn unregister_all_at(user: &RegKey, exe: &Path) -> Result<()> {
    let Some(apps) = optional_key(user, APPS)? else {
        return Ok(());
    };
    let owner = executable_text(exe)?;
    let mut registrations = Vec::new();
    for item in apps.enum_keys() {
        let key_name = item?;
        let key = apps.open_subkey(&key_name)?;
        if !string(&key, OWNER)?
            .as_deref()
            .is_some_and(|value| same_executable(value, &owner))
        {
            continue;
        }
        let extension = string(&key, "Extension")?
            .context("Registro de leitor incompleto; repare a preparação antes de remover")?;
        let extension = extension
            .strip_prefix('.')
            .context("Extensão registrada inválida")?;
        let registration = Registration::new(exe, extension)?;
        if registration.app_path != format!("{APPS}\\{key_name}") {
            bail!("Identificador de registro inválido");
        }
        registration.check_owner(user)?;
        registrations.push(extension.to_owned());
    }
    for extension in registrations {
        unregister_at(user, exe, &extension)?;
    }
    Ok(())
}

pub(super) fn unregister_all_readers(exe: &Path) -> Result<()> {
    unregister_all_at(&RegKey::predef(HKEY_CURRENT_USER), exe)
}

fn settings_uri(extension: &str) -> Result<String> {
    validate_extension(extension)?;
    // registeredAppUser expects the exact RegisteredApplications value name.
    let name = format!("CREXE — Leitor .{extension}");
    let escaped: String = name
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect();
    Ok(format!(
        "ms-settings:defaultapps?registeredAppUser={escaped}"
    ))
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn ShellExecuteW(
        window: *mut c_void,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show: i32,
    ) -> *mut c_void;
}

fn launch_settings(uri: &str) -> bool {
    let verb: Vec<u16> = "open\0".encode_utf16().collect();
    let uri: Vec<u16> = uri.encode_utf16().chain(Some(0)).collect();
    // ShellExecute only opens the fixed Windows Settings URI, never a document.
    unsafe {
        (ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            uri.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        ) as isize)
            > 32
    }
}

pub(super) fn choose_reader_default(extension: &str) -> Result<()> {
    // Shell association queries can return a fallback candidate for a new
    // extension even before the user has chosen a default. This explicit action
    // always offers Windows Settings; ordinary document opens do not prompt.
    if launch_settings(&settings_uri(extension)?) || launch_settings("ms-settings:defaultapps") {
        return Ok(());
    }
    bail!("Não foi possível abrir Configurações. Abra Aplicativos padrão no Windows e escolha CREXE — Leitor .{extension} para .{extension}.");
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        user: RegKey,
        path: String,
        root: RegKey,
        exe: std::path::PathBuf,
        _temp: tempfile::TempDir,
    }
    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let path = format!(
                "Software\\CREXE.Tests\\{}",
                temp.path().file_name().unwrap().to_string_lossy()
            );
            let user = RegKey::predef(HKEY_CURRENT_USER);
            let root = user.create_subkey(&path).unwrap().0;
            let exe = temp.path().join("usuário & nome com espaço\\crexe.exe");
            Self {
                user,
                path,
                root,
                exe,
                _temp: temp,
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = self.user.delete_subkey_all(&self.path);
        }
    }

    #[test]
    fn candidate_is_discoverable_and_preserves_defaults_and_foreign_values() {
        let f = Fixture::new();
        let ext = f.root.create_subkey("Software\\Classes\\.xyz").unwrap().0;
        ext.set_value("", &"Previous.App").unwrap();
        let choice = f.root.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts\\.xyz\\UserChoice").unwrap().0;
        choice.set_value("ProgId", &"User.Chosen").unwrap();
        ext.create_subkey("OpenWithProgids")
            .unwrap()
            .0
            .set_value("Other.App", &"")
            .unwrap();
        register_at(&f.root, &f.exe, "xyz", None).unwrap();
        register_at(&f.root, &f.exe, "xyz", None).unwrap();
        let r = Registration::new(&f.exe, "xyz").unwrap();
        assert_eq!(r.prog_id.len(), 39);
        assert_eq!(string(&ext, "").unwrap().as_deref(), Some("Previous.App"));
        assert_eq!(
            string(&choice, "ProgId").unwrap().as_deref(),
            Some("User.Chosen")
        );
        assert_eq!(
            string(&f.root.open_subkey(REGISTERED).unwrap(), &r.name).unwrap(),
            Some(r.capabilities.clone())
        );
        assert_eq!(
            string(
                &f.root.open_subkey(&r.capabilities).unwrap(),
                "ApplicationName"
            )
            .unwrap(),
            Some(r.name.clone())
        );
        assert_eq!(
            string(
                &f.root
                    .open_subkey(format!("{}\\shell\\open\\command", r.classes_path))
                    .unwrap(),
                ""
            )
            .unwrap(),
            Some(format!(
                "\"{}\" open-kept-reader --extension=xyz --ui -- \"%1\"",
                f.exe.display()
            ))
        );
        unregister_at(&f.root, &f.exe, "xyz").unwrap();
        unregister_at(&f.root, &f.exe, "xyz").unwrap();
        assert_eq!(string(&ext, "").unwrap().as_deref(), Some("Previous.App"));
        assert_eq!(
            string(&choice, "ProgId").unwrap().as_deref(),
            Some("User.Chosen")
        );
        assert_eq!(
            string(&ext.open_subkey("OpenWithProgids").unwrap(), "Other.App")
                .unwrap()
                .as_deref(),
            Some("")
        );
        assert!(optional_key(&f.root, &r.app_path).unwrap().is_none());
        assert!(optional_key(&f.root, &r.classes_path).unwrap().is_none());
    }

    #[test]
    fn first_registration_failure_rolls_back_each_completed_write() {
        for index in 0..12 {
            let f = Fixture::new();
            let ext = f.root.create_subkey("Software\\Classes\\.xyz").unwrap().0;
            ext.set_value("", &"Previous.App").unwrap();
            assert!(register_at(&f.root, &f.exe, "xyz", Some(index)).is_err());
            let r = Registration::new(&f.exe, "xyz").unwrap();
            assert!(optional_key(&f.root, &r.app_path).unwrap().is_none());
            assert!(optional_key(&f.root, &r.classes_path).unwrap().is_none());
            assert_eq!(string(&ext, "").unwrap().as_deref(), Some("Previous.App"));
        }
    }

    #[test]
    fn repeat_failure_keeps_previous_registration_intact() {
        let f = Fixture::new();
        register_at(&f.root, &f.exe, "xyz", None).unwrap();
        let r = Registration::new(&f.exe, "xyz").unwrap();
        for index in 0..12 {
            assert!(register_at(&f.root, &f.exe, "xyz", Some(index)).is_err());
            for value in r.values() {
                assert_eq!(
                    raw(&f.root.open_subkey(&value.path).unwrap(), &value.name).unwrap(),
                    value.value
                );
            }
        }
    }

    #[test]
    fn ownership_conflicts_refuse_without_touching_existing_handlers() {
        let f = Fixture::new();
        register_at(&f.root, &f.exe, "xyz", None).unwrap();
        let other = f.exe.with_file_name("other.exe");
        assert!(register_at(&f.root, &other, "xyz", None).is_err());
        assert!(unregister_at(&f.root, &other, "xyz").is_err());
        let r = Registration::new(&f.exe, "xyz").unwrap();
        let command = f
            .root
            .open_subkey_with_flags(
                format!("{}\\shell\\open\\command", r.classes_path),
                KEY_ALL_ACCESS,
            )
            .unwrap();
        command.set_value("", &"foreign command").unwrap();
        assert!(register_at(&f.root, &f.exe, "xyz", None).is_err());
        assert!(unregister_at(&f.root, &f.exe, "xyz").is_err());
        assert!(optional_key(&f.root, &r.app_path).unwrap().is_some());
    }

    #[test]
    fn removal_of_all_candidates_skips_other_installations_and_shared_keys() {
        let f = Fixture::new();
        register_at(&f.root, &f.exe, "xyz", None).unwrap();
        register_at(&f.root, &f.exe, "abc", None).unwrap();
        let other = f.exe.with_file_name("other.exe");
        register_at(&f.root, &other, "eml", None).unwrap();
        unregister_all_at(&f.root, &f.exe).unwrap();
        assert!(
            optional_key(&f.root, &Registration::new(&f.exe, "xyz").unwrap().app_path)
                .unwrap()
                .is_none()
        );
        assert!(
            optional_key(&f.root, &Registration::new(&other, "eml").unwrap().app_path)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn settings_uri_escapes_names_and_invalid_extensions_never_enter_registry_paths() {
        assert_eq!(
            settings_uri("xyz").unwrap(),
            "ms-settings:defaultapps?registeredAppUser=CREXE%20%E2%80%94%20Leitor%20.xyz"
        );
        assert!(settings_uri("éml").unwrap().ends_with(".%C3%A9ml"));
        for extension in [
            "", ".xyz", "XyZ", "abc\\def", "abc/def", "a b", "abc\n", "abc\0",
        ] {
            assert!(settings_uri(extension).is_err());
        }
    }

    #[test]
    fn candidate_ownership_uses_normalized_executable_paths() {
        let f = Fixture::new();
        let forward = std::path::PathBuf::from(f.exe.to_str().unwrap().replace('\\', "/"));
        let verbatim = std::path::PathBuf::from(format!("\\\\?\\{}", f.exe.display()));
        register_at(&f.root, &forward, "xyz", None).unwrap();
        // Records made before normalization remain removable without accepting
        // different arguments or a different executable.
        let r = Registration::new(&f.exe, "xyz").unwrap();
        for path in [&r.app_path, &r.classes_path] {
            f.root
                .open_subkey_with_flags(path, KEY_ALL_ACCESS)
                .unwrap()
                .set_value(OWNER, &forward.to_str().unwrap())
                .unwrap();
        }
        f.root
            .open_subkey_with_flags(
                format!("{}\\shell\\open\\command", r.classes_path),
                KEY_ALL_ACCESS,
            )
            .unwrap()
            .set_value(
                "",
                &format!(
                    "\"{}\" open-kept-reader --extension=xyz --ui -- \"%1\"",
                    forward.display()
                ),
            )
            .unwrap();
        register_at(&f.root, &verbatim, "xyz", None).unwrap();
        unregister_at(&f.root, &f.exe, "xyz").unwrap();
        assert!(optional_key(&f.root, &r.app_path).unwrap().is_none());
        register_at(&f.root, &forward, "xyz", None).unwrap();
        unregister_all_at(&f.root, &verbatim).unwrap();
        assert!(optional_key(&f.root, &r.app_path).unwrap().is_none());
    }
}
