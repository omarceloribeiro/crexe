//! Public CLI boundaries for kept readers. No test installs an engine, writes
//! Windows associations, chooses a default application, or contacts a provider.
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn invoke(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_crexe"))
        .args(args)
        .env("CREXE_HOME", home)
        .env_remove("OPENAI_API_KEY")
        .env_remove("CREXE_DEEPSEEK_API_KEY")
        .env_remove("DEEPSEEK_API_KEY")
        .output()
        .unwrap()
}

fn diagnostic(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn keep_as_default_requires_a_final_launch_before_any_preparation() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("not-created");
    let output = invoke(
        &home,
        &[
            "open-generating",
            "--keep-as-default",
            "--no-run",
            "--",
            "not-read.xyz",
        ],
    );
    assert!(!output.status.success());
    assert!(
        diagnostic(&output).contains("--no-run"),
        "{}",
        diagnostic(&output)
    );
    assert!(
        !home.exists(),
        "Conflicting arguments must not prepare a reader"
    );
}

#[test]
fn kept_reader_launcher_requires_the_registered_extension() {
    let temp = tempfile::tempdir().unwrap();
    let output = invoke(temp.path(), &["open-kept-reader", "--", "not-read.xyz"]);
    assert!(!output.status.success());
    assert!(
        diagnostic(&output).contains("--extension"),
        "{}",
        diagnostic(&output)
    );
}

#[test]
fn kept_reader_commands_are_discoverable_and_describe_the_explicit_choice() {
    let temp = tempfile::tempdir().unwrap();
    let help = invoke(temp.path(), &["--help"]);
    assert!(help.status.success());
    let text = diagnostic(&help);
    assert!(
        text.contains("open-kept-reader") && text.contains("reader-defaults"),
        "{text}"
    );
    let help = invoke(temp.path(), &["reader-defaults", "--help"]);
    assert!(help.status.success());
    let text = diagnostic(&help);
    for command in ["list", "forget", "choose"] {
        assert!(text.contains(command), "{text}");
    }
}

#[test]
fn local_launcher_does_not_load_generation_configuration_or_create_reader_state() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("isolated-home");
    fs::create_dir(&home).unwrap();
    fs::write(
        home.join("config.toml"),
        "invalid TOML provider configuration = [",
    )
    .unwrap();
    let file = temp.path().join("segundo documento ç & sinais.crexekeptqa");
    fs::write(&file, "SYNTHETIC_CONTENT_MUST_NEVER_BE_PLANNED").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_crexe"))
        .args(["open-kept-reader", "--extension", "crexekeptqa", "--config"])
        .arg(temp.path().join("missing-config.toml"))
        .args([
            "--provider",
            "does-not-exist",
            "--model",
            "different-model",
            "--",
        ])
        .arg(&file)
        .env("CREXE_HOME", &home)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "A missing kept reader must not silently generate"
    );
    let text = diagnostic(&output);
    assert!(
        !text.contains("missing-config.toml"),
        "Launcher read generation configuration: {text}"
    );
    assert!(
        !text.contains("SYNTHETIC_CONTENT_MUST_NEVER_BE_PLANNED"),
        "Launcher exposed document content: {text}"
    );
    assert!(
        !home.join("readers").exists(),
        "Launcher created reader planning state"
    );
    assert!(
        !home.join("cache").exists(),
        "Launcher created generation cache"
    );
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "SYNTHETIC_CONTENT_MUST_NEVER_BE_PLANNED"
    );
}

#[cfg(windows)]
#[test]
fn unsafe_types_cannot_be_registered_by_the_generic_reader_action() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("not-created");
    for extension in [
        "exe", "bat", "cmd", "ps1", "lnk", "msi", "crexe", "crexp", "crexb",
    ] {
        let file = temp.path().join(format!("synthetic.{extension}"));
        fs::write(&file, "this is not executable content").unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_crexe"))
            .args(["open-generating", "--keep-as-default", "--"])
            .arg(&file)
            .env("CREXE_HOME", &home)
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "Unexpected support for .{extension}"
        );
        assert!(
            !home.exists(),
            "Unsupported .{extension} created managed state"
        );
    }
}

#[cfg(not(windows))]
#[test]
fn default_application_integration_is_explicitly_windows_only() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("not-created");
    for args in [
        vec!["open-generating", "--keep-as-default", "--", "not-read.xyz"],
        vec![
            "open-kept-reader",
            "--extension",
            "xyz",
            "--",
            "not-read.xyz",
        ],
        vec!["reader-defaults", "list"],
    ] {
        let output = invoke(&home, &args);
        assert!(!output.status.success());
        assert!(
            diagnostic(&output).contains("Windows"),
            "{}",
            diagnostic(&output)
        );
        assert!(!home.exists());
    }
}
