//! Reading and signalling host processes from inside a Flatpak sandbox.
//!
//! A Flatpak app gets its own PID namespace and there is no permission to
//! share the host's: `--share=` accepts only `network` and `ipc`. So inside
//! the sandbox `/proc` holds a handful of PIDs, none of them the user's, and
//! `kill` cannot reach anything worth killing.
//!
//! The way out is `org.freedesktop.Flatpak`, the session helper that backs
//! `flatpak-spawn --host`. Everything here funnels through one host command
//! per poll: a shell loop over `/proc/<pid>/stat` for the numbers, plus a
//! single `ps` for command lines, since `/proc/<pid>/cmdline` is NUL
//! separated and awkward to read from a POSIX shell.

use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::process::Command;

/// Set inside every Flatpak sandbox.
pub fn is_sandboxed() -> bool {
    Path::new("/.flatpak-info").exists()
}

/// One process as the host reported it.
#[derive(Clone, Debug, PartialEq)]
pub struct HostProcess {
    pub pid: u32,
    pub parent: Option<u32>,
    /// Single-letter state from `/proc/<pid>/stat`: R, S, D, Z, T, I, ...
    pub state: char,
    /// utime + stime, in clock ticks.
    pub cpu_ticks: u64,
    pub memory_bytes: u64,
    pub name: String,
    pub cmd: String,
}

/// A poll of the host's process table.
#[derive(Clone, Debug, Default)]
pub struct HostSnapshot {
    /// Busy + idle ticks summed across all CPUs, for working out what share
    /// of the machine a process used between two polls.
    pub total_ticks: u64,
    pub processes: Vec<HostProcess>,
}

/// Emits `T <total ticks>`, then one `P` line per process, then `C` lines
/// pairing a PID with its command line.
///
/// `${l##*) }` drops everything up to the last `)`, which is how you skip a
/// `comm` field that itself contains spaces or brackets, as in `(Web Content)`.
const DUMP_SCRIPT: &str = r#"
read -r _ a b c d e f g h _ < /proc/stat
echo "T $((a+b+c+d+e+f+g+h))"
for p in /proc/[0-9]*; do
    read -r l < "$p/stat" 2>/dev/null || continue
    t=${l##*) }
    set -- $t
    [ $# -ge 22 ] || continue
    n=""
    read -r n < "$p/comm" 2>/dev/null
    echo "P ${p#/proc/} $2 $1 $((${12}+${13})) ${22} $n"
done
ps -eo pid=,args= 2>/dev/null | while read -r pid rest; do
    echo "C $pid $rest"
done
"#;

/// Polls the host's process table with a single `flatpak-spawn --host`.
pub fn snapshot() -> io::Result<HostSnapshot> {
    let output = host_command(&["sh", "-c", DUMP_SCRIPT])?;
    Ok(parse_dump(&output, page_size()))
}

/// Sends `signal` to every PID, returning how many were delivered.
///
/// One host command for the batch: spawning `flatpak-spawn` per process would
/// make "Kill All" across a large tree painfully slow.
pub fn signal(pids: &[u32], signal: &str) -> io::Result<usize> {
    if pids.is_empty() {
        return Ok(0);
    }

    // `kill` reports failures per PID but exits non-zero if any failed, so
    // count the survivors instead of trusting the exit status.
    let list: Vec<String> = pids.iter().map(u32::to_string).collect();
    let script = format!(
        "n=0; for p in {}; do kill -{} \"$p\" 2>/dev/null && n=$((n+1)); done; echo \"$n\"",
        list.join(" "),
        signal
    );

    let output = host_command(&["sh", "-c", &script])?;
    Ok(output.trim().parse().unwrap_or(0))
}

fn host_command(args: &[&str]) -> io::Result<String> {
    let output = Command::new("flatpak-spawn")
        .arg("--host")
        .args(args)
        .output()?;

    if !output.status.success() && output.stdout.is_empty() {
        return Err(io::Error::other(format!(
            "flatpak-spawn --host failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn page_size() -> u64 {
    // SAFETY: `sysconf` reads a static system value and has no preconditions.
    let size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if size > 0 { size as u64 } else { 4096 }
}

/// Parses [`DUMP_SCRIPT`] output. `rss` is in pages, hence `page_size`.
fn parse_dump(output: &str, page_size: u64) -> HostSnapshot {
    let mut snapshot = HostSnapshot::default();
    let mut by_pid: HashMap<u32, usize> = HashMap::new();

    for line in output.lines() {
        let Some((tag, rest)) = line.split_once(' ') else {
            continue;
        };

        match tag {
            "T" => snapshot.total_ticks = rest.trim().parse().unwrap_or(0),
            "P" => {
                // pid ppid state cpu_ticks rss_pages [comm...]
                let mut f = rest.splitn(6, ' ');
                let (Some(pid), Some(ppid), Some(state), Some(ticks), Some(rss)) =
                    (f.next(), f.next(), f.next(), f.next(), f.next())
                else {
                    continue;
                };
                let Ok(pid) = pid.parse::<u32>() else {
                    continue;
                };
                let name = f.next().unwrap_or("").trim().to_string();

                by_pid.insert(pid, snapshot.processes.len());
                snapshot.processes.push(HostProcess {
                    pid,
                    // PPID 0 means no parent we can act on.
                    parent: ppid.parse::<u32>().ok().filter(|&p| p != 0),
                    state: state.chars().next().unwrap_or('?'),
                    cpu_ticks: ticks.parse().unwrap_or(0),
                    memory_bytes: rss.parse::<u64>().unwrap_or(0).saturating_mul(page_size),
                    name,
                    cmd: String::new(),
                });
            }
            "C" => {
                let Some((pid, cmd)) = rest.split_once(' ') else {
                    continue;
                };
                if let Some(&i) = pid.trim().parse::<u32>().ok().and_then(|p| by_pid.get(&p)) {
                    snapshot.processes[i].cmd = cmd.trim().to_string();
                }
            }
            _ => {}
        }
    }

    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "T 1000\n\
        P 1 0 S 50 100 systemd\n\
        P 42 1 R 25 200 my-app\n\
        P 99 42 Z 0 0 zombie\n\
        C 1 /usr/lib/systemd/systemd --switched-root\n\
        C 42 /usr/bin/my-app --flag\n";

    #[test]
    fn parses_totals_processes_and_command_lines() {
        let s = parse_dump(SAMPLE, 4096);

        assert_eq!(s.total_ticks, 1000);
        assert_eq!(s.processes.len(), 3);

        let app = &s.processes[1];
        assert_eq!(app.pid, 42);
        assert_eq!(app.parent, Some(1));
        assert_eq!(app.state, 'R');
        assert_eq!(app.cpu_ticks, 25);
        assert_eq!(app.memory_bytes, 200 * 4096);
        assert_eq!(app.name, "my-app");
        assert_eq!(app.cmd, "/usr/bin/my-app --flag");
    }

    #[test]
    fn pid_1_has_no_parent() {
        let s = parse_dump(SAMPLE, 4096);
        assert_eq!(s.processes[0].parent, None, "PPID 0 is not a parent");
    }

    #[test]
    fn a_process_without_a_ps_line_keeps_an_empty_command() {
        let s = parse_dump(SAMPLE, 4096);
        // The zombie exited before `ps` ran, so it has no command line. The
        // row must still appear, since zombies are what the applet is for.
        assert_eq!(s.processes[2].pid, 99);
        assert_eq!(s.processes[2].cmd, "");
        assert_eq!(s.processes[2].state, 'Z');
    }

    /// Command lines contain spaces, so only the PID may be split off.
    #[test]
    fn command_lines_keep_their_arguments() {
        let s = parse_dump("P 7 1 S 0 0 x\nC 7 /usr/bin/x --a --b 'c d'\n", 4096);
        assert_eq!(s.processes[0].cmd, "/usr/bin/x --a --b 'c d'");
    }

    /// A `comm` of "(Web Content)" is why the script strips to the last ')'.
    #[test]
    fn names_may_contain_spaces() {
        let s = parse_dump("P 8 1 S 0 0 Web Content\n", 4096);
        assert_eq!(s.processes[0].name, "Web Content");
    }

    #[test]
    fn malformed_lines_are_skipped_not_fatal() {
        let s = parse_dump(
            "garbage\nP\nP notanumber 1 S 0 0 x\nT abc\nC 5\nP 9 1 S 0 0 ok\n",
            4096,
        );
        assert_eq!(s.total_ticks, 0);
        assert_eq!(s.processes.len(), 1);
        assert_eq!(s.processes[0].pid, 9);
    }

    #[test]
    fn empty_output_is_an_empty_snapshot() {
        let s = parse_dump("", 4096);
        assert_eq!(s.total_ticks, 0);
        assert!(s.processes.is_empty());
    }

    #[test]
    fn page_size_is_sane() {
        assert!(page_size().is_power_of_two());
        assert!(page_size() >= 4096);
    }

    /// Not sandboxed when the tests run natively, which is what keeps the
    /// collector on its `sysinfo` path outside Flatpak.
    #[test]
    fn sandbox_detection_matches_the_environment() {
        assert_eq!(is_sandboxed(), Path::new("/.flatpak-info").exists());
    }
}
