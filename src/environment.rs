//! Read-only OS observations. These never change the compilation target.
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct Architecture {
    pub engine: &'static str,
    pub host: Option<String>,
    pub source: &'static str,
    pub engine_matches_host: Option<bool>,
}

pub(crate) fn architecture() -> Architecture {
    let (host, source) = host_architecture();
    Architecture {
        engine: std::env::consts::ARCH,
        engine_matches_host: host.as_deref().map(|arch| arch == std::env::consts::ARCH),
        host,
        source,
    }
}

#[cfg(windows)]
fn host_architecture() -> (Option<String>, &'static str) {
    use windows_sys::Win32::{
        Foundation::HANDLE,
        System::{
            LibraryLoader::{GetModuleHandleW, GetProcAddress},
            SystemInformation::{
                IMAGE_FILE_MACHINE_AMD64, IMAGE_FILE_MACHINE_ARM64, IMAGE_FILE_MACHINE_I386,
            },
            Threading::GetCurrentProcess,
        },
    };
    // Resolve dynamically: an older OS may lack this function. Report unknown
    // instead of making the loader reject the entire engine on startup.
    type Query = unsafe extern "system" fn(HANDLE, *mut u16, *mut u16) -> i32;
    let host = unsafe {
        let module = GetModuleHandleW(windows_sys::core::w!("kernel32.dll"));
        let query = if module.is_null() {
            None
        } else {
            GetProcAddress(module, windows_sys::core::s!("IsWow64Process2"))
        };
        query.and_then(|query| {
            let query: Query = std::mem::transmute(query);
            let (mut process, mut native) = (0, 0);
            if query(GetCurrentProcess(), &mut process, &mut native) == 0 {
                return None;
            }
            match native {
                IMAGE_FILE_MACHINE_AMD64 => Some("x86_64".into()),
                IMAGE_FILE_MACHINE_ARM64 => Some("aarch64".into()),
                IMAGE_FILE_MACHINE_I386 => Some("x86".into()),
                _ => None,
            }
        })
    };
    (host, "Windows IsWow64Process2")
}

#[cfg(unix)]
fn kernel_architecture() -> Option<String> {
    let mut info = std::mem::MaybeUninit::<libc::utsname>::uninit();
    // uname initializes the struct only when it succeeds.
    unsafe {
        if libc::uname(info.as_mut_ptr()) != 0 {
            return None;
        }
        let info = info.assume_init();
        let machine = std::ffi::CStr::from_ptr(info.machine.as_ptr())
            .to_str()
            .ok()?;
        Some(
            match machine {
                "arm64" => "aarch64",
                "i386" | "i486" | "i586" | "i686" => "x86",
                other => other,
            }
            .to_owned(),
        )
    }
}

#[cfg(target_os = "macos")]
fn host_architecture() -> (Option<String>, &'static str) {
    let mut translated: libc::c_int = 0;
    let mut size = std::mem::size_of_val(&translated);
    let result = unsafe {
        libc::sysctlbyname(
            c"sysctl.proc_translated".as_ptr(),
            (&mut translated as *mut libc::c_int).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    let host = if result == 0 && translated == 1 {
        Some("aarch64".into())
    } else if result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
        kernel_architecture()
    } else {
        None
    };
    (host, "macOS sysctl.proc_translated / uname")
}

#[cfg(all(unix, not(target_os = "macos")))]
fn host_architecture() -> (Option<String>, &'static str) {
    (kernel_architecture(), "uname (OS-visible architecture)")
}

#[cfg(not(any(unix, windows)))]
fn host_architecture() -> (Option<String>, &'static str) {
    (None, "not available on this platform")
}
