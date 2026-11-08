//! Respawn guard. NOT IMPLEMENTED in this build: guards are recorded but never act.
use crate::model::{Member, Pid, Snapshot};

pub const AVAILABLE: bool = false;

#[derive(Clone, Debug)]
pub struct Guard {
    pub id: u64,
    pub label: String,
    pub members: Vec<Member>,
    pub sticky: bool,
}

#[derive(Clone, Debug)]
pub enum GuardEvent {
    Blocked { guard: u64, pid: Pid, name: String },
    UserLaunch { guard: u64, name: String },
}

#[derive(Default)]
pub struct Guards {
    list: Vec<Guard>,
}

impl Guards {
    pub fn new() -> Guards { Guards::default() }
    pub fn add(&mut self, g: Guard, _snap: &Snapshot) { self.remove(g.id); self.list.push(g); }
    pub fn remove(&mut self, id: u64) { self.list.retain(|g| g.id != id); }
    pub fn active(&self) -> Vec<(u64, String)> { self.list.iter().map(|g| (g.id, g.label.clone())).collect() }
    /// Not implemented: never reports events.
    pub fn check(&mut self, _snap: &Snapshot) -> Vec<GuardEvent> { Vec::new() }
}
