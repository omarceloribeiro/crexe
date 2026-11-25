//! End-to-end reader generation uses a controlled local provider and .NET 8.
//! No real model, paid API, graphical session or private document is used.
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    process::{Child, Command, Output, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const READER: &str = include_str!("fixtures/reader_console.cs");

#[derive(Default)]
struct ProviderState {
    requests: Mutex<Vec<Value>>,
    plans: AtomicUsize,
    generations: AtomicUsize,
    pause_plan: AtomicBool,
    pause_generation: AtomicBool,
    fail_generation: AtomicBool,
    repair_first: AtomicBool,
    fixed_viewer: AtomicBool,
    stop: AtomicBool,
}

struct Provider {
    url: String,
    state: Arc<ProviderState>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Provider {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let state = Arc::new(ProviderState::default());
        let worker_state = Arc::clone(&state);
        let worker = thread::spawn(move || {
            while !worker_state.stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => serve(stream, &worker_state),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("reader fixture provider: {error}"),
                }
            }
        });
        Self {
            url,
            state,
            worker: Some(worker),
        }
    }

    fn counts(&self) -> (usize, usize) {
        (
            self.state.plans.load(Ordering::SeqCst),
            self.state.generations.load(Ordering::SeqCst),
        )
    }

    fn wait_for_counts(&self, expected: (usize, usize)) {
        let start = Instant::now();
        while self.counts() != expected {
            assert!(
                start.elapsed() < Duration::from_secs(45),
                "Provider did not reach {expected:?}; got {:?}",
                self.counts()
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn assert_private(&self, needles: &[&str]) {
        for request in self.state.requests.lock().unwrap().iter() {
            let payload = request.to_string();
            for needle in needles {
                assert!(
                    !payload.contains(needle),
                    "Private document material reached provider: {needle}"
                );
            }
        }
    }
}

impl Drop for Provider {
    fn drop(&mut self) {
        self.state.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            if let Err(error) = worker.join() {
                if !thread::panicking() {
                    panic!("reader provider failed: {error:?}");
                }
            }
        }
    }
}

fn serve(mut stream: TcpStream, state: &ProviderState) {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    let start = loop {
        let n = stream.read(&mut chunk).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&chunk[..n]);
        if let Some(i) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            break i + 4;
        }
        assert!(bytes.len() < 1_000_000);
    };
    let header = String::from_utf8_lossy(&bytes[..start]).to_ascii_lowercase();
    assert!(header.starts_with("post /api/chat "));
    assert!(!header.contains("authorization:"));
    let length: usize = header
        .lines()
        .find_map(|line| line.strip_prefix("content-length:"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(length < 1_000_000);
    while bytes.len() < start + length {
        let n = stream.read(&mut chunk).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&chunk[..n]);
    }
    let payload: Value = serde_json::from_slice(&bytes[start..start + length]).unwrap();
    let planning = payload["format"]["required"] == json!(["prompt"]);
    state.requests.lock().unwrap().push(payload);
    let generation = if planning {
        state.plans.fetch_add(1, Ordering::SeqCst);
        0
    } else {
        state.generations.fetch_add(1, Ordering::SeqCst) + 1
    };
    let pause = if planning {
        &state.pause_plan
    } else {
        &state.pause_generation
    };
    let wait = Instant::now();
    while pause.load(Ordering::SeqCst) && !state.stop.load(Ordering::SeqCst) {
        assert!(
            wait.elapsed() < Duration::from_secs(60),
            "Test did not release provider barrier"
        );
        thread::sleep(Duration::from_millis(10));
    }
    if !planning && state.fail_generation.load(Ordering::SeqCst) {
        let _ = write!(
            stream,
            "HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        return;
    }
    let response = if planning {
        json!({"prompt":"Generate a simple, efficient local viewer following the supplied reader contract. Read the document only from the native argument at runtime. Never modify the selected document. Present UTF-8 text or a clearly identified hex preview."})
    } else {
        let source = if state.repair_first.load(Ordering::SeqCst) && generation == 1 {
            "deliberate fixture C# syntax failure".into()
        } else if state.fixed_viewer.load(Ordering::SeqCst) {
            READER.replace(
                "return Describe(bytes, length > Limit);",
                "return new Preview(\"text\", \"FIXED_PLACEHOLDER\", false);",
            )
        } else {
            READER.to_owned()
        };
        json!({"files":[{"path":"Program.cs","content":source}]})
    };
    let body = json!({"done":true,"done_reason":"stop","message":{"content":response.to_string()}})
        .to_string();
    let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
}

fn configure(provider: &Provider, home: &Path, requests: u8) {
    fs::create_dir_all(home).unwrap();
    fs::write(home.join("config.toml"), format!(
        "version=1\ndefault_provider='test'\n[execution]\nmax_provider_requests={requests}\n[providers.test]\nkind='ollama'\nmodel='reader-fixture'\nbase_url='{}'\ntimeout_seconds=90\n", provider.url
    )).unwrap();
}

fn command(home: &Path, cwd: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_crexe"));
    command
        .args(["open-generating", "--profile", "dotnet-console"])
        .env("CREXE_HOME", home)
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .current_dir(cwd);
    command
}

struct Process {
    child: Child,
    stdout: tempfile::NamedTempFile,
    stderr: tempfile::NamedTempFile,
}

impl Process {
    fn start(command: &mut Command) -> Self {
        let stdout = tempfile::NamedTempFile::new().unwrap();
        let stderr = tempfile::NamedTempFile::new().unwrap();
        command
            .stdout(Stdio::from(stdout.reopen().unwrap()))
            .stderr(Stdio::from(stderr.reopen().unwrap()));
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        Self {
            child: command.spawn().unwrap(),
            stdout,
            stderr,
        }
    }

    fn wait_for_text(&mut self, expected: &str) {
        let start = Instant::now();
        loop {
            let stdout = fs::read_to_string(self.stdout.path()).unwrap();
            if stdout.contains(expected) {
                return;
            }
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "Process finished before {expected}: {stdout}\n{}",
                fs::read_to_string(self.stderr.path()).unwrap()
            );
            assert!(
                start.elapsed() < Duration::from_secs(45),
                "Process did not reach {expected}: {stdout}"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn finish(mut self) -> Output {
        let start = Instant::now();
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            assert!(
                start.elapsed() < Duration::from_secs(180),
                "Reader fixture command exceeded 180 seconds\n{}\n{}",
                fs::read_to_string(self.stdout.path()).unwrap(),
                fs::read_to_string(self.stderr.path()).unwrap()
            );
            thread::sleep(Duration::from_millis(20));
        };
        Output {
            status,
            stdout: fs::read(self.stdout.path()).unwrap(),
            stderr: fs::read(self.stderr.path()).unwrap(),
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            #[cfg(windows)]
            {
                let _ = Command::new("taskkill")
                    .args(["/PID", &self.child.id().to_string(), "/T", "/F"])
                    .output();
            }
            #[cfg(unix)]
            unsafe {
                libc::kill(-(self.child.id() as i32), libc::SIGKILL);
            }
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn run(command: &mut Command) -> Output {
    Process::start(command).finish()
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

fn preview(output: Output) -> Value {
    let stdout = success(output);
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|value| value.get("kind").is_some() && value.get("content").is_some())
        .unwrap_or_else(|| panic!("Reader did not display the selected document: {stdout}"))
}

fn bytes_absent(bytes: &[u8], needles: &[&str]) {
    for needle in needles {
        assert!(
            !bytes
                .windows(needle.len())
                .any(|part| part == needle.as_bytes()),
            "Private document material persisted: {needle}"
        );
    }
}

fn tree_private(root: &Path, needles: &[&str]) {
    if !root.exists() {
        return;
    }
    for entry in fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            tree_private(&entry.path(), needles);
        } else {
            bytes_absent(&fs::read(entry.path()).unwrap(), needles);
        }
    }
}

#[test]
#[ignore = "requires .NET 8 SDK; controlled provider, no model or GUI"]
fn cold_open_cache_and_runtime_arguments_preserve_documents() {
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    let home = temp.path().join("engine");
    configure(&provider, &home, 3);
    let folder = temp.path().join("pasta privada com espaços e acentos ü");
    fs::create_dir(&folder).unwrap();
    let first = folder.join("-PRIVATE_FIRST_NAME.md");
    let second = folder.join("PRIVATE_SECOND_NAME.MD");
    let first_data = "# PRIVATE_FIRST_CONTENT\n- item um é\n";
    let second_data = "# PRIVATE_SECOND_CONTENT\n- item dois\n";
    fs::write(&first, first_data).unwrap();
    fs::write(&second, second_data).unwrap();
    let opened = preview(run(command(&home, &folder)
        .arg("--")
        .arg(first.file_name().unwrap())));
    assert_eq!(
        opened,
        json!({"kind":"text","content":first_data,"truncated":false})
    );
    assert_eq!(provider.counts(), (1, 1));
    assert_eq!(
        preview(run(command(&home, &folder).arg(&second)))["content"],
        second_data
    );
    fs::write(&second, "PRIVATE_UPDATED_CONTENT").unwrap();
    assert_eq!(
        preview(run(command(&home, &folder).arg(&second)))["content"],
        "PRIVATE_UPDATED_CONTENT"
    );
    assert_eq!(
        provider.counts(),
        (1, 1),
        "Document changes must not regenerate a type reader"
    );
    assert_eq!(fs::read_to_string(&first).unwrap(), first_data);
    assert_eq!(
        fs::read_to_string(&second).unwrap(),
        "PRIVATE_UPDATED_CONTENT"
    );
    let needles = [
        "PRIVATE_FIRST_NAME",
        "PRIVATE_SECOND_NAME",
        "PRIVATE_FIRST_CONTENT",
        "PRIVATE_SECOND_CONTENT",
        "PRIVATE_UPDATED_CONTENT",
        "pasta privada",
    ];
    provider.assert_private(&needles);
    tree_private(&home.join("readers"), &needles);
    tree_private(&home.join("cache"), &needles);
}

#[test]
#[ignore = "requires .NET 8 SDK; controlled provider, no model or GUI"]
fn arbitrary_extensions_no_extension_empty_binary_and_truncation_are_supported() {
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    let home = temp.path().join("engine");
    configure(&provider, &home, 3);
    let extension = format!(
        "unknown{}{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let file = temp
        .path()
        .join(format!("PRIVATE_UNKNOWN_NAME.{extension}"));
    fs::write(&file, "PRIVATE_UNKNOWN_CONTENT").unwrap();
    assert_eq!(
        preview(run(command(&home, temp.path()).arg(&file)))["content"],
        "PRIVATE_UNKNOWN_CONTENT"
    );
    assert_eq!(provider.counts(), (1, 1));
    assert!(provider.state.requests.lock().unwrap()[0]
        .to_string()
        .contains(&extension));
    fs::write(&file, [0, 255, 65, 128]).unwrap();
    assert_eq!(
        preview(run(command(&home, temp.path()).arg(&file))),
        json!({"kind":"hex","content":"00ff4180","truncated":false})
    );
    fs::write(&file, []).unwrap();
    assert_eq!(
        preview(run(command(&home, temp.path()).arg(&file))),
        json!({"kind":"text","content":"","truncated":false})
    );
    fs::write(&file, vec![b'x'; 65537]).unwrap();
    let large = preview(run(command(&home, temp.path()).arg(&file)));
    assert_eq!(large["truncated"], true);
    assert_eq!(large["content"].as_str().unwrap().len(), 65536);
    assert_eq!(fs::metadata(&file).unwrap().len(), 65537);
    assert_eq!(provider.counts(), (1, 1));
    let extensionless = temp.path().join("PRIVATE_EXTENSIONLESS_NAME");
    fs::write(&extensionless, "PRIVATE_EXTENSIONLESS_CONTENT").unwrap();
    assert_eq!(
        preview(run(command(&home, temp.path()).arg(&extensionless)))["content"],
        "PRIVATE_EXTENSIONLESS_CONTENT"
    );
    assert_eq!(provider.counts(), (2, 2));
    provider.assert_private(&[
        "PRIVATE_UNKNOWN_NAME",
        "PRIVATE_UNKNOWN_CONTENT",
        "PRIVATE_EXTENSIONLESS_NAME",
        "PRIVATE_EXTENSIONLESS_CONTENT",
    ]);
}

#[test]
#[ignore = "requires .NET 8 SDK; controlled provider, no model or GUI"]
fn one_repair_uses_the_same_budget_without_leaking_documents_or_exporting_them() {
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    provider.state.repair_first.store(true, Ordering::SeqCst);
    let home = temp.path().join("engine");
    configure(&provider, &home, 3);
    let file = temp.path().join("PRIVATE_REPAIR_DOCUMENT.md");
    let data = "PRIVATE_REPAIR_BODY_SENTINEL";
    fs::write(&file, data).unwrap();
    let export = temp.path().join("reader-source.zip");
    let stdout = success(run(command(&home, temp.path())
        .args(["--no-run", "--export"])
        .arg(&export)
        .arg(&file)));
    assert!(!stdout.contains(data));
    assert_eq!(
        provider.counts(),
        (1, 2),
        "One plan + initial code + one repair must share max_provider_requests=3"
    );
    let needles = ["PRIVATE_REPAIR_DOCUMENT", data];
    provider.assert_private(&needles);
    let mut archive = zip::ZipArchive::new(fs::File::open(&export).unwrap()).unwrap();
    for index in 0..archive.len() {
        let mut item = archive.by_index(index).unwrap();
        bytes_absent(item.name().as_bytes(), &needles);
        let mut bytes = Vec::new();
        item.read_to_end(&mut bytes).unwrap();
        bytes_absent(&bytes, &needles);
    }
    tree_private(&home.join("readers"), &needles);
    tree_private(&home.join("cache"), &needles);
    assert_eq!(
        preview(run(command(&home, temp.path()).arg(&file)))["content"],
        data
    );
    assert_eq!(provider.counts(), (1, 2));
    assert_eq!(fs::read_to_string(file).unwrap(), data);
}

#[test]
#[ignore = "requires .NET 8 SDK; controlled provider, no model or GUI"]
fn concurrent_cold_opens_prepare_once_and_deliver_each_document() {
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    provider.state.pause_plan.store(true, Ordering::SeqCst);
    let home = temp.path().join("engine");
    configure(&provider, &home, 3);
    let first = temp.path().join("first.md");
    let second = temp.path().join("second.md");
    fs::write(&first, "FIRST_CONCURRENT_DOCUMENT").unwrap();
    fs::write(&second, "SECOND_CONCURRENT_DOCUMENT").unwrap();
    let a = Process::start(command(&home, temp.path()).arg(&first));
    provider.wait_for_counts((1, 0));
    let mut b = Process::start(command(&home, temp.path()).arg(&second));
    b.wait_for_text("Aguardando leitor em preparação");
    provider.state.pause_plan.store(false, Ordering::SeqCst);
    assert_eq!(preview(a.finish())["content"], "FIRST_CONCURRENT_DOCUMENT");
    assert_eq!(preview(b.finish())["content"], "SECOND_CONCURRENT_DOCUMENT");
    assert_eq!(provider.counts(), (1, 1));
}

#[test]
#[ignore = "requires .NET 8 SDK; controlled provider, no model or GUI"]
fn concurrent_failure_is_shared_without_retry_and_manual_retry_reuses_plan() {
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    provider
        .state
        .pause_generation
        .store(true, Ordering::SeqCst);
    provider.state.fail_generation.store(true, Ordering::SeqCst);
    let home = temp.path().join("engine");
    configure(&provider, &home, 3);
    let first = temp.path().join("first.md");
    let second = temp.path().join("second.md");
    fs::write(&first, "FAILURE_FIRST_DOCUMENT").unwrap();
    fs::write(&second, "FAILURE_SECOND_DOCUMENT").unwrap();
    let a = Process::start(command(&home, temp.path()).arg(&first));
    provider.wait_for_counts((1, 1));
    let mut b = Process::start(command(&home, temp.path()).arg(&second));
    b.wait_for_text("Aguardando leitor em preparação");
    provider
        .state
        .pause_generation
        .store(false, Ordering::SeqCst);
    let first_failure = a.finish();
    let second_failure = b.finish();
    assert!(!first_failure.status.success());
    assert!(!second_failure.status.success());
    assert!(!first_failure.stderr.is_empty() && !second_failure.stderr.is_empty());
    assert_eq!(
        provider.counts(),
        (1, 1),
        "A waiting opener must not trigger another provider request after failure"
    );
    provider
        .state
        .fail_generation
        .store(false, Ordering::SeqCst);
    assert_eq!(
        preview(run(command(&home, temp.path()).arg(&second)))["content"],
        "FAILURE_SECOND_DOCUMENT"
    );
    assert_eq!(
        provider.counts(),
        (1, 2),
        "An explicit retry reuses a valid saved reader plan"
    );
}

#[test]
#[ignore = "requires .NET 8 SDK; controlled provider, no model or GUI"]
fn invalid_input_budget_and_missing_sdk_make_no_provider_request() {
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    let home = temp.path().join("engine");
    configure(&provider, &home, 3);
    for path in [temp.path().join("missing.md"), temp.path().to_path_buf()] {
        assert!(!run(command(&home, temp.path()).arg(path)).status.success());
    }
    let file = temp.path().join("valid.md");
    fs::write(&file, "PREFLIGHT_DOCUMENT").unwrap();
    assert!(!run(command(&home, temp.path()).env("PATH", "").arg(&file))
        .status
        .success());
    configure(&provider, &home, 1);
    let insufficient = run(command(&home, temp.path()).arg(&file));
    assert!(!insufficient.status.success());
    assert_eq!(provider.counts(), (0, 0));
    configure(&provider, &home, 3);
    let config = home.join("config.toml");
    fs::write(
        &config,
        fs::read_to_string(&config)
            .unwrap()
            .replace("model='reader-fixture'", "model=''"),
    )
    .unwrap();
    assert!(!run(command(&home, temp.path()).arg(&file)).status.success());
    assert_eq!(provider.counts(), (0, 0));
    assert_eq!(fs::read_to_string(file).unwrap(), "PREFLIGHT_DOCUMENT");
}

#[test]
#[ignore = "requires .NET 8 SDK; controlled provider, no model or GUI"]
fn recipe_delegation_does_not_plan_a_reader() {
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    let home = temp.path().join("engine");
    configure(&provider, &home, 3);
    let recipe = temp.path().join("existing.crexe");
    fs::write(
        &recipe,
        "Generate a small console application with a self-test.",
    )
    .unwrap();
    let stdout = success(run(command(&home, temp.path()).arg(&recipe)));
    assert!(stdout.contains("RECIPE_LEGACY_NO_DOCUMENT"));
    assert_eq!(provider.counts(), (0, 1));
}

#[test]
#[ignore = "requires .NET 8 SDK; controlled provider, no model or GUI"]
fn reader_that_displays_a_placeholder_is_rejected_before_publication() {
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    provider.state.fixed_viewer.store(true, Ordering::SeqCst);
    let home = temp.path().join("engine");
    configure(&provider, &home, 3);
    let file = temp.path().join("selected.md");
    fs::write(&file, "REAL_SELECTED_DOCUMENT_CONTENT").unwrap();
    let output = run(command(&home, temp.path())
        .args(["--max-repairs", "0"])
        .arg(&file));
    assert!(
        !output.status.success(),
        "A fixed-output viewer must fail the engine's runtime document contract"
    );
    assert_eq!(provider.counts(), (1, 1));
    assert!(!String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| serde_json::from_str::<Value>(line)
            .ok()
            .is_some_and(|value| value["content"] == "FIXED_PLACEHOLDER")));
    assert_eq!(
        fs::read_to_string(file).unwrap(),
        "REAL_SELECTED_DOCUMENT_CONTENT"
    );
}

#[cfg(unix)]
#[test]
#[ignore = "requires .NET 8 SDK; native argv bytes are checked using the controlled fixture"]
fn non_utf8_unix_filename_is_passed_as_native_argument() {
    use std::os::unix::ffi::OsStringExt;
    let temp = tempfile::tempdir().unwrap();
    let provider = Provider::start();
    let home = temp.path().join("engine");
    configure(&provider, &home, 3);
    let name = std::ffi::OsString::from_vec(b"reader-\xff-name.md".to_vec());
    let file = temp.path().join(name);
    fs::write(&file, "NATIVE_ARGUMENT_DOCUMENT").unwrap();
    assert_eq!(
        preview(run(command(&home, temp.path()).arg(&file)))["content"],
        "NATIVE_ARGUMENT_DOCUMENT"
    );
    assert_eq!(provider.counts(), (1, 1));
    assert_eq!(
        fs::read_to_string(file).unwrap(),
        "NATIVE_ARGUMENT_DOCUMENT"
    );
}
