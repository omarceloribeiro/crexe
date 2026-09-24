"""Native Windows progress-window smoke test using a local provider and real rustc.

Only enumerates/operates windows belonging to the engine process started here.
Captures that window (not the desktop). No external provider or user application.
"""
import argparse
import ctypes as C
from ctypes import wintypes as W
import http.server
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import threading
import time
import zlib


def capture(hwnd, path):
    user, gdi = C.windll.user32, C.windll.gdi32
    user.SetProcessDPIAware()  # Capture physical pixels, not DPI-virtualized bounds.
    user.GetWindowDC.restype = W.HDC
    gdi.CreateCompatibleDC.restype = W.HDC
    gdi.CreateCompatibleBitmap.restype = W.HBITMAP
    gdi.SelectObject.restype = W.HANDLE
    for name, args in {
        "CreateCompatibleDC": [W.HDC], "CreateCompatibleBitmap": [W.HDC, C.c_int, C.c_int],
        "SelectObject": [W.HDC, W.HANDLE], "DeleteDC": [W.HDC], "DeleteObject": [W.HANDLE],
        "GetDIBits": [W.HDC, W.HBITMAP, W.UINT, W.UINT, C.c_void_p, C.c_void_p, W.UINT],
    }.items():
        getattr(gdi, name).argtypes = args
    user.GetWindowDC.argtypes = [W.HWND]
    user.PrintWindow.argtypes = [W.HWND, W.HDC, W.UINT]
    user.ReleaseDC.argtypes = [W.HWND, W.HDC]
    rect = W.RECT()
    user.GetWindowRect(hwnd, C.byref(rect))
    width, height = rect.right - rect.left, rect.bottom - rect.top
    dc = user.GetWindowDC(hwnd)
    memory = gdi.CreateCompatibleDC(dc)
    bitmap = gdi.CreateCompatibleBitmap(dc, width, height)
    previous = gdi.SelectObject(memory, bitmap)
    try:
        assert user.PrintWindow(hwnd, memory, 0)
        gdi.SelectObject(memory, previous)
        info = C.create_string_buffer(struct.pack("<IiiHHIIiiII", 40, width, -height, 1, 32, 0, width * height * 4, 0, 0, 0, 0))
        pixels = C.create_string_buffer(width * height * 4)
        assert gdi.GetDIBits(memory, bitmap, 0, height, pixels, info, 0)
        rows = bytearray()
        raw = pixels.raw
        for y in range(height):
            rows.append(0)
            for x in range(width):
                pos = (y * width + x) * 4
                rows.extend((raw[pos + 2], raw[pos + 1], raw[pos]))
        def chunk(kind, data):
            return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
        path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))
    finally:
        gdi.DeleteObject(bitmap)
        gdi.DeleteDC(memory)
        user.ReleaseDC(hwnd, dc)


def window_for(pid):
    found = []
    user = C.windll.user32
    @C.WINFUNCTYPE(W.BOOL, W.HWND, W.LPARAM)
    def visit(hwnd, _):
        owner = W.DWORD()
        user.GetWindowThreadProcessId(hwnd, C.byref(owner))
        if owner.value == pid:
            name = C.create_unicode_buffer(128)
            user.GetClassNameW(hwnd, name, len(name))
            if name.value == "CREXE.Progress.v1":
                found.append(hwnd)
        return True
    user.EnumWindows(visit, 0)
    return found[0] if found else None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--rustc", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    root = Path(tempfile.mkdtemp(prefix="crexe-progress-"))
    gate = threading.Event()
    received = threading.Event()
    class Handler(http.server.BaseHTTPRequestHandler):
        def do_POST(self):
            self.rfile.read(int(self.headers["Content-Length"]))
            received.set()
            gate.wait(20)
            generated = {"files": [{"path": "main.rs", "content": 'fn main() { println!("fixture"); }'}]}
            body = json.dumps({"choices": [{"finish_reason": "stop", "message": {"content": json.dumps(generated)}}]}).encode()
            try:
                self.send_response(200)
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
                pass
        def log_message(self, *_):
            pass
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    home = root / "home"
    home.mkdir()
    (home / "config.toml").write_text(f'version=1\ndefault_provider="test"\n[providers.test]\nkind="openai"\nbase_url="http://127.0.0.1:{server.server_port}"\nmodel="fixture"\n', encoding="utf-8")
    recipe = root / "janela.crexe"
    recipe.write_text(json.dumps({"version": "1.0", "selectors": {"target": {"fallback": "native"}}, "prompt": {"user_by_target": {"native": "fixture"}}, "targets": {"native": {"build": {"steps": [{"cmd": [str(args.rustc), "main.rs", "-o", "fixture.exe"]}]}, "run": {"cmd": ["fixture.exe"]}}}}), encoding="utf-8")
    env = dict(os.environ, CREXE_HOME=str(home))
    command = [str(args.engine), str(recipe), "--ui", "--no-run", "--max-repairs", "0"]
    process = None
    try:
        process = subprocess.Popen(command, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        assert received.wait(15), "provider not reached"
        hwnd = window_for(process.pid)
        assert hwnd, "progress window missing"
        labels = []
        @C.WINFUNCTYPE(W.BOOL, W.HWND, W.LPARAM)
        def child_text(child, _):
            value = C.create_unicode_buffer(1024)
            C.windll.user32.GetWindowTextW(child, value, len(value))
            labels.append(value.value)
            C.windll.user32.UpdateWindow(child)
            return True
        C.windll.user32.EnumChildWindows(hwnd, child_text, 0)
        assert any("Gerando seu programa" in text for text in labels), labels
        C.windll.user32.UpdateWindow(hwnd)
        time.sleep(0.25)
        screenshot = root / "progress.png"
        capture(hwnd, screenshot)
        gate.set()
        assert process.wait(timeout=30) == 0
        assert window_for(process.pid) is None
        pointers = {str(p): p.read_bytes() for p in home.glob("cache/*/current.json")}
        assert pointers
        gate.clear()
        received.clear()
        process = subprocess.Popen(command + ["--rebuild"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        assert received.wait(15)
        hwnd = window_for(process.pid)
        assert hwnd
        started = time.monotonic()
        C.windll.user32.PostMessageW(hwnd, 0x0010, 0, 0)  # close only this test window
        assert process.wait(timeout=10) != 0
        elapsed = time.monotonic() - started
        assert pointers == {str(p): p.read_bytes() for p in home.glob("cache/*/current.json")}
        report = {"result": "PASS", "real_window": True, "labels": labels, "real_rust_build": True, "provider": "local HTTP fixture", "window_closed_after_build": True, "cancel_seconds": round(elapsed, 3), "previous_cache_preserved": True, "screenshot": str(screenshot), "paid_api_calls": 0}
        args.report.write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(json.dumps(report, indent=2))
    finally:
        gate.set()
        if process and process.poll() is None:
            process.kill()
            process.wait()
        server.shutdown()


if __name__ == "__main__":
    main()
