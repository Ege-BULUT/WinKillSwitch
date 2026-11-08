//! The confirmation dialog and the toast stack, both mounted into the overlay layer.

use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Dialog, Kit, Toast, ToastRegion};

use crate::app::App;

/// Append the confirmation dialog (if any) to `layer`.
pub fn confirm(s: &App, k: &Kit, layer: &mut Vec<El>) {
    let Some(cf) = &s.confirm else { return };
    let c = &k.c;

    let mut items = col().key("confirm-items").w_full().gap(3.0);
    for line in &cf.items {
        items = items.child(text(line.clone()).font(k.ty.text_small).color(c.text_muted).wrap());
    }
    let items = items.scroll(ScrollSpec { stick_bottom: false }).max_h(240.0).min_h(0.0);

    let action = cf.action.clone();
    let ok_variant = if cf.danger { ButtonVariant::Danger } else { ButtonVariant::Submit };
    let ok = Button::new(cf.ok_label.clone())
        .variant(ok_variant)
        .control_size(ControlSize::Regular)
        .on_press(move |s: &mut App| {
            s.confirm = None;
            crate::actions::run_confirmed(s, action.clone());
        })
        .build(k)
        .key("confirm-ok");
    let cancel = Button::new("Cancel")
        .variant(ButtonVariant::Ghost)
        .control_size(ControlSize::Regular)
        .on_press(|s: &mut App| s.confirm = None)
        .build(k)
        .key("confirm-cancel");

    let body = col()
        .w_full()
        .gap(12.0)
        .child(text(cf.message.clone()).font(k.ty.ui).color(c.text_muted).wrap())
        .child(if cf.items.is_empty() { el() } else { items })
        .child(row().w_full().gap(8.0).child(el().grow(1.0)).child(cancel).child(ok));

    Dialog::new("confirm", cf.title.clone())
        .open(true)
        .width(460.0)
        .click_outside(true)
        .on_close(|s: &mut App| s.confirm = None)
        .child(body)
        .build(k, layer);
}

/// Append the toast region to `layer`.
pub fn toasts(s: &App, k: &Kit, layer: &mut Vec<El>) {
    if s.toasts.is_empty() {
        return;
    }
    let mut region = ToastRegion::new();
    for t in &s.toasts {
        let id = t.id;
        let mut toast = Toast::new(format!("toast-{id}"), t.title.clone()).on_close(move |s: &mut App| s.dismiss_toast(id));
        if let Some(d) = &t.detail {
            toast = toast.description(d.clone());
        }
        if t.danger {
            toast = toast.icon(k, k.glyph.circle_x);
        }
        region = region.toast(toast.build(k));
    }
    layer.push(region.build(k));
}
