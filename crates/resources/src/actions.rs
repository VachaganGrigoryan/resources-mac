//! Ending processes and apps, and opening Disk Sweep. No privilege escalation: failures are
//! reported, not retried.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use objc2_app_kit::NSRunningApplication;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sig {
    /// Polite request (SIGTERM); the process may clean up or ignore it.
    Term,
    /// Immediate (SIGKILL); cannot be caught.
    Kill,
    /// Pause (SIGSTOP); upstream's "Halt".
    Stop,
    /// Resume a paused process (SIGCONT).
    Cont,
}

/// Sends `sig` to `pid`, returning the OS error text (e.g. "Operation not permitted") on failure.
pub fn signal(pid: u32, sig: Sig) -> Result<(), String> {
    // pid 0 and negative pids address process groups (or everything), never a single process.
    let pid = libc::pid_t::try_from(pid)
        .ok()
        .filter(|&p| p > 0)
        .ok_or("Invalid process id")?;
    let sig = match sig {
        Sig::Term => libc::SIGTERM,
        Sig::Kill => libc::SIGKILL,
        Sig::Stop => libc::SIGSTOP,
        Sig::Cont => libc::SIGCONT,
    };
    // SAFETY: kill(2) has no memory-safety preconditions.
    if unsafe { libc::kill(pid, sig) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().to_string())
    }
}

/// Asks a GUI app to quit the normal way (like ⌘Q). Returns false if the app is gone or refused.
pub fn quit_app(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
        .is_some_and(|app| app.terminate())
}

/// Where Disk Sweep, the companion clean-up app, can be started from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Launch {
    Binary(PathBuf),
    App(PathBuf),
}

const SWEEP_BIN: &str = "disk-sweep";
const SWEEP_APP: &str = "Disk Sweep.app";

/// First match: `disk-sweep` beside `exe` (cargo puts both in `target/<profile>/`), then
/// `Disk Sweep.app` in one of `app_dirs`, then `disk-sweep` on `path_var`.
pub fn locate(exe: &Path, app_dirs: &[PathBuf], path_var: Option<&OsStr>) -> Option<Launch> {
    let sibling = exe.parent().map(|dir| dir.join(SWEEP_BIN));
    if let Some(bin) = sibling.filter(|p| p.is_file()) {
        return Some(Launch::Binary(bin));
    }
    if let Some(app) = app_dirs
        .iter()
        .map(|d| d.join(SWEEP_APP))
        .find(|p| p.is_dir())
    {
        return Some(Launch::App(app));
    }
    std::env::split_paths(path_var?)
        .map(|d| d.join(SWEEP_BIN))
        .find(|p| p.is_file())
        .map(Launch::Binary)
}

/// [`locate`] for this install, looked up once.
pub fn disk_sweep() -> Option<&'static Launch> {
    static FOUND: OnceLock<Option<Launch>> = OnceLock::new();
    FOUND
        .get_or_init(|| {
            let exe = std::env::current_exe().ok()?;
            let mut apps = vec![PathBuf::from("/Applications")];
            apps.extend(std::env::home_dir().map(|h| h.join("Applications")));
            locate(&exe, &apps, std::env::var_os("PATH").as_deref())
        })
        .as_ref()
}

/// Starts Disk Sweep as its own app; it keeps running when Resources quits.
pub fn open_disk_sweep() -> Result<(), String> {
    let launch = disk_sweep().ok_or("Disk Sweep isn't installed")?;
    let mut child = match launch {
        Launch::Binary(bin) => Command::new(bin).spawn(),
        Launch::App(app) => Command::new("open").arg("-a").arg(app).spawn(),
    }
    .map_err(|e| e.to_string())?;
    // Reap it when it exits so it doesn't linger as a zombie while Resources runs.
    std::thread::spawn(move || child.wait());
    Ok(())
}
