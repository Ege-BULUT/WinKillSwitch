//! App state: the live snapshot, groups, selection, and what the user is looking at.

use std::collections::{HashSet, VecDeque};

use rune_kit::TableState;
use rune_kit::shell::PaneState;

use crate::model::{Appearance, Group, Member, Pid, Saved, Snapshot};
use crate::sys::guard::Guards;
use crate::sys::cleanup::{Category, Cleaned, Found};
use crate::sys::memory::{MemStatus, RamReport};
use crate::worker::{Cmd, Worker};
use crate::{store, sys};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Processes,
    Clean,
    PauseSwitch,
    Settings,
}

/// The memory cleaner's state.
#[derive(Default)]
pub struct RamState {
    pub running: bool,
    /// What the cleaner is doing right now, while it runs.
    pub stage: Option<&'static str>,
    pub last: Option<RamReport>,
    /// Clock time the last result arrived, for the count-up and ring pulse.
    pub shown_at: f64,
}

pub enum DiskPhase {
    Idle,
    Scanning { done: usize, total: usize, current: String, files: u64, bytes: u64 },
    Ready,
    Cleaning { done: usize, total: usize, current: String, freed: u64 },
    Done { freed: u64, deleted: u64, skipped: u64, at: f64 },
}

/// The "about this process" sheet.
pub struct InfoState {
    pub pid: Pid,
    /// Filled in once the worker has inspected the process.
    pub detail: Option<crate::sys::info::Detail>,
}

pub struct DiskItem {
    pub cat: Category,
    pub found: Found,
    pub selected: bool,
}

pub struct DiskState {
    pub phase: DiskPhase,
    pub items: Vec<DiskItem>,
    /// Free and total bytes on the system drive.
    pub drive: (u64, u64),
    /// Per category results of the last clean.
    pub results: Vec<(String, Cleaned)>,
}

/// A transient confirmation sheet.
pub struct Confirm {
    pub title: String,
    pub message: String,
    /// Lines naming each process the action touches.
    pub items: Vec<String>,
    pub ok_label: String,
    pub danger: bool,
    pub action: ConfirmAction,
}

#[derive(Clone)]
pub enum ConfirmAction {
    Kill { pids: Vec<Pid>, label: String, guard_members: Vec<Member> },
    DeleteGroup { id: u64 },
    PauseGroup { id: u64 },
    ResumeGroup { id: u64 },
}

pub struct Toast {
    pub id: u64,
    pub title: String,
    pub detail: Option<String>,
    pub danger: bool,
    pub born: f64,
    /// A process the toast is about; it gets a Show button.
    pub pid: Option<Pid>,
}

pub struct App {
    pub tab: Tab,
    pub snap: Snapshot,
    pub worker: Option<Worker>,
    pub guards: Guards,

    pub mem: MemStatus,
    /// Recent memory use (fraction in use), newest last, one entry per sample.
    pub mem_hist: VecDeque<f32>,
    /// The ring's displayed fraction, eased toward `mem` every frame.
    pub mem_shown: f32,
    /// The cached slice's displayed fraction, eased the same way.
    pub cache_shown: f32,
    pub ram: RamState,
    pub disk: DiskState,
    pub info: Option<InfoState>,
    /// Visible table columns by id; empty means the default set.
    pub columns: Vec<String>,
    /// Where the column picker opened, while it is open.
    pub column_menu: Option<rune::PointerPress>,
    /// The "who is using this file?" sheet, while open.
    pub locks: Option<crate::ui::locks::LocksState>,
    /// The "why is my PC slow?" sheet, while open.
    pub diag: Option<crate::ui::diagnose::DiagState>,
    /// A file is being dragged over the window.
    pub drop_hover: bool,
    /// The quick palette, while open.
    pub palette: Option<crate::ui::palette::PaletteState>,
    /// Dragging the find-by-window target, and what it is over right now.
    pub finding: bool,
    pub find_hover: Option<crate::sys::finder::Target>,
    /// Open the info sheet for this pid once it shows up in a sample (headless scenes).
    pub pending_info: Option<Pid>,

    pub groups: Vec<Group>,
    pub next_id: u64,
    /// The group shown in the sidebar selection, if any.
    pub selected_group: Option<u64>,

    pub search: String,
    pub search_edit: rune::TextEdit,
    pub table: TableState,
    /// Filtered, sorted pids for the current frame (rebuilt in `poll`).
    pub view_rows: Vec<Pid>,
    /// For each row in tree view: its depth and whether it has children shown or hidden.
    pub row_meta: Vec<(u16, bool)>,
    /// Tree branches the user folded.
    pub collapsed: HashSet<Pid>,
    /// The process row the user touched last, for "add all related".
    pub active_pid: Option<Pid>,

    pub rename: Option<(u64, rune::TextEdit)>,
    pub confirm: Option<Confirm>,
    pub toasts: Vec<Toast>,
    pub toast_seq: u64,
    /// New-process alert rules, what has been seen, the folders that count as risky, and the
    /// rule being typed in Settings.
    pub alerts: crate::sys::alerts::Alerts,
    pub alert_watch: crate::sys::alerts::Watcher,
    pub alert_roots: Vec<String>,
    pub alert_edit: rune::TextEdit,

    pub elevated: bool,
    pub clock: f64,
    pub last_sample: f64,

    pub sidebar_pane: PaneState,
    pub inspector_pane: PaneState,

    pub appearance: Appearance,
    /// Showing the fixed demo machine (screenshots): sampling is off.
    pub demo: bool,
}

impl App {
    pub fn new() -> App {
        let saved = store::load();
        let mem = sys::memory::status();
        App {
            tab: Tab::Processes,
            snap: Snapshot::default(),
            worker: Some(Worker::start()),
            guards: Guards::new(),
            mem,
            mem_hist: VecDeque::new(),
            mem_shown: mem.used_frac(),
            cache_shown: if mem.total > 0 { mem.cache as f32 / mem.total as f32 } else { 0.0 },
            ram: RamState::default(),
            info: None,
            columns: saved.columns.clone(),
            column_menu: None,
            palette: None,
            locks: None,
            diag: None,
            drop_hover: false,
            finding: false,
            find_hover: None,
            pending_info: None,
            disk: DiskState { phase: DiskPhase::Idle, items: Vec::new(), drive: sys::cleanup::system_drive(), results: Vec::new() },
            appearance: saved.appearance.clone(),
            demo: false,
            groups: saved.groups,
            next_id: saved.next_id.max(1),
            selected_group: None,
            search: String::new(),
            search_edit: rune::TextEdit::new(""),
            table: TableState::new().multi_select(true),
            view_rows: Vec::new(),
            row_meta: Vec::new(),
            collapsed: HashSet::new(),
            active_pid: None,
            rename: None,
            confirm: None,
            toasts: Vec::new(),
            toast_seq: 0,
            alerts: saved.alerts.clone(),
            alert_watch: Default::default(),
            alert_roots: crate::sys::alerts::risky_roots(),
            alert_edit: rune::TextEdit::new(""),
            elevated: sys::elevate::is_elevated(),
            clock: 0.0,
            last_sample: -10.0,
            sidebar_pane: PaneState::new(248.0).range(180.0, 420.0),
            inspector_pane: PaneState::new(320.0).range(260.0, 480.0),
        }
    }

    pub fn persist(&self) {
        if self.demo {
            return; // never write the made-up demo machine over the user's saved state
        }
        let saved = Saved { groups: self.groups.clone(), next_id: self.next_id, columns: self.columns.clone(), appearance: self.appearance.clone(), alerts: self.alerts.clone() };
        if let Err(e) = store::save(&saved) {
            eprintln!("winkillswitch: could not save state: {e}");
        }
    }

    pub fn group(&self, id: u64) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }
    pub fn group_mut(&mut self, id: u64) -> Option<&mut Group> {
        self.groups.iter_mut().find(|g| g.id == id)
    }

    pub fn new_group(&mut self, name: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.groups.push(Group { id, name, members: Vec::new(), pause: None });
        self.selected_group = Some(id);
        self.persist();
        id
    }

    pub fn toast(&mut self, title: impl Into<String>, detail: Option<String>, danger: bool) {
        self.toast_seq += 1;
        self.toasts.push(Toast { id: self.toast_seq, title: title.into(), detail, danger, born: self.clock, pid: None });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    /// A toast about one process, with a Show button.
    pub fn toast_about(&mut self, pid: Pid, title: impl Into<String>, detail: String) {
        self.toast(title, Some(detail), false);
        if let Some(t) = self.toasts.last_mut() {
            t.pid = Some(pid);
        }
    }

    pub fn dismiss_toast(&mut self, id: u64) {
        self.toasts.retain(|t| t.id != id);
    }

    /// Drop toasts older than six seconds (twelve for one with a Show button).
    pub fn expire_toasts(&mut self) {
        let now = self.clock;
        self.toasts.retain(|t| now - t.born < if t.pid.is_some() { 12.0 } else { 6.0 });
    }

    pub fn send(&self, c: Cmd) {
        if let Some(w) = &self.worker {
            w.send(c);
        }
    }

    /// Ask for a fresh sample now (after an action that changed the process list).
    pub fn refresh_now(&mut self) {
        self.send(Cmd::Refresh);
    }

    pub fn column_shown(&self, id: &str) -> bool {
        id == "name" || if self.columns.is_empty() { crate::ui::columns::DEFAULT.contains(&id) } else { self.columns.iter().any(|c| c == id) }
    }

    pub fn toggle_column(&mut self, id: &str) {
        if self.columns.is_empty() {
            self.columns = crate::ui::columns::DEFAULT.iter().map(|c| c.to_string()).collect();
        }
        if self.columns.iter().any(|c| c == id) {
            self.columns.retain(|c| c != id);
        } else {
            self.columns.push(id.to_string());
        }
        self.persist();
    }

    /// Make `pid` the active row: clear the search so it is in the list, then select it.
    /// Returns false when the process is not in the current sample (yet).
    pub fn select_pid(&mut self, pid: Pid) -> bool {
        if !self.search.is_empty() {
            self.search.clear();
            self.search_edit = rune::TextEdit::new("");
            self.rebuild_rows();
        }
        if !self.view_rows.contains(&pid) {
            return false;
        }
        self.active_pid = Some(pid);
        self.table.select(pid.to_string());
        true
    }

    /// Open the "about this process" sheet and start inspecting it.
    pub fn open_info(&mut self, pid: Pid) {
        let Some(p) = self.snap.get(pid) else { return };
        let cmd = Cmd::Inspect { pid, name: p.name.clone(), exe: p.exe.clone(), admin: self.elevated };
        self.info = Some(InfoState { pid, detail: None });
        self.send(cmd);
    }

    /// Lowercase image names of everything running, for the disk cleaner's "close X first" checks.
    pub fn running_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.snap.procs.iter().map(|p| p.name.to_lowercase()).collect();
        v.sort();
        v.dedup();
        v
    }

    pub fn record_mem(&mut self, m: MemStatus) {
        self.mem = m;
        self.mem_hist.push_back(m.used_frac());
        while self.mem_hist.len() > 80 {
            self.mem_hist.pop_front();
        }
    }

    /// Recompute the table's visible rows from the snapshot, search and sort.
    pub fn rebuild_rows(&mut self) {
        if self.appearance.tree {
            let (rows, meta) = self.tree_rows();
            self.view_rows = rows;
            self.row_meta = meta;
        } else {
            self.view_rows = self.filtered_pids();
            self.row_meta.clear();
        }
    }

    /// The parent a process really has: one that is still running and started before it.
    /// Windows reuses PIDs, so a "parent" that started later is someone else.
    fn real_parent(&self, p: &crate::model::ProcInfo) -> Option<Pid> {
        let pp = p.parent?;
        if pp == p.pid {
            return None;
        }
        let parent = self.snap.get(pp)?;
        (parent.start <= p.start || parent.start == 0 || p.start == 0).then_some(pp)
    }

    /// Depth-first rows for the tree view, siblings in the table's sort order. With a search,
    /// matches show with their ancestors so they keep their place.
    pub fn tree_rows(&self) -> (Vec<Pid>, Vec<(u16, bool)>) {
        use std::collections::HashMap;
        let q = self.search.trim().to_lowercase();
        let matches = |p: &crate::model::ProcInfo| {
            q.is_empty()
                || p.name.to_lowercase().contains(&q)
                || p.pid.to_string().contains(&q)
                || p.exe.as_ref().map(|e| e.to_string_lossy().to_lowercase().contains(&q)).unwrap_or(false)
        };
        let mut children: HashMap<Pid, Vec<&crate::model::ProcInfo>> = HashMap::new();
        let mut roots: Vec<&crate::model::ProcInfo> = Vec::new();
        for p in &self.snap.procs {
            match self.real_parent(p) {
                Some(pp) => children.entry(pp).or_default().push(p),
                None => roots.push(p),
            }
        }
        // Which pids to show: matches and every ancestor of a match.
        let mut keep: HashSet<Pid> = HashSet::new();
        for p in self.snap.procs.iter().filter(|p| matches(p)) {
            let mut cur = Some(p.pid);
            let mut guard = 0;
            while let Some(pid) = cur {
                if !keep.insert(pid) || guard > 64 {
                    break;
                }
                guard += 1;
                cur = self.snap.get(pid).and_then(|x| self.real_parent(x));
            }
        }
        let (col, dir) = self.table.sort().unwrap_or(("cpu", rune_kit::SortDir::Descending));
        let sort = |v: &mut Vec<&crate::model::ProcInfo>| v.sort_by(|a, b| dir.apply(crate::ui::columns::cmp(a, b, col)));
        sort(&mut roots);
        for v in children.values_mut() {
            sort(v);
        }
        let (mut rows, mut meta) = (Vec::new(), Vec::new());
        let mut seen: HashSet<Pid> = HashSet::new();
        // (process, depth, shown): rows under a folded branch are walked but not shown, so they
        // count as reached and are not mistaken for orphans below.
        let mut stack: Vec<(&crate::model::ProcInfo, u16, bool)> = roots.into_iter().rev().map(|p| (p, 0, true)).collect();
        while let Some((p, depth, shown)) = stack.pop() {
            if !keep.contains(&p.pid) || !seen.insert(p.pid) {
                continue;
            }
            let kids: Vec<&crate::model::ProcInfo> = children.get(&p.pid).map(|v| v.iter().copied().filter(|c| keep.contains(&c.pid)).collect()).unwrap_or_default();
            if shown {
                rows.push(p.pid);
                meta.push((depth, !kids.is_empty()));
            }
            let open = shown && (!self.collapsed.contains(&p.pid) || !q.is_empty());
            for c in kids.into_iter().rev() {
                stack.push((c, depth.saturating_add(1), open));
            }
        }
        // Anything not reached (a parent cycle, which reused PIDs can produce) still shows, at
        // the top level, rather than vanishing.
        for p in &self.snap.procs {
            if keep.contains(&p.pid) && !seen.contains(&p.pid) {
                seen.insert(p.pid);
                rows.push(p.pid);
                meta.push((0, false));
            }
        }
        (rows, meta)
    }

    /// Processes passing the search filter, already sorted by the table's column.
    pub fn filtered_pids(&self) -> Vec<Pid> {
        let q = self.search.trim().to_lowercase();
        let mut rows: Vec<&crate::model::ProcInfo> = self
            .snap
            .procs
            .iter()
            .filter(|p| {
                q.is_empty()
                    || p.name.to_lowercase().contains(&q)
                    || p.pid.to_string().contains(&q)
                    || p.exe.as_ref().map(|e| e.to_string_lossy().to_lowercase().contains(&q)).unwrap_or(false)
            })
            .collect();
        let (col, dir) = self.table.sort().unwrap_or(("cpu", rune_kit::SortDir::Descending));
        rows.sort_by(|a, b| dir.apply(crate::ui::columns::cmp(a, b, col)));
        rows.into_iter().map(|p| p.pid).collect()
    }

    /// The pids currently checked in the table.
    pub fn selected_pids(&self) -> Vec<Pid> {
        self.table.selected().filter_map(|s| s.parse::<Pid>().ok()).collect::<HashSet<_>>().into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> App {
        let mut a = App::new();
        crate::demo::apply(&mut a); // demo mode: nothing is ever saved
        a
    }

    #[test]
    fn select_pid_clears_the_search_and_selects() {
        let mut a = demo();
        let pid = a.snap.procs.iter().find(|p| p.name == "Spotify.exe").unwrap().pid;
        a.search = "zzz-nothing".into();
        a.rebuild_rows();
        assert!(a.view_rows.is_empty());
        assert!(a.select_pid(pid));
        assert!(a.search.is_empty(), "the search is cleared so the row can show");
        assert_eq!(a.active_pid, Some(pid));
        assert!(a.table.is_selected(&pid.to_string()));
    }

    #[test]
    fn select_pid_rejects_unknown_processes() {
        let mut a = demo();
        assert!(!a.select_pid(4_000_000));
    }

    #[test]
    fn columns_toggle_and_reset() {
        let mut a = demo();
        assert!(a.column_shown("gpu") && !a.column_shown("cmd"));
        a.toggle_column("cmd");
        a.toggle_column("gpu");
        assert!(a.column_shown("cmd") && !a.column_shown("gpu"));
        assert!(a.column_shown("name"), "name always shows");
        a.columns.clear();
        assert!(a.column_shown("gpu"), "empty means the default set");
    }
}

#[cfg(test)]
mod tree_tests {
    use super::*;
    use crate::model::{ProcInfo, ProcKind, Snapshot};

    fn proc(pid: Pid, parent: Option<Pid>, start: u64, name: &str, cpu: f32) -> ProcInfo {
        ProcInfo {
            pid,
            parent,
            name: name.into(),
            exe: None,
            cpu,
            mem: 0,
            start,
            disk: 0,
            gpu: 0.0,
            gpu_engine: String::new(),
            status: "Running",
            kind: ProcKind::App,
            cmd: String::new(),
            session: Some(1),
            publisher: String::new(),
            description: name.into(),
        }
    }

    fn app(procs: Vec<ProcInfo>) -> App {
        let mut a = App::new();
        crate::demo::apply(&mut a);
        a.snap = Snapshot { procs, ..Default::default() };
        a.appearance.tree = true;
        a.search.clear();
        a.collapsed.clear();
        a.rebuild_rows();
        a
    }

    #[test]
    fn depth_first_with_depths() {
        let a = app(vec![proc(1, None, 10, "explorer.exe", 1.0), proc(2, Some(1), 20, "chrome.exe", 5.0), proc(3, Some(2), 30, "chrome-gpu.exe", 2.0), proc(4, None, 5, "svchost.exe", 0.1)]);
        assert_eq!(a.view_rows, vec![1, 2, 3, 4]);
        assert_eq!(a.row_meta, vec![(0, true), (1, true), (2, false), (0, false)]);
    }

    #[test]
    fn a_reused_parent_pid_is_not_a_parent() {
        // pid 7 started after its "child": the real parent is gone and 7 was reused.
        let a = app(vec![proc(7, None, 100, "newcomer.exe", 0.0), proc(8, Some(7), 50, "orphan.exe", 0.0)]);
        assert_eq!(a.row_meta, vec![(0, false), (0, false)]);
    }

    #[test]
    fn folding_hides_descendants_and_search_keeps_ancestors() {
        let mut a = app(vec![proc(1, None, 10, "explorer.exe", 1.0), proc(2, Some(1), 20, "chrome.exe", 5.0), proc(3, Some(2), 30, "gpu-helper.exe", 2.0)]);
        a.collapsed.insert(1);
        a.rebuild_rows();
        assert_eq!(a.view_rows, vec![1]);
        a.search = "gpu-helper".into();
        a.rebuild_rows();
        assert_eq!(a.view_rows, vec![1, 2, 3], "a match shows with its ancestors, even inside a folded branch");
    }

    #[test]
    fn survives_parent_cycles() {
        let a = app(vec![proc(1, Some(2), 0, "a.exe", 0.0), proc(2, Some(1), 0, "b.exe", 0.0)]);
        assert_eq!(a.view_rows.len(), 2, "both still show");
    }
}
