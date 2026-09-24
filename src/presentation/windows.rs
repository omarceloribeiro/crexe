use anyhow::{Context, Result};
use std::{
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicIsize, Ordering},
        mpsc,
    },
    thread,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::{GetStockObject, COLOR_WINDOW, DEFAULT_GUI_FONT},
    System::{Console::FreeConsole, LibraryLoader::GetModuleHandleW},
    UI::{
        Controls::{
            InitCommonControlsEx, ICC_PROGRESS_CLASS, INITCOMMONCONTROLSEX, PBM_SETMARQUEE,
            PBS_MARQUEE,
        },
        WindowsAndMessaging::*,
    },
};

static WINDOW: AtomicIsize = AtomicIsize::new(0);
static LABEL: AtomicIsize = AtomicIsize::new(0);
const FINISH: u32 = WM_APP + 1;
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub(super) struct Window {
    worker: Option<thread::JoinHandle<()>>,
}
impl Window {
    pub fn start() -> Result<Self> {
        let (send, receive) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || unsafe {
            let class = wide("CREXE.Progress.v1");
            let instance = GetModuleHandleW(null());
            let definition = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance,
                lpszClassName: class.as_ptr(),
                hCursor: LoadCursorW(null_mut(), IDC_ARROW),
                hbrBackground: (COLOR_WINDOW + 1) as _,
                ..std::mem::zeroed()
            };
            RegisterClassW(&definition);
            let width = 460;
            let height = 185;
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                wide("Creative Executable").as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                (GetSystemMetrics(SM_CXSCREEN) - width) / 2,
                (GetSystemMetrics(SM_CYSCREEN) - height) / 2,
                width,
                height,
                null_mut(),
                null_mut(),
                instance,
                null(),
            );
            if hwnd.is_null() {
                let _ = send.send(false);
                return;
            }
            let title = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("Creative Executable").as_ptr(),
                WS_CHILD | WS_VISIBLE,
                24,
                24,
                410,
                28,
                hwnd,
                null_mut(),
                instance,
                null(),
            );
            let label = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("Preparando o ambiente…").as_ptr(),
                WS_CHILD | WS_VISIBLE,
                24,
                62,
                410,
                25,
                hwnd,
                null_mut(),
                instance,
                null(),
            );
            for control in [title, label] {
                SendMessageW(
                    control,
                    WM_SETFONT,
                    GetStockObject(DEFAULT_GUI_FONT) as usize,
                    1,
                );
            }
            InitCommonControlsEx(&INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_PROGRESS_CLASS,
            });
            let bar = CreateWindowExW(
                0,
                wide("msctls_progress32").as_ptr(),
                null(),
                WS_CHILD | WS_VISIBLE | PBS_MARQUEE,
                24,
                103,
                398,
                12,
                hwnd,
                null_mut(),
                instance,
                null(),
            );
            SendMessageW(bar, PBM_SETMARQUEE, 1, 35);
            WINDOW.store(hwnd as isize, Ordering::SeqCst);
            LABEL.store(label as isize, Ordering::SeqCst);
            ShowWindow(hwnd, SW_SHOW);
            let _ = send.send(true);
            let mut message: MSG = std::mem::zeroed();
            while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            LABEL.store(0, Ordering::SeqCst);
            WINDOW.store(0, Ordering::SeqCst);
        });
        if !receive.recv().context("Progress window thread failed")? {
            anyhow::bail!("Cannot create progress window");
        }
        // Detaches only this process; never hides the user's terminal window.
        unsafe {
            FreeConsole();
        }
        Ok(Self {
            worker: Some(worker),
        })
    }
    pub fn error(&mut self, message: &str) {
        finish();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        unsafe {
            MessageBoxW(
                null_mut(),
                wide(message).as_ptr(),
                wide("Creative Executable — Falha").as_ptr(),
                MB_OK | MB_ICONERROR,
            );
        }
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        finish();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
pub(super) fn phase(text: &str) {
    let hwnd = LABEL.load(Ordering::SeqCst) as HWND;
    if !hwnd.is_null() {
        unsafe {
            SendMessageW(hwnd, WM_SETTEXT, 0, wide(text).as_ptr() as isize);
        }
    }
}
pub(super) fn finish() {
    let hwnd = WINDOW.load(Ordering::SeqCst) as HWND;
    if !hwnd.is_null() {
        unsafe {
            PostMessageW(hwnd, FINISH, 0, 0);
        }
    }
}
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CLOSE => {
            super::super::executor::cancel();
            DestroyWindow(hwnd);
            0
        }
        FINISH => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}
