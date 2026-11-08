//! The "about this process" sheet: what it is, whether WinKillSwitch can manage it and why, and
//! the facts behind that, with the few actions people want next.

use rune::{El, Icon, col, el, icon, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Dialog, Spinner};

use crate::app::App;
use crate::model::now_secs;
use crate::sys::info::{Account, Signature, Verdict, known};
use crate::sys::shell;
use crate::ui::theme::Look;
use crate::ui::{fmt, icons};

fn ago(start: u64) -> String {
    if start == 0 {
        return "Unknown".into();
    }
    let s = now_secs().saturating_sub(start);
    match s {
        0..=59 => "Just now".into(),
        60..=3599 => format!("{} min ago", s / 60),
        3600..=86_399 => format!("{} h {} min ago", s / 3600, s % 3600 / 60),
        _ => format!("{} days ago", s / 86_400),
    }
}

fn fact(l: &Look, label: &str, value: El) -> El {
    row()
        .w_full()
        .gap(12.0)
        .py(7.0)
        .center_y()
        .child(text(label.to_string()).font(l.kit.ty.small).color(l.muted).w(110.0))
        .child(value.grow(1.0).min_w(0.0))
}

fn val(l: &Look, s: impl Into<String>) -> El {
    text(s.into()).font(l.kit.ty.ui).color(l.text).truncate()
}

pub fn sheet(s: &App, l: &Look, layer: &mut Vec<El>, window_h: f32) {
    let Some(info) = &s.info else { return };
    let k = &l.kit;
    let pid = info.pid;
    let Some(p) = s.snap.get(pid).cloned() else {
        // The process ended while the sheet was open.
        Dialog::new("info", "Process ended")
            .open(true)
            .width(420.0)
            .click_outside(true)
            .on_close(|s: &mut App| s.info = None)
            .child(text(format!("PID {pid} is no longer running.")).font(k.ty.ui).color(l.muted))
            .build(k, layer);
        return;
    };
    let detail = info.detail.clone();

    // Header: an initial in an accent tile, the friendly name, then the image name and PID.
    let initial = p.description.chars().next().unwrap_or('?').to_uppercase().to_string();
    let head = row()
        .w_full()
        .gap(14.0)
        .center_y()
        .child(row().size(48.0, 48.0).radius(14.0).center().fill(l.accent.alpha(0.2)).child(text(initial).font(k.ty.body_medium).color(l.accent)))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(2.0)
                .child(text(p.description.clone()).font(k.ty.body_medium).color(l.text).truncate())
                .child(text(format!("{} · PID {}", p.name, p.pid)).font(k.ty.mono).color(l.muted)),
        )
        .child(
            text(p.kind.label())
                .font(k.ty.text_xsmall_medium)
                .color(l.text)
                .px(10.0)
                .py(4.0)
                .radius(999.0)
                .fill(l.text.alpha(0.08)),
        );

    // What it is.
    let about = match known(&p.name) {
        Some((what, safe)) => col().gap(4.0).child(text(what).font(k.ty.ui).color(l.text).wrap()).child(text(safe).font(k.ty.small).color(l.muted).wrap()),
        None => {
            let folder = p.exe.as_ref().and_then(|e| e.parent()).map(|d| d.display().to_string()).unwrap_or_default();
            let by = if p.publisher.is_empty() { "An app with no publisher information".to_string() } else { format!("An app from {}", p.publisher) };
            col()
                .gap(4.0)
                .child(text(format!("{by}.")).font(k.ty.ui).color(l.text).wrap())
                .child(text(if folder.is_empty() { "Its location is not available.".to_string() } else { format!("Installed in {folder}.") }).font(k.ty.small).color(l.muted).wrap())
        }
    };

    // Can WinKillSwitch manage it, and if not, why.
    let access = match &detail {
        None => row().gap(10.0).center_y().child(Spinner::new().size(16.0).color(l.accent).build(k)).child(text("Checking access and signature…").font(k.ty.small).color(l.muted)),
        Some(d) => {
            let (c, ic): (rune::Color, &'static Icon) = match d.verdict {
                Verdict::Ok => (k.c.success.icon, &icons::CHECK),
                Verdict::NeedsAdmin => (k.c.warning.icon, &icons::ALERT),
                Verdict::Critical | Verdict::Protected => (l.accent, &icons::ALERT),
                Verdict::Denied => (k.c.danger.icon, &icons::ALERT),
            };
            let mut box_ = row()
                .w_full()
                .gap(12.0)
                .padding(14.0)
                .radius(14.0)
                .fill(c.alpha(0.12))
                .border(1.0, c.alpha(0.35))
                .child(row().size(30.0, 30.0).radius(15.0).center().fill(c.alpha(0.2)).child(icon(ic, 16.0).color(c)))
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(3.0)
                        .child(text(d.verdict.headline()).font(k.ty.ui_medium).color(l.text))
                        .child(text(d.reason.clone()).font(k.ty.small).color(l.muted).wrap()),
                );
            if d.verdict == Verdict::NeedsAdmin && !s.elevated {
                box_ = box_.child(
                    Button::new("Restart as admin")
                        .variant(ButtonVariant::Outline)
                        .control_size(ControlSize::Small)
                        .on_press(|_s: &mut App| {
                            if crate::sys::elevate::relaunch_as_admin("").is_ok() {
                                std::process::exit(0);
                            }
                        })
                        .build(k)
                        .key("info-admin"),
                );
            }
            box_
        }
    };

    // Facts.
    let sig = match detail.as_ref().map(|d| d.signature) {
        None => val(l, "Checking…"),
        Some(Signature::Valid) => row().gap(6.0).center_y().child(icon(&icons::CHECK, 13.0).color(k.c.success.icon)).child(val(l, "Signed, signature is valid")),
        Some(Signature::Unsigned) => text("Not signed").font(k.ty.ui).color(k.c.warning.icon),
        Some(Signature::Invalid) => text("Signed, but the signature is not trusted").font(k.ty.ui).color(k.c.danger.icon),
        Some(Signature::Unknown) => val(l, "No file to check"),
    };
    let runs_as = match detail.as_ref() {
        None => "Checking…".to_string(),
        Some(d) => {
            let who = match d.account {
                Account::System => "SYSTEM",
                Account::LocalService => "Local Service",
                Account::NetworkService => "Network Service",
                Account::User => "You",
                Account::Unknown => "Unknown",
            };
            if d.elevated && d.account == Account::User { format!("{who}, as administrator") } else { who.to_string() }
        }
    };
    let parent = p.parent.and_then(|pp| s.snap.get(pp).map(|x| (pp, x.name.clone())));
    let parent_el = match parent {
        Some((pp, name)) => text(format!("{name} · PID {pp}"))
            .font(k.ty.ui)
            .color(l.accent)
            .cursor(rune::Cursor::Pointer)
            .role("link")
            .on_press(move |s: &mut App| s.open_info(pp)),
        None => val(l, "None running"),
    };
    // Children grouped by name: "chrome.exe ×3, crashpad.exe".
    let mut children: Vec<(String, usize)> = Vec::new();
    for c in s.snap.procs.iter().filter(|c| c.parent == Some(pid)) {
        match children.iter_mut().find(|(n, _)| *n == c.name) {
            Some((_, count)) => *count += 1,
            None => children.push((c.name.clone(), 1)),
        }
    }
    let kids = if children.is_empty() {
        "None".to_string()
    } else {
        let shown: Vec<String> = children.iter().take(3).map(|(n, c)| if *c > 1 { format!("{n} \u{00d7}{c}") } else { n.clone() }).collect();
        let mut v = shown.join(", ");
        if children.len() > 3 {
            v += &format!(" and {} more", children.len() - 3);
        }
        v
    };
    let facts = col()
        .w_full()
        .child(fact(l, "Publisher", val(l, if p.publisher.is_empty() { "Unknown".into() } else { p.publisher.clone() })))
        .child(fact(l, "Signature", sig))
        .child(fact(l, "Runs as", val(l, runs_as)))
        .child(fact(l, "Started", val(l, ago(p.start))))
        .child(fact(l, "Using", val(l, format!("{} CPU · {} memory", fmt::pct(p.cpu), fmt::bytes(p.mem)))))
        .child(fact(l, "Parent", parent_el))
        .child(fact(l, "Children", val(l, kids)));

    // Location and command line, each copyable.
    let path = p.exe.as_ref().map(|e| e.display().to_string()).unwrap_or_default();
    let mut where_ = col().w_full().gap(8.0);
    if !path.is_empty() {
        let (p1, p2) = (path.clone(), p.exe.clone());
        where_ = where_.child(
            col()
                .w_full()
                .gap(6.0)
                .padding(12.0)
                .radius(12.0)
                .fill(l.text.alpha(0.04))
                .child(text("Location").font(k.ty.text_xsmall_medium).color(l.muted))
                .child(text(path.clone()).font(k.ty.mono).color(l.text).wrap_anywhere())
                .child(
                    row()
                        .gap(8.0)
                        .child(Button::new("Open file location").variant(ButtonVariant::Ghost).control_size(ControlSize::Small).icon(&icons::WINDOW).on_press(move |_s: &mut App| {
                            if let Some(e) = &p2 {
                                shell::reveal(e);
                            }
                        }).build(k).key("info-reveal"))
                        .child(Button::new("Copy path").variant(ButtonVariant::Ghost).control_size(ControlSize::Small).on_press_tx(move |_s: &mut App, tx| tx.copy(p1.clone())).build(k).key("info-copy")),
                ),
        );
    }
    if !p.cmd.is_empty() && p.cmd != path {
        let c1 = p.cmd.clone();
        where_ = where_.child(
            col()
                .w_full()
                .gap(6.0)
                .padding(12.0)
                .radius(12.0)
                .fill(l.text.alpha(0.04))
                .child(row().w_full().justify_between().center_y().child(text("Command line").font(k.ty.text_xsmall_medium).color(l.muted)).child(
                    Button::new("Copy").variant(ButtonVariant::Ghost).control_size(ControlSize::Small).on_press_tx(move |_s: &mut App, tx| tx.copy(c1.clone())).build(k).key("info-copy-cmd"),
                ))
                .child(text(p.cmd.clone()).font(k.ty.mono).color(l.text).wrap_anywhere().max_lines(4)),
        );
    }

    let q = shell::query(&format!("{} {} windows process", p.name, p.publisher));
    let mut actions = row().w_full().gap(8.0).flex_wrap().child(
        Button::new("Search the web")
            .variant(ButtonVariant::Outline)
            .control_size(ControlSize::Regular)
            .icon(&icons::GLOBE)
            .on_press(move |_s: &mut App| shell::open_url(&format!("https://duckduckgo.com/?q={q}")))
            .build(k)
            .key("info-search"),
    );
    if let Some(gid) = s.selected_group {
        let gname = s.group(gid).map(|g| g.name.clone()).unwrap_or_default();
        actions = actions.child(
            Button::new(format!("Add to {gname}"))
                .variant(ButtonVariant::Outline)
                .control_size(ControlSize::Regular)
                .icon(&icons::PLUS)
                .on_press(move |s: &mut App| crate::actions::add_pid_to_group(s, gid, pid))
                .build(k)
                .key("info-add"),
        );
    }
    actions = actions.child(el().grow(1.0)).child(
        Button::new("Done").variant(ButtonVariant::Contrast).control_size(ControlSize::Regular).on_press(|s: &mut App| s.info = None).build(k).key("info-done"),
    );

    let body = col()
        .w_full()
        .px(18.0)
        .pb(18.0)
        .gap(16.0)
        .child(head)
        .child(about)
        .child(access)
        .child(facts)
        .child(where_)
        .child(actions)
        .scroll(rune::ScrollSpec { stick_bottom: false })
        .max_h((window_h - 150.0).clamp(240.0, 720.0))
        .min_h(0.0);

    Dialog::new("info", "About this process").open(true).fit().width(560.0).click_outside(true).on_close(|s: &mut App| s.info = None).child(body).build(k, layer);
}
