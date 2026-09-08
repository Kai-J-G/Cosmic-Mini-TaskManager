//! Process signalling: stop (`SIGSTOP`), resume (`SIGCONT`), kill (`SIGKILL`).

use libc::{SIGCONT, SIGKILL, SIGSTOP, c_int, kill, pid_t};

pub fn stop_process(pid: u32) -> Result<(), String> {
    send_signal(pid, SIGSTOP)
}

pub fn resume_process(pid: u32) -> Result<(), String> {
    send_signal(pid, SIGCONT)
}

pub fn kill_process(pid: u32) -> Result<(), String> {
    send_signal(pid, SIGKILL)
}

/// Kills every listed PID, returning how many signals were delivered.
/// Failures are ignored: a process that already exited is not worth reporting.
pub fn kill_all(pids: &[u32]) -> usize {
    pids.iter()
        .filter(|&&pid| kill_process(pid).is_ok())
        .count()
}

/// The caller pairs this error with the action it attempted, so the message
/// here is just the reason (`Operation not permitted`, `No such process`, ...).
fn send_signal(pid: u32, sig: c_int) -> Result<(), String> {
    // `kill` reads non-positive PIDs as "the caller's process group" (0) or
    // "every process we may signal" (-1). Our PIDs come from sysinfo and are
    // always real, but a bad value must never be allowed to reach the syscall.
    let target = pid_t::try_from(pid).unwrap_or(-1);
    if target <= 0 {
        return Err(format!("Invalid PID {pid}"));
    }

    // SAFETY: `kill` has no memory-safety preconditions, and `target` is a
    // positive PID, so at worst the call fails with ESRCH or EPERM.
    if unsafe { kill(target, sig) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn stop_resume_kill_a_real_process() {
        let mut child = Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("failed to spawn sleep");
        let pid = child.id();

        assert!(stop_process(pid).is_ok());
        assert!(resume_process(pid).is_ok());
        assert!(kill_process(pid).is_ok());

        let _ = child.wait();
    }

    #[test]
    fn signalling_a_dead_process_reports_why() {
        let mut child = Command::new("true").spawn().expect("failed to spawn true");
        let pid = child.id();
        let _ = child.wait(); // reaped, so the PID is gone rather than a zombie

        let error = kill_process(pid).expect_err("a dead PID should fail");
        assert!(
            error.contains("No such process"),
            "unexpected error: {error}"
        );
    }

    /// PID 0 and anything past `pid_t` must be rejected before the syscall,
    /// since `kill` would otherwise broadcast to a process group.
    #[test]
    fn broadcast_pids_are_rejected() {
        for pid in [0, u32::MAX, i32::MAX as u32 + 1] {
            let error = send_signal(pid, SIGCONT).expect_err("should be rejected");
            assert_eq!(error, format!("Invalid PID {pid}"));
        }
    }

    #[test]
    fn kill_all_counts_only_delivered_signals() {
        let mut child = Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("failed to spawn sleep");
        let live = child.id();

        assert_eq!(kill_all(&[live, 0]), 1);

        let _ = child.wait();
    }
}
