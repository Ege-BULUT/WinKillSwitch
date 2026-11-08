//! Ending processes.
//! NOT IMPLEMENTED in this build: `end_all` ends nothing and reports every pid as failed.
use crate::model::{Pid, Snapshot};

pub const AVAILABLE: bool = false;

#[derive(Clone, Debug, Default)]
pub struct KillReport {
    pub ended: Vec<(Pid, String)>,
    /// pid, name, reason
    pub failed: Vec<(Pid, String, String)>,
}

/// End each pid, in the order given. Not implemented.
pub fn end_all(snap: &Snapshot, pids: &[Pid]) -> KillReport {
    let failed = pids
        .iter()
        .map(|&pid| {
            let name = snap.get(pid).map(|p| p.name.clone()).unwrap_or_default();
            (pid, name, "ending processes is not implemented in this build".to_string())
        })
        .collect();
    KillReport { ended: Vec::new(), failed }
}
