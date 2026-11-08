//! The Settings "Rule packs" section: ready-made profiles to switch on and off, and importing or
//! exporting your own rules as a file.

use rune::{El, FileDialogOptions, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize};

use crate::app::App;
use crate::sys::packs::{self, Pack};
use crate::ui::icons;
use crate::ui::theme::Look;

fn summary(p: &Pack) -> String {
    let mut parts = Vec::new();
    if !p.limits.is_empty() {
        parts.push(format!("{} limit{}", p.limits.len(), if p.limits.len() == 1 { "" } else { "s" }));
    }
    let alerts = p.watch.len() + usize::from(p.risky_folders == Some(true));
    if alerts > 0 {
        parts.push(format!("{alerts} alert{}", if alerts == 1 { "" } else { "s" }));
    }
    parts.join(" \u{00b7} ")
}

/// Merge a pack in, save, enforce, and say what happened.
pub fn apply(s: &mut App, p: &Pack) {
    let n = packs::apply(p, &mut s.alerts, &mut s.limits);
    s.persist();
    s.enforce_limits();
    s.toast(format!("{} on", p.name), Some(if n == 0 { "Every rule was already in place.".into() } else { format!("{n} rule{} added or updated.", if n == 1 { "" } else { "s" }) }), false);
}

fn remove(s: &mut App, p: &Pack) {
    packs::remove(p, &mut s.alerts, &mut s.limits);
    s.persist();
    s.enforce_limits();
    s.toast(format!("{} off", p.name), Some("Its rules were removed; ones you changed were kept.".into()), false);
}

fn import(s: &mut App, path: std::path::PathBuf) {
    let read = std::fs::metadata(&path)
        .map_err(|e| e.to_string())
        .and_then(|m| if m.len() > 1 << 20 { Err("The file is larger than 1 MB.".into()) } else { std::fs::read_to_string(&path).map_err(|e| e.to_string()) });
    match read.and_then(|j| packs::parse(&j)) {
        Ok(p) => apply(s, &p),
        Err(e) => s.toast("Could not import that pack", Some(e), true),
    }
}

fn export(s: &mut App, path: std::path::PathBuf) {
    let p = packs::export("My WinKillSwitch rules", &s.alerts, &s.limits);
    let json = serde_json::to_string_pretty(&p).unwrap_or_default();
    match std::fs::write(&path, json + "\n") {
        Ok(()) => s.toast("Rules exported", Some(path.display().to_string()), false),
        Err(e) => s.toast("Could not save the file", Some(e.to_string()), true),
    }
}

pub fn section(s: &App, l: &Look) -> El {
    let k = &l.kit;
    let mut cards = row().w_full().gap(12.0).flex_wrap().items_start();
    for (i, p) in packs::builtin().into_iter().enumerate() {
        let on = packs::is_applied(&p, &s.alerts, &s.limits);
        let pack = p.clone();
        let toggle = Button::new(if on { "Turn off" } else { "Turn on" })
            .variant(if on { ButtonVariant::Ghost } else { ButtonVariant::Contrast })
            .control_size(ControlSize::Small)
            .on_press(move |s: &mut App| if on { remove(s, &pack) } else { apply(s, &pack) })
            .build(k)
            .key(format!("pack-toggle-{i}"));
        cards = cards.child(
            col()
                .key(format!("pack-{i}"))
                .grow(1.0)
                .min_w(220.0)
                .max_w(320.0)
                .padding(14.0)
                .gap(8.0)
                .radius(14.0)
                .fill(l.card)
                .border(if on { 1.5 } else { 1.0 }, if on { l.accent } else { l.line })
                .child(
                    row()
                        .w_full()
                        .gap(8.0)
                        .center_y()
                        .child(text(p.name.clone()).font(k.ty.ui_medium).color(l.text).grow(1.0))
                        .child(if on { text("On").font(k.ty.text_xsmall_medium).color(l.accent).px(8.0).py(2.0).radius(999.0).fill(l.accent.alpha(0.16)) } else { el() }),
                )
                .child(text(p.description.clone()).font(k.ty.small).color(l.muted).wrap())
                .child(row().w_full().center_y().gap(8.0).child(text(summary(&p)).font(k.ty.text_small).color(l.muted).grow(1.0)).child(toggle)),
        );
    }
    let files = row()
        .gap(8.0)
        .flex_wrap()
        .child(
            Button::new("Import a pack")
                .variant(ButtonVariant::Outline)
                .control_size(ControlSize::Regular)
                .icon(&icons::PLUS)
                .on_press_tx(|_s: &mut App, tx| {
                    tx.open_file(FileDialogOptions { title: "Import a rule pack".into(), ..Default::default() }.filter("Rule pack", &["json"]), |s: &mut App, r, _| {
                        if let Ok(Some(p)) = r {
                            import(s, p);
                        }
                    });
                })
                .build(k)
                .key("pack-import"),
        )
        .child(
            Button::new("Export my rules")
                .variant(ButtonVariant::Ghost)
                .control_size(ControlSize::Regular)
                .on_press_tx(|_s: &mut App, tx| {
                    tx.save_file(
                        FileDialogOptions { title: "Export your rules".into(), file_name: Some("winkillswitch-rules.json".into()), ..Default::default() }.filter("Rule pack", &["json"]),
                        |s: &mut App, r, _| {
                            if let Ok(Some(p)) = r {
                                export(s, p);
                            }
                        },
                    );
                })
                .build(k)
                .key("pack-export"),
        );
    col().w_full().gap(12.0).child(cards).child(files)
}
