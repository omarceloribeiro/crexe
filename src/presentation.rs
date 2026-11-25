//! Presentation is optional; platform-specific code stays outside orchestration.
#[cfg(windows)]
mod windows;
use std::sync::atomic::{AtomicBool, Ordering};
static DESKTOP: AtomicBool = AtomicBool::new(false);
static WINDOW_OBSERVED: AtomicBool = AtomicBool::new(false);

pub(crate) fn window_observed() -> bool {
    WINDOW_OBSERVED.load(Ordering::SeqCst)
}

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
        WINDOW_OBSERVED.store(false, Ordering::SeqCst);
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

pub(crate) fn details(message: &str) {
    println!("{message}");
    #[cfg(windows)]
    windows::details(message);
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

pub(crate) fn confirm_edit_empty() -> anyhow::Result<bool> {
    #[cfg(windows)]
    {
        windows::confirm_edit_empty()
    }
    #[cfg(not(windows))]
    {
        Ok(false)
    }
}

pub(crate) fn confirm_reader_default(extension: &str) -> bool {
    #[cfg(windows)]
    {
        rfd::MessageDialog::new()
            .set_title("Leitor mantido — CREXE")
            .set_description(format!("O leitor foi mantido para arquivos .{extension}.\n\nDeseja escolher este leitor como programa padrão no Windows? Nas Configurações, selecione CREXE — Leitor .{extension}.\n\nA escolha vale para todos os arquivos dessa extensão. Você pode cancelar e escolher depois."))
            .set_buttons(rfd::MessageButtons::YesNo)
            .show() == rfd::MessageDialogResult::Yes
    }
    #[cfg(not(windows))]
    {
        let _ = extension;
        false
    }
}

#[cfg(windows)]
pub(crate) use windows::Activation;
#[cfg(windows)]
pub(crate) fn activation(executable: &std::path::Path, pid: u32, launch: bool) -> Activation {
    Activation::new(executable, pid, launch && desktop())
}
