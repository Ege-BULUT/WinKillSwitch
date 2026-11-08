//! The "Why is my PC slow?" sheet: a few seconds of watching, then a short checklist, worst first,
//! each problem with the one button that helps.

use rune::{El, col, el, icon, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Dialog, Progress};

use crate::app::App;
use crate::model::Snapshot;
use crate::sys::diagnose::{self, Finding, Fix, Inputs, Level, SAMPLES};
use crate::ui::icons;
use crate::ui::palette::Action;
use crate::ui::theme::Look;

pub struct DiagState {
    pub samples: Vec<Snapshot>,
    pub findings: Option<Vec<Finding>>,
}

/// Start watching. Each sampler tick adds a sample; after `SAMPLES` the answer is ready.
pub fn start(s: &mut App) {
    s.diag = Some(DiagState { samples: vec![s.snap.clone()], findings: None });
    if s.demo {
        // The demo machine does not tick; answer from its one frozen sample.
        let snap = s.snap.clone();
        add_sample(s, &snap, true);
    } else {
        s.refresh_now();
    }
}

pub fn add_sample(s: &mut App, snap: &Snapshot, finish_now: bool) {
    let (mem, uptime, demo, demo_drive) = (s.mem, uptime(s), s.demo, s.disk.drive);
    let Some(d) = s.diag.as_mut().filter(|d| d.findings.is_none()) else { return };
    if !finish_now {
        d.samples.push(snap.clone());
    }
    if d.samples.len() >= SAMPLES || finish_now {
        let drive = if demo { demo_drive } else { crate::sys::cleanup::system_drive() };
        d.findings = Some(diagnose::diagnose(&Inputs { samples: &d.samples, mem, drive, uptime_secs: uptime }));
        d.samples.clear();
    }
}

fn uptime(s: &App) -> u64 {
    if s.demo {
        return 9 * 86_400 + 4 * 3600;
    }
    sysinfo::System::uptime()
}

fn level_style(l: &Look, lv: Level) -> (rune::Color, &'static rune::Icon) {
    let k = &l.kit;
    match lv {
        Level::Good => (k.c.success.icon, &icons::CHECK),
        Level::Warn => (k.c.warning.icon, &icons::ALERT),
        Level::Bad => (k.c.danger.icon, &icons::ALERT),
    }
}

fn fix_button(l: &Look, i: usize, fix: &Fix) -> Option<El> {
    let (label, action) = match fix {
        Fix::None => return None,
        Fix::CleanMemory => ("Clean memory", Action::CleanMemory),
        Fix::AnalyzeDisk => ("Analyze disk", Action::AnalyzeDisk),
        Fix::ShowProcess(pid) => ("Show it", Action::Process(*pid)),
    };
    Some(
        Button::new(label)
            .variant(ButtonVariant::Outline)
            .control_size(ControlSize::Small)
            .on_press_tx(move |s: &mut App, tx| {
                s.diag = None;
                crate::ui::palette::run(s, action.clone(), tx);
            })
            .build(&l.kit)
            .key(format!("diag-fix-{i}")),
    )
}

fn finding_row(l: &Look, i: usize, f: &Finding) -> El {
    let k = &l.kit;
    let (tint, ic) = level_style(l, f.level);
    let mut r = row()
        .key(format!("diag-{i}"))
        .w_full()
        .gap(12.0)
        .padding(12.0)
        .radius(14.0)
        .items_start()
        .fill(if f.level == Level::Good { l.text.alpha(0.035) } else { tint.alpha(0.10) })
        .child(row().size(30.0, 30.0).radius(15.0).center().fill(tint.alpha(0.18)).child(icon(ic, 15.0).color(tint)))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(2.0)
                .child(text(f.area).font(k.ty.text_xsmall_medium).color(l.muted))
                .child(text(f.title.clone()).font(k.ty.ui_medium).color(l.text).wrap())
                .child(text(f.detail.clone()).font(k.ty.small).color(l.muted).wrap()),
        );
    if let Some(b) = fix_button(l, i, &f.fix) {
        r = r.child(b);
    }
    r
}

pub fn sheet(s: &App, l: &Look, layer: &mut Vec<El>, window_h: f32) {
    let Some(d) = &s.diag else { return };
    let k = &l.kit;
    let mut body = col().w_full().px(18.0).pb(18.0).gap(12.0);
    match &d.findings {
        None => {
            let n = d.samples.len().min(SAMPLES);
            body = body
                .child(
                    col()
                        .w_full()
                        .py(14.0)
                        .gap(10.0)
                        .items_center()
                        .child(row().size(64.0, 64.0).radius(32.0).center().fill(l.accent.alpha(0.16)).child(icon(&icons::CLOCK, 28.0).color(l.accent)))
                        .child(text("Watching your PC for a few seconds…").font(k.ty.ui).color(l.text))
                        .child(text("Processor, memory, disk, graphics and uptime.").font(k.ty.small).color(l.muted)),
                )
                .child(Progress::new(Some(n as f32 / SAMPLES as f32)).build(k).w_full().key("diag-progress"));
        }
        Some(findings) => {
            let (lv, line) = diagnose::verdict(findings);
            let (tint, ic) = level_style(l, lv);
            body = body.child(
                row()
                    .w_full()
                    .gap(12.0)
                    .center_y()
                    .child(row().size(40.0, 40.0).radius(20.0).center().fill(tint.alpha(0.18)).child(icon(ic, 20.0).color(tint)))
                    .child(text(line).font(k.ty.body_medium).color(l.text).grow(1.0).min_w(0.0).wrap()),
            );
            let max_h = (window_h - 260.0).max(220.0);
            let mut list = col().w_full().gap(8.0);
            for (i, f) in findings.iter().enumerate() {
                list = list.child(finding_row(l, i, f));
            }
            body = body.child(list.scroll(rune::ScrollSpec { stick_bottom: false }).max_h(max_h).min_h(0.0)).child(
                row().w_full().child(el().grow(1.0)).child(
                    Button::new("Check again")
                        .variant(ButtonVariant::Contrast)
                        .control_size(ControlSize::Regular)
                        .icon(&icons::REFRESH)
                        .on_press(start)
                        .build(k)
                        .key("diag-again"),
                ),
            );
        }
    }
    Dialog::new("diag", "Why is my PC slow?").open(true).fit().width(580.0).click_outside(true).on_close(|s: &mut App| s.diag = None).child(body).build(k, layer);
}
