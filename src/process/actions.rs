//! Process signaling actions: Stop (SIGSTOP), Resume (SIGCONT), Terminate (SIGTERM), and Kill (SIGKILL).

use libc::{c_int, kill, pid_t, SIGCONT, SIGKILL, SIGSTOP, SIGTERM};

pub fn stop_process(pid: u32) -> Result<(), String> {
    send_signal(pid as pid_t, SIGSTOP)
}

pub fn resume_process(pid: u32) -> Result<(), String> {
    send_signal(pid as pid_t, SIGCONT)
}

#[allow(dead_code)]
pub fn terminate_process(pid: u32) -> Result<(), String> {
    send_signal(pid as pid_t, SIGTERM)
}

pub fn kill_process(pid: u32) -> Result<(), String> {
    send_signal(pid as pid_t, SIGKILL)
}

pub fn kill_all(pids: &[u32]) -> usize {
    let mut count = 0;
    for &pid in pids {
        if kill_process(pid).is_ok() {
            count += 1;
        }
    }
    count
}

fn send_signal(pid: pid_t, sig: c_int) -> Result<(), String> {
    unsafe {
        if kill(pid, sig) == 0 {
            Ok(())
        } else {
            let err = std::io::Error::last_os_error();
            Err(format!("Failed to send signal {} to PID {}: {}", sig, pid, err))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn test_process_lifecycle_signals() {
        // Spawn a sleep process
        let mut child = Command::new("sleep").arg("30").spawn().expect("failed to spawn sleep");
        let pid = child.id();

        // Test stop
        assert!(stop_process(pid).is_ok());

        // Test resume
        assert!(resume_process(pid).is_ok());

        // Test kill
        assert!(kill_process(pid).is_ok());

        let _ = child.wait();
    }
}
