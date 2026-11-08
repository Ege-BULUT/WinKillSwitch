//! The quick palette: one box to reach processes, groups and commands by typing. Ctrl+K inside
//! the app, Ctrl+Shift+K from anywhere. Up and Down move, Enter runs, Escape closes.

use rune::{El, Icon, Key, TextEdit, col, el, icon, row, text};
use rune_kit::TextInput;

use crate::app::{App, DiskPhase, Tab};
use crate::model::{Pid, ThemeMode};
use crate::ui::palettes;
use crate::ui::theme::Look;
use crate::ui::{fmt, icons};
use crate::worker::Cmd;

pub struct PaletteState {
    pub edit: TextEdit,
    pub cursor: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Tab(Tab),
    CleanMemory,
    AnalyzeDisk,
    Theme(&'static str),
    Mode(ThemeMode),
    NewGroup,
    Locks,
    Diagnose,
    Admin,
    Process(Pid),
    Group(u64),
}

#[derive(Clone, Debug)]
pub struct Item {
    pub title: String,
    pub detail: String,
    pub kind: &'static str,
    pub action: Action,
}

/// 3 for a prefix match, 2 for a word start, 1 for anywhere, 0 for none.
fn score(hay: &str, q: &str) -> u8 {
    let h = hay.to_lowercase();
    if h.starts_with(q) {
        3
    } else if h.split([' ', '.', '-', '_', '\\']).any(|w| w.starts_with(q)) {
        2
    } else if h.contains(q) {
        1
    } else {
        0
    }
}

fn commands(s: &App) -> Vec<Item> {
    let cmd = |title: &str, detail: &str, action| Item { title: title.into(), detail: detail.into(), kind: "Command", action };
    let mut v = vec![
        cmd("Why is my PC slow?", "A few seconds of watching, then a plain answer", Action::Diagnose),
        cmd("Clean memory", "Trim working sets and clear the standby cache", Action::CleanMemory),
        cmd("Analyze disk", "Find temporary files, caches and leftovers", Action::AnalyzeDisk),
        cmd("Go to Processes", "", Action::Tab(Tab::Processes)),
        cmd("Go to Clean", "", Action::Tab(Tab::Clean)),
        cmd("Go to PauseSwitch", "", Action::Tab(Tab::PauseSwitch)),
        cmd("Go to Settings", "", Action::Tab(Tab::Settings)),
        cmd("New group", "", Action::NewGroup),
        cmd("Who is using a file?", "Find what keeps a file or folder locked", Action::Locks),
        cmd("Light mode", "Appearance", Action::Mode(ThemeMode::Light)),
        cmd("Dark mode", "Appearance", Action::Mode(ThemeMode::Dark)),
        cmd("Follow system light or dark", "Appearance", Action::Mode(ThemeMode::System)),
    ];
    for p in palettes::ALL.iter() {
        v.push(cmd(&format!("Theme: {}", p.name), "Appearance", Action::Theme(p.id)));
    }
    if !s.elevated {
        v.push(cmd("Restart as administrator", "Needed for system services and caches", Action::Admin));
    }
    v
}

pub fn items(s: &App) -> Vec<Item> {
    let q = s.palette.as_ref().map(|p| p.edit.text.trim().to_lowercase()).unwrap_or_default();
    if q.is_empty() {
        return commands(s).into_iter().take(7).collect();
    }
    let mut scored: Vec<(u8, u8, Item)> = Vec::new();
    for c in commands(s) {
        let sc = score(&c.title, &q).max(score(&c.detail, &q).min(1));
        if sc > 0 {
            scored.push((sc, 2, c));
        }
    }
    for g in &s.groups {
        let sc = score(&g.name, &q);
        if sc > 0 {
            scored.push((sc, 1, Item { title: g.name.clone(), detail: format!("{} processes", g.members.len()), kind: "Group", action: Action::Group(g.id) }));
        }
    }
    // Processes, one entry per name (the busiest instance), ranked by match then CPU.
    let mut seen: Vec<String> = Vec::new();
    let mut procs: Vec<&crate::model::ProcInfo> = s.snap.procs.iter().collect();
    procs.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));
    for p in procs {
        let key = p.name.to_lowercase();
        if seen.contains(&key) {
            continue;
        }
        let sc = score(&p.name, &q).max(score(&p.description, &q));
        if sc > 0 {
            seen.push(key);
            let count = s.snap.procs.iter().filter(|x| x.name == p.name).count();
            let detail = if count > 1 { format!("{} · {} running · {}", p.description, count, fmt::bytes(p.mem)) } else { format!("{} · PID {} · {}", p.description, p.pid, fmt::bytes(p.mem)) };
            scored.push((sc, 0, Item { title: p.name.clone(), detail, kind: "Process", action: Action::Process(p.pid) }));
        }
    }
    // Better match first; on a tie commands, then groups, then processes.
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    scored.into_iter().map(|x| x.2).take(9).collect()
}

pub fn open(s: &mut App) {
    let mut edit = TextEdit::new("");
    edit.focused = true;
    s.palette = Some(PaletteState { edit, cursor: 0 });
}

pub fn run(s: &mut App, a: Action, tx: &mut rune::Tx) {
    s.palette = None;
    match a {
        Action::Tab(t) => s.tab = t,
        Action::CleanMemory => {
            s.tab = Tab::Clean;
            if !s.ram.running {
                s.ram.running = true;
                s.send(Cmd::CleanRam { elevated: s.elevated });
            }
        }
        Action::AnalyzeDisk => {
            s.tab = Tab::Clean;
            let running = s.running_names();
            s.disk.phase = DiskPhase::Scanning { done: 0, total: 1, current: "your drive".into(), files: 0, bytes: 0 };
            s.send(Cmd::ScanDisk { running, elevated: s.elevated });
        }
        Action::Theme(id) => {
            s.appearance.palette = id.to_string();
            s.persist();
        }
        Action::Mode(m) => {
            s.appearance.mode = m;
            s.persist();
        }
        Action::NewGroup => {
            let id = s.new_group("New group".into());
            let mut edit = TextEdit::new("New group");
            edit.anchor = 0;
            edit.focused = true;
            s.rename = Some((id, edit));
        }
        Action::Locks => crate::ui::locks::open(s),
        Action::Diagnose => crate::ui::diagnose::start(s),
        Action::Admin => {
            if crate::sys::elevate::relaunch_as_admin("").is_ok() {
                std::process::exit(0);
            }
        }
        Action::Process(pid) => {
            s.tab = Tab::Processes;
            if s.select_pid(pid) {
                s.table.reveal(pid.to_string(), tx);
                s.open_info(pid);
            }
        }
        Action::Group(id) => {
            s.tab = Tab::Processes;
            s.selected_group = Some(id);
        }
    }
}

fn kind_icon(kind: &str) -> &'static Icon {
    match kind {
        "Process" => &icons::PROCESSES,
        "Group" => &icons::CHIP,
        _ => &icons::CLEAN,
    }
}

pub fn overlay(s: &App, l: &Look, layer: &mut Vec<El>) {
    let Some(p) = &s.palette else { return };
    let k = &l.kit;
    let list = items(s);
    let cursor = p.cursor.min(list.len().saturating_sub(1));

    let input = TextInput::new(&p.edit)
        .placeholder("Type a process, group or command")
        .leading_icon(k.glyph.search)
        .large()
        .plain()
        .on_edit(|s: &mut App| &mut s.palette.as_mut().expect("palette open").edit)
        .on_change(|s: &mut App, _| {
            if let Some(p) = s.palette.as_mut() {
                p.cursor = 0;
            }
        })
        .on_key(|s: &mut App, ev, tx| {
            let n = items(s).len();
            let Some(p) = s.palette.as_mut() else { return false };
            match ev.key {
                Key::Down => p.cursor = if n == 0 { 0 } else { (p.cursor + 1) % n },
                Key::Up => p.cursor = if n == 0 { 0 } else { (p.cursor + n - 1) % n },
                Key::Escape => s.palette = None,
                Key::Enter => {
                    let c = p.cursor;
                    if let Some(it) = items(s).into_iter().nth(c) {
                        run(s, it.action, tx);
                    }
                }
                _ => return false,
            }
            true
        })
        .build(k)
        .key("palette-input");

    let mut rows = col().key("palette-rows").w_full().gap(2.0);
    if list.is_empty() {
        rows = rows.child(text("Nothing matches. Try a process name, a group, or \u{201c}clean\u{201d}.").font(k.ty.small).color(l.muted).padding(14.0));
    }
    for (i, it) in list.iter().enumerate() {
        let on = i == cursor;
        let action = it.action.clone();
        rows = rows.child(
            row()
                .key(format!("pal-row-{i}"))
                .w_full()
                .h(48.0)
                .px(12.0)
                .gap(12.0)
                .center_y()
                .radius(12.0)
                .fill(if on { l.accent.alpha(0.2) } else { l.text.alpha(0.0) })
                .hover(l.card_hover)
                .cursor(rune::Cursor::Pointer)
                .role("option")
                .checked(on)
                .label(it.title.clone())
                .on_press_tx(move |s: &mut App, tx| run(s, action.clone(), tx))
                .child(row().size(30.0, 30.0).radius(9.0).center().fill(l.accent.alpha(if on { 0.3 } else { 0.12 })).child(icon(kind_icon(it.kind), 15.0).color(l.accent)))
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(1.0)
                        .child(text(it.title.clone()).font(k.ty.ui_medium).color(l.text).truncate())
                        .children((!it.detail.is_empty()).then(|| text(it.detail.clone()).font(k.ty.text_xsmall).color(l.muted).truncate())),
                )
                .child(text(if on { "Enter".to_string() } else { it.kind.to_string() }).font(k.ty.text_xsmall_medium).color(if on { l.accent } else { l.muted })),
        );
    }

    let hint = row()
        .w_full()
        .gap(16.0)
        .px(6.0)
        .child(text("\u{2191}\u{2193} move").font(k.ty.text_xsmall).color(l.muted))
        .child(text("Enter open").font(k.ty.text_xsmall).color(l.muted))
        .child(text("Esc close").font(k.ty.text_xsmall).color(l.muted))
        .child(el().grow(1.0))
        .child(text(format!("{} from anywhere", crate::sys::hotkey::LABEL)).font(k.ty.text_xsmall).color(l.muted));

    let card = col()
        .key("palette-card")
        .w(640.0)
        .max_w(680.0)
        .padding(10.0)
        .gap(8.0)
        .radius(22.0)
        .fill(crate::ui::theme::color(l.v.surface).alpha(if l.dark { 0.92 } else { 0.95 }))
        .border(1.0, l.text.alpha(if l.dark { 0.16 } else { 0.4 }))
        .shadow(24.0, 70.0, rune::Color::BLACK.alpha(0.45))
        .child(input)
        .child(el().w_full().h(1.0).fill(l.line))
        .child(rows)
        .child(hint)
        .modal(true, |s: &mut App| s.palette = None)
        .modal_click_outside(true);

    layer.push(el().key("palette-scrim").absolute(0.0, 0.0).w_full().h_full().fill(rune::Color::BLACK.alpha(if l.dark { 0.45 } else { 0.25 })).a11y_hidden());
    layer.push(col().key("palette-layer").absolute(0.0, 0.0).w_full().h_full().pt(90.0).px(16.0).items_center().child(card));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_query(q: &str) -> Vec<Item> {
        let mut a = App::new();
        crate::demo::apply(&mut a);
        open(&mut a);
        a.palette.as_mut().unwrap().edit = TextEdit::new(q);
        items(&a)
    }

    #[test]
    fn empty_query_lists_commands() {
        let v = with_query("");
        assert!(!v.is_empty() && v.iter().all(|i| i.kind == "Command"));
    }

    #[test]
    fn finds_a_process_by_prefix() {
        let v = with_query("spot");
        assert_eq!(v.first().map(|i| i.title.as_str()), Some("Spotify.exe"));
        assert!(matches!(v[0].action, Action::Process(_)));
    }

    #[test]
    fn one_entry_per_process_name() {
        let v = with_query("chrome");
        assert_eq!(v.iter().filter(|i| i.title == "chrome.exe").count(), 1);
        assert!(v.iter().find(|i| i.title == "chrome.exe").unwrap().detail.contains("running"), "chrome has several instances");
    }

    #[test]
    fn commands_and_groups_match() {
        assert_eq!(with_query("clean").first().map(|i| i.title.as_str()), Some("Clean memory"));
        assert!(with_query("brow").iter().any(|i| i.kind == "Group" && i.title == "Browsers"));
        assert!(with_query("tokyo").iter().any(|i| i.action == Action::Theme("tokyo-night")));
    }
}
