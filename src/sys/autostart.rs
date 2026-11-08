//! PauseSwitch autostart changes.
//! NOT IMPLEMENTED in this build: nothing is discovered, changed or restored.
use std::path::Path;
use crate::model::{AutostartChange, AutostartEntry, Member};

pub const AVAILABLE: bool = false;

pub fn discover(_members: &[Member], _root: Option<&Path>) -> Vec<AutostartEntry> { Vec::new() }

pub fn apply(_entries: &[AutostartEntry]) -> (Vec<AutostartChange>, Vec<String>) {
    (Vec::new(), vec!["autostart changes are not implemented in this build".into()])
}

pub fn revert(_changes: &[AutostartChange]) -> Vec<String> { Vec::new() }
