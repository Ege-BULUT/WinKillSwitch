//! Shared data types. Every module reads these; only `store` persists them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub type Pid = u32;

/// How a process is classified, for the Type column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcKind {
    App,
    Background,
    Windows,
}

impl ProcKind {
    pub fn label(self) -> &'static str {
        match self {
            ProcKind::App => "App",
            ProcKind::Background => "Background process",
            ProcKind::Windows => "Windows process",
        }
    }
}

/// One running process, as sampled.
#[derive(Clone, Debug)]
pub struct ProcInfo {
    pub pid: Pid,
    pub parent: Option<Pid>,
    pub name: String,
    pub exe: Option<PathBuf>,
    /// Percent of one logical core summed across cores divided by core count (0..=100).
    pub cpu: f32,
    /// Resident memory in bytes.
    pub mem: u64,
    /// Seconds since the Unix epoch.
    pub start: u64,
    /// Disk I/O rate in bytes/second (read + write) over the last sample interval.
    pub disk: u64,
    /// GPU use in percent (busiest engine type), and that engine type ("3D", "Video Decode").
    pub gpu: f32,
    pub gpu_engine: String,
    /// "Running" / "Suspended" / other.
    pub status: &'static str,
    pub kind: ProcKind,
    /// Full command line, joined, when available.
    pub cmd: String,
    /// Windows session id.
    pub session: Option<u32>,
    /// CompanyName from the exe's version resource (empty if none).
    pub publisher: String,
    /// FileDescription from the exe's version resource (falls back to the image name).
    pub description: String,
}

impl ProcInfo {
    /// Lowercased full exe path, the identity used for matching.
    pub fn exe_key(&self) -> Option<String> {
        self.exe.as_deref().map(path_key)
    }
}

/// Lowercase, backslash-normalised path string used for comparisons.
pub fn path_key(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\").to_lowercase()
}

/// A full process list at one moment.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub procs: Vec<ProcInfo>,
    pub total_mem: u64,
    pub used_mem: u64,
    /// Whole-machine CPU percent.
    pub cpu_total: f32,
}

impl Snapshot {
    pub fn get(&self, pid: Pid) -> Option<&ProcInfo> {
        self.procs.iter().find(|p| p.pid == pid)
    }
    pub fn children_map(&self) -> HashMap<Pid, Vec<Pid>> {
        let mut m: HashMap<Pid, Vec<Pid>> = HashMap::new();
        for p in &self.procs {
            if let Some(pp) = p.parent {
                m.entry(pp).or_default().push(p.pid);
            }
        }
        m
    }
}

/// A process identity kept in a group: the exe path when known, else the image name.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Member {
    /// Image name as shown, e.g. `chrome.exe`.
    pub name: String,
    /// Lowercased full path (`path_key`), when the process exposed one.
    pub exe: Option<String>,
}

impl Member {
    pub fn from_proc(p: &ProcInfo) -> Member {
        Member { name: p.name.clone(), exe: p.exe_key() }
    }
    pub fn matches(&self, p: &ProcInfo) -> bool {
        match (&self.exe, p.exe_key()) {
            (Some(a), Some(b)) => *a == b,
            _ => self.name.eq_ignore_ascii_case(&p.name),
        }
    }
    /// Stable id for tables and selection.
    pub fn key(&self) -> String {
        self.exe.clone().unwrap_or_else(|| self.name.to_lowercase())
    }
}

/// Where an autostart entry lives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutostartKind {
    /// A `Run` registry value; `hklm` false means HKCU. `wow64` is the 32-bit view (`Run32`).
    RunKey { hklm: bool, wow64: bool, value: String },
    /// A shortcut in a Startup folder; `common` is the all-users folder.
    StartupFolder { common: bool, file: String },
    /// A Windows service by its key name.
    Service { name: String },
    /// A scheduled task by its full path (`\Vendor\TaskName`).
    Task { path: String },
}

/// A discovered autostart entry for a group.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutostartEntry {
    pub kind: AutostartKind,
    /// What to show the user.
    pub label: String,
    /// The command or binary it launches.
    pub target: String,
    /// Currently enabled (would start automatically).
    pub enabled: bool,
}

/// One change PauseSwitch made, with what it replaced, so it can be put back exactly.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutostartChange {
    /// A `StartupApproved` value was written; `previous` is the old bytes (None: there was none).
    StartupApproved { hklm: bool, subkey: String, value: String, previous: Option<Vec<u8>> },
    /// A service's start type was set to Disabled; `previous_start` is the old SERVICE_* start type.
    Service { name: String, previous_start: u32, was_running: bool },
    /// A scheduled task was disabled; it was enabled before.
    Task { path: String },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PauseRecord {
    pub applied_at: u64,
    pub changes: Vec<AutostartChange>,
    /// Entries that couldn't be changed, with the reason.
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub id: u64,
    pub name: String,
    pub members: Vec<Member>,
    /// Some while PauseSwitch is on for this group.
    pub pause: Option<PauseRecord>,
}

impl Group {
    pub fn is_paused(&self) -> bool {
        self.pause.is_some()
    }
    pub fn has(&self, m: &Member) -> bool {
        self.members.iter().any(|x| x.key() == m.key())
    }
    pub fn add(&mut self, m: Member) -> bool {
        if self.has(&m) {
            return false;
        }
        self.members.push(m);
        true
    }
    pub fn remove_key(&mut self, key: &str) -> bool {
        let n = self.members.len();
        self.members.retain(|m| m.key() != key);
        n != self.members.len()
    }
}

/// Everything persisted to disk.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Saved {
    pub groups: Vec<Group>,
    pub next_id: u64,
    /// Ordered ids of the columns the user has shown. Empty means "use the default set".
    #[serde(default)]
    pub columns: Vec<String>,
    #[serde(default)]
    pub appearance: Appearance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GlassStyle {
    /// Clear liquid glass with a refracting rim (the showreel's tab bar).
    Liquid,
    /// Ribbed, reeded glass (the showreel's music player).
    Fluted,
    /// Heavily frosted, for the calmest backdrop.
    Frosted,
}

/// How the app looks; saved with everything else.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub palette: String,
    pub mode: ThemeMode,
    /// Index into the palette variant's accents.
    pub accent: usize,
    pub glass: GlassStyle,
    /// 0 = light, 1 = medium, 2 = strong tint and frost.
    pub strength: u8,
    /// The background gradient drifts slowly (costs a little GPU while the window shows).
    pub animated: bool,
    /// Ctrl+Shift+K opens the quick palette from anywhere.
    pub hotkey: bool,
    /// Show processes as a parent and child tree instead of a flat list.
    pub tree: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            palette: "aurora".into(),
            mode: ThemeMode::System,
            accent: 0,
            glass: GlassStyle::Liquid,
            strength: 1,
            animated: true,
            hotkey: true,
            tree: false,
        }
    }
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
