//! "Starts with Windows": a read-only inventory of every program set to start on its own, from
//! the registry, the Startup folders, scheduled tasks, services and the unusual spots.

use rune::{El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, SegmentedControl, Spinner, Switch};

use crate::app::App;
use crate::sys::startup::{Item, Kind};
use crate::ui::icons;
use crate::ui::theme::Look;
use crate::worker::Cmd;

const KINDS: [Option<Kind>; 5] = [None, Some(Kind::Registry), Some(Kind::Folder), Some(Kind::Task), Some(Kind::Service)];

#[derive(Default)]
pub struct StartupState {
    pub items: Option<Vec<Item>>,
    pub show_microsoft: bool,
    pub kind: usize,
}

pub fn load(s: &mut App) {
    s.startup.items = None;
    s.send(Cmd::Startup);
}

fn visible<'a>(st: &'a StartupState) -> impl Iterator<Item = &'a Item> {
    let kind = KINDS[st.kind.min(KINDS.len() - 1)];
    st.items.iter().flatten().filter(move |i| (st.show_microsoft || !i.microsoft) && kind.is_none_or(|k| i.kind == k || i.kind == Kind::Unusual))
}

fn item_row(l: &Look, i: usize, it: &Item) -> El {
    let k = &l.kit;
    let warn = it.kind == Kind::Unusual;
    let tint = if warn { k.c.warning.icon } else { l.accent };
    let mut meta = vec![it.kind.label().to_string(), if it.publisher.is_empty() { "No publisher".into() } else { it.publisher.clone() }];
    if !it.enabled {
        meta.push("Disabled".into());
    }
    let mut r = row()
        .key(format!("su-{i}"))
        .w_full()
        .gap(12.0)
        .padding(10.0)
        .radius(12.0)
        .center_y()
        .fill(if warn { tint.alpha(0.10) } else { l.text.alpha(0.035) })
        .child(
            row()
                .size(34.0, 34.0)
                .radius(10.0)
                .center()
                .fill(tint.alpha(if it.enabled { 0.18 } else { 0.08 }))
                .child(text(it.name.chars().next().unwrap_or('?').to_uppercase().to_string()).font(k.ty.ui_medium).color(if it.enabled { tint } else { l.muted })),
        )
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(1.0)
                .child(text(it.name.clone()).font(k.ty.ui_medium).color(if it.enabled { l.text } else { l.muted }).truncate())
                .child(text(meta.join(" \u{00b7} ")).font(k.ty.text_small).color(l.muted).truncate())
                .child(text(it.command.clone()).font(k.ty.mono).color(l.muted.alpha(0.8)).truncate()),
        );
    if let Some(exe) = it.exe.clone().filter(|p| p.exists()) {
        r = r.child(
            Button::new("Show file")
                .variant(ButtonVariant::Ghost)
                .control_size(ControlSize::Small)
                .on_press(move |_s: &mut App| crate::sys::shell::reveal(&exe))
                .build(k)
                .key(format!("su-show-{i}")),
        );
    }
    r
}

pub fn view(s: &App, l: &Look) -> El {
    let k = &l.kit;
    let st = &s.startup;
    let mut body = col().key("startup-body").w_full().gap(12.0);
    body = body.child(
        text("Every program set to start on its own, wherever it hides. Disabled entries are ones Task Manager or the program turned off. This list only reads; it changes nothing.")
            .font(k.ty.small)
            .color(l.muted)
            .wrap(),
    );
    let Some(items) = &st.items else {
        return body.child(row().gap(10.0).py(20.0).center_y().child(Spinner::new().size(18.0).color(l.accent).build(k)).child(text("Looking everywhere…").font(k.ty.ui).color(l.text)));
    };
    let count = |k: Option<Kind>| items.iter().filter(|i| (st.show_microsoft || !i.microsoft) && k.is_none_or(|k| i.kind == k)).count();
    let labels: Vec<String> = KINDS
        .iter()
        .map(|kd| match kd {
            None => format!("All {}", count(None)),
            Some(kd) => format!("{} {}", kd.label().replace("Startup folder", "Folders").replace("Scheduled task", "Tasks").replace("Service", "Services"), count(Some(*kd))),
        })
        .collect();
    let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    let kinds = SegmentedControl::new(&label_refs, Some(st.kind))
        .label("Where")
        .fit()
        .on_pick(|s: &mut App, i, _| s.startup.kind = i)
        .build(k);
    let hidden = items.iter().filter(|i| i.microsoft).count();
    let ms = row()
        .gap(10.0)
        .center_y()
        .child(text(format!("Show Microsoft ({hidden})")).font(k.ty.small).color(l.muted))
        .child(
            Switch::new(st.show_microsoft)
                .label("Show Microsoft entries")
                .hide_label()
                .on_toggle(|s: &mut App| s.startup.show_microsoft = !s.startup.show_microsoft)
                .build(k),
        );
    let refresh = Button::new("Refresh").variant(ButtonVariant::Ghost).control_size(ControlSize::Small).icon(&icons::REFRESH).on_press(load).build(k).key("su-refresh");
    body = body.child(row().w_full().gap(12.0).center_y().flex_wrap().child(kinds).child(el().grow(1.0)).child(ms).child(refresh));

    let unusual = items.iter().filter(|i| i.kind == Kind::Unusual).count();
    if unusual > 0 {
        body = body.child(
            text(format!("{unusual} unusual entr{} below: Windows' sign-in settings point somewhere other than the default. If you did not set that up, look closely.", if unusual == 1 { "y" } else { "ies" }))
                .font(k.ty.small)
                .color(k.c.warning.icon)
                .wrap(),
        );
    }
    let mut list = col().w_full().gap(6.0);
    let mut n = 0;
    for (i, it) in visible(st).enumerate() {
        list = list.child(item_row(l, i, it));
        n += 1;
    }
    if n == 0 {
        list = list.child(text("Nothing here.").font(k.ty.small).color(l.muted).py(12.0));
    }
    body.child(list)
}

/// The PauseSwitch tab's top switcher between groups and the startup inventory.
pub fn switcher(s: &App, l: &Look) -> El {
    SegmentedControl::new(&["Groups", "Starts with Windows"], Some(s.pause_view))
        .label("PauseSwitch view")
        .fit()
        .on_pick(|s: &mut App, i, _| {
            s.pause_view = i;
            if i == 1 && s.startup.items.is_none() {
                load(s);
            }
        })
        .build(&l.kit)
}

pub fn scroll(e: El) -> El {
    e.scroll(ScrollSpec { stick_bottom: false }).grow(1.0).min_h(0.0)
}
