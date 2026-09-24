use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    process::{Command, Output},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

fn engine() -> &'static str {
    env!("CARGO_BIN_EXE_crexe")
}

struct Provider {
    url: String,
    count: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    fail: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Provider {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let fail = Arc::new(AtomicBool::new(false));
        let worker_fail = Arc::clone(&fail);
        let (worker_count, worker_stop) = (Arc::clone(&count), Arc::clone(&stop));
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => serve(stream, &worker_count, &worker_fail),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => panic!("mock provider: {error}"),
                }
            }
        });
        Self {
            url,
            count,
            stop,
            fail,
            worker: Some(worker),
        }
    }
}

impl Drop for Provider {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Err(error) = self.worker.take().unwrap().join() {
            if !thread::panicking() {
                panic!("mock provider failed: {error:?}");
            }
        }
    }
}

fn serve(mut stream: TcpStream, count: &AtomicUsize, fail: &AtomicBool) {
    // On Windows, accepted sockets inherit the listener's nonblocking mode.
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut request = Vec::new();
    let mut buffer = [0; 8192];
    let body_start;
    loop {
        let n = stream.read(&mut buffer).unwrap();
        assert!(n > 0);
        request.extend_from_slice(&buffer[..n]);
        if let Some(pos) = request.windows(4).position(|v| v == b"\r\n\r\n") {
            body_start = pos + 4;
            break;
        }
        assert!(request.len() < 1_000_000);
    }
    let header = String::from_utf8_lossy(&request[..body_start]);
    let ollama = header.starts_with("POST /api/chat ");
    assert!(ollama || header.starts_with("POST /chat/completions "));
    if ollama {
        assert!(!header.to_lowercase().contains("authorization:"));
    }
    let length: usize = header
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|v| v.trim().parse().unwrap())
        })
        .unwrap();
    assert!(length < 1_000_000);
    while request.len() - body_start < length {
        let n = stream.read(&mut buffer).unwrap();
        assert!(n > 0);
        request.extend_from_slice(&buffer[..n]);
    }
    let payload: Value = serde_json::from_slice(&request[body_start..body_start + length]).unwrap();
    let number = count.fetch_add(1, Ordering::SeqCst) + 1;
    let message = if payload.to_string().contains("GREEN") {
        "CREXE_FIXTURE_GREEN"
    } else {
        "CREXE_FIXTURE_WHITE"
    };
    let source = if fail.load(Ordering::SeqCst)
        || (payload.to_string().contains("REPAIRME") && number == 1)
    {
        "deliberate syntax error".to_string()
    } else if payload.to_string().contains("SLOW") {
        "fn main() { std::thread::sleep(std::time::Duration::from_secs(60)); }".to_string()
    } else {
        format!("fn main() {{ assert!(std::env::var_os(\"CREXE_TEST_KEY\").is_none()); assert!(std::env::var_os(\"OPENAI_API_KEY\").is_none()); println!(\"{message}\"); }}")
    };
    let generated = json!({"files": [{"path": "src/main.rs", "content": source} ]});
    let body = if ollama { json!({"done": true, "done_reason": "stop", "message": {"content": generated.to_string()}}) }
        else { json!({"choices": [{"message": {"content": generated.to_string()}, "finish_reason": "stop"}]}) }.to_string();
    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
}

fn recipe(_provider: &Provider) -> Value {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let name = Path::new(&rustc).file_name().unwrap().to_string_lossy();
    json!({
        "version": "1.0",
        "prompt": {"system": "Fixture generator", "user_by_target": {"native": "WHITE"}},
        "selectors": {"target": {"fallback": "native"}},
        "generator": {"provider": "openai-compatible", "baseUrl": "http://untrusted.invalid", "apiKeyEnv": "CREXE_TEST_KEY", "model": "fixture"},
        "policies": {"commandAllowlist": {std::env::consts::OS: [name]}},
        "targets": {"native": {"build": {"steps": [{"cmd": [rustc, "src/main.rs", "-o", "fixture-app{{EXE_EXT}}"]}]}, "run": {"cmd": ["fixture-app{{EXE_EXT}}"]}}}
    })
}

fn configure(provider: &Provider, root: &Path) {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("config.toml"),
        format!(
            r#"version = 1
default_provider = "test"
[providers.test]
kind = "openai"
base_url = "{}"
model = "fixture"
api_key_env = "CREXE_TEST_KEY"
"#,
            provider.url
        ),
    )
    .unwrap();
}

fn invoke(executable: &Path, cwd: &Path, home: &Path, file: &Path, direct: bool) -> Output {
    let mut command = Command::new(executable);
    if !direct {
        command.arg("exec");
    }
    command
        .arg(file)
        .args(["--max-repairs", "0"])
        .current_dir(cwd)
        .env("CREXE_HOME", home)
        .env("CREXE_TEST_KEY", "fake-not-a-secret")
        .env("OPENAI_API_KEY", "synthetic-do-not-inherit")
        .output()
        .unwrap()
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

#[test]
fn installed_engine_runs_from_other_cwd_regenerates_and_reuses_cache() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("usuário com espaços");
    let other = temp.path().join("outra pasta");
    fs::create_dir(&other).unwrap();
    success(
        Command::new(engine())
            .arg("install")
            .env("CREXE_HOME", &root)
            .output()
            .unwrap(),
    );
    let installed =
        root.join("releases/v1")
            .join(if cfg!(windows) { "crexe.exe" } else { "crexe" });
    assert!(
        success(Command::new(&installed).arg("--version").output().unwrap())
            .contains(env!("CARGO_PKG_VERSION"))
    );
    let provider = Provider::start();
    configure(&provider, &root);
    let mut spec = recipe(&provider);
    let recipe_path = temp.path().join("calculadora de teste.crexe");
    fs::write(&recipe_path, serde_json::to_vec_pretty(&spec).unwrap()).unwrap();
    assert!(
        success(invoke(&installed, &other, &root, &recipe_path, false))
            .contains("CREXE_FIXTURE_WHITE")
    );
    let cached = success(invoke(&installed, &other, &root, &recipe_path, true));
    assert!(cached.contains("Cache hit") && cached.contains("CREXE_FIXTURE_WHITE"));
    assert_eq!(provider.count.load(Ordering::SeqCst), 1);
    spec["prompt"]["user_by_target"]["native"] = json!("GREEN");
    fs::write(&recipe_path, serde_json::to_vec_pretty(&spec).unwrap()).unwrap();
    assert!(
        success(invoke(&installed, &other, &root, &recipe_path, true))
            .contains("CREXE_FIXTURE_GREEN")
    );
    let cached = success(invoke(&installed, &other, &root, &recipe_path, true));
    assert!(cached.contains("Cache hit") && cached.contains("CREXE_FIXTURE_GREEN"));
    assert_eq!(provider.count.load(Ordering::SeqCst), 2);

    let green_entry = fs::read_dir(root.join("cache"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|entry| {
            let manifest: Value =
                serde_json::from_slice(&fs::read(entry.join("current.json")).unwrap()).unwrap();
            let revision = entry.join(manifest["revision"].as_str().unwrap());
            fs::read_to_string(revision.join("src/main.rs"))
                .unwrap()
                .contains("GREEN")
        })
        .unwrap();
    let pointer_before = fs::read(green_entry.join("current.json")).unwrap();
    provider.fail.store(true, Ordering::SeqCst);
    let failed = Command::new(&installed)
        .arg(&recipe_path)
        .arg("--rebuild")
        .args(["--max-repairs", "0"])
        .current_dir(&other)
        .env("CREXE_HOME", &root)
        .env("CREXE_TEST_KEY", "fake-not-a-secret")
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("previous cache preserved"));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("Command failed"));
    assert_eq!(
        fs::read(green_entry.join("current.json")).unwrap(),
        pointer_before
    );
    provider.fail.store(false, Ordering::SeqCst);
    assert!(
        success(invoke(&installed, &other, &root, &recipe_path, true))
            .contains("CREXE_FIXTURE_GREEN")
    );
    assert_eq!(provider.count.load(Ordering::SeqCst), 3);

    let manifest: Value = serde_json::from_slice(&pointer_before).unwrap();
    let revision = green_entry.join(manifest["revision"].as_str().unwrap());
    fs::remove_file(revision.join(if cfg!(windows) {
        "fixture-app.exe"
    } else {
        "fixture-app"
    }))
    .unwrap();
    let recovered = success(invoke(&installed, &other, &root, &recipe_path, true));
    assert!(!recovered.contains("Cache hit") && recovered.contains("CREXE_FIXTURE_GREEN"));
    assert_eq!(provider.count.load(Ordering::SeqCst), 4);
    let temporary = recovered
        .lines()
        .find_map(|line| line.strip_prefix("Workspace: "))
        .unwrap();
    assert!(
        !Path::new(temporary).exists(),
        "temporary workspace was not cleaned"
    );
}

#[test]
fn invalid_version_does_not_call_provider() {
    let provider = Provider::start();
    let temp = tempfile::tempdir().unwrap();
    configure(&provider, &temp.path().join("engine"));
    let mut spec = recipe(&provider);
    spec["version"] = json!(2);
    let file = temp.path().join("future.crexe");
    fs::write(&file, serde_json::to_vec(&spec).unwrap()).unwrap();
    let output = invoke(
        Path::new(engine()),
        temp.path(),
        &temp.path().join("engine"),
        &file,
        true,
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Unsupported recipe version"));
    assert_eq!(provider.count.load(Ordering::SeqCst), 0);
}

#[test]
fn help_and_version_work_without_credentials() {
    assert!(success(Command::new(engine()).arg("--help").output().unwrap()).contains("associate"));
    assert!(
        success(Command::new(engine()).arg("--version").output().unwrap())
            .contains(env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn explicit_run_timeout_terminates_the_child() {
    let provider = Provider::start();
    let temp = tempfile::tempdir().unwrap();
    configure(&provider, &temp.path().join("engine"));
    let mut spec = recipe(&provider);
    spec["prompt"]["user_by_target"]["native"] = json!("SLOW");
    spec["policies"]["timeoutSeconds"] = json!({"run": 1});
    let file = temp.path().join("timeout.crexe");
    fs::write(&file, serde_json::to_vec(&spec).unwrap()).unwrap();
    let start = std::time::Instant::now();
    let output = invoke(
        Path::new(engine()),
        temp.path(),
        &temp.path().join("engine"),
        &file,
        false,
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Command timed out"));
    assert!(start.elapsed() < Duration::from_secs(20));
}

#[test]
fn native_ollama_repairs_a_real_compiler_error_and_exports_portable_sources() {
    let provider = Provider::start();
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("engine");
    configure(&provider, &home);
    let config = home.join("config.toml");
    let settings = fs::read_to_string(&config)
        .unwrap()
        .replace("kind = \"openai\"", "kind = \"ollama\"")
        .replace("api_key_env = \"CREXE_TEST_KEY\"", "");
    fs::write(config, settings).unwrap();
    let mut spec = recipe(&provider);
    spec["prompt"]["user_by_target"]["native"] = json!("REPAIRME");
    let file = temp.path().join("repair.crexe");
    let archive = temp.path().join("complete project.zip");
    fs::write(&file, serde_json::to_vec(&spec).unwrap()).unwrap();
    let output = success(
        Command::new(engine())
            .arg(&file)
            .args(["--no-run", "--max-repairs", "1", "--export"])
            .arg(&archive)
            .env("CREXE_HOME", &home)
            .env("OPENAI_API_KEY", "synthetic-not-for-ollama")
            .output()
            .unwrap(),
    );
    assert!(output.contains("requesting correction 1/1"));
    assert_eq!(provider.count.load(Ordering::SeqCst), 2);
    let mut zip = zip::ZipArchive::new(fs::File::open(archive).unwrap()).unwrap();
    assert!(zip.by_name("src/main.rs").is_ok());
    assert!(zip
        .by_name(if cfg!(windows) {
            "build.bat"
        } else {
            "build.sh"
        })
        .is_ok());
    let extracted = temp.path().join("extracted project");
    zip.extract(&extracted).unwrap();
    let result = if cfg!(windows) {
        Command::new("cmd")
            .args(["/c", "build.bat"])
            .current_dir(&extracted)
            .output()
            .unwrap()
    } else {
        Command::new("sh")
            .arg("build.sh")
            .current_dir(&extracted)
            .output()
            .unwrap()
    };
    success(result);
    let cached = success(
        Command::new(engine())
            .arg(&file)
            .arg("--no-run")
            .env("CREXE_HOME", &home)
            .env_remove("CREXE_TEST_KEY")
            .output()
            .unwrap(),
    );
    assert!(cached.contains("Cache hit"));
    assert_eq!(provider.count.load(Ordering::SeqCst), 2);
}
