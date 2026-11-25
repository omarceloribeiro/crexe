//! Host-specific SDK/restore checks, with an additional opt-in real graphics check.
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

fn run(command: &mut Command) -> String {
    let mut out = tempfile::tempfile().unwrap();
    let mut err = tempfile::tempfile().unwrap();
    command
        .stdout(out.try_clone().unwrap())
        .stderr(err.try_clone().unwrap());
    let mut child = command.spawn().unwrap();
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(180) {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("SDK fixture timeout");
        }
        std::thread::sleep(Duration::from_millis(30));
    };
    use std::io::{Seek, SeekFrom};
    out.seek(SeekFrom::Start(0)).unwrap();
    err.seek(SeekFrom::Start(0)).unwrap();
    let mut stdout = String::new();
    let mut stderr = String::new();
    out.take(1024 * 1024).read_to_string(&mut stdout).unwrap();
    err.take(1024 * 1024).read_to_string(&mut stderr).unwrap();
    assert!(status.success(), "stdout={stdout}\nstderr={stderr}");
    stdout
}

fn source(graphics: bool) -> (&'static str, Vec<Value>) {
    if cfg!(windows) {
        (
            "dotnet-opengl",
            vec![
                json!({"path":"Program.cs", "content":if graphics {include_str!("fixtures/opengl_windows.cs")} else {"static class Program { public static int Main(string[] args) { return Logic.Add(20,22)==42 && Logic.Add(-3,2)==-1 ? 0:1; } }"}}),
                json!({"path":"Logic.cs", "content":"static class Logic { public static int Add(int a,int b) => a+b; }"}),
            ],
        )
    } else if cfg!(target_os = "macos") {
        (
            "objc-cocoa-opengl",
            vec![
                json!({"path":"main.mm", "content":if graphics {include_str!("fixtures/opengl_macos.mm")} else {"int add(int,int); int main(){return add(20,22)==42 && add(-3,2)==-1 ? 0:1;}"}}),
                json!({"path":"logic.mm", "content":"int add(int a,int b){return a+b;}"}),
            ],
        )
    } else {
        (
            "c-gtk-opengl",
            vec![
                json!({"path":"main.c", "content":if graphics {include_str!("fixtures/opengl_linux.c")} else {"int add(int,int); int main(){return add(20,22)==42 && add(-3,2)==-1 ? 0:1;}"}}),
                json!({"path":"logic.c", "content":"int add(int a,int b){return a+b;}"}),
            ],
        )
    }
}

#[test]
#[ignore = "requires host SDKs and OpenGL libraries; Windows restores pinned NuGet dependencies"]
fn host_profile_switch_preserves_logic_exports_and_rebuilds_without_provider() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir(&home).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let captured = Arc::new(Mutex::new(Vec::<Value>::new()));
    let requests = captured.clone();
    let worker = std::thread::spawn(move || {
        for index in 0..4 {
            let started = Instant::now();
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && started.elapsed() < Duration::from_secs(180) =>
                    {
                        std::thread::sleep(Duration::from_millis(10))
                    }
                    Err(e) => panic!("fixture accept: {e}"),
                }
            };
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
                if let Some(p) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
                    break p + 4;
                }
            };
            let headers = String::from_utf8_lossy(&bytes[..start]).to_lowercase();
            let length: usize = headers
                .lines()
                .find_map(|l| {
                    l.strip_prefix("content-length:")
                        .map(|n| n.trim().parse().unwrap())
                })
                .unwrap();
            assert!(length < 1024 * 1024);
            while bytes.len() - start < length {
                let n = stream.read(&mut buffer).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
            }
            let payload: Value = serde_json::from_slice(&bytes[start..start + length]).unwrap();
            requests.lock().unwrap().push(payload);
            let (profile, mut files) = source(index > 0);
            if index == 1 {
                files[0]["content"] = json!("deliberate compiler error requiring a repair");
            }
            let generated = if index == 1 {
                json!({"profile":profile,"change_kind":"recreate","reason":"The request now requires a real OpenGL scene.","files":files})
            } else {
                json!({"files":files})
            };
            let body=json!({"choices":[{"finish_reason":"stop","message":{"content":generated.to_string()}}]}).to_string();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        }
    });
    fs::write(home.join("config.toml"),format!("version=1\ndefault_provider='test'\n[providers.test]\nkind='openai'\nmodel='fixture'\nbase_url='http://{address}'\n")).unwrap();
    let engine = env!("CARGO_BIN_EXE_crexe");
    let file = temp.path().join("calculator.crexe");
    let archive = temp.path().join("source.zip");
    for (index, prompt) in [
        "Calculator with addition.",
        "Preserve addition and now use a real OpenGL scene.",
        "Keep the OpenGL scene and refine the color.",
    ]
    .iter()
    .enumerate()
    {
        fs::write(&file, prompt).unwrap();
        let mut command = Command::new(engine);
        command
            .arg(&file)
            .args(["--max-repairs", "1"])
            .env("CREXE_HOME", &home);
        let check_graphics = index == 2 && std::env::var_os("CREXE_TEST_GRAPHICS").is_some();
        if !check_graphics {
            command.arg("--no-run");
        }
        if index == 2 {
            command.arg("--export").arg(&archive);
        }
        let output = run(&mut command);
        if check_graphics {
            assert!(output.contains("Verificando suporte OpenGL"));
        }
        if index == 1 {
            let pointer = fs::read_dir(home.join("cache/.documents"))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            let pointer: Value = serde_json::from_slice(&fs::read(pointer).unwrap()).unwrap();
            let metadata: Value = serde_json::from_slice(
                &fs::read(
                    home.join("cache")
                        .join(pointer["revision"].as_str().unwrap())
                        .join("CREXE-GENERATION.json"),
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(metadata["profile"], source(true).0);
            assert_eq!(metadata["change_kind"], "recreate");
            assert!(metadata["reason"].as_str().unwrap().contains("OpenGL"));
        }
    }
    worker.join().unwrap();
    let requests = captured.lock().unwrap();
    assert_eq!(requests.len(), 4);
    let second = requests[1]["messages"][1]["content"].as_str().unwrap();
    assert!(second.contains("PREVIOUS APPROVED SOURCE FILES"));
    assert!(second.contains("Calculator with addition."));
    assert!(requests[2]["messages"][0]["content"]
        .as_str()
        .unwrap()
        .contains(&format!("Prefer profile {}", source(true).0)));
    let repair = requests[2]["messages"][1]["content"].as_str().unwrap();
    assert!(repair.contains("deliberate compiler error"));
    assert!(repair.contains("Compiler/test diagnostics"));
    assert!(!repair.contains("PREVIOUS APPROVED SOURCE FILES"));
    assert!(requests[3]["messages"][0]["content"]
        .as_str()
        .unwrap()
        .contains(&format!("Prefer profile {}", source(true).0)));
    assert!(run(Command::new(engine)
        .arg(&file)
        .arg("--no-run")
        .env("CREXE_HOME", &home))
    .contains("Cache hit"));
    let extracted = temp.path().join("extracted");
    zip::ZipArchive::new(fs::File::open(&archive).unwrap())
        .unwrap()
        .extract(&extracted)
        .unwrap();
    let manifest: Value =
        serde_json::from_slice(&fs::read(extracted.join("CREXE-PROJECT.json")).unwrap()).unwrap();
    assert_eq!(manifest["profile"], source(true).0);
    assert!(!extracted.join("CREXE-GENERATION.json").exists());
    if cfg!(windows) {
        assert!(extracted.join("packages.lock.json").is_file());
    }
    let rebuilt = temp.path().join("rebuilt");
    run(Command::new(engine)
        .arg("build")
        .arg(&extracted)
        .arg("--output")
        .arg(&rebuilt)
        .env("CREXE_HOME", &home));
    let published = temp.path().join("published");
    run(Command::new(engine)
        .arg("publish")
        .arg(&extracted)
        .arg("--output")
        .arg(&published)
        .env("CREXE_HOME", &home));
    if std::env::var_os("CREXE_TEST_GRAPHICS").is_some() {
        let executable = if cfg!(windows) {
            rebuilt.join("build/CrexeApp.exe")
        } else {
            rebuilt.join("CrexeApp")
        };
        assert!(run(Command::new(executable)
            .arg("--crexe-graphics-check")
            .current_dir(Path::new(&rebuilt)))
        .contains("CREXE_GRAPHICS_OK"));
    }
}
