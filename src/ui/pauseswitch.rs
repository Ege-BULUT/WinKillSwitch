//! The PauseSwitch tab: every group with a pause toggle and its held-off entries.

use rune::{Cx, El, col, el, row, text};
use rune_kit::Switch;

use crate::ui::theme::Look;

use crate::app::App;
use crate::sys;

pub fn view(s: &App, _cx: &mut Cx<App>, l: &Look) -> El {
    let k = &l.kit;
    let c = &k.c;
    let mut body = col().key("pause-body").w_full().gap(12.0).max_w(640.0);

    body = body.child(
        text("PauseSwitch stops a group's apps from starting on their own: at sign in, as a service or from a scheduled task. Switch it off and every setting goes back exactly as it was.")
            .font(k.ty.small)
            .color(l.muted)
            .wrap(),
    );

    if !sys::autostart::AVAILABLE {
        body = body.child(
            col()
                .w_full()
                .px(12.0)
                .py(10.0)
                .radius(k.radius.md)
                .fill(c.warning.bg)
                .child(
                    text("Autostart control is not in this build yet. Turning a switch on records the intent but changes nothing on your system.")
                        .font(k.ty.small)
                        .color(c.warning.fg)
                        .wrap(),
                ),
        );
    }

    if s.groups.is_empty() {
        body = body.child(text("Create a group first.").font(k.ty.ui).color(l.muted));
    }

    for g in &s.groups {
        let id = g.id;
        let paused = g.is_paused();
        let toggle = Switch::new(paused)
            .on_toggle(move |s: &mut App| {
                let on = s.group(id).map(|g| g.is_paused()).unwrap_or(false);
                if on {
                    crate::actions::resume_group_now(s, id);
                } else {
                    crate::actions::pause_group(s, id);
                }
            })
            .build(k);

        let mut card = col()
            .key(format!("pause-{id}"))
            .w_full()
            .padding(14.0)
            .gap(8.0)
            .radius(k.radius.md)
            .fill(l.card)
            .border(1.0, l.line);

        card = card.child(
            row()
                .w_full()
                .center_y()
                .gap(10.0)
                .child(
                    col()
                        .grow(1.0)
                        .gap(2.0)
                        .child(text(g.name.clone()).font(k.ty.ui_medium).color(l.text))
                        .child(
                            text(format!(
                                "{} process(es){}",
                                g.members.len(),
                                if paused { " \u{2022} paused" } else { "" }
                            ))
                            .font(k.ty.small)
                            .color(l.muted),
                        ),
                )
                .child(toggle),
        );

        if let Some(rec) = &g.pause {
            if rec.changes.is_empty() && rec.errors.is_empty() {
                card = card.child(text("No autostart entries were changed.").font(k.ty.small).color(l.muted));
            }
            for err in &rec.errors {
                card = card.child(
                    row()
                        .gap(6.0)
                        .center_y()
                        .child(el().size(5.0, 5.0).radius(2.5).fill(c.danger.icon))
                        .child(text(err.clone()).font(k.ty.text_small).color(l.muted).wrap().grow(1.0).min_w(0.0)),
                );
            }
        }

        body = body.child(card);
    }

    let inner = if s.pause_view == 1 { crate::ui::startup::view(s, l).max_w(900.0) } else { body };
    col()
        .w_full()
        .h_full()
        .min_h(0.0)
        .padding(28.0)
        .gap(18.0)
        .items_center()
        .child(crate::ui::startup::switcher(s, l))
        .child(crate::ui::startup::scroll(col().key("pause-scroll").w_full().items_center().child(inner)))
}
