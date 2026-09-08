//! Types for system processes, resource metrics, and filtering.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessState {
    Running,
    Sleeping,
    Stopped,
    Zombie,
    DiskSleep,
    Other,
}

impl ProcessState {
    pub fn label(&self) -> String {
        match self {
            Self::Running => crate::fl!("status-run"),
            Self::Sleeping => crate::fl!("status-sleep"),
            Self::Stopped => crate::fl!("status-stopped"),
            Self::Zombie => crate::fl!("status-zombie"),
            Self::DiskSleep => crate::fl!("status-hung"),
            Self::Other => crate::fl!("status-other"),
        }
    }

    pub fn is_unresponsive_or_stopped(&self) -> bool {
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
    pub used_swap_bytes: u64,
    pub total_swap_bytes: u64,
    pub swap_percent: f32,
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
    pub fn label(&self) -> String {
        match self {
            Self::All => crate::fl!("tab-all"),
            Self::Apps => crate::fl!("tab-apps"),
            Self::TopCpu => crate::fl!("tab-top-cpu"),
            Self::TopMemory => crate::fl!("tab-top-ram"),
            Self::Unresponsive => crate::fl!("tab-unresponsive"),
        }
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = 1024 * 1024 * 1024;

    if bytes >= GIB {
        format!("{:.1} GB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.0} KB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}
