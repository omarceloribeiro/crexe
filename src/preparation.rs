//! A short-lived per-document preparation lease, independent of the cache lock.
use anyhow::{Context, Result};
use fs2::FileExt;
use interprocess::local_socket::{
    prelude::*, GenericNamespaced, ListenerNonblockingMode, ListenerOptions, Stream,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

pub(crate) struct Session {
    lock: Option<fs::File>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Session {
    pub fn acquire(file: &Path, signature: &[u8]) -> Result<Option<Self>> {
        let root = super::installation::root()?.join("sessions");
        super::paths::reject_links(&root)?;
        fs::create_dir_all(&root)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        }
        let file = file
            .canonicalize()
            .context("Cannot resolve the .crexe document")?;
        let path = file.to_string_lossy();
        let path = if cfg!(windows) {
            path.to_lowercase()
        } else {
            path.into_owned()
        };
        let id = format!(
            "{:x}",
            Sha256::digest(format!("{}:{path}", root.canonicalize()?.display()))
        );
        let endpoint = format!("crexe-{}", &id[..40]);
        let lock_path = root.join(format!("{id}.lock"));
        super::paths::reject_links(&lock_path)?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        let signature: [u8; 32] = Sha256::digest(signature).into();
        let start = Instant::now();
        loop {
            match lock.try_lock_exclusive() {
                Ok(()) => break,
                Err(error)
                    if error.raw_os_error() == fs2::lock_contended_error().raw_os_error() =>
                {
                    if notify(&endpoint, &signature).is_ok() {
                        return Ok(None);
                    }
                    if start.elapsed() >= Duration::from_secs(3) {
                        // Never start an extra request while the OS lease is held.
                        println!(
                            "Este arquivo já está em preparação; aguarde a execução existente."
                        );
                        return Ok(None);
                    }
                    thread::sleep(Duration::from_millis(30));
                }
                Err(error) => return Err(error).context("Cannot acquire preparation lease"),
            }
        }
        let options = ListenerOptions::new()
            .name(endpoint.to_ns_name::<GenericNamespaced>()?)
            .try_overwrite(true)
            .nonblocking(ListenerNonblockingMode::Both);
        #[cfg(windows)]
        let options = {
            use interprocess::os::windows::{
                local_socket::ListenerOptionsExt, security_descriptor::SecurityDescriptor,
            };
            let descriptor = widestring::U16CString::from_str("D:P(A;;GA;;;OW)")?;
            options.security_descriptor(SecurityDescriptor::deserialize(&descriptor)?)
        };
        let listener = options.create_sync()?;
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok(mut stream) => {
                        #[cfg(unix)]
                        if stream.peer_creds().ok().and_then(|p| p.euid())
                            != Some(unsafe { libc::geteuid() })
                        {
                            continue;
                        }
                        let mut requested = [0; 32];
                        let deadline = Instant::now() + Duration::from_millis(300);
                        if transfer_read(&mut stream, &mut requested, deadline).is_ok() {
                            let changed = requested != signature;
                            let mut reply = [0; 5];
                            reply[..4].copy_from_slice(&std::process::id().to_le_bytes());
                            reply[4] = u8::from(changed);
                            if transfer_write(&mut stream, &reply, deadline).is_ok() {
                                let mut ack = [0];
                                let _ = transfer_read(&mut stream, &mut ack, deadline);
                                super::presentation::activate(changed);
                            }
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Some(Self {
            lock: Some(lock),
            stop,
            worker: Some(worker),
        }))
    }
    pub fn release(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.lock.take();
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.release();
    }
}

fn notify(endpoint: &str, signature: &[u8; 32]) -> Result<()> {
    use interprocess::{local_socket::ConnectOptions, ConnectWaitMode};
    let mut stream = ConnectOptions::new()
        .name(endpoint.to_ns_name::<GenericNamespaced>()?)
        .wait_mode(ConnectWaitMode::Timeout(Duration::from_millis(100)))
        .nonblocking_stream(true)
        .connect_sync()?;
    let deadline = Instant::now() + Duration::from_millis(300);
    transfer_write(&mut stream, signature, deadline)?;
    let mut reply = [0; 5];
    transfer_read(&mut stream, &mut reply, deadline)?;
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(u32::from_le_bytes(
            reply[..4].try_into().unwrap(),
        ));
    }
    transfer_write(&mut stream, &[1], deadline)?;
    println!(
        "{}",
        if reply[4] == 0 {
            "Este arquivo já está em preparação; a execução existente foi notificada."
        } else {
            "Arquivo ou opções alterados. Conclua ou cancele a preparação existente antes de reabrir."
        }
    );
    Ok(())
}

fn transfer_read(stream: &mut Stream, mut bytes: &mut [u8], deadline: Instant) -> io::Result<()> {
    while !bytes.is_empty() {
        match stream.read(bytes) {
            // Some Windows nonblocking pipe reads report zero while no bytes are
            // ready. A bounded retry also handles a disconnected peer safely.
            Ok(0) if cfg!(windows) && Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(5));
            }
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(n) => bytes = &mut bytes[n..],
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(io::ErrorKind::TimedOut.into());
                }
                thread::sleep(Duration::from_millis(5));
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
fn transfer_write(stream: &mut Stream, mut bytes: &[u8], deadline: Instant) -> io::Result<()> {
    while !bytes.is_empty() {
        match stream.write(bytes) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(n) => bytes = &bytes[n..],
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(io::ErrorKind::TimedOut.into());
                }
                thread::sleep(Duration::from_millis(5));
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
