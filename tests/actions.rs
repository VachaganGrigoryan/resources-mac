use std::os::unix::process::ExitStatusExt;
use std::process::Command;

use resources_mac::actions::{Sig, signal};
use resources_mac::collect::apps::running_apps;

#[test]
fn signal_term_kills_child() {
    let mut child = Command::new("sleep").arg("30").spawn().unwrap();
    signal(child.id(), Sig::Term).unwrap();
    let status = child.wait().unwrap();
    assert_eq!(status.signal(), Some(libc::SIGTERM));
}

#[test]
fn signal_dead_pid_errors() {
    let mut child = Command::new("true").spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    let err = signal(pid, Sig::Kill).unwrap_err();
    assert!(err.contains("No such process"), "{err}");
}

#[test]
fn signal_root_process_is_not_permitted() {
    // launchd (pid 1) belongs to root; an unprivileged kill must fail, not panic.
    let err = signal(1, Sig::Term).unwrap_err();
    assert!(err.contains("not permitted"), "{err}");
}

#[test]
fn running_apps_does_not_panic() {
    let _ = running_apps();
}

#[test]
fn signal_rejects_pid_zero() {
    // kill(0, …) would signal our own process group; must be refused.
    assert!(signal(0, Sig::Term).is_err());
}

#[test]
fn stop_and_cont_signal_child() {
    let mut child = Command::new("sleep").arg("30").spawn().unwrap();
    let pid = child.id() as libc::pid_t;
    let mut status = 0;

    signal(child.id(), Sig::Stop).unwrap();
    // SAFETY: waitpid only writes the status of our own child.
    unsafe { libc::waitpid(pid, &mut status, libc::WUNTRACED) };
    assert!(libc::WIFSTOPPED(status), "child not stopped");

    signal(child.id(), Sig::Cont).unwrap();
    // SAFETY: as above.
    unsafe { libc::waitpid(pid, &mut status, libc::WCONTINUED) };
    assert!(libc::WIFCONTINUED(status), "child not continued");

    child.kill().unwrap();
    child.wait().unwrap();
}
