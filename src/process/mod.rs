//! Process monitoring and management.

pub mod actions;
pub mod collector;
pub mod host;
pub mod types;

pub use collector::{ProcessCollector, filter_and_sort_processes};
pub use types::{FilterTab, ProcessItem, ProcessState, SystemOverview, format_bytes, tree_pids};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::process::{Child, Command};

    fn alive(pid: u32) -> bool {
        Path::new(&format!("/proc/{pid}")).exists()
    }

    /// Spins until `cond` holds, or gives up. Cheap busy-wait: these
    /// transitions take microseconds and the test must not sleep.
    fn settle(mut cond: impl FnMut() -> bool) -> bool {
        (0..200_000).any(|_| cond())
    }

    /// `sh` with two `sleep` children, one of which has a grandchild.
    fn spawn_tree(collector: &mut ProcessCollector) -> (Child, Vec<u32>) {
        let child = Command::new("sh")
            .arg("-c")
            .arg("sleep 300 & sh -c 'sleep 300' & sleep 300")
            .spawn()
            .expect("spawn process tree");
        let root = child.id();

        let mut tree = Vec::new();
        settle(|| {
            let (_, procs) = collector.collect();
            tree = tree_pids(root, &procs);
            tree.len() >= 4
        });

        assert!(
            tree.len() >= 3,
            "expected a multi-level tree for PID {root}, got {tree:?}"
        );
        (child, tree)
    }

    /// The bug this exists to prevent: signalling only the clicked PID leaves
    /// its children running, reparented to init.
    #[test]
    fn killing_only_the_root_leaks_orphans() {
        let mut collector = ProcessCollector::new();
        let (mut child, tree) = spawn_tree(&mut collector);
        let descendants = tree[1..].to_vec();

        actions::kill_process(tree[0]).expect("kill root");
        let _ = child.wait();
        settle(|| !alive(tree[0]));

        let survivors: Vec<u32> = descendants.iter().copied().filter(|&p| alive(p)).collect();
        assert!(
            !survivors.is_empty(),
            "expected orphans after a root-only kill; got none, so this test \
             no longer demonstrates anything"
        );

        // Don't leave the orphans behind.
        actions::kill_all(&descendants);
    }

    #[test]
    fn killing_the_tree_leaves_no_orphans() {
        let mut collector = ProcessCollector::new();
        let (mut child, tree) = spawn_tree(&mut collector);
        let descendants = tree[1..].to_vec();

        actions::kill_process(tree[0]).expect("kill root");
        actions::kill_all(&descendants);
        let _ = child.wait();

        let leaked: Vec<u32> = descendants
            .iter()
            .copied()
            .filter(|&pid| !settle(|| !alive(pid)))
            .collect();
        assert!(leaked.is_empty(), "orphans left running: {leaked:?}");
    }
}
