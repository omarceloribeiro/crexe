"""Opt-in graphical fixture for the kept-reader Windows acceptance flow.

Preparation runs only a local synthetic provider. It does not select a Windows
default: the operator must use the actual Settings UI. Close the reader and
choose another default before --cleanup if Windows currently selects CREXE.
No personal documents, existing extensions, model services or API keys are used.
"""
import argparse
import hashlib
import http.server
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import threading
import time
import uuid


MARKER = "crexe-kept-reader-acceptance-v1"


def require_windows():
    if os.name != "nt":
        raise SystemExit("This opt-in acceptance fixture runs only on Windows.")


def reject_links(path):
    for item in [path, *path.parents]:
        if item.exists():
            info = item.lstat()
            if stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & stat.FILE_ATTRIBUTE_REPARSE_POINT:
                raise SystemExit(f"Refusing a linked/reparse path: {item}")


def absolute_directory(raw):
    path = Path(raw)
    if not path.is_absolute():
        raise SystemExit("Use an absolute acceptance directory.")
    reject_links(path)
    return path.resolve()


def write_json(path, value):
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def registry_exists(hive, path):
    import winreg
    try:
        with winreg.OpenKey(hive, path):
            return True
    except FileNotFoundError:
        return False


def fixture_environment(home):
    env = os.environ.copy()
    env["CREXE_HOME"] = str(home)
    env["DOTNET_CLI_TELEMETRY_OPTOUT"] = "1"
    for name in ["OPENAI_API_KEY", "CREXE_DEEPSEEK_API_KEY", "DEEPSEEK_API_KEY", "ANTHROPIC_API_KEY", "AZURE_OPENAI_API_KEY"]:
        env.pop(name, None)
    return env


def cleanup(raw):
    import winreg
    directory = absolute_directory(raw)
    metadata = json.loads((directory / "acceptance.json").read_text(encoding="utf-8"))
    if metadata.get("fixture") != MARKER or metadata.get("directory") != str(directory):
        raise SystemExit("This is not a directory created by this fixture.")
    extension = metadata.get("extension", "")
    if not re.fullmatch(r"crexeqa[0-9a-f]{16}", extension):
        raise SystemExit("Invalid synthetic extension in acceptance metadata.")
    home = directory / "home"
    installed = home / "releases" / "v1" / "crexe.exe"
    reject_links(installed)
    if hashlib.sha256(installed.read_bytes()).hexdigest() != metadata["engine_sha256"]:
        raise SystemExit("Installed fixture engine changed; inspect the evidence before cleanup.")
    choice = rf"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.{extension}\UserChoice"
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, choice) as key:
            current, _ = winreg.QueryValueEx(key, "ProgId")
    except FileNotFoundError:
        current = None
    expected = "CREXE.Reader." + hashlib.sha256(extension.encode()).hexdigest()[:23] + ".v1"
    if current == expected:
        raise SystemExit(
            f"Windows still selects CREXE for .{extension}. Close the reader and choose another app "
            "in Windows Settings/Open with, then repeat --cleanup. This script never writes or deletes UserChoice."
        )
    result = subprocess.run(
        [str(installed), "reader-defaults", "forget", "--extension", extension],
        env=fixture_environment(home), capture_output=True, text=True, encoding="utf-8", errors="replace",
        timeout=30, creationflags=subprocess.CREATE_NO_WINDOW,
    )
    write_json(directory / "cleanup.json", {
        "exit_code": result.returncode, "stdout": result.stdout, "stderr": result.stderr,
        "user_choice_modified": False, "evidence_files_preserved": True,
    })
    if result.returncode:
        raise SystemExit(result.stdout + result.stderr)
    print(f"Removed only the fixture's candidate for .{extension}. Evidence preserved at {directory}.")


def winforms_source():
    source = (Path(__file__).parent / "fixtures" / "reader_console.cs").read_text(encoding="utf-8")
    output_encoding = "Console.OutputEncoding = new UTF8Encoding(false);"
    if source.count(output_encoding) != 1:
        raise RuntimeError("Controlled reader fixture output changed; review the GUI adapter.")
    # A WinExe apphost launched without a console cannot call SetConsoleOutputEncoding.
    # Keep the redirected self-test/reader-check output UTF-8 through its stream.
    source = source.replace(output_encoding, "Console.SetOut(new StreamWriter(Console.OpenStandardOutput(), new UTF8Encoding(false)) { AutoFlush = true });")
    main = "    private static int Main(string[] args)"
    if source.count(main) != 1:
        raise RuntimeError("Controlled reader fixture Main changed; review the GUI adapter.")
    source = source.replace(main, "    [STAThread]\n" + main)
    old = "            Console.WriteLine(JsonSerializer.Serialize(ReadDocument(args[0], true)));\n            return 0;"
    if source.count(old) != 1:
        raise RuntimeError("Controlled reader fixture startup changed; review the GUI adapter.")
    new = r'''            Preview preview = ReadDocument(args[0], true);
            System.Windows.Forms.Application.EnableVisualStyles();
            System.Windows.Forms.Application.SetCompatibleTextRenderingDefault(false);
            using var window = new System.Windows.Forms.Form {
                Text = "CREXE · " + Path.GetFileName(args[0]),
                Width = 820, Height = 540,
                StartPosition = System.Windows.Forms.FormStartPosition.CenterScreen
            };
            string text = preview.kind == "hex" ? "Prévia hexadecimal\r\n" + preview.content
                : preview.content.Length == 0 ? "Arquivo vazio" : preview.content;
            if (preview.truncated) text += "\r\n\r\n[Prévia limitada a 64 KiB]";
            window.Controls.Add(new System.Windows.Forms.TextBox {
                Multiline = true, ReadOnly = true, WordWrap = false,
                ScrollBars = System.Windows.Forms.ScrollBars.Both,
                Dock = System.Windows.Forms.DockStyle.Fill,
                Font = new System.Drawing.Font("Consolas", 14),
                Text = text
            });
            System.Windows.Forms.Application.Run(window);
            return 0;'''
    return source.replace(old, new)


def prepare(engine_raw, directory_raw):
    import winreg
    directory = absolute_directory(directory_raw)
    if directory.exists() and (not directory.is_dir() or any(directory.iterdir())):
        raise SystemExit("The acceptance directory must be new or empty.")
    engine = Path(engine_raw).resolve(strict=True)
    if not engine.is_file():
        raise SystemExit("--engine must name the built CREXE executable.")
    reject_links(engine)
    source = winforms_source()
    extension = "crexeqa" + uuid.uuid4().hex[:16]
    for hive, key in [
        (winreg.HKEY_CLASSES_ROOT, f".{extension}"),
        (winreg.HKEY_CURRENT_USER, rf"Software\Classes\.{extension}"),
        (winreg.HKEY_CURRENT_USER, rf"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.{extension}"),
    ]:
        if registry_exists(hive, key):
            raise SystemExit("Synthetic extension already exists; refusing to modify it.")
    directory.mkdir(parents=True, exist_ok=True)
    home = directory / "home"
    documents = directory / "documentos sintéticos"
    home.mkdir()
    documents.mkdir()
    first = documents / f"01 Primeiro documento.{extension}"
    second = documents / f"02 Segundo documento ç e espaços.{extension}"
    first.write_text("PRIMEIRO DOCUMENTO SINTÉTICO\n\nEste leitor foi preparado pelo provider local de teste.\nNenhuma LLM real foi usada.\n", encoding="utf-8")
    second.write_text("SEGUNDO DOCUMENTO — CONTEÚDO DIFERENTE\n\nO duplo clique deve abrir este arquivo com o programa mantido.\nA geração não deve acontecer novamente.\n", encoding="utf-8")
    requests = []
    counts = {"planning": 0, "generation": 0}
    lock = threading.Lock()

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_POST(self):
            length = int(self.headers.get("Content-Length", "0"))
            if self.path != "/api/chat" or not 0 < length < 2_000_000:
                self.send_error(400)
                return
            payload = json.loads(self.rfile.read(length))
            planning = payload.get("format", {}).get("required") == ["prompt"]
            with lock:
                requests.append(payload)
                counts["planning" if planning else "generation"] += 1
                write_json(directory / "requests.json", requests)
                write_json(directory / "counts.json", counts)
            response = {"prompt": "Create an efficient local read-only viewer honoring every engine reader contract requirement. Read only the runtime file argument. Use Windows Forms."} if planning else {"files": [{"path": "Program.cs", "content": source}]}
            body = json.dumps({"done": True, "done_reason": "stop", "message": {"content": json.dumps(response)}}).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *_):
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    (home / "config.toml").write_text(
        "version=1\ndefault_provider='acceptance-fixture'\n[execution]\nmax_provider_requests=1\n"
        "[providers.acceptance-fixture]\nkind='ollama'\nmodel='controlled-winforms-fixture'\n"
        f"base_url='http://127.0.0.1:{server.server_port}'\ntimeout_seconds=90\n", encoding="utf-8",
    )
    env = fixture_environment(home)
    installed = home / "releases" / "v1" / "crexe.exe"
    try:
        install = subprocess.run([str(engine), "install"], env=env, capture_output=True, timeout=30,
                                 creationflags=subprocess.CREATE_NO_WINDOW)
        (directory / "install.log").write_bytes(install.stdout + install.stderr)
        if install.returncode:
            raise SystemExit("Fixture installation failed; inspect install.log.")
        metadata = {
            "fixture": MARKER, "directory": str(directory), "extension": extension,
            "installed_engine": str(installed), "engine_sha256": hashlib.sha256(installed.read_bytes()).hexdigest(),
            "first": str(first), "second": str(second), "provider": "controlled local HTTP fixture, no real model",
            "default_selection": "manual Windows system UI required; not performed by this script",
            "original_sha256": {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in [first, second]},
        }
        write_json(directory / "acceptance.json", metadata)
        write_json(directory / "counts.json", counts)
        with (directory / "engine.log").open("wb") as log:
            process = subprocess.Popen([
                str(installed), "open-generating", "--keep-as-default", "--profile", "dotnet-winforms", "--ui", "--", str(first),
            ], env=env, cwd=documents, stdout=log, stderr=log, creationflags=subprocess.CREATE_NO_WINDOW)
            metadata["engine_pid"] = process.pid
            write_json(directory / "acceptance.json", metadata)
            print(json.dumps(metadata, ensure_ascii=False, indent=2), flush=True)
            print("Fixture provider remains available. Complete GUI acceptance, close reader windows, then stop this script with Ctrl+C.", flush=True)
            announced_exit = False
            while True:
                code = process.poll()
                if code is not None and not announced_exit:
                    metadata["engine_exit_code"] = code
                    write_json(directory / "acceptance.json", metadata)
                    print(f"Initial engine process ended with {code}; provider remains available for inspection.", flush=True)
                    announced_exit = True
                time.sleep(0.25)
    except KeyboardInterrupt:
        print("Provider stopped. Evidence preserved. Use --cleanup after choosing another Windows default if necessary.", flush=True)
    finally:
        server.shutdown()
        server.server_close()
        worker.join(timeout=5)


def main():
    require_windows()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path)
    parser.add_argument("--directory", type=Path)
    parser.add_argument("--cleanup", type=Path)
    args = parser.parse_args()
    if args.cleanup:
        if args.engine or args.directory:
            parser.error("Use --cleanup by itself.")
        cleanup(args.cleanup)
    else:
        if not args.engine or not args.directory:
            parser.error("Preparation requires --engine and --directory.")
        prepare(args.engine, args.directory)


if __name__ == "__main__":
    main()
