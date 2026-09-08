//! Types for system processes, resource metrics, and filtering.

use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Sleeping,
    Stopped,
    Zombie,
    DiskSleep,
    Other,
}

impl ProcessState {
    pub fn label(self) -> String {
        match self {
            Self::Running => crate::fl!("status-run"),
            Self::Sleeping => crate::fl!("status-sleep"),
            Self::Stopped => crate::fl!("status-stopped"),
            Self::Zombie => crate::fl!("status-zombie"),
            Self::DiskSleep => crate::fl!("status-hung"),
            Self::Other => crate::fl!("status-other"),
        }
    }

    /// Stopped, dead, or stuck in uninterruptible I/O: states the user
    /// probably wants to know about.
    pub fn is_unresponsive_or_stopped(self) -> bool {
        matches!(self, Self::Stopped | Self::Zombie | Self::DiskSleep)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessItem {
    pub pid: u32,
    /// PPID. `None` only for PID 1.
    pub parent: Option<u32>,
    pub name: String,
    pub cmd: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub status: ProcessState,
    pub is_gui_app: bool,
    pub icon_name: String,
}

impl ProcessItem {
    pub fn memory_formatted(&self) -> String {
        format_bytes(self.memory_bytes)
    }

    pub fn is_unresponsive_or_stopped(&self) -> bool {
        self.status.is_unresponsive_or_stopped()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemOverview {
    pub total_cpu_percent: f32,
    pub used_memory_bytes: u64,
    pub total_memory_bytes: u64,
    pub memory_percent: f32,
    pub total_processes: usize,
    pub unresponsive_or_stopped_count: usize,
}

/// `root` followed by all of its descendants, breadth-first.
///
/// Killing a process leaves its children running, reparented to init, so a
/// "kill" that means it has to sweep the whole subtree. The list is built from
/// the current snapshot: once the root dies its children are reparented, and
/// walking the tree afterwards would find nothing.
pub fn tree_pids(root: u32, processes: &[ProcessItem]) -> Vec<u32> {
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for item in processes {
        if let Some(parent) = item.parent {
            children.entry(parent).or_default().push(item.pid);
        }
    }

    // Root first: killing it before its children stops it spawning more.
    let mut ordered = vec![root];
    let mut seen = HashSet::from([root]);
    let mut queue = VecDeque::from([root]);

    while let Some(pid) = queue.pop_front() {
        for &child in children.get(&pid).into_iter().flatten() {
            // A PID cycle is impossible on Linux, but a malformed snapshot
            // must not hang the UI thread.
            if seen.insert(child) {
                ordered.push(child);
                queue.push_back(child);
            }
        }
    }

    ordered
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FilterTab {
    #[default]
    All,
    Apps,
    TopCpu,
    TopMemory,
    Unresponsive,
}

impl FilterTab {
    pub fn label(self) -> String {
        match self {
            Self::All => crate::fl!("tab-all"),
            Self::Apps => crate::fl!("tab-apps"),
            Self::TopCpu => crate::fl!("tab-top-cpu"),
            Self::TopMemory => crate::fl!("tab-top-ram"),
            Self::Unresponsive => crate::fl!("tab-unresponsive"),
        }
    }
}

/// Formats a byte count with binary units, matching how `sysinfo` reports memory.
pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * KIB;
    const GIB: f64 = 1024.0 * MIB;

    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.0} KiB", bytes / KIB)
    } else {
        format!("{bytes:.0} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc_with_parent(pid: u32, parent: Option<u32>) -> ProcessItem {
        ProcessItem {
            pid,
            parent,
            name: format!("p{pid}"),
            cmd: String::new(),
            cpu_usage: 0.0,
            memory_bytes: 0,
            status: ProcessState::Running,
            is_gui_app: false,
            icon_name: String::new(),
        }
    }

    /// 1 -> 10 -> {100, 101 -> 1000}, plus an unrelated 20.
    fn tree() -> Vec<ProcessItem> {
        vec![
            proc_with_parent(1, None),
            proc_with_parent(10, Some(1)),
            proc_with_parent(100, Some(10)),
            proc_with_parent(101, Some(10)),
            proc_with_parent(1000, Some(101)),
            proc_with_parent(20, Some(1)),
        ]
    }

    #[test]
    fn tree_walk_collects_every_descendant() {
        // Grandchildren too: these are the orphans a plain kill leaves behind.
        assert_eq!(tree_pids(10, &tree()), [10, 100, 101, 1000]);
    }

    #[test]
    fn tree_walk_puts_the_root_first() {
        // The root is signalled first so it cannot fork more children while
        // the rest of the sweep runs.
        assert_eq!(tree_pids(10, &tree())[0], 10);
    }

    #[test]
    fn tree_walk_ignores_unrelated_branches() {
        assert_eq!(tree_pids(20, &tree()), [20]);
        assert!(!tree_pids(101, &tree()).contains(&100));
    }

    #[test]
    fn tree_walk_of_a_leaf_is_just_itself() {
        assert_eq!(tree_pids(1000, &tree()), [1000]);
        // A PID that is not in the snapshot at all still returns itself, so a
        // process that exits between poll and click is still signalled.
        assert_eq!(tree_pids(4242, &tree()), [4242]);
    }

    #[test]
    fn tree_walk_terminates_on_a_parent_cycle() {
        // Impossible on Linux, but a malformed snapshot must not hang the UI.
        let cyclic = vec![proc_with_parent(5, Some(6)), proc_with_parent(6, Some(5))];
        let walked = tree_pids(5, &cyclic);
        assert_eq!(walked, [5, 6]);
    }

    #[test]
    fn bytes_are_formatted_with_binary_units() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1 KiB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MiB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024 / 2), "1.5 GiB");
        // No overflow or panic at the top of the range.
        assert!(format_bytes(u64::MAX).ends_with(" GiB"));
    }
}
