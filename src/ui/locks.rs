//! "Who is using this file?": drop a file or folder on the window (or pick one) to see which
//! processes hold it open, each a click away from its details.

use std::path::PathBuf;

use rune::{El, FileDialogOptions, col, el, icon, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Dialog, Spinner};

use crate::app::App;
use crate::sys::locks::Report;
use crate::ui::icons;
use crate::ui::theme::Look;
use crate::worker::Cmd;

pub struct LocksState {
    pub target: Option<PathBuf>,
    pub report: Option<Report>,
    pub busy: bool,
}

pub fn open(s: &mut App) {
    s.locks = Some(LocksState { target: None, report: None, busy: false });
}

pub fn check(s: &mut App, path: PathBuf) {
    s.locks = Some(LocksState { target: Some(path.clone()), report: None, busy: true });
    s.send(Cmd::Locks(path));
}

/// The full-window hint while a file is dragged over WinKillSwitch.
pub fn drop_hint(s: &App, l: &Look, layer: &mut Vec<El>) {
    if !s.drop_hover {
        return;
    }
    let k = &l.kit;
    layer.push(
        col()
            .key("drop-hint")
            .absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .padding(24.0)
            .center()
            .fill(rune::Color::BLACK.alpha(0.35))
            .inert()
            .child(
                col()
                    .padding(28.0)
                    .gap(10.0)
                    .items_center()
                    .radius(24.0)
                    .fill(crate::ui::theme::color(l.v.surface).alpha(0.95))
                    .border(2.0, l.accent)
                    .child(row().size(64.0, 64.0).radius(32.0).center().fill(l.accent.alpha(0.18)).child(icon(&icons::LOCK, 30.0).color(l.accent)))
                    .child(text("Drop to see who is using it").font(k.ty.body_medium).color(l.text))
                    .child(text("Files and folders both work.").font(k.ty.small).color(l.muted)),
            ),
    );
}

pub fn sheet(s: &App, l: &Look, layer: &mut Vec<El>) {
    let Some(st) = &s.locks else { return };
    let k = &l.kit;

    let choose = row()
        .gap(8.0)
        .child(
            Button::new("Choose a file")
                .variant(ButtonVariant::Outline)
                .control_size(ControlSize::Regular)
                .on_press_tx(|_s: &mut App, tx| {
                    tx.open_file(FileDialogOptions { title: "Who is using this file?".into(), ..Default::default() }, |s: &mut App, r, _| {
                        if let Ok(Some(p)) = r {
                            check(s, p);
                        }
                    });
                })
                .build(k)
                .key("locks-file"),
        )
        .child(
            Button::new("Choose a folder")
                .variant(ButtonVariant::Ghost)
                .control_size(ControlSize::Regular)
                .on_press_tx(|_s: &mut App, tx| {
                    tx.open_folder(FileDialogOptions { title: "Who is using this folder?".into(), ..Default::default() }, |s: &mut App, r, _| {
                        if let Ok(Some(p)) = r {
                            check(s, p);
                        }
                    });
                })
                .build(k)
                .key("locks-folder"),
        );

    let mut body = col().w_full().px(18.0).pb(18.0).gap(14.0);
    match (&st.target, &st.report, st.busy) {
        (None, _, _) => {
            body = body
                .child(
                    col()
                        .w_full()
                        .py(18.0)
                        .gap(8.0)
                        .items_center()
                        .child(row().size(64.0, 64.0).radius(32.0).center().fill(l.accent.alpha(0.16)).child(icon(&icons::LOCK, 28.0).color(l.accent)))
                        .child(text("Drop a file or folder on the window, or choose one.").font(k.ty.ui).color(l.text))
                        .child(text("You will see every app and service holding it open.").font(k.ty.small).color(l.muted)),
                )
                .child(row().w_full().justify_center().child(choose));
        }
        (Some(path), _, true) => {
            body = body
                .child(text(path.display().to_string()).font(k.ty.mono).color(l.muted).wrap_anywhere())
                .child(row().gap(10.0).center_y().child(Spinner::new().size(18.0).color(l.accent).build(k)).child(text("Asking Windows…").font(k.ty.ui).color(l.text)));
        }
        (Some(path), Some(r), false) => {
            body = body.child(text(path.display().to_string()).font(k.ty.mono).color(l.muted).wrap_anywhere());
            if let Some(e) = &r.error {
                body = body.child(text(e.clone()).font(k.ty.ui).color(k.c.danger.icon).wrap());
            } else if r.lockers.is_empty() {
                let success = k.c.success.icon;
                body = body.child(
                    row()
                        .w_full()
                        .gap(12.0)
                        .padding(14.0)
                        .radius(14.0)
                        .fill(success.alpha(0.12))
                        .child(icon(&icons::CHECK, 18.0).color(success))
                        .child(
                            col()
                                .grow(1.0)
                                .min_w(0.0)
                                .gap(2.0)
                                .child(text("Nothing is using it right now").font(k.ty.ui_medium).color(l.text))
                                .child(
                                    text("If Windows still refuses to delete or move it, the file may be read only or need administrator rights.")
                                        .font(k.ty.small)
                                        .color(l.muted)
                                        .wrap(),
                                ),
                        ),
                );
            } else {
                body = body.child(
                    text(format!(
                        "{} {} {} open{}",
                        r.lockers.len(),
                        if r.lockers.len() == 1 { "process holds" } else { "processes hold" },
                        if r.files > 1 { format!("files in this folder ({} checked)", r.files) } else { "it".to_string() },
                        if r.truncated { ", the first 2,000 files" } else { "" }
                    ))
                    .font(k.ty.ui_medium)
                    .color(l.text),
                );
                let mut list = col().w_full().gap(6.0);
                for lk in &r.lockers {
                    let pid = lk.pid;
                    let name = s.snap.get(pid).map(|p| p.name.clone()).unwrap_or_else(|| "Unknown".into());
                    let title = if lk.app.is_empty() { name.clone() } else { lk.app.clone() };
                    let detail = if lk.service.is_empty() { format!("{name} · PID {pid}") } else { format!("Service {} · PID {pid}", lk.service) };
                    list = list.child(
                        row()
                            .key(format!("lock-{pid}"))
                            .w_full()
                            .gap(12.0)
                            .padding(10.0)
                            .radius(12.0)
                            .center_y()
                            .fill(l.text.alpha(0.04))
                            .child(row().size(34.0, 34.0).radius(10.0).center().fill(l.accent.alpha(0.18)).child(text(title.chars().next().unwrap_or('?').to_uppercase().to_string()).font(k.ty.ui_medium).color(l.accent)))
                            .child(col().grow(1.0).min_w(0.0).gap(1.0).child(text(title).font(k.ty.ui_medium).color(l.text).truncate()).child(text(detail).font(k.ty.text_small).color(l.muted).truncate()))
                            .child(text(lk.kind).font(k.ty.text_xsmall_medium).color(l.text).px(9.0).py(3.0).radius(999.0).fill(l.text.alpha(0.08)))
                            .child(
                                Button::new("Details")
                                    .variant(ButtonVariant::Ghost)
                                    .control_size(ControlSize::Small)
                                    .on_press(move |s: &mut App| {
                                        s.locks = None;
                                        s.tab = crate::app::Tab::Processes;
                                        s.select_pid(pid);
                                        s.open_info(pid);
                                    })
                                    .build(k)
                                    .key(format!("lock-details-{pid}")),
                            ),
                    );
                }
                body = body.child(list.scroll(rune::ScrollSpec { stick_bottom: false }).max_h(320.0).min_h(0.0)).child(
                    text("Close the file in that app, or end the app, then try again.").font(k.ty.small).color(l.muted).wrap(),
                );
            }
            let again = path.clone();
            body = body.child(
                row()
                    .w_full()
                    .gap(8.0)
                    .flex_wrap()
                    .child(choose)
                    .child(el().grow(1.0))
                    .child(
                        Button::new("Check again")
                            .variant(ButtonVariant::Contrast)
                            .control_size(ControlSize::Regular)
                            .icon(&icons::REFRESH)
                            .on_press(move |s: &mut App| check(s, again.clone()))
                            .build(k)
                            .key("locks-again"),
                    ),
            );
        }
        (Some(_), None, false) => {}
    }

    Dialog::new("locks", "Who is using this file?").open(true).fit().width(560.0).click_outside(true).on_close(|s: &mut App| s.locks = None).child(body).build(k, layer);
}
