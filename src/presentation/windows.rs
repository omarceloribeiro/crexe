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
const ACTIVATE: u32 = WM_APP + 2;
const CHANGED: u32 = WM_APP + 3;
const ACTIVATE_APP: u32 = WM_APP + 4;
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
            let height = 210;
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
                wide("Preparando o ambienteâ€¦").as_ptr(),
                WS_CHILD | WS_VISIBLE,
                24,
                62,
                410,
                48,
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
                130,
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

pub(super) fn activate(changed: bool) {
    let hwnd = WINDOW.load(Ordering::SeqCst) as HWND;
    if !hwnd.is_null() {
        unsafe {
            PostMessageW(hwnd, if changed { CHANGED } else { ACTIVATE }, 0, 0);
        }
    }
}

pub(super) fn show_error(message: &str) {
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(message).as_ptr(),
            wide("Creative Executable â€” Falha").as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

pub(crate) struct Activation {
    started: std::time::Instant,
    enabled: bool,
}
impl Activation {
    pub fn new(executable: &std::path::Path, pid: u32, enabled: bool) -> Self {
        let enabled = enabled && gui_executable(executable);
        if enabled {
            phase("Abrindo seu programaâ€¦");
            unsafe {
                AllowSetForegroundWindow(pid);
            }
        }
        Self {
            started: std::time::Instant::now(),
            enabled,
        }
    }
    pub fn poll(&mut self, belongs: &dyn Fn(u32) -> bool) -> bool {
        if !self.enabled {
            return true;
        }
        struct Search<'a> {
            belongs: &'a dyn Fn(u32) -> bool,
            found: HWND,
        }
        unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> i32 {
            let search = &mut *(data as *mut Search<'_>);
            if IsWindowVisible(hwnd) == 0
                || !GetWindow(hwnd, GW_OWNER).is_null()
                || GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW != 0
            {
                return 1;
            }
            // GUI frameworks briefly expose tiny helper windows during startup.
            // They are not the app's usable main window (minimized apps are kept).
            let mut rect = std::mem::zeroed();
            if IsIconic(hwnd) == 0
                && GetWindowRect(hwnd, &mut rect) != 0
                && (rect.right - rect.left < 64 || rect.bottom - rect.top < 48)
            {
                return 1;
            }
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if (search.belongs)(pid) {
                search.found = hwnd;
                return 0;
            }
            1
        }
        let mut search = Search {
            belongs,
            found: null_mut(),
        };
        unsafe {
            EnumWindows(Some(visit), &mut search as *mut _ as LPARAM);
            if !search.found.is_null() {
                if IsIconic(search.found) != 0 {
                    ShowWindowAsync(search.found, SW_RESTORE);
                    // Allow the application's own thread to process the restore.
                    if self.started.elapsed().as_secs() < 15 {
                        return false;
                    }
                }
                let progress = WINDOW.load(Ordering::SeqCst) as HWND;
                // Request from the thread that owns our foreground progress UI.
                let activated = if progress.is_null() {
                    SetForegroundWindow(search.found) as isize
                } else {
                    SendMessageW(progress, ACTIVATE_APP, search.found as usize, 0)
                };
                if activated == 0 {
                    let flash = FLASHWINFO {
                        cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
                        hwnd: search.found,
                        dwFlags: FLASHW_TRAY,
                        uCount: 3,
                        dwTimeout: 0,
                    };
                    FlashWindowEx(&flash);
                    eprintln!("Application restored; Windows declined foreground activation.");
                }
                return true;
            }
        }
        if self.started.elapsed().as_secs() >= 15 {
            eprintln!("Application started; no visible GUI window detected within 15 seconds.");
            return true;
        }
        false
    }
}

fn gui_executable(path: &std::path::Path) -> bool {
    use std::io::{Read, Seek, SeekFrom};
    let read = || -> std::io::Result<bool> {
        let mut file = std::fs::File::open(path)?;
        let mut dos = [0; 64];
        file.read_exact(&mut dos)?;
        if &dos[..2] != b"MZ" {
            return Ok(false);
        }
        let offset = u32::from_le_bytes(dos[60..64].try_into().unwrap());
        file.seek(SeekFrom::Start(offset as u64))?;
        let mut pe = [0; 94];
        file.read_exact(&mut pe)?;
        Ok(&pe[..4] == b"PE\0\0" && u16::from_le_bytes([pe[92], pe[93]]) == 2)
    };
    read().unwrap_or(false)
}
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        ACTIVATE_APP => SetForegroundWindow(wparam as HWND) as isize,
        ACTIVATE | CHANGED => {
            if message == CHANGED {
                phase("Arquivo ou opÃ§Ãµes alterados. Conclua ou cancele esta preparaÃ§Ã£o antes de reabrir.");
            }
            ShowWindow(hwnd, SW_RESTORE);
            if SetForegroundWindow(hwnd) == 0 {
                FlashWindowEx(&FLASHWINFO {
                    cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
                    hwnd,
                    dwFlags: FLASHW_TRAY,
                    uCount: 3,
                    dwTimeout: 0,
                });
            }
            0
        }
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
