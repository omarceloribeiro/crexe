use anyhow::{bail, Context, Result};
use std::{
    env, fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

static CANCELLED: AtomicBool = AtomicBool::new(false);

pub(crate) fn cancelled() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}

pub(crate) fn cancel() {
    CANCELLED.store(true, Ordering::SeqCst);
}

pub(crate) fn install_cancel_handler() -> Result<()> {
    ctrlc::set_handler(cancel)?;
    Ok(())
}

pub(crate) fn resolve_tool(name: &str) -> Result<PathBuf> {
    let path = Path::new(name);
    if path.is_absolute() {
        if !path.is_file() {
            bail!("Required executable not found: {name}");
        }
        return Ok(path.to_path_buf());
    }
    if name.contains(['/', '\\']) {
        bail!("Tool paths must be absolute or names resolved from PATH: {name}");
    }
    for directory in env::split_paths(&env::var_os("PATH").unwrap_or_default()) {
        if !directory.is_absolute() {
            continue;
        }
        for suffix in if cfg!(windows) {
            vec!["", ".exe", ".com", ".cmd", ".bat"]
        } else {
            vec![""]
        } {
            let candidate = directory.join(format!("{name}{suffix}"));
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    bail!("Required tool '{name}' was not found in PATH. Install/configure the toolchain for this profile.")
}

pub(crate) fn run(
    args: &[String],
    cwd: &Path,
    timeout: Option<Duration>,
    capture: bool,
    secret_names: &[String],
) -> Result<()> {
    execute(args, cwd, timeout, capture, true, secret_names).map(|_| ())
}

pub(crate) fn output(args: &[String], cwd: &Path, secret_names: &[String]) -> Result<String> {
    execute(
        args,
        cwd,
        Some(Duration::from_secs(15)),
        true,
        false,
        secret_names,
    )
}

fn execute(
    args: &[String],
    cwd: &Path,
    timeout: Option<Duration>,
    capture: bool,
    print_output: bool,
    secret_names: &[String],
) -> Result<String> {
    if args.is_empty() {
        bail!("Empty command");
    }
    if CANCELLED.load(Ordering::SeqCst) {
        bail!("Cancelled");
    }
    let joined = args.join(" ").to_ascii_lowercase();
    if joined.contains("windowssandbox")
        || joined.contains("containers-disposableclientvm")
        || args
            .iter()
            .any(|arg| arg.to_ascii_lowercase().ends_with(".wsb"))
    {
        bail!("Windows Sandbox is prohibited in this version");
    }
    let executable = resolve_tool(&args[0])?;
    let mut command = Command::new(executable);
    command.args(&args[1..]).current_dir(cwd);
    if super::presentation::desktop() {
        // FreeConsole invalidates inherited Windows standard handles. Supply real
        // null handles before spawning the app; build capture replaces these below.
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
    }
    #[cfg(windows)]
    if capture {
        use std::os::windows::process::CommandExt;
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }
    for (key, _) in env::vars_os() {
        let name = key.to_string_lossy().to_ascii_uppercase();
        if name == "OPENAI_API_KEY"
            || name == "ANTHROPIC_API_KEY"
            || name == "AZURE_OPENAI_API_KEY"
            || secret_names
                .iter()
                .any(|secret| secret.eq_ignore_ascii_case(&name))
        {
            command.env_remove(key);
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut stdout = tempfile::tempfile()?;
    let mut stderr = tempfile::tempfile()?;
    if capture {
        command
            .stdout(Stdio::from(stdout.try_clone()?))
            .stderr(Stdio::from(stderr.try_clone()?));
    }
    let mut child = command
        .spawn()
        .with_context(|| format!("Failed to start {}", args[0]))?;
    #[cfg(windows)]
    let job = match windows::Job::assign(&child) {
        Ok(job) => job,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let start = Instant::now();
    let failure;
    loop {
        if let Some(status) = child.try_wait()? {
            #[cfg(windows)]
            drop(job);
            #[cfg(unix)]
            kill_group(child.id());
            if capture
                && (stdout.metadata()?.len() > 2 * 1024 * 1024
                    || stderr.metadata()?.len() > 2 * 1024 * 1024)
            {
                bail!("Command output exceeded 2 MiB: {}", args[0]);
            }
            let captured = if capture {
                tail(&mut stdout)?
            } else {
                String::new()
            };
            let diagnostics = if capture {
                format!("{captured}{}", tail(&mut stderr)?)
            } else {
                String::new()
            };
            if !status.success() {
                bail!("Command failed ({status}): {}\n{diagnostics}", args[0]);
            }
            if print_output && !diagnostics.trim().is_empty() {
                print!("{diagnostics}");
            }
            return Ok(captured);
        }
        if CANCELLED.load(Ordering::SeqCst) {
            failure = "Cancelled";
            break;
        }
        if timeout.is_some_and(|limit| start.elapsed() >= limit) {
            failure = "Command timed out";
            break;
        }
        if capture
            && (stdout.metadata()?.len() > 2 * 1024 * 1024
                || stderr.metadata()?.len() > 2 * 1024 * 1024)
        {
            failure = "Command output exceeded 2 MiB";
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    #[cfg(windows)]
    drop(job);
    #[cfg(unix)]
    kill_group(child.id());
    let _ = child.kill();
    let _ = child.wait();
    bail!(
        "{failure}: {}\n{}{}",
        args[0],
        tail(&mut stdout)?,
        tail(&mut stderr)?
    );
}

fn tail(file: &mut fs::File) -> Result<String> {
    let size = file.metadata()?.len();
    file.seek(SeekFrom::Start(size.saturating_sub(65536)))?;
    let mut bytes = Vec::new();
    file.take(65536).read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(unix)]
fn kill_group(pid: u32) {
    // Only the process group created for this command.
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
    };

    pub(super) struct Job(HANDLE);
    impl Job {
        pub fn assign(child: &std::process::Child) -> Result<Self> {
            unsafe {
                let job = Self(CreateJobObjectW(std::ptr::null(), std::ptr::null()));
                if job.0.is_null() {
                    return Err(std::io::Error::last_os_error().into());
                }
                let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                if SetInformationJobObject(
                    job.0,
                    JobObjectExtendedLimitInformation,
                    &limits as *const _ as *const _,
                    std::mem::size_of_val(&limits) as u32,
                ) == 0
                    || AssignProcessToJobObject(job.0, child.as_raw_handle() as HANDLE) == 0
                {
                    return Err(std::io::Error::last_os_error())
                        .context("Cannot manage child process lifetime");
                }
                Ok(job)
            }
        }
    }
    impl Drop for Job {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}
