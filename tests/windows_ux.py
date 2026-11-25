"""Desktop UX regression using only owned windows and a local HTTP fixture.

Requires Windows and .NET 8 SDK. Never accesses real API keys or user applications.
Foreground is recorded separately: Windows may legally refuse an activation.
"""
import argparse
import ctypes as C
from ctypes import wintypes as W
import http.server
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import time

from windows_progress import capture, window_for

USER = C.windll.user32
USER.GetForegroundWindow.restype = W.HWND
USER.GetWindow.argtypes = [W.HWND, W.UINT]
USER.GetWindow.restype = W.HWND
USER.GetWindowThreadProcessId.argtypes = [W.HWND, C.POINTER(W.DWORD)]
USER.IsWindowVisible.argtypes = [W.HWND]
USER.IsIconic.argtypes = [W.HWND]
USER.PostMessageW.argtypes = [W.HWND, W.UINT, W.WPARAM, W.LPARAM]
USER.SetForegroundWindow.argtypes = [W.HWND]
USER.GetWindowTextW.argtypes = [W.HWND, W.LPWSTR, C.c_int]


def main_window(pid):
    found = []
    @C.WINFUNCTYPE(W.BOOL, W.HWND, W.LPARAM)
    def visit(hwnd, _):
        owner = W.DWORD()
        USER.GetWindowThreadProcessId(hwnd, C.byref(owner))
        if owner.value == pid and USER.IsWindowVisible(hwnd) and not USER.GetWindow(hwnd, 4):
            title = C.create_unicode_buffer(256)
            USER.GetWindowTextW(hwnd, title, len(title))
            if title.value.startswith("CREXE"):
                found.append(hwnd)
        return True
    USER.EnumWindows(visit, 0)
    return found[0] if found else None


def until(check, timeout=20):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = check()
        if value:
            return value
        time.sleep(0.05)
    raise AssertionError("Timed out waiting for fixture state")


PROJECT = '''<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>WinExe</OutputType><TargetFramework>net8.0-windows</TargetFramework><UseWindowsForms>true</UseWindowsForms><ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable></PropertyGroup></Project>'''
SOURCE = '''using System;
using System.IO;
using System.Threading;
using System.Windows.Forms;
static class Program {
 [STAThread] static void Main() {
  Thread.Sleep(1500);
  ApplicationConfiguration.Initialize();
  using var form = new Form { Text = "CREXE owned launch fixture", Width = 440, Height = 220 };
  form.Controls.Add(new Label { Text = "CREXE — teste de abertura", AutoSize = true, Left = 30, Top = 50 });
  if (Environment.GetEnvironmentVariable("CREXE_TEST_MINIMIZE") == "1") form.WindowState = FormWindowState.Minimized;
  form.Shown += (_, _) => File.AppendAllText(Environment.GetEnvironmentVariable("CREXE_TEST_EVENTS")!, Environment.ProcessId + "\\n");
  Application.Run(form);
 }
}'''


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    root = Path(tempfile.mkdtemp(prefix="crexe-ux-"))
    processes, windows = [], []
    count = []
    class Handler(http.server.BaseHTTPRequestHandler):
        def do_POST(self):
            self.rfile.read(int(self.headers["Content-Length"]))
            count.append(1)
            generated = {"files": [{"path": "Fixture.csproj", "content": PROJECT}, {"path": "Program.cs", "content": SOURCE}]}
            body = json.dumps({"choices": [{"finish_reason": "stop", "message": {"content": json.dumps(generated)}}]}).encode()
            self.send_response(200)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        def log_message(self, *_):
            pass
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    home = root / "home"
    home.mkdir()
    env = dict(os.environ, CREXE_HOME=str(home))
    results = []
    try:
        # No-argument launch opens settings; closing it does not create configuration.
        config_ui = subprocess.Popen([str(args.engine)], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        processes.append(config_ui)
        hwnd = until(lambda: main_window(config_ui.pid))
        windows.append(hwnd)
        time.sleep(1)
        screenshot = root / "configure.png"
        capture(hwnd, screenshot)
        USER.PostMessageW(hwnd, 0x0010, 0, 0)
        assert config_ui.wait(timeout=15) == 0
        assert not (home / "config.toml").exists(), "closing settings must not save"
        (home / "config.toml").write_text(f'version=1\ndefault_provider="test"\n[providers.test]\nkind="openai"\nbase_url="http://127.0.0.1:{server.server_port}"\nmodel="fixture"\n', encoding="utf-8")
        recipe = root / "app.crexe"
        recipe.write_text(json.dumps({"version": "1.0", "selectors": {"target": {"fallback": "native"}}, "prompt": {"user_by_target": {"native": "fixture"}}, "targets": {"native": {"build": {"steps": [{"cmd": ["dotnet", "build", "Fixture.csproj", "-c", "Release", "-o", "out", "--nologo"]}]}, "run": {"cmd": ["out/Fixture.exe"]}}}}), encoding="utf-8")
        events = root / "launches.txt"
        command = [str(args.engine), str(recipe), "--ui", "--max-repairs", "0"]
        for index, minimized in enumerate([False, True, False]):
            current_env = dict(env, CREXE_TEST_EVENTS=str(events), CREXE_TEST_MINIMIZE=str(int(minimized)))
            with (root / f"launch-{index}.log").open("wb") as diagnostics:
                process = subprocess.Popen(command, env=current_env, stdout=subprocess.DEVNULL, stderr=diagnostics)
            processes.append(process)
            progress = until(lambda: window_for(process.pid))
            USER.SetForegroundWindow(progress)  # Only this test's own progress window.
            time.sleep(0.5)
            assert window_for(process.pid), "progress closed before slow application startup"
            def launched():
                if events.exists():
                    lines = events.read_text().splitlines()
                    if len(lines) > index:
                        return int(lines[index])
                assert process.poll() is None, "engine exited before application opened"
                return None
            pid = until(launched, timeout=90)
            app = until(lambda: main_window(pid))
            windows.append(app)
            until(lambda: not window_for(process.pid), timeout=20)
            assert not USER.IsIconic(app), "application remained minimized"
            # Foreground activation crosses GUI threads and may settle after the call.
            settled = time.monotonic() + 1
            while USER.GetForegroundWindow() != app and time.monotonic() < settled:
                time.sleep(0.02)
            results.append({"cache_hit": index > 0, "fixture_initially_minimized": minimized,
                            "application_restored": True, "foreground": USER.GetForegroundWindow() == app,
                            "activation_diagnostic": (root / f"launch-{index}.log").read_text(encoding="utf-8", errors="replace"),
                            "progress_closed_after_window_ready": True})
            # Leave each fixture open: next click must launch another instance.
            assert process.poll() is None
        assert len(count) == 1, f"expected one generation, got {len(count)}"
        assert len(events.read_text().splitlines()) == 3
        for hwnd in windows:
            USER.PostMessageW(hwnd, 0x0010, 0, 0)
        for process in processes:
            assert process.wait(timeout=15) == 0
        report = {"result": "PASS", "settings_open_and_cancel": True, "configure_screenshot": str(screenshot),
                  "provider": "local HTTP fixture", "paid_api_calls": 0, "generation_requests": len(count),
                  "later_click_opens_new_instance": True, "launches": results,
                  "scope": "owned windows in current desktop session; Explorer outside Codex still needs manual acceptance"}
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(json.dumps(report, indent=2))
    finally:
        for hwnd in windows:
            USER.PostMessageW(hwnd, 0x0010, 0, 0)
        for process in processes:
            if process.poll() is None:
                process.kill()
                process.wait()
        server.shutdown()


if __name__ == "__main__":
    main()
