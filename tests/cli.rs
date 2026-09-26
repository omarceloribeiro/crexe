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
    pause: Arc<AtomicBool>,
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
        let pause = Arc::new(AtomicBool::new(false));
        let worker_pause = Arc::clone(&pause);
        let worker_fail = Arc::clone(&fail);
        let (worker_count, worker_stop) = (Arc::clone(&count), Arc::clone(&stop));
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => serve(stream, &worker_count, &worker_fail, &worker_pause),
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
            pause,
            worker: Some(worker),
        }
    }
}

impl Drop for Provider {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.pause.store(false, Ordering::SeqCst);
        if let Err(error) = self.worker.take().unwrap().join() {
            if !thread::panicking() {
                panic!("mock provider failed: {error:?}");
            }
        }
    }
}

fn serve(mut stream: TcpStream, count: &AtomicUsize, fail: &AtomicBool, pause: &AtomicBool) {
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
    let deepseek = payload["model"] == "deepseek-fixture";
    if deepseek {
        assert_eq!(payload["thinking"]["type"], "disabled");
        assert_eq!(payload["stream"], false);
        assert_eq!(payload["response_format"]["type"], "json_object");
        assert!(payload.get("max_tokens").is_some());
        assert!(payload.get("max_completion_tokens").is_none());
    }
    let number = count.fetch_add(1, Ordering::SeqCst) + 1;
    let started = std::time::Instant::now();
    while pause.load(Ordering::SeqCst) && started.elapsed() < Duration::from_secs(20) {
        thread::sleep(Duration::from_millis(10));
    }
    let message = if payload.to_string().contains("GREEN") {
        "CREXE_FIXTURE_GREEN"
    } else {
        "CREXE_FIXTURE_WHITE"
    };
    let source = if fail.load(Ordering::SeqCst)
        || (payload.to_string().contains("REPAIRME") && number == 1)
    {
        "deliberate syntax error".to_string()
    } else if payload.to_string().contains("LOUD") {
        "fn main() { print!(\"{}\", \"x\".repeat(3 * 1024 * 1024)); }".to_string()
    } else if payload.to_string().contains("SLOW") {
        "fn main() { std::thread::sleep(std::time::Duration::from_secs(60)); }".to_string()
    } else {
        format!("fn main() {{ for name in [\"CREXE_TEST_KEY\", \"OPENAI_API_KEY\", \"crexe_deepseek_api_key\", \"DEEPSEEK_API_KEY\"] {{ assert!(std::env::var_os(name).is_none()); }} println!(\"{message}\"); }}")
    };
    let generated = if deepseek {
        json!({"files": [
            {"path":"src/main.rs","content":"mod app; fn main() { app::run(); }"},
            {"path":"src/app.rs","content":source.replace("fn main()", "pub fn run()")}
        ]})
    } else {
        json!({"files": [{"path": "src/main.rs", "content": source} ]})
    };
    let body = if payload.to_string().contains("INVALID_DEEPSEEK") {
        json!({"choices":[{"finish_reason":"stop","message":{"content":"{\"files\":[]}","reasoning_content":"not-a-source"}}]})
    } else if payload.to_string().contains("TRUNCATED_DEEPSEEK") {
        json!({"choices":[{"finish_reason":"length","message":{"content":generated.to_string()}}]})
    } else if ollama { json!({"done": true, "done_reason": "stop", "message": {"content": generated.to_string()}}) }
        else { json!({"choices": [{"message": {"content": generated.to_string()}, "finish_reason": "stop"}]}) }.to_string();
    let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
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
        .env("crexe_deepseek_api_key", "synthetic-do-not-inherit")
        .env("DEEPSEEK_API_KEY", "synthetic-do-not-inherit")
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
fn deepseek_repairs_multifile_projects_changes_intent_reuses_cache_and_exports() {
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    let home = temp.path().join("home");
    configure(&provider, &home);
    let config_path = home.join("config.toml");
    let config = fs::read_to_string(&config_path)
        .unwrap()
        .replace("kind = \"openai\"", "kind = \"deepseek\"")
        .replace("model = \"fixture\"", "model = \"deepseek-fixture\"");
    fs::write(&config_path, &config).unwrap();
    let mut spec = recipe(&provider);
    spec["prompt"]["user_by_target"]["native"] = json!("REPAIRME WHITE");
    let file = temp.path().join("calculator.crexe");
    fs::write(&file, spec.to_string()).unwrap();
    let output = success(
        Command::new(engine())
            .arg(&file)
            .args(["--max-repairs", "1"])
            .env("CREXE_HOME", &home)
            .env("CREXE_TEST_KEY", "synthetic")
            .output()
            .unwrap(),
    );
    assert!(output.contains("CREXE_FIXTURE_WHITE"));
    assert_eq!(provider.count.load(Ordering::SeqCst), 2);
    assert!(
        success(invoke(Path::new(engine()), temp.path(), &home, &file, true)).contains("Cache hit")
    );
    // Credential source changes must not invalidate the DeepSeek generation identity.
    fs::write(
        &config_path,
        config.replace("CREXE_TEST_KEY", "crexe_deepseek_api_key")
            + "\n[providers.inactive]\nkind='openai'\nbase_url='https://unused.invalid'\nmodel='unused'\napi_key_env='CREXE_TEST_KEY'\n",
    )
    .unwrap();
    assert!(
        success(invoke(Path::new(engine()), temp.path(), &home, &file, true)).contains("Cache hit")
    );
    assert_eq!(provider.count.load(Ordering::SeqCst), 2);
    spec["prompt"]["user_by_target"]["native"] = json!("GREEN");
    fs::write(&file, spec.to_string()).unwrap();
    assert!(
        success(invoke(Path::new(engine()), temp.path(), &home, &file, true))
            .contains("CREXE_FIXTURE_GREEN")
    );
    assert!(
        success(invoke(Path::new(engine()), temp.path(), &home, &file, true)).contains("Cache hit")
    );
    assert_eq!(provider.count.load(Ordering::SeqCst), 3);
    let revision = fs::read_dir(home.join("cache"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find_map(|entry| {
            let manifest: Value =
                serde_json::from_slice(&fs::read(entry.join("current.json")).unwrap()).unwrap();
            let revision = entry.join(manifest["revision"].as_str().unwrap());
            fs::read_to_string(revision.join("src/app.rs"))
                .unwrap()
                .contains("GREEN")
                .then_some(revision)
        })
        .unwrap();
    let archive = temp.path().join("export.zip");
    success(
        Command::new(engine())
            .arg("export")
            .arg(&revision)
            .arg("--output")
            .arg(&archive)
            .env("CREXE_HOME", &home)
            .output()
            .unwrap(),
    );
    let extracted = temp.path().join("extracted");
    let mut zip = zip::ZipArchive::new(fs::File::open(archive).unwrap()).unwrap();
    zip.extract(&extracted).unwrap();
    assert!(extracted.join("src/app.rs").is_file());
    let rebuilt = temp.path().join("rebuilt");
    success(
        Command::new(engine())
            .arg("build")
            .arg(&extracted)
            .arg("--output")
            .arg(&rebuilt)
            .env("CREXE_HOME", &home)
            .output()
            .unwrap(),
    );
    assert!(success(
        Command::new(engine())
            .arg("run")
            .arg(&rebuilt)
            .env("CREXE_HOME", &home)
            .output()
            .unwrap()
    )
    .contains("CREXE_FIXTURE_GREEN"));
    assert_eq!(provider.count.load(Ordering::SeqCst), 3);
}

#[test]
fn invalid_deepseek_output_never_publishes_cache_or_triggers_hidden_retries() {
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    let home = temp.path().join("home");
    configure(&provider, &home);
    let config = home.join("config.toml");
    fs::write(
        &config,
        fs::read_to_string(&config)
            .unwrap()
            .replace("kind = \"openai\"", "kind = \"deepseek\""),
    )
    .unwrap();
    let mut spec = recipe(&provider);
    let file = temp.path().join("invalid.crexe");
    for prompt in ["INVALID_DEEPSEEK", "TRUNCATED_DEEPSEEK"] {
        spec["prompt"]["user_by_target"]["native"] = json!(prompt);
        fs::write(&file, spec.to_string()).unwrap();
        let result = Command::new(engine())
            .arg(&file)
            .args(["--max-repairs", "2"])
            .env("CREXE_HOME", &home)
            .env("CREXE_TEST_KEY", "synthetic")
            .output()
            .unwrap();
        assert!(!result.status.success());
        for entry in fs::read_dir(home.join("cache")).unwrap() {
            assert!(!entry.unwrap().path().join("current.json").exists());
        }
    }
    assert_eq!(provider.count.load(Ordering::SeqCst), 2);
}

#[test]
fn repeated_clicks_share_preparation_but_later_open_another_cached_instance() {
    use std::process::Stdio;
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    provider.pause.store(true, Ordering::SeqCst);
    let home = temp.path().join("home");
    configure(&provider, &home);
    let file = temp.path().join("repeated.crexe");
    let original = serde_json::to_string(&recipe(&provider)).unwrap();
    fs::write(&file, &original).unwrap();
    let spawn = || {
        Command::new(engine())
            .arg(&file)
            .args(["--max-repairs", "0"])
            .env("CREXE_HOME", &home)
            .env("CREXE_TEST_KEY", "synthetic-only")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    };
    let owner = spawn();
    let started = std::time::Instant::now();
    while provider.count.load(Ordering::SeqCst) == 0 {
        assert!(started.elapsed() < Duration::from_secs(10));
        thread::sleep(Duration::from_millis(10));
    }
    let duplicates: Vec<_> = (0..9).map(|_| spawn()).collect();
    for duplicate in duplicates {
        let duplicate_output = duplicate.wait_with_output().unwrap();
        let output = success(duplicate_output);
        assert!(
            output.contains("notificada"),
            "IPC should reach existing process: {output}"
        );
        assert!(!output.contains("CREXE_FIXTURE_WHITE"));
    }
    assert_eq!(provider.count.load(Ordering::SeqCst), 1);
    fs::write(&file, original.replace("WHITE", "GREEN")).unwrap();
    let changed = success(spawn().wait_with_output().unwrap());
    assert!(changed.contains("alterados"));
    assert_eq!(provider.count.load(Ordering::SeqCst), 1);
    fs::write(&file, &original).unwrap();
    provider.pause.store(false, Ordering::SeqCst);
    let output = success(owner.wait_with_output().unwrap());
    assert_eq!(output.matches("CREXE_FIXTURE_WHITE").count(), 1);
    assert!(success(spawn().wait_with_output().unwrap()).contains("CREXE_FIXTURE_WHITE"));
    assert_eq!(
        provider.count.load(Ordering::SeqCst),
        1,
        "reopening must reuse cache"
    );
}

#[test]
fn preparation_recovers_after_owner_crash_without_deleting_lock_metadata() {
    use std::process::Stdio;
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    provider.pause.store(true, Ordering::SeqCst);
    let home = temp.path().join("home");
    configure(&provider, &home);
    let file = temp.path().join("recover.crexe");
    fs::write(&file, serde_json::to_string(&recipe(&provider)).unwrap()).unwrap();
    let mut owner = Command::new(engine())
        .arg(&file)
        .env("CREXE_HOME", &home)
        .env("CREXE_TEST_KEY", "synthetic-only")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let started = std::time::Instant::now();
    while provider.count.load(Ordering::SeqCst) == 0 {
        assert!(started.elapsed() < Duration::from_secs(10));
        thread::sleep(Duration::from_millis(10));
    }
    owner.kill().unwrap();
    owner.wait().unwrap();
    provider.pause.store(false, Ordering::SeqCst);
    assert!(
        success(invoke(Path::new(engine()), temp.path(), &home, &file, true))
            .contains("CREXE_FIXTURE_WHITE")
    );
    assert_eq!(provider.count.load(Ordering::SeqCst), 2);
}

#[test]
fn diagnostics_are_offline_and_sdk_probes_are_explicit_and_policy_controlled() {
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    let home = temp.path().join("home");
    configure(&provider, &home);
    let tools = temp.path().join("tools");
    fs::create_dir(&tools).unwrap();
    let marker = temp.path().join("sdk-queried");
    let source = temp.path().join("sdk.rs");
    fs::write(
        &source,
        r#"
fn main() {
    assert_eq!(std::env::args().nth(1).as_deref(), Some("--list-sdks"));
    assert!(std::env::var_os("CREXE_TEST_KEY").is_none());
    std::fs::write(std::env::var("CREXE_QUERY_MARKER").unwrap(), "queried").unwrap();
    println!("{} [fixture]", std::env::var("CREXE_SDK_VERSION").unwrap());
}
"#,
    )
    .unwrap();
    let compiler = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    success(
        Command::new(compiler)
            .arg(&source)
            .arg("-o")
            .arg(tools.join(if cfg!(windows) {
                "dotnet.exe"
            } else {
                "dotnet"
            }))
            .output()
            .unwrap(),
    );
    let recipe = temp.path().join("intenção.crexe");
    fs::write(&recipe, "Mostrar uma saudação.").unwrap();
    let invoke = || {
        let mut cmd = Command::new(engine());
        cmd.current_dir(temp.path())
            .env("CREXE_HOME", &home)
            .env("PATH", &tools)
            .env("CREXE_QUERY_MARKER", &marker)
            .env("CREXE_SDK_VERSION", "8.0.425")
            .env("CREXE_TEST_KEY", "synthetic-do-not-display-or-inherit")
            .args(["--profile", "dotnet-console"]);
        cmd
    };
    let doctor = success(invoke().arg("doctor").output().unwrap());
    assert!(doctor.contains("Host architecture:") && doctor.contains(".NET 8 SDK"));
    assert!(!doctor.contains("synthetic-do-not-display-or-inherit"));
    let inspected: Value = serde_json::from_str(&success(
        invoke().arg("inspect").arg(&recipe).output().unwrap(),
    ))
    .unwrap();
    assert_eq!(inspected["engine_profile"], "dotnet-console");
    assert_eq!(inspected["architecture"]["engine"], std::env::consts::ARCH);
    assert!(inspected["architecture"]["host"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert_eq!(inspected["requirements"]["tools"], json!(["dotnet"]));
    assert!(
        !marker.exists(),
        "Read-only diagnostics must not execute the SDK"
    );
    assert!(
        success(invoke().args(["doctor", "--check"]).output().unwrap())
            .contains("SDK verification passed")
    );
    assert!(marker.exists());

    let wrong_sdk = invoke()
        .args(["doctor", "--check"])
        .env("CREXE_SDK_VERSION", "9.0.100")
        .output()
        .unwrap();
    assert!(!wrong_sdk.status.success());
    assert!(String::from_utf8_lossy(&wrong_sdk.stderr).contains("requires the .NET 8 SDK"));
    let missing = invoke()
        .args(["doctor", "--check"])
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("not found in PATH"));

    fs::remove_file(&marker).unwrap();
    let config_path = home.join("config.toml");
    let config = fs::read_to_string(&config_path).unwrap() + "\n[execution]\nallowed_tools = []\n";
    fs::write(config_path, config).unwrap();
    let denied = invoke().args(["doctor", "--check"]).output().unwrap();
    assert!(!denied.status.success());
    assert!(String::from_utf8_lossy(&denied.stderr).contains("not approved"));
    assert!(!marker.exists(), "Denied SDK queries must not run");
    assert_eq!(provider.count.load(Ordering::SeqCst), 0);
    assert!(!home.join("cache").exists());
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
fn fast_test_output_overflow_is_rejected_before_cache_publication() {
    let provider = Provider::start();
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("engine");
    configure(&provider, &home);
    let mut spec = recipe(&provider);
    spec["prompt"]["user_by_target"]["native"] = json!("LOUD");
    let run = spec["targets"]["native"]["run"]["cmd"].clone();
    spec["targets"]["native"]["test"] = json!({"steps": [{"cmd": run}]});
    let file = temp.path().join("logs.crexe");
    fs::write(&file, serde_json::to_vec(&spec).unwrap()).unwrap();
    let failed = invoke(Path::new(engine()), temp.path(), &home, &file, true);
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("output exceeded 2 MiB"));
    assert_eq!(provider.count.load(Ordering::SeqCst), 1);
    for entry in fs::read_dir(home.join("cache")).unwrap() {
        assert!(!entry.unwrap().path().join("current.json").exists());
    }
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

#[test]
fn project_operations_rebuild_test_publish_and_export_without_a_provider() {
    let provider = Provider::start();
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("engine");
    configure(&provider, &home);
    let mut spec = recipe(&provider);
    let run = spec["targets"]["native"]["run"]["cmd"].clone();
    spec["targets"]["native"]["test"] = json!({"steps": [{"cmd": run}]});
    let mut publish = spec["targets"]["native"]["build"]["steps"][0]["cmd"].clone();
    publish[3] = json!(if cfg!(windows) {
        "published.exe"
    } else {
        "published"
    });
    spec["targets"]["native"]["publish"] = json!({"steps": [{"cmd": publish}]});
    let file = temp.path().join("project.crexe");
    fs::write(&file, serde_json::to_vec(&spec).unwrap()).unwrap();
    let generated = success(
        Command::new(engine())
            .arg(&file)
            .arg("--no-run")
            .env("CREXE_HOME", &home)
            .env("CREXE_TEST_KEY", "synthetic")
            .output()
            .unwrap(),
    );
    let revision = std::path::PathBuf::from(
        generated
            .lines()
            .find_map(|line| line.strip_prefix("Validated revision: "))
            .unwrap(),
    );
    let manifest = fs::read(revision.join("CREXE-PROJECT.json")).unwrap();
    // Stop the server. Every following operation must work without an API key/network.
    provider.stop.store(true, Ordering::SeqCst);
    let built = temp.path().join("rebuilt project");
    let published = temp.path().join("published project");
    for (operation, output) in [
        ("build", Some(&built)),
        ("test", None),
        ("publish", Some(&published)),
    ] {
        let mut command = Command::new(engine());
        command
            .arg(operation)
            .arg(&revision)
            .env("CREXE_HOME", &home)
            .env_remove("CREXE_TEST_KEY");
        if let Some(path) = output {
            command.arg("--output").arg(path);
        }
        success(command.output().unwrap());
    }
    assert!(published
        .join(if cfg!(windows) {
            "published.exe"
        } else {
            "published"
        })
        .is_file());
    assert_eq!(
        fs::read(revision.join("CREXE-PROJECT.json")).unwrap(),
        manifest
    );
    assert!(success(
        Command::new(engine())
            .arg("run")
            .arg(&built)
            .env("CREXE_HOME", &home)
            .output()
            .unwrap()
    )
    .contains("CREXE_FIXTURE_WHITE"));
    let archive = temp.path().join("project-export.zip");
    success(
        Command::new(engine())
            .arg("export")
            .arg(&built)
            .arg("--output")
            .arg(&archive)
            .env("CREXE_HOME", &home)
            .output()
            .unwrap(),
    );
    let mut zip = zip::ZipArchive::new(fs::File::open(archive).unwrap()).unwrap();
    assert!(zip.by_name("CREXE-PROJECT.json").is_ok());
    assert!(zip.by_name("src/main.rs").is_ok());
    let failed = Command::new(engine())
        .arg("build")
        .arg(&revision)
        .arg("--output")
        .arg(&built)
        .env("CREXE_HOME", &home)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("already exists"));
    assert_eq!(provider.count.load(Ordering::SeqCst), 1);
}

#[test]
fn recipe_cannot_override_local_tool_or_generation_budget() {
    let provider = Provider::start();
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("engine");
    configure(&provider, &home);
    let config = home.join("config.toml");
    let original = fs::read_to_string(&config).unwrap();
    fs::write(
        &config,
        format!("{original}\n[execution]\nallowed_tools=[]\n"),
    )
    .unwrap();
    let file = temp.path().join("denied.crexe");
    fs::write(&file, serde_json::to_vec(&recipe(&provider)).unwrap()).unwrap();
    let blocked = invoke(Path::new(engine()), temp.path(), &home, &file, true);
    assert!(!blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("not approved"));
    assert_eq!(provider.count.load(Ordering::SeqCst), 0);
    fs::write(
        &config,
        format!("{original}\n[execution]\nmax_output_tokens_total=128\n"),
    )
    .unwrap();
    let blocked = invoke(Path::new(engine()), temp.path(), &home, &file, true);
    assert!(!blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("output-token budget exhausted"));
    assert_eq!(provider.count.load(Ordering::SeqCst), 0);
    fs::write(&config, &original).unwrap();
    let mut shell_spec = recipe(&provider);
    let shell = if cfg!(windows) { "cmd" } else { "sh" };
    shell_spec["targets"]["native"]["build"]["steps"][0]["cmd"] = json!([shell]);
    shell_spec["policies"]["commandAllowlist"][std::env::consts::OS] = json!([shell]);
    fs::write(&file, serde_json::to_vec(&shell_spec).unwrap()).unwrap();
    let blocked = invoke(Path::new(engine()), temp.path(), &home, &file, true);
    assert!(!blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("allow_shells"));
    assert_eq!(provider.count.load(Ordering::SeqCst), 0);

    fs::write(
        &config,
        format!("{original}\n[execution]\nmax_provider_requests=1\n"),
    )
    .unwrap();
    let mut broken = recipe(&provider);
    broken["prompt"]["user_by_target"]["native"] = json!("REPAIRME");
    fs::write(&file, serde_json::to_vec(&broken).unwrap()).unwrap();
    let blocked = Command::new(engine())
        .arg(&file)
        .args(["--no-run", "--max-repairs", "2"])
        .env("CREXE_HOME", &home)
        .env("CREXE_TEST_KEY", "synthetic")
        .output()
        .unwrap();
    assert!(!blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("request budget exhausted"));
    assert_eq!(provider.count.load(Ordering::SeqCst), 1);
}
