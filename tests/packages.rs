//! Package transport exercises the real engine with a local synthetic provider.
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

fn command(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_crexe"));
    cmd.env("CREXE_HOME", home)
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1");
    cmd
}
fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}
fn archive(path: &Path, entries: &[(&str, &[u8])]) {
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    for (name, bytes) in entries {
        zip.start_file(
            *name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)
                .unix_permissions(0o644),
        )
        .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
}
fn find_file(root: &Path, name: &str) -> PathBuf {
    for entry in fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == name {
            return entry.path();
        }
        if entry.file_type().unwrap().is_dir() {
            let found = find_file(&entry.path(), name);
            if found.exists() {
                return found;
            }
        }
    }
    root.join("missing-fixture-file")
}

#[test]
fn dispatch_inspects_sdd_by_content_and_rejects_unsupported_archives_before_configuration() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let file = temp.path().join("especificação.crexe");
    archive(
        &file,
        &[
            ("requirements.md", b"# Requirements\nCalculate 6 * 7"),
            ("tests.md", b"# Acceptance\n42"),
        ],
    );
    let info: Value = serde_json::from_str(&success(
        command(&home).arg("inspect").arg(&file).output().unwrap(),
    ))
    .unwrap();
    assert_eq!(info["format"], "crexp");
    assert!(
        !home.exists(),
        "Inspection must not materialize or configure anything"
    );
    assert!(!command(&home)
        .arg("edit")
        .arg(&file)
        .output()
        .unwrap()
        .status
        .success());
    for entries in [
        vec![("notes.md", b"Unrelated notes".as_slice())],
        vec![
            ("requirements.md", b"Spec".as_slice()),
            ("../escape.md", b"No".as_slice()),
        ],
        vec![
            ("META-INF/crexe-bundle.json", b"{}".as_slice()),
            ("requirements.md", b"Spec".as_slice()),
        ],
        vec![
            ("requirements.md", b"Spec".as_slice()),
            ("payload.sh", b"No".as_slice()),
        ],
    ] {
        archive(&file, &entries);
        let result = command(&home).arg(&file).arg("--no-run").output().unwrap();
        assert!(!result.status.success());
        assert!(
            !home.exists(),
            "Invalid archives must fail before local setup/provider"
        );
    }
    for extension in ["crexp", "crexb"] {
        let file = temp.path().join(format!("app.{extension}"));
        fs::write(&file, "Not a ZIP").unwrap();
        let output = command(&home).arg(&file).output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("valid ZIP"));
        let output = command(&home)
            .arg("open-generating")
            .arg(&file)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("Sem suporte"));
    }
    let mislabeled = temp.path().join("spec.crexb");
    archive(&mislabeled, &[("requirements.md", b"Specification")]);
    let output = command(&home).arg(&mislabeled).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("never falls back to generation"));
    fs::write(&file, b"PK\x03").unwrap();
    assert!(!command(&home)
        .arg("inspect")
        .arg(&file)
        .output()
        .unwrap()
        .status
        .success());
    fs::write(&file, "PK é uma abreviação. Crie um programa simples.").unwrap();
    let info: Value = serde_json::from_str(&success(
        command(&home)
            .arg("inspect")
            .arg(&file)
            .args(["--profile", "dotnet-console"])
            .output()
            .unwrap(),
    ))
    .unwrap();
    assert_eq!(info["format"], "plaintext");
}

const SOURCE: &str = r#"using System;
using System.IO;
class Program {
    static int Main(string[] args) {
        if (Array.IndexOf(args, "--crexe-self-test") >= 0) return 6 * 7 == 42 ? 0 : 1;
        var data = Environment.GetEnvironmentVariable("CREXE_DATA_DIR") ?? throw new Exception("No data dir");
        if (Path.GetFullPath(Environment.CurrentDirectory) != Path.GetFullPath(data)) return 3;
        var file = Path.Combine(data, "open-count.txt");
        var count = File.Exists(file) ? int.Parse(File.ReadAllText(file)) : 0;
        File.WriteAllText(file, (++count).ToString());
        Console.WriteLine("BUNDLE42:" + count);
        return 0;
    }
}
"#;

struct Provider {
    url: String,
    requests: Arc<Mutex<Vec<Value>>>,
    fail: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Provider {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let fail = Arc::new(AtomicBool::new(false));
        let (captured, stopped, failed) = (requests.clone(), stop.clone(), fail.clone());
        let worker = thread::spawn(move || {
            while !stopped.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => serve(stream, &captured, &failed),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => panic!("provider: {error}"),
                }
            }
        });
        Self {
            url,
            requests,
            fail,
            stop,
            worker: Some(worker),
        }
    }
    fn configure(&self, home: &Path) {
        fs::create_dir_all(home).unwrap();
        fs::write(home.join("config.toml"), format!("version=1\ndefault_provider='fixture'\n[execution]\nmax_provider_requests=1\n[providers.fixture]\nkind='ollama'\nmodel='package-fixture'\nbase_url='{}'\ntimeout_seconds=30\n", self.url)).unwrap();
    }
    fn count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}
fn serve(mut stream: TcpStream, requests: &Mutex<Vec<Value>>, fail: &AtomicBool) {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    let start = loop {
        let n = stream.read(&mut buffer).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&buffer[..n]);
        assert!(bytes.len() < 1_000_000);
        if let Some(pos) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            break pos + 4;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..start]).to_ascii_lowercase();
    let length: usize = headers
        .lines()
        .find_map(|line| line.strip_prefix("content-length:"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(length < 1_000_000);
    while bytes.len() < start + length {
        let n = stream.read(&mut buffer).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&buffer[..n]);
    }
    requests
        .lock()
        .unwrap()
        .push(serde_json::from_slice(&bytes[start..start + length]).unwrap());
    if fail.load(Ordering::SeqCst) {
        write!(
            stream,
            "HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
    } else {
        let generated = json!({"files":[{"path":"Program.cs","content":SOURCE}]});
        let body =
            json!({"message":{"role":"assistant","content":generated.to_string()},"done":true,"done_reason":"stop"})
                .to_string();
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
    }
}

#[test]
#[ignore = "requires .NET 8 SDK; local provider, no model or display"]
fn package_generates_once_and_bundle_runs_on_fresh_home_without_provider_or_compiler() {
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    let home = temp.path().join("sender");
    provider.configure(&home);
    let recipe = temp.path().join("personal outer filename.crexp");
    archive(
        &recipe,
        &[
            (
                "requirements.md",
                b"# Requirements\nCalculate 6 * 7. REQUIREMENT_SENTINEL",
            ),
            ("tests.md", b"# Tests\nExpect 42. TEST_SENTINEL"),
        ],
    );
    let original = fs::read(&recipe).unwrap();
    for _ in 0..2 {
        success(
            command(&home)
                .arg(&recipe)
                .args([
                    "--profile",
                    "dotnet-console",
                    "--no-run",
                    "--max-repairs",
                    "0",
                ])
                .output()
                .unwrap(),
        );
    }
    assert_eq!(provider.count(), 1);
    let request = provider.requests.lock().unwrap()[0].to_string();
    assert!(request.contains("REQUIREMENT_SENTINEL") && request.contains("TEST_SENTINEL"));
    assert!(!request.contains("personal outer filename"));
    let bundle = temp.path().join("shared.crexb");
    success(
        command(&home)
            .args(["bundle", "create"])
            .arg(&recipe)
            .arg("--output")
            .arg(&bundle)
            .output()
            .unwrap(),
    );
    let receiver = temp.path().join("receiver");
    let inspect: Value = serde_json::from_str(&success(
        command(&receiver)
            .arg("inspect")
            .arg(&bundle)
            .output()
            .unwrap(),
    ))
    .unwrap();
    assert_eq!(inspect["compatible"], true);
    assert!(!receiver.exists());
    success(
        command(&receiver)
            .arg(&bundle)
            .arg("--no-run")
            .output()
            .unwrap(),
    );
    assert!(!receiver.exists());
    let extracted = temp.path().join("restored.crexp");
    success(
        command(&receiver)
            .args(["bundle", "extract-recipe"])
            .arg(&bundle)
            .arg("--output")
            .arg(&extracted)
            .output()
            .unwrap(),
    );
    assert_eq!(fs::read(extracted).unwrap(), original);
    // No provider config or executable lookup for a compiler is available here.
    for count in 1..=2 {
        let stdout = success(
            command(&receiver)
                .arg(&bundle)
                .env("PATH", "")
                .output()
                .unwrap(),
        );
        assert!(stdout.contains(&format!("BUNDLE42:{count}")), "{stdout}");
    }
    assert_eq!(provider.count(), 1);
    assert!(!receiver.join("config.toml").exists());
    let data = find_file(&receiver.join("apps"), "open-count.txt");
    assert_eq!(fs::read_to_string(&data).unwrap(), "2");
    let cached_source = find_file(&receiver.join("cache/bundles"), "Program.cs");
    fs::write(cached_source, "modified cache").unwrap();
    let failure = command(&receiver).arg(&bundle).output().unwrap();
    assert!(!failure.status.success());
    assert_eq!(fs::read_to_string(data).unwrap(), "2");
    assert_eq!(provider.count(), 1);
}

#[test]
#[ignore = "requires .NET 8 SDK; local provider, no model or display"]
fn failed_regeneration_and_unaccepted_edits_do_not_replace_bundle_original() {
    assert_failed_regeneration_preserves_original(false);
}

#[test]
#[ignore = "requires .NET 8 SDK; local provider, no model or display"]
fn edited_crexp_retains_incremental_baseline_and_exports_last_accepted_package() {
    assert_failed_regeneration_preserves_original(true);
}

fn assert_failed_regeneration_preserves_original(package: bool) {
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    let home = temp.path().join("sender");
    provider.configure(&home);
    let extension = if package { "crexp" } else { "crexe" };
    let recipe = temp.path().join(format!("app.{extension}"));
    if package {
        archive(
            &recipe,
            &[("requirements.md", b"Simple application. ACCEPTED_ORIGINAL")],
        );
    } else {
        fs::write(&recipe, "Simple application. ACCEPTED_ORIGINAL").unwrap();
    }
    let original = fs::read(&recipe).unwrap();
    success(
        command(&home)
            .arg(&recipe)
            .args([
                "--profile",
                "dotnet-console",
                "--no-run",
                "--max-repairs",
                "0",
            ])
            .output()
            .unwrap(),
    );
    provider.fail.store(true, Ordering::SeqCst);
    if package {
        archive(
            &recipe,
            &[("requirements.md", b"Unaccepted edit after successful build")],
        );
    } else {
        fs::write(&recipe, "Unaccepted edit after successful build").unwrap();
    }
    assert!(!command(&home)
        .arg(&recipe)
        .args([
            "--profile",
            "dotnet-console",
            "--no-run",
            "--rebuild",
            "--max-repairs",
            "0"
        ])
        .output()
        .unwrap()
        .status
        .success());
    let calls = provider.count();
    assert!(
        provider
            .requests
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .to_string()
            .contains("Program.cs"),
        "Regeneration must include the accepted source baseline"
    );
    let bundle = temp.path().join("accepted.crexb");
    success(
        command(&home)
            .args(["bundle", "create"])
            .arg(&recipe)
            .arg("--target")
            .arg("current")
            .arg("--output")
            .arg(&bundle)
            .output()
            .unwrap(),
    );
    let extracted = temp.path().join(format!("original.{extension}"));
    success(
        command(&home)
            .args(["bundle", "extract-recipe"])
            .arg(&bundle)
            .arg("--output")
            .arg(&extracted)
            .output()
            .unwrap(),
    );
    assert_eq!(fs::read(extracted).unwrap(), original);
    assert_eq!(provider.count(), calls);
}
