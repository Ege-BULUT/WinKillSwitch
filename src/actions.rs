//! What buttons do: assemble the process list for an action, show a confirmation, and on
//! confirm hand off to the `sys` layer. Killing, related-resolution, the respawn guard and
//! autostart changes are not implemented in this build; those paths report that plainly
//! instead of doing anything, so nothing on the machine is touched.

use crate::app::{App, Confirm, ConfirmAction};
use crate::model::{Member, Pid};
use crate::sys;

fn proc_line(app: &App, pid: Pid) -> String {
    match app.snap.get(pid) {
        Some(p) => format!("{}  ·  pid {}  ·  {}", p.name, p.pid, crate::ui::fmt::bytes(p.mem)),
        None => format!("pid {pid}"),
    }
}

/// Ask before ending a single process.
pub fn kill_one(app: &mut App, pid: Pid) {
    let name = app.snap.get(pid).map(|p| p.name.clone()).unwrap_or_else(|| format!("pid {pid}"));
    app.confirm = Some(Confirm {
        title: format!("End {name}?"),
        message: "This process will be asked to stop. Unsaved work in it may be lost.".into(),
        items: vec![proc_line(app, pid)],
        ok_label: "End process".into(),
        danger: true,
        action: ConfirmAction::Kill { pids: vec![pid], label: name, guard_members: Vec::new() },
    });
}

/// Ask before ending a process and everything related to it.
pub fn kill_all_related(app: &mut App, pid: Pid) {
    let name = app.snap.get(pid).map(|p| p.name.clone()).unwrap_or_else(|| format!("pid {pid}"));
    let rel = sys::related::related(&app.snap, pid);
    let pids = if rel.pids.is_empty() { vec![pid] } else { rel.pids.clone() };
    let items: Vec<String> = pids.iter().map(|&p| proc_line(app, p)).collect();
    let extra = if sys::related::AVAILABLE {
        String::new()
    } else {
        "\n\nNote: finding related processes is not available in this build, so only the one process is listed.".into()
    };
    app.confirm = Some(Confirm {
        title: format!("End {name} and everything related?"),
        message: format!(
            "Every process listed below will be ended, and winkillswitch will keep ending {name} if it restarts on its own, until you start it yourself again.{extra}"
        ),
        items,
        ok_label: "End all related".into(),
        danger: true,
        action: ConfirmAction::Kill { pids, label: name, guard_members: rel.members },
    });
}

/// Ask before ending every running member of a group.
pub fn kill_group(app: &mut App, id: u64) {
    let Some(g) = app.group(id) else { return };
    let name = g.name.clone();
    let members = g.members.clone();
    let pids = sys::related::running_members(&app.snap, &members);
    if pids.is_empty() {
        app.toast(format!("Nothing running in \u{201c}{name}\u{201d}"), None, false);
        return;
    }
    let items: Vec<String> = pids.iter().map(|&p| proc_line(app, p)).collect();
    app.confirm = Some(Confirm {
        title: format!("Kill group \u{201c}{name}\u{201d}?"),
        message: "Every running process in this group will be ended.".into(),
        items,
        ok_label: "Kill group".into(),
        danger: true,
        action: ConfirmAction::Kill { pids, label: name, guard_members: members },
    });
}

/// Run a confirmed action.
pub fn run_confirmed(app: &mut App, action: ConfirmAction) {
    match action {
        ConfirmAction::Kill { pids, label, guard_members } => {
            let report = sys::kill::end_all(&app.snap, &pids);
            if !guard_members.is_empty() {
                let gid = app.toast_seq.wrapping_add(10_000);
                app.guards.add(
                    sys::guard::Guard { id: gid, label: label.clone(), members: guard_members, sticky: false },
                    &app.snap,
                );
            }
            if report.ended.is_empty() && !report.failed.is_empty() {
                let reason = report.failed.first().map(|f| f.2.clone()).unwrap_or_default();
                app.toast(format!("Could not end {label}"), Some(reason), true);
            } else {
                app.toast(
                    format!("Ended {} process(es)", report.ended.len()),
                    (!report.failed.is_empty()).then(|| format!("{} could not be ended", report.failed.len())),
                    false,
                );
            }
            app.refresh_now();
        }
        ConfirmAction::DeleteGroup { id } => {
            if let Some(g) = app.group(id) {
                if g.is_paused() {
                    app.toast("Turn PauseSwitch off before deleting", None, true);
                    return;
                }
            }
            app.groups.retain(|g| g.id != id);
            if app.selected_group == Some(id) {
                app.selected_group = None;
            }
            app.guards.remove(id);
            app.persist();
        }
        ConfirmAction::PauseGroup { id } => pause_group_now(app, id),
        ConfirmAction::ResumeGroup { id } => resume_group_now(app, id),
    }
}

/// Ask before turning PauseSwitch on for a group.
pub fn pause_group(app: &mut App, id: u64) {
    let Some(g) = app.group(id) else { return };
    let name = g.name.clone();
    let root = g.members.iter().find_map(|m| m.exe.as_ref()).map(std::path::PathBuf::from);
    let entries = sys::autostart::discover(&g.members, root.as_deref());
    let items: Vec<String> = if entries.is_empty() {
        vec!["No autostart entries were found for this group.".into()]
    } else {
        entries.iter().map(|e| format!("{}  ·  {}", e.label, if e.enabled { "on" } else { "off" })).collect()
    };
    let note = if sys::autostart::AVAILABLE {
        "These will be turned off. You can turn them back on by switching PauseSwitch off, which restores each to exactly how it was."
    } else {
        "Autostart control is not available in this build, so nothing will actually change."
    };
    app.confirm = Some(Confirm {
        title: format!("Turn on PauseSwitch for \u{201c}{name}\u{201d}?"),
        message: note.into(),
        items,
        ok_label: "Turn on".into(),
        danger: false,
        action: ConfirmAction::PauseGroup { id },
    });
}

fn pause_group_now(app: &mut App, id: u64) {
    let Some(g) = app.group(id) else { return };
    let root = g.members.iter().find_map(|m| m.exe.as_ref()).map(std::path::PathBuf::from);
    let entries = sys::autostart::discover(&g.members, root.as_deref());
    let (changes, errors) = sys::autostart::apply(&entries);
    if let Some(g) = app.group_mut(id) {
        g.pause = Some(crate::model::PauseRecord { applied_at: crate::model::now_secs(), changes, errors: errors.clone() });
    }
    app.persist();
    if sys::autostart::AVAILABLE {
        app.toast(format!("PauseSwitch on"), None, false);
    } else {
        app.toast("PauseSwitch recorded", Some("Autostart control is not available in this build.".into()), false);
    }
}

/// Turn PauseSwitch off and restore the original autostart settings.
pub fn resume_group_now(app: &mut App, id: u64) {
    let record = app.group(id).and_then(|g| g.pause.clone());
    if let Some(rec) = record {
        let errors = sys::autostart::revert(&rec.changes);
        if errors.is_empty() {
            app.toast("PauseSwitch off", Some("Autostart settings restored.".into()), false);
        } else {
            app.toast("PauseSwitch off", Some(format!("{} entr(ies) could not be restored", errors.len())), true);
        }
    }
    if let Some(g) = app.group_mut(id) {
        g.pause = None;
    }
    app.persist();
}

// --- Group membership editing ---

pub fn add_selected_to_group(app: &mut App, id: u64) {
    let pids = app.selected_pids();
    let members: Vec<Member> = pids.iter().filter_map(|&p| app.snap.get(p)).map(Member::from_proc).collect();
    add_members(app, id, members);
}

pub fn add_pid_to_group(app: &mut App, id: u64, pid: Pid) {
    if let Some(p) = app.snap.get(pid) {
        let m = Member::from_proc(p);
        add_members(app, id, vec![m]);
    }
}

pub fn add_all_related_to_group(app: &mut App, id: u64) {
    let Some(pid) = app.active_pid else {
        app.toast("Select a process first", None, false);
        return;
    };
    let rel = sys::related::related(&app.snap, pid);
    let members = if rel.members.is_empty() {
        app.snap.get(pid).map(Member::from_proc).into_iter().collect()
    } else {
        rel.members
    };
    add_members(app, id, members);
}

fn add_members(app: &mut App, id: u64, members: Vec<Member>) {
    let mut added = 0;
    if let Some(g) = app.group_mut(id) {
        for m in members {
            if g.add(m) {
                added += 1;
            }
        }
    }
    app.persist();
    app.toast(format!("Added {added} to group"), None, false);
}

pub fn remove_member(app: &mut App, id: u64, key: String) {
    if let Some(g) = app.group_mut(id) {
        g.remove_key(&key);
    }
    app.persist();
}

/// Remove the active process (and everything related) from the group.
pub fn remove_all_related_from_group(app: &mut App, id: u64) {
    let Some(pid) = app.active_pid else { return };
    let rel = sys::related::related(&app.snap, pid);
    let mut keys: Vec<String> = rel.members.iter().map(Member::key).collect();
    if keys.is_empty() {
        if let Some(p) = app.snap.get(pid) {
            keys.push(Member::from_proc(p).key());
        }
    }
    if let Some(g) = app.group_mut(id) {
        g.members.retain(|m| !keys.contains(&m.key()));
    }
    app.persist();
}

pub fn confirm_delete_group(app: &mut App, id: u64) {
    let Some(g) = app.group(id) else { return };
    app.confirm = Some(Confirm {
        title: format!("Delete \u{201c}{}\u{201d}?", g.name),
        message: "The group and its member list are removed. Running processes are not affected.".into(),
        items: Vec::new(),
        ok_label: "Delete".into(),
        danger: true,
        action: ConfirmAction::DeleteGroup { id },
    });
}

/// The find-by-window drag ended over a window.
pub fn found_window(app: &mut App, t: crate::sys::finder::Target, tx: &mut rune::Tx) {
    if t.pid == std::process::id() {
        app.toast("That window is WinKillSwitch itself", None, false);
        return;
    }
    if app.select_pid(t.pid) {
        app.table.reveal(t.pid.to_string(), tx);
        let name = app.snap.get(t.pid).map(|p| p.name.clone()).unwrap_or_default();
        app.toast(format!("Found {name}"), (!t.title.is_empty()).then(|| t.title.clone()), false);
    } else {
        app.toast(format!("PID {} is not in the list yet", t.pid), Some("Try again in a moment.".into()), false);
        app.refresh_now();
    }
}
