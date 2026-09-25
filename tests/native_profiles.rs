//! SDK tests use fixed local provider responses, never a paid API or real model.
use serde_json::json;
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    net::TcpListener,
    path::Path,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

trait CheckedOutput {
    fn checked_output(&mut self) -> Output;
}

impl CheckedOutput for Command {
    fn checked_output(&mut self) -> Output {
        println!(
            "Native fixture step: {:?} {:?}",
            self.get_program(),
            self.get_args().collect::<Vec<_>>()
        );
        let mut stdout = tempfile::tempfile().unwrap();
        let mut stderr = tempfile::tempfile().unwrap();
        self.stdout(Stdio::from(stdout.try_clone().unwrap()))
            .stderr(Stdio::from(stderr.try_clone().unwrap()));
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            self.process_group(0);
        }
        let mut child = self.spawn().unwrap();
        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if start.elapsed() > Duration::from_secs(120) {
                #[cfg(windows)]
                {
                    let _ = Command::new("taskkill")
                        .args(["/PID", &child.id().to_string(), "/T", "/F"])
                        .output();
                }
                #[cfg(unix)]
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "Native fixture step exceeded 120 seconds: {:?}",
                    self.get_program()
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        stdout.seek(SeekFrom::Start(0)).unwrap();
        stderr.seek(SeekFrom::Start(0)).unwrap();
        let mut out = Vec::new();
        let mut err = Vec::new();
        stdout.take(1024 * 1024).read_to_end(&mut out).unwrap();
        stderr.take(1024 * 1024).read_to_end(&mut err).unwrap();
        Output {
            status,
            stdout: out,
            stderr: err,
        }
    }
}

fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn sources() -> (&'static str, Vec<serde_json::Value>) {
    if cfg!(windows) {
        (
            "cpp-win32",
            vec![
                json!({"path": "UI sources/main.cpp", "content": "#include <windows.h>\n#include <shellapi.h>\n#include <cwchar>\nint add(int,int);\nint WINAPI wWinMain(HINSTANCE,HINSTANCE,PWSTR,int){ int argc=0; LPWSTR* argv=CommandLineToArgvW(GetCommandLineW(),&argc); if(!argv) return 2; bool test=argc>1 && wcscmp(argv[1],L\"--crexe-self-test\")==0; LocalFree(argv); if(test) return add(20,22)==42 && add(-3,2)==-1 ? 0:1; MessageBoxW(nullptr,L\"Fixture\",L\"CREXE\",MB_OK); return 0; }\n"}),
                json!({"path": "logic/soma.cpp", "content": "int add(int a,int b){return a+b;}\n"}),
            ],
        )
    } else if cfg!(target_os = "macos") {
        (
            "objc-cocoa",
            vec![
                json!({"path": "UI sources/main.mm", "content": "#import <Cocoa/Cocoa.h>\n#include <cstring>\nint add(int,int);\nint main(int argc,char**argv){ if(argc>1 && std::strcmp(argv[1],\"--crexe-self-test\")==0) return add(20,22)==42 && add(-3,2)==-1 ? 0:1; @autoreleasepool { [NSApplication sharedApplication]; NSWindow *w=[[NSWindow alloc] initWithContentRect:NSMakeRect(0,0,320,200) styleMask:NSWindowStyleMaskTitled|NSWindowStyleMaskClosable backing:NSBackingStoreBuffered defer:NO]; [w setTitle:@\"CREXE Fixture\"]; [w makeKeyAndOrderFront:nil]; [NSApp run]; } return 0; }\n"}),
                json!({"path": "logic/soma.mm", "content": "int add(int a,int b){return a+b;}\n"}),
            ],
        )
    } else {
        (
            "c-gtk",
            vec![
                json!({"path": "UI sources/main.c", "content": "#include <gtk/gtk.h>\n#include <string.h>\nint add(int,int);\nint main(int argc,char**argv){if(argc>1 && strcmp(argv[1],\"--crexe-self-test\")==0) return add(20,22)==42 && add(-3,2)==-1 ? 0:1; gtk_init(&argc,&argv); GtkWidget*w=gtk_window_new(GTK_WINDOW_TOPLEVEL); gtk_window_set_title(GTK_WINDOW(w),\"CREXE Fixture\"); g_signal_connect(w,\"destroy\",G_CALLBACK(gtk_main_quit),NULL); gtk_widget_show_all(w); gtk_main(); return 0;}\n"}),
                json!({"path": "logic/soma.c", "content": "int add(int a,int b){return a+b;}\n"}),
            ],
        )
    }
}

#[test]
#[ignore = "requires the native desktop SDK; enabled explicitly by platform CI"]
fn native_profile_builds_multiple_sources_exports_rebuilds_and_publishes() {
    let engine = env!("CARGO_BIN_EXE_crexe");
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("engine");
    fs::create_dir(&home).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (profile, files) = sources();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(15)))
            .unwrap();
        let mut request = Vec::new();
        let mut chunk = [0; 4096];
        let start = loop {
            let count = stream.read(&mut chunk).unwrap();
            assert!(count > 0);
            request.extend_from_slice(&chunk[..count]);
            if let Some(pos) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                break pos + 4;
            }
            assert!(request.len() < 1_000_000);
        };
        let headers = String::from_utf8_lossy(&request[..start]).to_lowercase();
        assert!(headers.starts_with("post /api/chat "));
        assert!(!headers.contains("authorization:"));
        let length: usize = headers
            .lines()
            .find_map(|line| {
                line.strip_prefix("content-length:")
                    .map(|s| s.trim().parse().unwrap())
            })
            .unwrap();
        assert!(length < 1_000_000);
        while request.len() - start < length {
            let count = stream.read(&mut chunk).unwrap();
            assert!(count > 0);
            request.extend_from_slice(&chunk[..count]);
        }
        let payload: serde_json::Value =
            serde_json::from_slice(&request[start..start + length]).unwrap();
        assert!(payload["messages"]
            .to_string()
            .contains("--crexe-self-test"));
        let body = json!({"done":true, "done_reason":"stop", "message":{"content":json!({"files":files}).to_string()}}).to_string();
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
    });
    fs::write(home.join("config.toml"),format!("version=1\ndefault_provider='test'\n[providers.test]\nkind='ollama'\nmodel='fixture'\nbase_url='http://127.0.0.1:{port}'\n")).unwrap();
    let recipe = temp.path().join("app nativo.crexe");
    fs::write(&recipe, "Gerar um aplicativo nativo simples de soma.").unwrap();
    let zip_path = temp.path().join("fontes.zip");
    let generated = success(
        Command::new(engine)
            .arg(&recipe)
            .args([
                "--profile",
                profile,
                "--no-run",
                "--max-repairs",
                "0",
                "--export",
            ])
            .arg(&zip_path)
            .env("CREXE_HOME", &home)
            .checked_output(),
    );
    server.join().unwrap(); // The only provider request is complete; the server no longer exists.
    assert!(generated.contains("Validated revision:"));
    let cached = success(
        Command::new(engine)
            .arg(&recipe)
            .args(["--profile", profile, "--no-run"])
            .env("CREXE_HOME", &home)
            .env("PATH", "")
            .checked_output(),
    );
    assert!(
        cached.contains("Cache hit"),
        "Cache reuse must not need compilers or a provider"
    );
    let extracted = temp.path().join("fontes extraidos");
    zip::ZipArchive::new(fs::File::open(zip_path).unwrap())
        .unwrap()
        .extract(&extracted)
        .unwrap();
    let manifest = fs::read_to_string(extracted.join("CREXE-PROJECT.json")).unwrap();
    assert!(!manifest.contains("$CREXE_SOURCES"));
    let command = if cfg!(windows) { "cmd" } else { "sh" };
    let mut build = Command::new(command);
    if cfg!(windows) {
        build.args(["/c", "build.bat"]);
    } else {
        build.arg("build.sh");
    }
    success(build.current_dir(&extracted).checked_output());
    let mut script_test = Command::new(command);
    if cfg!(windows) {
        script_test.args(["/c", "test.bat"]);
    } else {
        script_test.arg("test.sh");
    }
    success(script_test.current_dir(&extracted).checked_output());
    let app = if cfg!(windows) {
        "CrexeApp.exe"
    } else {
        "CrexeApp"
    };
    success(
        Command::new(extracted.join(app))
            .arg("--crexe-self-test")
            .checked_output(),
    );
    let published = temp.path().join("published");
    success(
        Command::new(engine)
            .arg("publish")
            .arg(&extracted)
            .arg("--output")
            .arg(&published)
            .env("CREXE_HOME", &home)
            .checked_output(),
    );
    let artifact = if cfg!(windows) {
        "CrexeApp-publish.exe"
    } else {
        "CrexeApp-publish"
    };
    assert!(Path::new(&published).join(artifact).is_file());
    success(
        Command::new(published.join(artifact))
            .arg("--crexe-self-test")
            .checked_output(),
    );
    println!("{profile}: multi-file compile, self-test, cache without compiler, ZIP rebuild and local publish passed (one fixture request, no real model)");
}
