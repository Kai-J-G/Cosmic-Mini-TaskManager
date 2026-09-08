//! Types for system processes, resource metrics, and filtering.

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
