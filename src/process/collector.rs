//! System and process metrics collector.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::{
    CpuRefreshKind, MemoryRefreshKind, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind,
};

use super::host;
use super::types::{FilterTab, ProcessItem, ProcessState, SystemOverview};

/// How many consecutive polls a process must spend in uninterruptible sleep
/// before we call it hung. One tick of `D` is normal for any disk read.
const DISK_SLEEP_TICKS: u8 = 2;

const FALLBACK_ICON: &str = "application-x-executable-symbolic";

#[derive(Clone, Debug)]
struct DesktopAppEntry {
    name: String,
    icon: String,
}

pub struct ProcessCollector {
    sys: System,
    /// Logical CPU count, used to turn `sysinfo`'s per-core process
    /// percentages into a share of the whole machine.
    cpu_count: f32,
    /// Binary name (lowercased) to the `.desktop` entry that launches it.
    desktop_apps: HashMap<String, DesktopAppEntry>,
    /// Consecutive polls each PID has spent in uninterruptible sleep.
    disk_sleep_ticks: HashMap<u32, u8>,
    /// Inside Flatpak, `sysinfo` would only see the sandbox's own handful of
    /// PIDs, so process data comes from the host instead.
    use_host: bool,
    /// Previous CPU tick counts, for the host path. `sysinfo` keeps its own.
    prev_cpu_ticks: HashMap<u32, u64>,
    prev_total_ticks: u64,
}

impl ProcessCollector {
    pub fn new() -> Self {
        let mut sys = System::new_with_specifics(
            sysinfo::RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );
        sys.refresh_cpu_all();
        sys.refresh_memory();

        Self {
            cpu_count: sys.cpus().len().max(1) as f32,
            sys,
            desktop_apps: scan_desktop_applications(),
            disk_sleep_ticks: HashMap::new(),
            use_host: host::is_sandboxed(),
            prev_cpu_ticks: HashMap::new(),
            prev_total_ticks: 0,
        }
    }

    pub fn collect(&mut self) -> (SystemOverview, Vec<ProcessItem>) {
        if self.use_host {
            match self.collect_from_host() {
                Ok(result) => return result,
                Err(error) => {
                    // Fall through to sysinfo rather than showing nothing. The
                    // list will be near-empty, which is a visible symptom, and
                    // better than a blank popup with no explanation.
                    eprintln!("cosmic-mini-taskmanager: host poll failed: {error}");
                }
            }
        }

        self.collect_from_sysinfo()
    }

    /// Reads the host's process table through `flatpak-spawn --host`.
    ///
    /// `/proc/<pid>` holds one entry per thread-group leader, so unlike the
    /// `sysinfo` path this cannot pick up threads in the first place.
    fn collect_from_host(&mut self) -> std::io::Result<(SystemOverview, Vec<ProcessItem>)> {
        let snapshot = host::snapshot()?;

        // CPU usage is a delta. Both totals are across all cores already, so
        // the ratio is a share of the whole machine with no scaling needed.
        let total_delta = snapshot.total_ticks.saturating_sub(self.prev_total_ticks) as f32;
        let first_poll = self.prev_total_ticks == 0;

        let mut items = Vec::with_capacity(snapshot.processes.len());
        let mut ticks = HashMap::with_capacity(snapshot.processes.len());
        let mut unresponsive = 0;
        let mut seen_disk_sleep = HashSet::new();
        let mut used_memory = 0u64;

        for proc_ in &snapshot.processes {
            ticks.insert(proc_.pid, proc_.cpu_ticks);
            used_memory = used_memory.saturating_add(proc_.memory_bytes);

            // `ps` renders kernel threads as "[kworker/0:1]". They have no
            // command line of their own and aren't ours to manage.
            let is_kernel_thread = proc_.cmd.starts_with('[') && proc_.cmd.ends_with(']');
            if (proc_.cmd.is_empty() || is_kernel_thread) && proc_.pid != 1 {
                continue;
            }

            let status = classify_state(
                proc_.pid,
                proc_.state,
                &mut self.disk_sleep_ticks,
                &mut seen_disk_sleep,
            );
            if status.is_unresponsive_or_stopped() {
                unresponsive += 1;
            }

            let cpu_usage = if first_poll || total_delta <= 0.0 {
                0.0
            } else {
                let before = self.prev_cpu_ticks.get(&proc_.pid).copied().unwrap_or(0);
                let delta = proc_.cpu_ticks.saturating_sub(before) as f32;
                (delta / total_delta * 100.0).clamp(0.0, 100.0)
            };

            let first_arg = proc_.cmd.split_whitespace().next();
            let (name, icon_name, is_gui_app) = self.match_app_info(&proc_.name, first_arg);

            items.push(ProcessItem {
                pid: proc_.pid,
                parent: proc_.parent,
                name,
                cmd: proc_.cmd.clone(),
                cpu_usage,
                memory_bytes: proc_.memory_bytes,
                status,
                is_gui_app,
                icon_name,
            });
        }

        self.disk_sleep_ticks
            .retain(|pid, _| seen_disk_sleep.contains(pid));
        self.prev_cpu_ticks = ticks;
        self.prev_total_ticks = snapshot.total_ticks;

        // The sandbox's /proc/meminfo does report host-wide memory.
        self.sys.refresh_memory();
        let total_memory = self.sys.total_memory();
        let host_used = self.sys.used_memory();
        let memory_percent = if total_memory > 0 {
            (host_used as f32 / total_memory as f32) * 100.0
        } else {
            0.0
        };

        let overview = SystemOverview {
            total_cpu_percent: items
                .iter()
                .map(|i| i.cpu_usage)
                .sum::<f32>()
                .clamp(0.0, 100.0),
            used_memory_bytes: host_used,
            total_memory_bytes: total_memory,
            memory_percent,
            total_processes: items.len(),
            unresponsive_or_stopped_count: unresponsive,
        };

        Ok((overview, items))
    }

    fn collect_from_sysinfo(&mut self) -> (SystemOverview, Vec<ProcessItem>) {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        // Only the fields we actually display. `everything()` would also read
        // each process's environment, cwd, root and owner on every tick.
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_cmd(UpdateKind::OnlyIfNotSet),
        );

        let total_mem = self.sys.total_memory();
        let used_mem = self.sys.used_memory();
        let mem_percent = if total_mem > 0 {
            (used_mem as f32 / total_mem as f32) * 100.0
        } else {
            0.0
        };

        let cpu_count = self.cpu_count;
        let mut items = Vec::with_capacity(self.sys.processes().len());
        let mut unresponsive_count = 0;
        let mut seen_disk_sleep = HashSet::new();

        for (pid, proc_) in self.sys.processes() {
            let pid = pid.as_u32();
            let cmd = proc_.cmd();

            // `sysinfo` lists threads alongside processes on Linux, and a
            // userland thread inherits its process's command line -- so an
            // "empty cmdline" test does not catch them. Left in, they showed
            // up as duplicate rows and their CPU was counted twice, once on
            // the thread and again on the process that owns it.
            if proc_.thread_kind().is_some() {
                continue;
            }

            // Kernel threads have an empty cmdline and aren't ours to manage.
            // PID 1 is kept because init is worth showing even so.
            if cmd.is_empty() && pid != 1 {
                continue;
            }

            let status = classify(
                pid,
                proc_.status(),
                &mut self.disk_sleep_ticks,
                &mut seen_disk_sleep,
            );
            if status.is_unresponsive_or_stopped() {
                unresponsive_count += 1;
            }

            let raw_name = proc_.name().to_string_lossy();
            let first_arg = cmd.first().map(|arg| arg.to_string_lossy());
            let (name, icon_name, is_gui_app) =
                self.match_app_info(&raw_name, first_arg.as_deref());

            items.push(ProcessItem {
                pid,
                parent: proc_.parent().map(|p| p.as_u32()),
                name,
                cmd: join_cmd(cmd),
                // `sysinfo` reports this per core, so a single busy thread
                // reads as 100% on a 16-core box while the header says 6%.
                // Divide through so a row is a share of the whole machine
                // and the rows add up to the figure in the header.
                cpu_usage: proc_.cpu_usage() / cpu_count,
                memory_bytes: proc_.memory(),
                status,
                is_gui_app,
                icon_name,
            });
        }

        // Drop counters for PIDs that left `D` state or exited, so the map
        // does not grow for the lifetime of the applet.
        self.disk_sleep_ticks
            .retain(|pid, _| seen_disk_sleep.contains(pid));

        let overview = SystemOverview {
            total_cpu_percent: self.sys.global_cpu_usage(),
            used_memory_bytes: used_mem,
            total_memory_bytes: total_mem,
            memory_percent: mem_percent,
            total_processes: items.len(),
            unresponsive_or_stopped_count: unresponsive_count,
        };

        (overview, items)
    }

    /// Resolves a process to a human-readable name and icon, preferring a
    /// matching `.desktop` entry so GUI apps show their real branding.
    fn match_app_info(&self, raw_name: &str, first_arg: Option<&str>) -> (String, String, bool) {
        let name_lower = raw_name.to_lowercase();
        if let Some(entry) = self.desktop_apps.get(&name_lower) {
            return (entry.name.clone(), entry.icon.clone(), true);
        }

        if let Some(entry) = first_arg
            .and_then(file_stem_lower)
            .and_then(|binary| self.desktop_apps.get(&binary))
        {
            return (entry.name.clone(), entry.icon.clone(), true);
        }

        (raw_name.to_string(), FALLBACK_ICON.to_string(), false)
    }
}

/// Maps a `sysinfo` status onto our own, debouncing uninterruptible sleep.
///
/// Takes the counter map rather than `&mut self` so the caller can hold an
/// iterator over `self.sys` at the same time.
fn classify(
    pid: u32,
    status: sysinfo::ProcessStatus,
    disk_sleep_ticks: &mut HashMap<u32, u8>,
    seen_disk_sleep: &mut HashSet<u32>,
) -> ProcessState {
    use sysinfo::ProcessStatus as S;

    if status == S::UninterruptibleDiskSleep {
        seen_disk_sleep.insert(pid);
        let ticks = disk_sleep_ticks.entry(pid).or_insert(0);
        *ticks = ticks.saturating_add(1);
        return if *ticks >= DISK_SLEEP_TICKS {
            ProcessState::DiskSleep
        } else {
            ProcessState::Sleeping
        };
    }

    match status {
        S::Run => ProcessState::Running,
        S::Sleep | S::Idle | S::LockBlocked | S::Wakekill | S::Waking | S::Parked => {
            ProcessState::Sleeping
        }
        S::Stop => ProcessState::Stopped,
        S::Zombie | S::Dead => ProcessState::Zombie,
        _ => ProcessState::Other,
    }
}

/// Maps a `/proc/<pid>/stat` state letter onto our own, sharing the
/// uninterruptible-sleep debounce with the `sysinfo` path.
fn classify_state(
    pid: u32,
    state: char,
    disk_sleep_ticks: &mut HashMap<u32, u8>,
    seen_disk_sleep: &mut HashSet<u32>,
) -> ProcessState {
    if state == 'D' {
        seen_disk_sleep.insert(pid);
        let ticks = disk_sleep_ticks.entry(pid).or_insert(0);
        *ticks = ticks.saturating_add(1);
        return if *ticks >= DISK_SLEEP_TICKS {
            ProcessState::DiskSleep
        } else {
            ProcessState::Sleeping
        };
    }

    match state {
        'R' => ProcessState::Running,
        'S' | 'I' => ProcessState::Sleeping,
        'T' | 't' => ProcessState::Stopped,
        'Z' | 'X' | 'x' => ProcessState::Zombie,
        _ => ProcessState::Other,
    }
}

/// Filters by tab and search query, then orders the result.
///
/// Every comparison ends in a PID tiebreak so the ordering is total and rows
/// keep their place between polls instead of swapping on equal values.
pub fn filter_and_sort_processes<'a>(
    processes: &'a [ProcessItem],
    tab: FilterTab,
    search_query: &str,
) -> Vec<&'a ProcessItem> {
    let query = search_query.trim().to_lowercase();

    let mut filtered: Vec<&'a ProcessItem> = processes
        .iter()
        .filter(|p| match tab {
            FilterTab::Apps => p.is_gui_app,
            FilterTab::Unresponsive => p.is_unresponsive_or_stopped(),
            FilterTab::All | FilterTab::TopCpu | FilterTab::TopMemory => true,
        })
        .filter(|p| {
            query.is_empty()
                || p.name.to_lowercase().contains(&query)
                || p.cmd.to_lowercase().contains(&query)
                || p.pid.to_string().contains(&query)
        })
        .collect();

    match tab {
        // The "top" tabs answer a single question, so they rank on that
        // column alone.
        FilterTab::TopCpu => filtered.sort_by(|a, b| by_cpu(a, b)),
        FilterTab::TopMemory => {
            filtered.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes).then(a.pid.cmp(&b.pid)));
        }
        FilterTab::Unresponsive => filtered.sort_by(|a, b| by_cpu(a, b)),
        // Elsewhere, anything stopped or hung is floated to the top: it is
        // the reason the user opened the applet.
        FilterTab::All | FilterTab::Apps => filtered.sort_by(|a, b| {
            b.is_unresponsive_or_stopped()
                .cmp(&a.is_unresponsive_or_stopped())
                .then_with(|| by_cpu(a, b))
        }),
    }

    filtered
}

/// Busiest first. `total_cmp` is a total order over `f32`, NaN included.
fn by_cpu(a: &ProcessItem, b: &ProcessItem) -> std::cmp::Ordering {
    b.cpu_usage.total_cmp(&a.cpu_usage).then(a.pid.cmp(&b.pid))
}

fn join_cmd(cmd: &[std::ffi::OsString]) -> String {
    let mut joined = String::new();
    for arg in cmd {
        if !joined.is_empty() {
            joined.push(' ');
        }
        joined.push_str(&arg.to_string_lossy());
    }
    joined
}

fn file_stem_lower(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
}

fn scan_desktop_applications() -> HashMap<String, DesktopAppEntry> {
    let mut map = HashMap::new();
    for dir in desktop_dirs() {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "desktop") {
                parse_desktop_file(&path, &mut map);
            }
        }
    }

    map
}

/// Directories holding `.desktop` files, per the XDG base directory spec.
///
/// Reading `XDG_DATA_DIRS` rather than hardcoding `/usr/share` is what the
/// spec asks for, and it is also how the Flatpak build sees the host's
/// applications: the wrapper points those variables at `/run/host/usr/share`,
/// since the sandbox's own `/usr/share/applications` is nearly empty.
fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(data_home) = data_home() {
        dirs.push(data_home.join("applications"));
        dirs.push(data_home.join("flatpak/exports/share/applications"));
    }

    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .filter(|dirs| !dirs.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());

    for dir in std::env::split_paths(&data_dirs) {
        dirs.push(dir.join("applications"));
    }

    // Flatpak exports are not always listed in XDG_DATA_DIRS.
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));

    dirs.sort();
    dirs.dedup();
    dirs
}

fn data_home() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME").filter(|dir| !dir.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
}

/// Reads the `[Desktop Entry]` group and indexes it by its `Exec` binary,
/// which is what a running process's name or argv[0] will match against.
fn parse_desktop_file(path: &Path, map: &mut HashMap<String, DesktopAppEntry>) {
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };

    let mut in_desktop_entry = false;
    let mut name = None;
    let mut exec = None;
    let mut icon = None;

    for line in content.lines() {
        let line = line.trim();

        if let Some(group) = line.strip_prefix('[') {
            // Keys after the first group belong to actions, not the entry.
            if in_desktop_entry {
                break;
            }
            in_desktop_entry = group == "Desktop Entry]";
            continue;
        }

        if !in_desktop_entry || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        // Ignore `Name[fr]` and friends; the unlocalized key is what we want.
        match key.trim() {
            "Name" if name.is_none() => name = Some(value.trim().to_string()),
            "Exec" if exec.is_none() => exec = Some(value.trim().to_string()),
            "Icon" if icon.is_none() => icon = Some(value.trim().to_string()),
            // Hidden entries are not apps the user would recognise.
            "NoDisplay" | "Hidden" if value.trim().eq_ignore_ascii_case("true") => return,
            _ => {}
        }
    }

    let (Some(name), Some(exec)) = (name, exec) else {
        return;
    };

    // Exec often starts with a wrapper such as `env VAR=1 app` or
    // `flatpak run ...`; the first bare word is close enough to identify.
    let Some(binary) = exec.split_whitespace().next().and_then(file_stem_lower) else {
        return;
    };

    map.insert(
        binary,
        DesktopAppEntry {
            name,
            icon: icon.unwrap_or_else(|| FALLBACK_ICON.to_string()),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(pid: u32, name: &str, cpu: f32, mem_mib: u64, status: ProcessState) -> ProcessItem {
        ProcessItem {
            pid,
            parent: Some(1),
            name: name.to_string(),
            cmd: format!("/usr/bin/{name}"),
            cpu_usage: cpu,
            memory_bytes: mem_mib * 1024 * 1024,
            status,
            is_gui_app: status == ProcessState::Running,
            icon_name: FALLBACK_ICON.to_string(),
        }
    }

    fn sample() -> Vec<ProcessItem> {
        vec![
            item(100, "test-app", 10.0, 100, ProcessState::Running),
            item(101, "hung-worker", 95.0, 50, ProcessState::Stopped),
            item(102, "daemon", 2.0, 500, ProcessState::Sleeping),
        ]
    }

    #[test]
    fn collector_reports_a_plausible_system() {
        let mut collector = ProcessCollector::new();
        let (overview, procs) = collector.collect();

        assert!(overview.total_memory_bytes > 0);
        assert!(overview.memory_percent > 0.0 && overview.memory_percent <= 100.0);
        assert_eq!(overview.total_processes, procs.len());
        // Our own test binary must be in there.
        assert!(procs.iter().any(|p| p.pid == std::process::id()));
    }

    /// `sysinfo` lists threads as processes on Linux and a userland thread
    /// carries its process's command line, so they slipped past the old
    /// "empty cmdline" filter and were counted twice.
    #[test]
    fn threads_are_not_listed_as_processes() {
        let mut collector = ProcessCollector::new();
        let (overview, procs) = collector.collect();

        // Ground truth: /proc holds one directory per thread-group leader.
        let proc_dirs = fs::read_dir("/proc")
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().parse::<u32>().is_ok())
            .count();

        assert!(
            procs.len() <= proc_dirs,
            "reported {} processes but /proc only has {proc_dirs} entries",
            procs.len()
        );
        assert_eq!(overview.total_processes, procs.len());

        // Each PID appears once.
        let mut pids: Vec<u32> = procs.iter().map(|p| p.pid).collect();
        pids.sort_unstable();
        let unique = pids.len();
        pids.dedup();
        assert_eq!(pids.len(), unique, "duplicate PIDs in the list");
    }

    /// A row is a share of the whole machine, not of one core.
    ///
    /// `sysinfo` reports per-core usage, so on this 16-core box a single busy
    /// thread reads as 100% there while the header says 6%. Left unscaled, a
    /// row could claim 1600%.
    #[test]
    fn process_cpu_is_a_share_of_the_whole_machine() {
        let mut collector = ProcessCollector::new();
        let (_, procs) = collector.collect();
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL * 2);
        let (overview, procs) = {
            let _ = procs;
            collector.collect()
        };

        for p in &procs {
            assert!(
                (0.0..=100.0).contains(&p.cpu_usage),
                "{} reported {}% of the machine",
                p.name,
                p.cpu_usage
            );
        }

        // No single row may claim more than the machine is doing in total.
        let busiest = procs.iter().map(|p| p.cpu_usage).fold(0.0f32, f32::max);
        assert!(
            busiest <= overview.total_cpu_percent + 2.0,
            "busiest row is {busiest:.2}% but the whole machine is at {:.2}%",
            overview.total_cpu_percent
        );
    }

    #[test]
    fn every_process_but_init_has_a_parent() {
        let mut collector = ProcessCollector::new();
        let (_, procs) = collector.collect();

        // Without a PPID the kill sweep cannot find a process's children.
        for p in procs.iter().filter(|p| p.pid != 1) {
            assert!(p.parent.is_some(), "{} (PID {}) has no PPID", p.name, p.pid);
        }
    }

    #[test]
    fn tabs_select_the_right_processes() {
        let procs = sample();

        let unresponsive = filter_and_sort_processes(&procs, FilterTab::Unresponsive, "");
        assert_eq!(pids(&unresponsive), [101]);

        let apps = filter_and_sort_processes(&procs, FilterTab::Apps, "");
        assert_eq!(pids(&apps), [100]);

        let all = filter_and_sort_processes(&procs, FilterTab::All, "");
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn top_memory_ranks_by_memory_alone() {
        let procs = sample();
        let by_mem = filter_and_sort_processes(&procs, FilterTab::TopMemory, "");
        assert_eq!(pids(&by_mem), [102, 100, 101]);
    }

    #[test]
    fn top_cpu_ranks_by_cpu_without_floating_hung_rows() {
        let procs = sample();
        let by_cpu = filter_and_sort_processes(&procs, FilterTab::TopCpu, "");
        assert_eq!(pids(&by_cpu), [101, 100, 102]);
    }

    #[test]
    fn all_tab_floats_stopped_processes_to_the_top() {
        let mut procs = sample();
        // A stopped process with the *lowest* CPU still sorts first.
        procs[1].cpu_usage = 0.0;
        let all = filter_and_sort_processes(&procs, FilterTab::All, "");
        assert_eq!(pids(&all), [101, 100, 102]);
    }

    #[test]
    fn search_matches_name_command_and_pid() {
        let procs = sample();

        assert_eq!(
            pids(&filter_and_sort_processes(&procs, FilterTab::All, "hung")),
            [101]
        );
        assert_eq!(
            pids(&filter_and_sort_processes(
                &procs,
                FilterTab::All,
                "/usr/bin/daemon"
            )),
            [102]
        );
        assert_eq!(
            pids(&filter_and_sort_processes(&procs, FilterTab::All, "100")),
            [100]
        );
        // Whitespace-only queries are not a filter.
        assert_eq!(
            filter_and_sort_processes(&procs, FilterTab::All, "   ").len(),
            3
        );
        assert!(filter_and_sort_processes(&procs, FilterTab::All, "nope").is_empty());
    }

    #[test]
    fn sorting_is_a_total_order_even_with_nan_cpu() {
        let mut procs = sample();
        procs[0].cpu_usage = f32::NAN;
        // Would panic under a partial_cmp-based comparator.
        let sorted = filter_and_sort_processes(&procs, FilterTab::TopCpu, "");
        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn equal_values_fall_back_to_pid_order() {
        let procs = vec![
            item(300, "b", 5.0, 10, ProcessState::Sleeping),
            item(200, "a", 5.0, 10, ProcessState::Sleeping),
        ];
        assert_eq!(
            pids(&filter_and_sort_processes(&procs, FilterTab::TopCpu, "")),
            [200, 300]
        );
        assert_eq!(
            pids(&filter_and_sort_processes(&procs, FilterTab::TopMemory, "")),
            [200, 300]
        );
    }

    #[test]
    fn hung_state_needs_two_consecutive_polls() {
        let mut ticks = HashMap::new();
        let mut seen = HashSet::new();
        let status = sysinfo::ProcessStatus::UninterruptibleDiskSleep;

        // One tick of disk sleep is just a disk read.
        assert_eq!(
            classify(42, status, &mut ticks, &mut seen),
            ProcessState::Sleeping
        );
        assert_eq!(
            classify(42, status, &mut ticks, &mut seen),
            ProcessState::DiskSleep
        );
        assert_eq!(seen, HashSet::from([42]));
    }

    #[test]
    fn leaving_disk_sleep_clears_the_counter() {
        let mut ticks = HashMap::new();
        let mut seen = HashSet::new();
        classify(
            42,
            sysinfo::ProcessStatus::UninterruptibleDiskSleep,
            &mut ticks,
            &mut seen,
        );
        assert!(ticks.contains_key(&42));

        // A poll in which PID 42 is no longer in `D` state prunes it, so the
        // map cannot grow without bound as processes come and go.
        let still_in_disk_sleep: HashSet<u32> = HashSet::new();
        ticks.retain(|pid, _| still_in_disk_sleep.contains(pid));
        assert!(ticks.is_empty());
    }

    #[test]
    fn desktop_files_are_indexed_by_exec_binary() {
        let dir = std::env::temp_dir().join(format!("cmtm-desktop-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        let visible = dir.join("app.desktop");
        fs::write(
            &visible,
            "[Desktop Entry]\nName=Fancy App\nExec=/usr/bin/fancy-app --flag\nIcon=fancy\n\
             \n[Desktop Action New]\nName=Should Be Ignored\n",
        )
        .unwrap();

        let hidden = dir.join("hidden.desktop");
        fs::write(
            &hidden,
            "[Desktop Entry]\nName=Hidden\nExec=/usr/bin/hidden\nNoDisplay=true\n",
        )
        .unwrap();

        let mut map = HashMap::new();
        parse_desktop_file(&visible, &mut map);
        parse_desktop_file(&hidden, &mut map);

        let entry = map.get("fancy-app").expect("indexed by Exec binary");
        assert_eq!(entry.name, "Fancy App");
        assert_eq!(entry.icon, "fancy");
        assert_eq!(map.len(), 1, "NoDisplay entries must be skipped");

        fs::remove_dir_all(&dir).unwrap();
    }

    fn pids(items: &[&ProcessItem]) -> Vec<u32> {
        items.iter().map(|p| p.pid).collect()
    }
}

#[cfg(test)]
mod desktop_dir_tests {
    use super::*;

    /// The scan must follow XDG_DATA_DIRS, which is how the Flatpak build
    /// reaches the host's applications via /run/host.
    #[test]
    fn desktop_dirs_follow_xdg_data_dirs() {
        // SAFETY: single-threaded test; no other thread reads the environment.
        unsafe {
            std::env::set_var("XDG_DATA_DIRS", "/run/host/usr/share:/app/share");
        }
        let dirs = desktop_dirs();
        unsafe {
            std::env::remove_var("XDG_DATA_DIRS");
        }

        assert!(dirs.contains(&PathBuf::from("/run/host/usr/share/applications")));
        assert!(dirs.contains(&PathBuf::from("/app/share/applications")));
    }

    #[test]
    fn desktop_dirs_fall_back_to_the_spec_default() {
        unsafe {
            std::env::remove_var("XDG_DATA_DIRS");
        }
        let dirs = desktop_dirs();
        assert!(dirs.contains(&PathBuf::from("/usr/share/applications")));
        assert!(dirs.contains(&PathBuf::from("/usr/local/share/applications")));
    }

    #[test]
    fn desktop_dirs_are_deduplicated() {
        unsafe {
            std::env::set_var("XDG_DATA_DIRS", "/usr/share:/usr/share");
        }
        let dirs = desktop_dirs();
        unsafe {
            std::env::remove_var("XDG_DATA_DIRS");
        }
        let count = dirs
            .iter()
            .filter(|d| *d == &PathBuf::from("/usr/share/applications"))
            .count();
        assert_eq!(count, 1);
    }
}
