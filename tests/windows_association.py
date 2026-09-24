"""Explicit Windows smoke test: installed .crexe association, fake provider, real Rust build.

Run only after installing and associating the engine. No external API is called.
All test project/cache/marker files are placed in a newly created temporary root.
The report records that ShellExecute was tested, not a manual Explorer double click.
"""
import argparse
import hashlib
import http.server
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--rustc", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    if os.name != "nt":
        raise SystemExit("Windows-only smoke test")
    qa = Path(tempfile.mkdtemp(prefix="crexe-association-"))
    marker = qa / "launches.txt"
    recipe = qa / "calculadora de teste.crexe"
    other = qa / "pasta externa"
    other.mkdir()
    os.environ["CREXE_HOME"] = str(qa / "perfil com espaços")
    os.environ["CREXE_TEST_KEY"] = "synthetic-local-key"
    os.environ["CREXE_QA_MARKER"] = str(marker)
    count = 0

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_POST(self):
            nonlocal count
            payload = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            count += 1
            color = "GREEN" if "GREEN" in json.dumps(payload) else "WHITE"
            source = ('use std::io::Write; fn main() { let p = std::env::var("CREXE_QA_MARKER").unwrap(); '
                      'let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p).unwrap(); '
                      f'writeln!(f, "{color}").unwrap(); }}')
            generated = {"files": [{"path": "src/main.rs", "content": source}]}
            response = json.dumps({"choices": [{"message": {"content": json.dumps(generated)}, "finish_reason": "stop"}]}).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(response)))
            self.end_headers()
            self.wfile.write(response)

        def log_message(self, *_):
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    config_root = Path(os.environ["CREXE_HOME"])
    config_root.mkdir(parents=True, exist_ok=True)
    (config_root / "config.toml").write_text(f'''version = 1
default_provider = "test"
[providers.test]
kind = "openai"
base_url = "http://127.0.0.1:{server.server_port}"
model = "fixture"
api_key_env = "CREXE_TEST_KEY"
''', encoding="utf-8")
    spec = {
        "version": "1.0",
        "selectors": {"target": {"fallback": "windows"}},
        "prompt": {"system": "Local fixture", "user_by_target": {"windows": "WHITE"}},
        "generator": {"provider": "openai-compatible", "baseUrl": f"http://127.0.0.1:{server.server_port}", "apiKeyEnv": "CREXE_TEST_KEY", "model": "fixture"},
        "targets": {"windows": {"build": {"steps": [{"cmd": [str(args.rustc), "src/main.rs", "-o", "fixture-app.exe"]}]}, "run": {"cmd": ["fixture-app.exe"]}}},
    }

    def lines():
        return marker.read_text().splitlines() if marker.exists() else []

    def open_and_wait(expected):
        os.startfile(str(recipe), "open", cwd=str(other), show_cmd=0)
        until = time.monotonic() + 30
        while time.monotonic() < until:
            if lines() == expected:
                return
            time.sleep(0.1)
        raise AssertionError(f"ShellExecute did not produce {expected}; observed {lines()}; files preserved at {qa}")

    try:
        recipe.write_text(json.dumps(spec), encoding="utf-8")
        # First generation via CLI, later launches through the Windows association.
        completed = subprocess.run([str(args.engine), "exec", str(recipe)], cwd=other,
                                   capture_output=True, text=True, timeout=60)
        (qa / "first-run.log").write_text(completed.stdout + completed.stderr, encoding="utf-8")
        assert completed.returncode == 0, completed.stderr
        assert lines() == ["WHITE"] and count == 1
        open_and_wait(["WHITE", "WHITE"])
        assert count == 1
        spec["prompt"]["user_by_target"]["windows"] = "GREEN"
        recipe.write_text(json.dumps(spec), encoding="utf-8")
        open_and_wait(["WHITE", "WHITE", "GREEN"])
        assert count == 2
        open_and_wait(["WHITE", "WHITE", "GREEN", "GREEN"])
        assert count == 2
        report = {"result": "PASS", "platform": "Windows x64", "provider": "local HTTP fixture",
                  "real_compilation": True, "association_launch": "Windows ShellExecute",
                  "manual_explorer_double_click": False, "gui_calculator": False,
                  "launches": lines(), "generation_requests": count,
                  "installed_engine_sha256": hashlib.sha256(args.engine.read_bytes()).hexdigest(),
                  "workspace": str(qa), "paid_api_calls": 0}
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(json.dumps(report, indent=2))
    finally:
        server.shutdown()
        server.server_close()
        worker.join()


if __name__ == "__main__":
    main()
