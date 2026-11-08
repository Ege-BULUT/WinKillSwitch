//! Which processes belong to the same app as a given one.
//! NOT IMPLEMENTED in this build: every function returns an empty result.
use std::path::{Path, PathBuf};
use crate::model::{Member, Pid, Snapshot};

pub const AVAILABLE: bool = false;

/// The processes found for a seed, ordered so ancestors come before descendants.
#[derive(Clone, Debug, Default)]
pub struct Related {
    pub pids: Vec<Pid>,
    /// Distinct identities among `pids`, for adding to a group.
    pub members: Vec<Member>,
    /// The app's install folder, when one was found.
    pub root: Option<PathBuf>,
}

/// The app's install folder for an exe. Not implemented: always None.
pub fn app_root(_exe: &Path) -> Option<PathBuf> { None }

/// Everything related to `seed`. Not implemented: always empty.
pub fn related(_snap: &Snapshot, _seed: Pid) -> Related { Related::default() }

/// Running processes matching any member. Not implemented: always empty.
pub fn running_members(_snap: &Snapshot, _members: &[Member]) -> Vec<Pid> { Vec::new() }
