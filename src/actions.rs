//! Ending processes and apps. No privilege escalation: failures are reported, not retried.

use objc2_app_kit::NSRunningApplication;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sig {
    /// Polite request (SIGTERM); the process may clean up or ignore it.
    Term,
    /// Immediate (SIGKILL); cannot be caught.
    Kill,
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
