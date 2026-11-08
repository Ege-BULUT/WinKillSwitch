//! The Clean tab: a memory card with a ring gauge and one big button, and a storage card that
//! analyzes, lists what it found and cleans what you keep ticked. Both show what they freed, as a
//! number that counts up and as a change in the visual.

use rune::{Cx, El, Icon, ScrollSpec, col, el, icon, row, text};
use rune_kit::{Button, ButtonVariant, Checkbox, ControlSize, Progress, Spinner};

use crate::app::{App, DiskPhase};
use crate::sys::cleanup::Icon as CatIcon;
use crate::ui::theme::Look;
use crate::ui::{fmt, icons, viz};
use crate::worker::Cmd;
use crate::REVEAL;

fn card(l: &Look, key: &str, title: &str, ic: &'static Icon, aside: String, body: El) -> El {
    col()
        .key(key.to_string())
        .grow(1.0)
        .min_w(330.0)
        .padding(22.0)
        .gap(18.0)
        .radius(20.0)
        .fill(l.card)
        .border(1.0, l.line)
        .child(
            row()
                .w_full()
                .center_y()
                .gap(10.0)
                .child(row().size(32.0, 32.0).radius(10.0).center().fill(l.accent.alpha(0.18)).child(icon(ic, 17.0).color(l.accent)))
                .child(text(title.to_string()).font(l.kit.ty.body_medium).color(l.text).grow(1.0))
                .child(text(aside).font(l.kit.ty.small).color(l.muted)),
        )
        .child(body)
}

/// The one big action button both cards use.
fn big_button(l: &Look, key: &str, label: String, busy: bool, enabled: bool, on: impl Fn(&mut App) + 'static) -> El {
    let fill = if enabled || busy { l.accent } else { l.text.alpha(0.12) };
    let ink = if enabled || busy { l.on_accent } else { l.muted };
    let mut b = row()
        .key(key.to_string())
        .w_full()
        .h(52.0)
        .radius(26.0)
        .center()
        .gap(10.0)
        .linear_gradient(1.6, vec![(0.0, fill), (1.0, if enabled { fill.mix(l.text, 0.22) } else { fill })])
        .shadow(8.0, 26.0, if enabled { l.accent.alpha(0.35) } else { rune::Color::TRANSPARENT })
        .role("button")
        .label(label.clone());
    if busy {
        b = b.child(Spinner::new().size(18.0).color(ink).build(&l.kit));
    }
    b = b.child(text(label).font(l.kit.ty.ui_medium).color(ink));
    if enabled && !busy {
        b = b.cursor(rune::Cursor::Pointer).focusable(true).press_scale(0.97).hover(l.text.alpha(0.06)).on_press(on);
    }
    b
}

fn memory(s: &App, l: &Look) -> El {
    let k = &l.kit;
    let m = s.mem;
    let used = s.mem_shown.clamp(0.0, 1.0);
    let cache = s.cache_shown.clamp(0.0, 1.0);
    let success = k.c.success.icon;

    // After a clean, the freed slice glows green and fades, and the number counts up.
    // Cached memory in the palette's second accent, so the two slices read apart at a glance.
    let cached_color = crate::ui::theme::color(l.v.accents[(s.appearance.accent + 1) % l.v.accents.len()].1).alpha(0.75);
    let mut segments = vec![(0.0, used, l.accent), (used, (used + cache).min(1.0), cached_color)];
    let mut result: Option<El> = None;
    if let Some(r) = &s.ram.last {
        let t = (s.clock - s.ram.shown_at) / REVEAL;
        let freed = r.freed();
        if t < 1.0 && freed > 0 {
            let before = r.before.used_frac();
            segments.push((used, before.max(used), success.alpha((1.0 - t as f32).max(0.0))));
        }
        let shown = (freed as f64 * viz::ease(t)) as u64;
        let line = if freed > 64 * 1024 * 1024 {
            format!("Freed {} · trimmed {} apps", fmt::bytes(shown), r.trimmed)
        } else {
            format!("Memory was already lean · trimmed {} apps", r.trimmed)
        };
        result = Some(
            row()
                .key("ram-result")
                .gap(8.0)
                .center_y()
                .padding(10.0)
                .px(14.0)
                .radius(12.0)
                .fill(success.alpha(0.14))
                .child(icon(&icons::CHECK, 14.0).color(success))
                .child(text(line).font(k.ty.ui_medium).color(l.text)),
        );
    }

    let gauge = el()
        .size(220.0, 220.0)
        .child(viz::ring(220.0, 18.0, l.text.alpha(0.08), &segments).absolute(0.0, 0.0))
        .child(
            col()
                .absolute(0.0, 0.0)
                .size(220.0, 220.0)
                .center()
                .gap(2.0)
                .child(text(format!("{:.0}%", used * 100.0)).font(rune::TextStyle { size: 44.0, line_height: 0.0, ..k.ty.body_medium }).color(l.text))
                .child(text(format!("{} in use", fmt::bytes(m.in_use()))).font(k.ty.small).color(l.muted)),
        );

    let chip = |c: rune::Color, label: &str, v: u64| {
        row()
            .gap(7.0)
            .center_y()
            .child(el().size(9.0, 9.0).radius(4.5).fill(c))
            .child(text(label.to_string()).font(k.ty.small).color(l.muted))
            .child(text(fmt::bytes(v)).font(k.ty.small_medium).color(l.text))
    };
    let legend = row()
        .gap(18.0)
        .flex_wrap()
        .justify_center()
        .child(chip(l.accent, "In use", m.in_use()))
        .child(chip(cached_color, "Cached", m.cache))
        .child(chip(l.text.alpha(0.18), "Free", m.free()));

    let hist: Vec<f32> = s.mem_hist.iter().copied().collect();
    let spark = col()
        .gap(6.0)
        .items_center()
        .child(viz::sparkline(&hist, 300.0, 46.0, l.accent, l.accent.alpha(0.14)))
        .child(text("Memory use, last two minutes").font(k.ty.text_xsmall).color(l.muted));

    let elevated = s.elevated;
    let button = big_button(
        l,
        "clean-ram",
        if s.ram.running { format!("{}…", s.ram.stage.unwrap_or("Cleaning memory")) } else { "Clean memory".into() },
        s.ram.running,
        !s.ram.running,
        move |s: &mut App| {
            s.ram.running = true;
            s.send(Cmd::CleanRam { elevated });
        },
    );
    let note = if elevated {
        "Trims every app's working set and clears the standby cache. Nothing is closed; Windows refills memory as apps need it."
    } else {
        "Trims every app's working set. Nothing is closed. Run as administrator to also clear the standby cache."
    };

    let mut body = col().w_full().gap(18.0).items_center().child(gauge).child(legend).child(spark);
    if let Some(r) = result {
        body = body.child(r);
    }
    card(
        l,
        "card-memory",
        "Memory",
        &icons::CHIP,
        format!("{} total", fmt::bytes(m.total)),
        body.child(button).child(text(note).font(k.ty.text_xsmall).color(l.muted).wrap()),
    )
}

fn cat_icon(i: CatIcon) -> &'static Icon {
    match i {
        CatIcon::Temp => &icons::CLOCK,
        CatIcon::Windows => &icons::WINDOW,
        CatIcon::Crash => &icons::ALERT,
        CatIcon::Thumbs => &icons::IMAGE,
        CatIcon::Browser => &icons::GLOBE,
        CatIcon::Shader => &icons::CUBE,
        CatIcon::Update => &icons::REFRESH,
        CatIcon::Bin => &icons::TRASH,
    }
}

fn drive_bar(s: &App, l: &Look) -> El {
    let (free, total) = s.disk.drive;
    let used = if total > 0 { 1.0 - free as f32 / total as f32 } else { 0.0 };
    // The bar grows to full width; the used part is a fraction of it.
    col()
        .w_full()
        .gap(6.0)
        .child(
            el().key("drive-bar").w_full().h(10.0).radius(5.0).fill(l.text.alpha(0.08)).child(
                el().h(10.0).w_frac(used).radius(5.0).linear_gradient(1.6, vec![(0.0, l.accent), (1.0, l.accent.mix(l.text, 0.3))]).motion(rune::Spring::new(0.6, 0.15)),
            ),
        )
        .child(
            row()
                .w_full()
                .justify_between()
                .child(text(format!("System drive · {} used", fmt::bytes(total.saturating_sub(free)))).font(k_small(l)).color(l.muted))
                .child(text(format!("{} free of {}", fmt::bytes(free), fmt::bytes(total))).font(k_small(l)).color(l.muted)),
        )
}

fn k_small(l: &Look) -> rune::TextStyle {
    l.kit.ty.text_small
}

fn storage(s: &App, l: &Look) -> El {
    let k = &l.kit;
    let mut body = col().w_full().gap(16.0).child(drive_bar(s, l));
    let elevated = s.elevated;
    let analyze = move |s: &mut App| {
        let running = s.running_names();
        s.disk.phase = DiskPhase::Scanning { done: 0, total: 1, current: "your drive".into(), files: 0, bytes: 0 };
        s.send(Cmd::ScanDisk { running, elevated });
    };

    match &s.disk.phase {
        DiskPhase::Idle => {
            body = body
                .child(
                    col()
                        .w_full()
                        .py(26.0)
                        .gap(12.0)
                        .items_center()
                        .child(row().size(84.0, 84.0).radius(42.0).center().fill(l.accent.alpha(0.14)).child(icon(&icons::SEARCH, 34.0).color(l.accent)))
                        .child(text("Find temporary files, caches and leftovers you can safely remove.").font(k.ty.ui).color(l.text).wrap().max_w(330.0))
                        .child(text("Nothing is deleted until you review the list.").font(k.ty.small).color(l.muted)),
                )
                .child(big_button(l, "disk-analyze", "Analyze".into(), false, true, analyze));
        }
        DiskPhase::Scanning { done, total, current, files, bytes } => {
            body = body.child(
                col()
                    .w_full()
                    .py(40.0)
                    .gap(14.0)
                    .items_center()
                    .child(Spinner::new().size(34.0).color(l.accent).build(k))
                    .child(text(format!("Looking at {current}…")).font(k.ty.ui).color(l.text))
                    .child(text(format!("{} files · {} found", fmt::count(*files), fmt::bytes(*bytes))).font(k.ty.mono).color(l.muted))
                    .child(row().w_full().max_w(320.0).child(Progress::new(Some(*done as f32 / (*total).max(1) as f32)).build(k).w_full())),
            );
        }
        DiskPhase::Ready => {
            let selected: u64 = s.disk.items.iter().filter(|i| i.selected).map(|i| i.found.bytes).sum();
            let total: u64 = s.disk.items.iter().filter(|i| i.found.blocked.is_none()).map(|i| i.found.bytes).sum();
            let mut list = col().key("disk-list").w_full().gap(6.0);
            for (i, it) in s.disk.items.iter().enumerate() {
                let blocked = it.found.blocked.clone();
                let usable = blocked.is_none() && it.found.bytes > 0;
                let mut meta = row().gap(8.0).center_y().child(text(fmt::bytes(it.found.bytes)).font(k.ty.mono).color(l.text));
                if let Some(b) = &blocked {
                    meta = row()
                        .gap(8.0)
                        .center_y()
                        .child(text(b.clone()).font(k.ty.text_xsmall_medium).color(k.c.warning.icon))
                        .child(text(fmt::bytes(it.found.bytes)).font(k.ty.mono).color(l.muted));
                }
                list = list.child(
                    row()
                        .key(format!("disk-{}", it.cat.id))
                        .w_full()
                        .gap(12.0)
                        .padding(10.0)
                        .radius(12.0)
                        .center_y()
                        .fill(if it.selected { l.accent.alpha(0.1) } else { l.text.alpha(0.03) })
                        .child(
                            Checkbox::new(it.selected)
                                .label(it.cat.name.clone())
                                .hide_label()
                                .disabled(!usable)
                                .on_toggle(move |s: &mut App| {
                                    if let Some(x) = s.disk.items.get_mut(i) {
                                        x.selected = !x.selected;
                                    }
                                })
                                .build(k),
                        )
                        .child(icon(cat_icon(it.cat.icon), 16.0).color(if usable { l.accent } else { l.muted }))
                        .child(
                            col()
                                .grow(1.0)
                                .min_w(0.0)
                                .gap(1.0)
                                .child(text(it.cat.name.clone()).font(k.ty.ui_medium).color(l.text))
                                .child(text(it.cat.detail).font(k.ty.text_xsmall).color(l.muted).wrap()),
                        )
                        .child(meta),
                );
            }
            body = body
                .child(
                    row()
                        .w_full()
                        .center_y()
                        .justify_between()
                        .child(
                            col()
                                .gap(0.0)
                                .child(text(fmt::bytes(total)).font(rune::TextStyle { size: 34.0, line_height: 0.0, ..k.ty.body_medium }).color(l.text))
                                .child(text("can be cleaned").font(k.ty.small).color(l.muted)),
                        )
                        .child(Button::new("Analyze again").variant(ButtonVariant::Ghost).control_size(ControlSize::Small).icon(&icons::REFRESH).on_press(analyze).build(k).key("disk-rescan")),
                )
                .child(list.scroll(ScrollSpec { stick_bottom: false }).max_h(300.0).min_h(0.0))
                .child(big_button(
                    l,
                    "disk-clean",
                    if selected > 0 { format!("Clean {}", fmt::bytes(selected)) } else { "Select something to clean".into() },
                    false,
                    selected > 0,
                    move |s: &mut App| {
                        let cats = s.disk.items.iter().filter(|i| i.selected).map(|i| i.cat.clone()).collect();
                        let running = s.running_names();
                        s.disk.phase = DiskPhase::Cleaning { done: 0, total: 1, current: "selected items".into(), freed: 0 };
                        s.send(Cmd::CleanDisk { cats, running, elevated });
                    },
                ));
        }
        DiskPhase::Cleaning { done, total, current, freed } => {
            body = body.child(
                col()
                    .w_full()
                    .py(40.0)
                    .gap(14.0)
                    .items_center()
                    .child(Spinner::new().size(34.0).color(l.accent).build(k))
                    .child(text(format!("Cleaning {current}…")).font(k.ty.ui).color(l.text))
                    .child(text(format!("{} freed so far", fmt::bytes(*freed))).font(k.ty.small).color(l.muted))
                    .child(row().w_full().max_w(320.0).child(Progress::new(Some(*done as f32 / (*total).max(1) as f32)).build(k).w_full())),
            );
        }
        DiskPhase::Done { freed, deleted, skipped, at } => {
            let success = k.c.success.icon;
            let t = (s.clock - at) / REVEAL;
            let shown = (*freed as f64 * viz::ease(t)) as u64;
            let mut rows = col().w_full().gap(4.0);
            for (id, r) in &s.disk.results {
                let name = s.disk.items.iter().find(|i| i.cat.id == id).map(|i| i.cat.name.clone()).unwrap_or_default();
                rows = rows.child(
                    row()
                        .w_full()
                        .justify_between()
                        .child(text(name).font(k.ty.small).color(l.muted))
                        .child(text(fmt::bytes(r.freed)).font(k.ty.mono).color(l.text)),
                );
            }
            body = body
                .child(
                    col()
                        .w_full()
                        .py(18.0)
                        .gap(8.0)
                        .items_center()
                        .child(row().size(64.0, 64.0).radius(32.0).center().fill(success.alpha(0.16)).child(icon(&icons::CHECK, 30.0).color(success)))
                        .child(text(format!("{} freed", fmt::bytes(shown))).font(rune::TextStyle { size: 34.0, line_height: 0.0, ..k.ty.body_medium }).color(l.text))
                        .child(
                            text(format!(
                                "Deleted {} files{}",
                                fmt::count(*deleted),
                                if *skipped > 0 { format!(" · skipped {} in use", fmt::count(*skipped)) } else { String::new() }
                            ))
                            .font(k.ty.small)
                            .color(l.muted),
                        ),
                )
                .child(rows)
                .child(big_button(l, "disk-again", "Analyze again".into(), false, true, analyze));
        }
    }

    let (free, _) = s.disk.drive;
    card(l, "card-storage", "Storage", &icons::DRIVE, format!("{} free", fmt::bytes(free)), body)
}

pub fn view(s: &App, _cx: &mut Cx<App>, l: &Look) -> El {
    let k = &l.kit;
    let head = col()
        .gap(4.0)
        .child(text("Clean up").font(rune::TextStyle { size: 24.0, line_height: 0.0, ..k.ty.body_medium }).color(l.text))
        .child(text("Give memory back to the apps you are using and clear out files nobody needs.").font(k.ty.small).color(l.muted));
    let mut cards = row().key("clean-cards").w_full().gap(16.0).flex_wrap().items_start();
    if crate::ui::MEMORY_CLEANER {
        cards = cards.child(memory(s, l));
    }
    if crate::ui::DISK_CLEANER {
        cards = cards.child(storage(s, l));
    }
    col()
        .w_full()
        .h_full()
        .min_h(0.0)
        .child(col().key("clean-body").w_full().padding(24.0).gap(20.0).child(head).child(cards).scroll(ScrollSpec { stick_bottom: false }).grow(1.0).min_h(0.0))
}
