//! Presentation is optional; platform-specific code stays outside orchestration.
#[cfg(windows)]
mod windows;
use std::sync::atomic::{AtomicBool, Ordering};
static DESKTOP: AtomicBool = AtomicBool::new(false);

pub(crate) fn desktop() -> bool {
    DESKTOP.load(Ordering::SeqCst)
}

pub(crate) struct Progress {
    #[cfg(windows)]
    _inner: Option<windows::Window>,
}

impl Progress {
    pub fn start(enabled: bool) -> anyhow::Result<Self> {
        DESKTOP.store(enabled && cfg!(windows), Ordering::SeqCst);
        #[cfg(windows)]
        {
            Ok(Self {
                _inner: if enabled {
                    Some(windows::Window::start()?)
                } else {
                    None
                },
            })
        }
        #[cfg(not(windows))]
        {
            let _ = enabled;
            Ok(Self {})
        }
    }
}

pub(crate) fn phase(message: &str) {
    println!("{message}");
    #[cfg(windows)]
    windows::phase(message);
}

pub(crate) fn finish() {
    #[cfg(windows)]
    windows::finish();
}

pub(crate) fn activate(changed: bool) {
    #[cfg(windows)]
    windows::activate(changed);
    #[cfg(not(windows))]
    let _ = changed;
}

pub(crate) fn show_error(message: &str) {
    #[cfg(windows)]
    windows::show_error(message);
    #[cfg(not(windows))]
    eprintln!("{message}");
}

#[cfg(windows)]
pub(crate) use windows::Activation;
#[cfg(windows)]
pub(crate) fn activation(executable: &std::path::Path, pid: u32, launch: bool) -> Activation {
    Activation::new(executable, pid, launch && desktop())
}
