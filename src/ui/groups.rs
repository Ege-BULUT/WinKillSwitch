//! The groups sidebar and the group-detail inspector, both drawn straight onto glass.

use rune::{Cx, El, ScrollSpec, TextEdit, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Switch, TextInput};

use crate::app::App;
use crate::ui::icons;
use crate::ui::theme::Look;

pub fn sidebar(s: &App, l: &Look) -> El {
    let k = &l.kit;
    let brand = row()
        .w_full()
        .gap(10.0)
        .center_y()
        .child(
            el().size(28.0, 28.0)
                .radius(9.0)
                .linear_gradient(2.4, vec![(0.0, l.accent), (1.0, l.accent.mix(l.text, 0.35))])
                .center()
                .child(rune::icon(&icons::SKULL, 16.0).color(l.on_accent)),
        )
        .child(
            col()
                .gap(0.0)
                .child(text("WinKillSwitch").font(k.ty.ui_medium).color(l.text))
                .child(text(if s.elevated { "Administrator" } else { "Standard user" }).font(k.ty.text_xsmall).color(l.muted)),
        );

    let new_btn = IconButton::new(&icons::PLUS)
        .variant(IconButtonVariant::GhostMuted)
        .label("New group")
        .on_press(|s: &mut App| {
            let id = s.new_group("New group".into());
            let mut edit = TextEdit::new("New group");
            edit.anchor = 0;
            edit.focused = true;
            s.rename = Some((id, edit));
        })
        .build(k)
        .key("new-group");

    let header = row()
        .w_full()
        .center_y()
        .justify_between()
        .child(text(format!("GROUPS · {}", s.groups.len())).font(k.ty.text_xsmall_medium).color(l.muted))
        .child(new_btn);

    let mut list = col().key("group-list").w_full().gap(3.0);
    if s.groups.is_empty() {
        list = list.child(
            text("No groups yet. Press + to create one, then add rows from Processes.")
                .font(k.ty.small)
                .color(l.muted)
                .wrap(),
        );
    }
    for g in &s.groups {
        let id = g.id;
        let on = s.selected_group == Some(id);
        let renaming = s.rename.as_ref().filter(|(rid, _)| *rid == id);
        let label: El = match renaming {
            Some((_, edit)) => TextInput::new(edit)
                .plain()
                .on_edit(|s: &mut App| &mut s.rename.as_mut().expect("renaming").1)
                .on_submit(|s: &mut App, _| commit_rename(s))
                .on_key(|s: &mut App, ev, _| {
                    if matches!(ev.key, rune::Key::Escape) {
                        s.rename = None;
                        return true;
                    }
                    false
                })
                .build(k)
                .grow(1.0),
            None => text(g.name.clone()).font(k.ty.ui).color(l.text).truncate().grow(1.0),
        };
        let running = g.members.iter().filter(|m| s.snap.procs.iter().any(|p| m.matches(p))).count();
        list = list.child(
            row()
                .key(format!("group-{id}"))
                .w_full()
                .h(38.0)
                .px(10.0)
                .gap(10.0)
                .center_y()
                .radius(10.0)
                .fill(if on { l.accent.alpha(if l.dark { 0.24 } else { 0.18 }) } else { l.card.alpha(0.0) })
                .hover(l.card_hover)
                .cursor(rune::Cursor::Pointer)
                .role("option")
                .label(g.name.clone())
                .checked(on)
                .on_press(move |s: &mut App| s.selected_group = if s.selected_group == Some(id) { None } else { Some(id) })
                .on_double_press(move |s: &mut App| start_rename(s, id))
                .child(el().size(8.0, 8.0).radius(4.0).fill(if g.is_paused() { k.c.warning.icon } else if running > 0 { k.c.success.icon } else { l.muted.alpha(0.5) }))
                .child(label)
                .child(text(format!("{}", g.members.len())).font(k.ty.text_small).color(l.muted)),
        );
    }

    col()
        .w_full()
        .h_full()
        .padding(16.0)
        .gap(18.0)
        .child(brand)
        .child(col().w_full().gap(8.0).grow(1.0).min_h(0.0).child(header).child(list.scroll(ScrollSpec { stick_bottom: false }).grow(1.0).min_h(0.0)))
        .child(if s.elevated {
            el()
        } else {
            Button::new("Restart as administrator")
                .variant(ButtonVariant::Ghost)
                .control_size(ControlSize::Small)
                .on_press(|_s: &mut App| {
                    if crate::sys::elevate::relaunch_as_admin("").is_ok() {
                        std::process::exit(0);
                    }
                })
                .build(k)
                .key("elevate")
        })
}

fn start_rename(s: &mut App, id: u64) {
    let name = s.group(id).map(|g| g.name.clone()).unwrap_or_default();
    let mut edit = TextEdit::new(name);
    edit.anchor = 0;
    edit.focused = true;
    s.rename = Some((id, edit));
}

fn commit_rename(s: &mut App) {
    if let Some((rid, edit)) = s.rename.take() {
        let name = edit.text.trim().to_string();
        if !name.is_empty() {
            if let Some(g) = s.group_mut(rid) {
                g.name = name;
            }
            s.persist();
        }
    }
}

/// The right-hand inspector: details of the selected group.
pub fn inspector(s: &App, _cx: &mut Cx<App>, l: &Look) -> Option<El> {
    let id = s.selected_group?;
    let g = s.group(id)?;
    let k = &l.kit;

    let header = row()
        .w_full()
        .center_y()
        .gap(6.0)
        .child(text(g.name.clone()).font(k.ty.body_medium).color(l.text).truncate().grow(1.0))
        .child(
            IconButton::new(&icons::RENAME)
                .variant(IconButtonVariant::GhostMuted)
                .label("Rename")
                .on_press(move |s: &mut App| start_rename(s, id))
                .build(k)
                .key("insp-rename"),
        )
        .child(
            IconButton::new(&icons::TRASH)
                .variant(IconButtonVariant::GhostMuted)
                .label("Delete group")
                .on_press(move |s: &mut App| crate::actions::confirm_delete_group(s, id))
                .build(k)
                .key("insp-delete"),
        );

    let mut list = col().key("members").w_full().gap(4.0);
    if g.members.is_empty() {
        list = list.child(
            text("No processes yet. Select rows in Processes and press \u{201c}Add to group\u{201d}.")
                .font(k.ty.small)
                .color(l.muted)
                .wrap(),
        );
    }
    for m in &g.members {
        let key = m.key();
        let running = s.snap.procs.iter().any(|p| m.matches(p));
        list = list.child(
            row()
                .key(format!("mem-{key}"))
                .w_full()
                .h(34.0)
                .px(10.0)
                .gap(8.0)
                .center_y()
                .radius(9.0)
                .fill(l.card)
                .child(el().size(7.0, 7.0).radius(3.5).fill(if running { k.c.success.icon } else { l.muted.alpha(0.5) }))
                .child(text(m.name.clone()).font(k.ty.ui).color(l.text).truncate().grow(1.0))
                .child(
                    IconButton::new(&icons::UNLINK)
                        .variant(IconButtonVariant::GhostMuted)
                        .label("Remove from group")
                        .on_press({
                            let key = key.clone();
                            move |s: &mut App| crate::actions::remove_member(s, id, key.clone())
                        })
                        .build(k)
                        .key(format!("rm-{key}")),
                ),
        );
    }

    let rel = |label: &str, ic, add: bool, key: &str| {
        Button::new(label.to_string())
            .variant(ButtonVariant::Ghost)
            .control_size(ControlSize::Small)
            .icon(ic)
            .disabled(s.active_pid.is_none())
            .on_press(move |s: &mut App| {
                if add {
                    crate::actions::add_all_related_to_group(s, id)
                } else {
                    crate::actions::remove_all_related_from_group(s, id)
                }
            })
            .build(k)
            .key(key.to_string())
    };

    let paused = g.is_paused();
    let pause = row()
        .w_full()
        .gap(12.0)
        .center_y()
        .padding(12.0)
        .radius(12.0)
        .fill(l.card)
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(2.0)
                .child(text("PauseSwitch").font(k.ty.ui_medium).color(l.text))
                .child(
                    text(if paused { "Autostart is paused for this group." } else { "Keep this group from starting on its own." })
                        .font(k.ty.small)
                        .color(l.muted)
                        .wrap(),
                ),
        )
        .child(
            Switch::new(paused)
                .label("PauseSwitch")
                .hide_label()
                .on_toggle(move |s: &mut App| {
                    let on = s.group(id).map(|g| g.is_paused()).unwrap_or(false);
                    if on { crate::actions::resume_group_now(s, id) } else { crate::actions::pause_group(s, id) }
                })
                .build(k),
        );
    let kill = Button::new("End group")
        .variant(ButtonVariant::Danger)
        .control_size(ControlSize::Regular)
        .icon(&icons::SKULL)
        .disabled(g.members.is_empty())
        .on_press(move |s: &mut App| crate::actions::kill_group(s, id))
        .build(k)
        .key("kill-group");

    Some(
        col()
            .key("inspector")
            .w_full()
            .h_full()
            .padding(16.0)
            .gap(14.0)
            .child(header)
            .child(row().w_full().gap(4.0).flex_wrap().child(rel("Add related", &icons::LINK, true, "add-rel")).child(rel("Remove related", &icons::UNLINK, false, "rm-rel")))
            .child(list.scroll(ScrollSpec { stick_bottom: false }).grow(1.0).min_h(0.0))
            .child(pause)
            .child(kill),
    )
}
