//! System and process metrics collector.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, ProcessRefreshKind, ProcessesToUpdate, System};

use super::types::{FilterTab, ProcessItem, ProcessState, SystemOverview};

#[derive(Clone, Debug)]
struct DesktopAppEntry {
    name: String,
    icon: String,
}

pub struct ProcessCollector {
    sys: System,
    desktop_apps: HashMap<String, DesktopAppEntry>,
    disk_sleep_counter: HashMap<u32, u8>,
}

impl ProcessCollector {
    pub fn new() -> Self {
        let mut sys = System::new_with_specifics(
            sysinfo::RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        sys.refresh_cpu_all();
        sys.refresh_memory();

        let desktop_apps = scan_desktop_applications();

        Self {
            sys,
            desktop_apps,
            disk_sleep_counter: HashMap::new(),
        }
    }

    pub fn collect(&mut self) -> (SystemOverview, Vec<ProcessItem>) {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::everything(),
        );

        let total_cpu = self.sys.global_cpu_usage();
        let used_mem = self.sys.used_memory();
        let total_mem = self.sys.total_memory();
        let mem_percent = if total_mem > 0 {
            (used_mem as f32 / total_mem as f32) * 100.0
        } else {
            0.0
        };

        let used_swap = self.sys.used_swap();
        let total_swap = self.sys.total_swap();
        let swap_percent = if total_swap > 0 {
            (used_swap as f32 / total_swap as f32) * 100.0
        } else {
            0.0
        };

        let mut items = Vec::new();
        let mut unresponsive_count = 0;

        for (pid, proc_) in self.sys.processes() {
            let pid_u32 = pid.as_u32();

            // Skip kernel workers / internal threads (empty cmdline and pid != 1)
            let cmd_parts: Vec<String> = proc_
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect();
            let raw_name = proc_.name().to_string_lossy().to_string();

            if cmd_parts.is_empty() && pid_u32 != 1 {
                continue;
            }

            let status = match proc_.status() {
                sysinfo::ProcessStatus::Run => {
                    self.disk_sleep_counter.remove(&pid_u32);
                    ProcessState::Running
                }
                sysinfo::ProcessStatus::Sleep
                | sysinfo::ProcessStatus::Idle
                | sysinfo::ProcessStatus::LockBlocked
                | sysinfo::ProcessStatus::Wakekill
                | sysinfo::ProcessStatus::Waking
                | sysinfo::ProcessStatus::Parked => {
                    self.disk_sleep_counter.remove(&pid_u32);
                    ProcessState::Sleeping
                }
                sysinfo::ProcessStatus::Stop => {
                    self.disk_sleep_counter.remove(&pid_u32);
                    ProcessState::Stopped
                }
                sysinfo::ProcessStatus::Zombie | sysinfo::ProcessStatus::Dead => {
                    self.disk_sleep_counter.remove(&pid_u32);
                    ProcessState::Zombie
                }
                sysinfo::ProcessStatus::UninterruptibleDiskSleep => {
                    let count = self.disk_sleep_counter.entry(pid_u32).or_insert(0);
                    *count = count.saturating_add(1);
                    if *count >= 2 {
                        ProcessState::DiskSleep
                    } else {
                        ProcessState::Sleeping
                    }
                }
                _ => {
                    self.disk_sleep_counter.remove(&pid_u32);
                    ProcessState::Other
                }
            };

            let is_unresponsive = status.is_unresponsive_or_stopped();
            if is_unresponsive {
                unresponsive_count += 1;
            }

            let (display_name, icon_name, is_gui_app) = self.match_app_info(&raw_name, &cmd_parts);

            let item = ProcessItem {
                pid: pid_u32,
                name: display_name,
                cmd: cmd_parts.join(" "),
                cpu_usage: proc_.cpu_usage(),
                memory_bytes: proc_.memory(),
                status,
                is_gui_app,
                icon_name,
            };

            items.push(item);
        }

        let overview = SystemOverview {
            total_cpu_percent: total_cpu,
            used_memory_bytes: used_mem,
            total_memory_bytes: total_mem,
            memory_percent: mem_percent,
            used_swap_bytes: used_swap,
            total_swap_bytes: total_swap,
            swap_percent,
            total_processes: items.len(),
            unresponsive_or_stopped_count: unresponsive_count,
        };

        (overview, items)
    }

    fn match_app_info(&self, raw_name: &str, cmd_parts: &[String]) -> (String, String, bool) {
        let name_lower = raw_name.to_lowercase();
        if let Some(entry) = self.desktop_apps.get(&name_lower) {
            return (entry.name.clone(), entry.icon.clone(), true);
        }

        if let Some(first_cmd) = cmd_parts.first() {
            let file_name = Path::new(first_cmd)
                .file_name()
                .map(|f| f.to_string_lossy().to_string().to_lowercase())
                .unwrap_or_default();

            if let Some(entry) = self.desktop_apps.get(&file_name) {
                return (entry.name.clone(), entry.icon.clone(), true);
            }
        }

        let fallback_icon = match name_lower.as_str() {
            n if n.contains("firefox") => "firefox",
            n if n.contains("chrome") || n.contains("chromium") => "google-chrome",
            n if n.contains("code") || n.contains("vscodium") => "code",
            n if n.contains("alacritty") => "Alacritty",
            n if n.contains("kitty") => "kitty",
            n if n.contains("foot") => "foot",
            n if n.contains("terminal") => "utilities-terminal",
            n if n.contains("cosmic") => "system-run-symbolic",
            n if n.contains("steam") => "steam",
            n if n.contains("discord") => "discord",
            n if n.contains("spotify") => "spotify",
            _ => "application-x-executable-symbolic",
        };

        (raw_name.to_string(), fallback_icon.to_string(), false)
    }
}

pub fn filter_and_sort_processes<'a>(
    processes: &'a [ProcessItem],
    tab: FilterTab,
    search_query: &str,
) -> Vec<&'a ProcessItem> {
    let query = search_query.trim().to_lowercase();

    let mut filtered: Vec<&'a ProcessItem> = processes
        .iter()
        .filter(|p| {
            if !query.is_empty() {
                let matches_name = p.name.to_lowercase().contains(&query);
                let matches_cmd = p.cmd.to_lowercase().contains(&query);
                let matches_pid = p.pid.to_string().contains(&query);
                if !matches_name && !matches_cmd && !matches_pid {
                    return false;
                }
            }

            match tab {
                FilterTab::All => true,
                FilterTab::Apps => p.is_gui_app,
                FilterTab::TopCpu => true,
                FilterTab::TopMemory => true,
                FilterTab::Unresponsive => p.is_unresponsive_or_stopped(),
            }
        })
        .collect();

    match tab {
        FilterTab::Unresponsive => {
            filtered.sort_by(|a, b| {
                let a_cpu = (a.cpu_usage * 10.0) as i32;
                let b_cpu = (b.cpu_usage * 10.0) as i32;
                if a_cpu != b_cpu {
                    b_cpu.cmp(&a_cpu)
                } else {
                    a.pid.cmp(&b.pid)
                }
            });
        }
        FilterTab::TopMemory => {
            filtered.sort_by(|a, b| {
                let a_mem = a.memory_bytes / (1024 * 1024);
                let b_mem = b.memory_bytes / (1024 * 1024);
                if a_mem != b_mem {
                    b_mem.cmp(&a_mem)
                } else {
                    a.pid.cmp(&b.pid)
                }
            });
        }
        FilterTab::TopCpu | FilterTab::All | FilterTab::Apps => {
            filtered.sort_by(|a, b| {
                let a_hung = a.is_unresponsive_or_stopped();
                let b_hung = b.is_unresponsive_or_stopped();
                if a_hung != b_hung {
                    return b_hung.cmp(&a_hung);
                }
                let a_cpu = (a.cpu_usage * 10.0) as i32;
                let b_cpu = (b.cpu_usage * 10.0) as i32;
                if a_cpu != b_cpu {
                    b_cpu.cmp(&a_cpu)
                } else {
                    a.pid.cmp(&b.pid)
                }
            });
        }
    }

    filtered
}

fn scan_desktop_applications() -> HashMap<String, DesktopAppEntry> {
    let mut map = HashMap::new();
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
    ];

    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(format!("{}/.local/share/applications", home)));
        dirs.push(PathBuf::from(format!(
            "{}/.local/share/flatpak/exports/share/applications",
            home
        )));
    }

    for dir in dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    parse_desktop_file(&path, &mut map);
                }
            }
        }
    }

    map
}

fn parse_desktop_file(path: &Path, map: &mut HashMap<String, DesktopAppEntry>) {
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };

    let mut in_desktop_entry = false;
    let mut name: Option<String> = None;
    let mut exec: Option<String> = None;
    let mut icon: Option<String> = None;
    let mut no_display = false;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }

        if !in_desktop_entry || line.starts_with('#') {
            continue;
        }

        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim();
            let val = val.trim();
            match key {
                "Name" if name.is_none() => name = Some(val.to_string()),
                "Exec" if exec.is_none() => exec = Some(val.to_string()),
                "Icon" if icon.is_none() => icon = Some(val.to_string()),
                "NoDisplay" => no_display = val.eq_ignore_ascii_case("true"),
                _ => {}
            }
        }
    }

    if no_display {
        return;
    }

    if let (Some(app_name), Some(exec_str)) = (name, exec) {
        let bin_name = exec_str
            .split_whitespace()
            .next()
            .map(|s| {
                Path::new(s)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| s.to_string())
            })
            .unwrap_or_default();

        if !bin_name.is_empty() {
            let icon_name = icon.unwrap_or_else(|| "application-x-executable-symbolic".to_string());
            let entry = DesktopAppEntry {
                name: app_name,
                icon: icon_name,
            };
            map.insert(bin_name.to_lowercase(), entry);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_runs_and_overview_valid() {
        let mut collector = ProcessCollector::new();
        let (overview, procs) = collector.collect();
        assert!(overview.total_memory_bytes > 0);
        assert!(!procs.is_empty());
    }

    #[test]
    fn test_filter_and_sort_tabs() {
        let procs = vec![
            ProcessItem {
                pid: 100,
                name: "test-app".to_string(),
                cmd: "test-app --gui".to_string(),
                cpu_usage: 10.0,
                memory_bytes: 100 * 1024 * 1024,
                status: ProcessState::Running,
                is_gui_app: true,
                icon_name: "test".to_string(),
            },
            ProcessItem {
                pid: 101,
                name: "hung-worker".to_string(),
                cmd: "worker".to_string(),
                cpu_usage: 95.0,
                memory_bytes: 50 * 1024 * 1024,
                status: ProcessState::Stopped,
                is_gui_app: false,
                icon_name: "worker".to_string(),
            },
            ProcessItem {
                pid: 102,
                name: "daemon".to_string(),
                cmd: "daemon".to_string(),
                cpu_usage: 2.0,
                memory_bytes: 500 * 1024 * 1024,
                status: ProcessState::Sleeping,
                is_gui_app: false,
                icon_name: "daemon".to_string(),
            },
        ];

        let unresp = filter_and_sort_processes(&procs, FilterTab::Unresponsive, "");
        assert_eq!(unresp.len(), 1);
        assert_eq!(unresp[0].pid, 101);

        let apps = filter_and_sort_processes(&procs, FilterTab::Apps, "");
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].pid, 100);

        let mem = filter_and_sort_processes(&procs, FilterTab::TopMemory, "");
        assert_eq!(mem[0].pid, 102);

        let search = filter_and_sort_processes(&procs, FilterTab::All, "hung");
        assert_eq!(search.len(), 1);
        assert_eq!(search[0].pid, 101);
    }
}
