//! Process monitoring and management subsystem.
//!
//! Submodules:
//! - [`actions`]: POSIX signal dispatch (`SIGSTOP`, `SIGCONT`, `SIGTERM`, `SIGKILL`).
//! - [`collector`]: System resource querying, debounced process state polling, and desktop app resolution.
//! - [`types`]: Data models for processes, filter tabs, system overview metrics, and formatting helpers.

pub mod actions;
pub mod collector;
pub mod types;

pub use collector::{filter_and_sort_processes, ProcessCollector};
pub use types::{format_bytes, FilterTab, ProcessItem, ProcessState, SystemOverview};

