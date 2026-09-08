//! System and process metrics collector.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::{
    CpuRefreshKind, MemoryRefreshKind, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind,
};

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
    /// Binary name (lowercased) to the `.desktop` entry that launches it.
    desktop_apps: HashMap<String, DesktopAppEntry>,
    /// Consecutive polls each PID has spent in uninterruptible sleep.
    disk_sleep_ticks: HashMap<u32, u8>,
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
            sys,
            desktop_apps: scan_desktop_applications(),
            disk_sleep_ticks: HashMap::new(),
        }
    }

    pub fn collect(&mut self) -> (SystemOverview, Vec<ProcessItem>) {
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

        let mut items = Vec::with_capacity(self.sys.processes().len());
        let mut unresponsive_count = 0;
        let mut seen_disk_sleep = HashSet::new();

        for (pid, proc_) in self.sys.processes() {
            let pid = pid.as_u32();
            let cmd = proc_.cmd();

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
                name,
                cmd: join_cmd(cmd),
                cpu_usage: proc_.cpu_usage(),
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
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
    ];

    if let Some(data_home) = data_home() {
        dirs.push(data_home.join("applications"));
        dirs.push(data_home.join("flatpak/exports/share/applications"));
    }

    let mut map = HashMap::new();
    for dir in dirs {
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
