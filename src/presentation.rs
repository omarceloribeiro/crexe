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
    inner: Option<windows::Window>,
}

impl Progress {
    pub fn start(enabled: bool) -> anyhow::Result<Self> {
        DESKTOP.store(enabled && cfg!(windows), Ordering::SeqCst);
        #[cfg(windows)]
        {
            Ok(Self {
                inner: if enabled {
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
    pub fn error(&mut self, message: &str) {
        #[cfg(windows)]
        if let Some(window) = &mut self.inner {
            window.error(message);
        }
        #[cfg(not(windows))]
        eprintln!("{message}");
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
