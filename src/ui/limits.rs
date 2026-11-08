//! The "Set limits" dialog: a CPU ceiling and a memory ceiling for one program, applied to every
//! copy of it now and whenever it starts again.

use rune::{El, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Dialog, SegmentedControl};

use crate::app::App;
use crate::sys::limits::{CPU_STEPS, Limit, MEM_STEPS};
use crate::ui::theme::Look;

pub struct LimitDialog {
    pub name: String,
    pub cpu: usize,
    pub mem: usize,
}

pub fn open(s: &mut App, name: String) {
    let cur = s.limits.iter().find(|l| l.name.eq_ignore_ascii_case(&name));
    let cpu = cur.and_then(|l| CPU_STEPS.iter().position(|c| *c == l.cpu)).unwrap_or(0);
    let mem = cur.and_then(|l| MEM_STEPS.iter().position(|m| *m == l.mem_mb)).unwrap_or(0);
    s.limit_dialog = Some(LimitDialog { name, cpu, mem });
}

/// Save `limit` (or remove it when it limits nothing) and apply at once.
pub fn set(s: &mut App, limit: Limit) {
    s.limits.retain(|l| !l.name.eq_ignore_ascii_case(&limit.name));
    if limit.cpu.is_some() || limit.mem_mb.is_some() {
        s.limits.push(limit);
    }
    s.persist();
    let errors = s.enforcer.sync(&s.limits, &s.snap);
    report(s, errors);
}

pub fn report(s: &mut App, errors: Vec<String>) {
    if let Some(first) = errors.first() {
        let more = if errors.len() > 1 { format!(" and {} more", errors.len() - 1) } else { String::new() };
        s.toast("Could not limit every process", Some(format!("{first}{more}")), true);
    }
}

pub fn dialog(s: &App, l: &Look, layer: &mut Vec<El>) {
    let Some(d) = &s.limit_dialog else { return };
    let k = &l.kit;
    let label = |t: &str, hint: &str| col().gap(2.0).child(text(t.to_string()).font(k.ty.ui_medium).color(l.text)).child(text(hint.to_string()).font(k.ty.small).color(l.muted).wrap());
    let cpu = SegmentedControl::new(&["No limit", "50%", "25%", "10%"], Some(d.cpu))
        .label("Processor limit")
        .fit()
        .on_pick(|s: &mut App, i, _| {
            if let Some(d) = s.limit_dialog.as_mut() {
                d.cpu = i.min(CPU_STEPS.len() - 1);
            }
        })
        .build(k);
    let mem = SegmentedControl::new(&["No limit", "4 GB", "2 GB", "1 GB", "512 MB"], Some(d.mem))
        .label("Memory limit")
        .fit()
        .on_pick(|s: &mut App, i, _| {
            if let Some(d) = s.limit_dialog.as_mut() {
                d.mem = i.min(MEM_STEPS.len() - 1);
            }
        })
        .build(k);
    let running = s.snap.procs.iter().filter(|p| p.name.eq_ignore_ascii_case(&d.name)).count();
    let exists = s.limits.iter().any(|x| x.name.eq_ignore_ascii_case(&d.name));

    let mut buttons = row().w_full().gap(8.0).center_y();
    if exists {
        buttons = buttons.child(
            Button::new("Remove limits")
                .variant(ButtonVariant::Ghost)
                .control_size(ControlSize::Regular)
                .on_press(|s: &mut App| {
                    if let Some(d) = s.limit_dialog.take() {
                        set(s, Limit { name: d.name.clone(), cpu: None, mem_mb: None });
                        s.toast(format!("Limits removed from {}", d.name), None, false);
                    }
                })
                .build(k)
                .key("limit-remove"),
        );
    }
    buttons = buttons.child(el().grow(1.0)).child(
        Button::new("Apply")
            .variant(ButtonVariant::Contrast)
            .control_size(ControlSize::Regular)
            .on_press(|s: &mut App| {
                if let Some(d) = s.limit_dialog.take() {
                    let limit = Limit { name: d.name.clone(), cpu: CPU_STEPS[d.cpu], mem_mb: MEM_STEPS[d.mem] };
                    let what = crate::sys::limits::describe(&limit);
                    set(s, limit);
                    if !what.is_empty() {
                        s.toast(format!("{} limited", d.name), Some(what), false);
                    }
                }
            })
            .build(k)
            .key("limit-apply"),
    );

    let body = col()
        .w_full()
        .px(18.0)
        .pb(18.0)
        .gap(18.0)
        .child(
            text(format!(
                "Applies to {} and every time it starts again.",
                match running {
                    0 => format!("{} when it runs", d.name),
                    1 => format!("the running {}", d.name),
                    n => format!("all {n} running {} processes", d.name),
                }
            ))
            .font(k.ty.ui)
            .color(l.text)
            .wrap(),
        )
        .child(col().w_full().gap(10.0).child(label("Processor", "All its processes together never use more than this share of your processor. It runs slower but keeps working.")).child(cpu))
        .child(col().w_full().gap(10.0).child(label("Memory", "Each process keeps at most this much in RAM; Windows moves the rest to disk, which can make it slower.")).child(mem))
        .child(buttons);

    Dialog::new("limits", format!("Set limits for {}", d.name)).open(true).fit().width(560.0).click_outside(true).on_close(|s: &mut App| s.limit_dialog = None).child(body).build(k, layer);
}

/// The Settings list of saved limits.
pub fn settings_list(s: &App, l: &Look) -> El {
    let k = &l.kit;
    let mut list = col().w_full().max_w(560.0).gap(8.0);
    if s.limits.is_empty() {
        return list.child(
            text("No limits yet. Open a process's details and choose Set limits to cap its processor or memory use.").font(k.ty.small).color(l.muted).wrap().padding(12.0).radius(12.0).fill(l.card),
        );
    }
    for (i, lim) in s.limits.iter().enumerate() {
        let name = lim.name.clone();
        let n = s.enforcer.count(&lim.name);
        list = list.child(
            row()
                .key(format!("limit-row-{i}"))
                .w_full()
                .gap(12.0)
                .padding(12.0)
                .radius(12.0)
                .center_y()
                .fill(l.card)
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(2.0)
                        .child(text(lim.name.clone()).font(k.ty.ui_medium).color(l.text).truncate())
                        .child(
                            text(format!("{} \u{00b7} {}", crate::sys::limits::describe(lim), if n == 0 { "not running".to_string() } else { format!("{n} running") }))
                                .font(k.ty.small)
                                .color(l.muted),
                        ),
                )
                .child(Button::new("Edit").variant(ButtonVariant::Ghost).control_size(ControlSize::Small).on_press(move |s: &mut App| open(s, name.clone())).build(k).key(format!("limit-edit-{i}"))),
        );
    }
    list
}
