//! New-process alerts: say when a program you asked about starts, or when anything starts from a
//! folder downloads and installers land in.

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::model::{Pid, ProcInfo, Snapshot};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Alerts {
    /// Alert when a program starts from Downloads or a temporary folder.
    pub risky_folders: bool,
    /// Image names to watch, `*` matches any run of characters ("steam*.exe").
    pub watch: Vec<String>,
}

impl Alerts {
    pub fn active(&self) -> bool {
        self.risky_folders || !self.watch.is_empty()
    }
}

/// Case-insensitive match with `*` wildcards; a pattern without `.exe` also matches `name.exe`.
pub fn glob(pattern: &str, name: &str) -> bool {
    let p = pattern.trim().to_lowercase();
    let n = name.to_lowercase();
    if p.is_empty() {
        return false;
    }
    fn go(p: &[u8], n: &[u8]) -> bool {
        match p.first() {
            None => n.is_empty(),
            Some(b'*') => (0..=n.len()).any(|i| go(&p[1..], &n[i..])),
            Some(c) => n.first() == Some(c) && go(&p[1..], &n[1..]),
        }
    }
    go(p.as_bytes(), n.as_bytes()) || (!p.contains('.') && go(format!("{p}.exe").as_bytes(), n.as_bytes()))
}

/// The folders "risky" means, from the environment: Downloads and the temp folders.
pub fn risky_roots() -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(home) = std::env::var("USERPROFILE") {
        v.push(format!("{home}\\Downloads"));
    }
    for k in ["TEMP", "TMP"] {
        if let Ok(t) = std::env::var(k) {
            v.push(t);
        }
    }
    if let Ok(w) = std::env::var("SystemRoot") {
        v.push(format!("{w}\\Temp"));
    }
    v.into_iter().map(|r| crate::model::path_key(Path::new(&r)).trim_end_matches('\\').to_string() + "\\").collect()
}

/// Why `p` deserves an alert, if it does.
pub fn reason(a: &Alerts, roots: &[String], p: &ProcInfo) -> Option<String> {
    if let Some(w) = a.watch.iter().find(|w| glob(w, &p.name)) {
        return Some(format!("Matches your rule \"{}\"", w.trim()));
    }
    if a.risky_folders {
        if let Some(exe) = p.exe_key() {
            if roots.iter().any(|r| exe.starts_with(r.as_str())) {
                let folder = if exe.contains("\\downloads\\") { "Downloads" } else { "a temporary folder" };
                return Some(format!("Started from {folder}"));
            }
        }
    }
    None
}

/// Tracks which processes have been seen, so only new ones are judged.
#[derive(Default)]
pub struct Watcher {
    seen: HashSet<Pid>,
    primed: bool,
}

impl Watcher {
    /// Processes in `snap` that are new since the last call and match a rule. The first call
    /// only learns what is already running.
    pub fn check<'a>(&mut self, a: &Alerts, roots: &[String], snap: &'a Snapshot) -> Vec<(&'a ProcInfo, String)> {
        let mut out = Vec::new();
        let now: HashSet<Pid> = snap.procs.iter().map(|p| p.pid).collect();
        if self.primed && a.active() {
            for p in &snap.procs {
                if !self.seen.contains(&p.pid) {
                    if let Some(r) = reason(a, roots, p) {
                        out.push((p, r));
                    }
                }
            }
        }
        self.seen = now;
        self.primed = true;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ProcKind;

    fn proc(pid: Pid, name: &str, exe: &str) -> ProcInfo {
        ProcInfo {
            pid,
            parent: None,
            name: name.into(),
            exe: Some(exe.into()),
            cpu: 0.0,
            mem: 0,
            start: 0,
            disk: 0,
            gpu: 0.0,
            gpu_engine: String::new(),
            status: "Running",
            kind: ProcKind::App,
            cmd: String::new(),
            session: None,
            publisher: String::new(),
            description: String::new(),
        }
    }

    fn snap(procs: Vec<ProcInfo>) -> Snapshot {
        Snapshot { procs, ..Default::default() }
    }

    #[test]
    fn globs() {
        assert!(glob("steam*.exe", "SteamWebHelper.exe"));
        assert!(glob("discord", "Discord.exe"));
        assert!(glob("*", "x.exe"));
        assert!(!glob("discord", "DiscordPTB.exe"));
        assert!(!glob("", "a.exe"));
        assert!(!glob("chrome.exe", "chromedriver.exe"));
    }

    #[test]
    fn only_new_matching_processes_alert() {
        let a = Alerts { risky_folders: true, watch: vec!["steam*".into()] };
        let roots = vec![r"c:\users\me\downloads\".to_string()];
        let mut w = Watcher::default();
        let first = snap(vec![proc(1, "steam.exe", r"C:\Steam\steam.exe")]);
        assert!(w.check(&a, &roots, &first).is_empty(), "already running when watching began");
        let second = snap(vec![
            proc(1, "steam.exe", r"C:\Steam\steam.exe"),
            proc(2, "steamwebhelper.exe", r"C:\Steam\steamwebhelper.exe"),
            proc(3, "setup.exe", r"C:\Users\me\Downloads\setup.exe"),
            proc(4, "notepad.exe", r"C:\Windows\notepad.exe"),
        ]);
        let hits: Vec<Pid> = w.check(&a, &roots, &second).iter().map(|(p, _)| p.pid).collect();
        assert_eq!(hits, vec![2, 3]);
        assert!(w.check(&a, &roots, &second).is_empty(), "each process alerts once");
    }

    #[test]
    fn off_means_silent() {
        let mut w = Watcher::default();
        let a = Alerts::default();
        w.check(&a, &[], &snap(vec![]));
        assert!(w.check(&a, &[], &snap(vec![proc(5, "x.exe", r"C:\x.exe")])).is_empty());
    }
}
