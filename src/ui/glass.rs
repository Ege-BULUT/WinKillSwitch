//! Glass building blocks: a panel whose backdrop is the blurred, tinted gradient behind it, and the
//! floating tab bar with a glass drop that springs to the selected tab.

use rune::{El, Icon, Spring, col, el, icon, row, text};

use crate::app::{App, Tab};
use crate::model::{Appearance, GlassStyle};
use crate::ui::gfx::{self, FLUTED, LENS, ROLE_BAR, ROLE_DROP, ROLE_PANEL};
use crate::ui::theme::Look;

fn strength(a: &Appearance) -> f32 {
    match a.glass {
        GlassStyle::Frosted => a.strength as f32 + 1.5,
        _ => a.strength as f32,
    }
}

/// A glass panel around `content`. Size the panel from the caller (`.grow`, `.w`).
pub fn panel(l: &Look, a: &Appearance, key: &str, content: El) -> El {
    let r = 20.0;
    let kind = if a.glass == GlassStyle::Fluted { FLUTED } else { LENS };
    let mut d = gfx::data(l.dark, 0, ROLE_PANEL);
    d[1] = strength(a);
    col()
        .key(key.to_string())
        .radius(r)
        .shadow(10.0, 34.0, rune::Color::BLACK.alpha(if l.dark { 0.35 } else { 0.12 }))
        .child(rune::effect(kind).effect_data(d).effect_backdrop(true).absolute(0.0, 0.0).w_full().h_full().radius(r))
        // A wash of the palette's surface keeps ink readable whatever colour is behind the glass.
        .child(el().absolute(0.0, 0.0).w_full().h_full().radius(r).fill(surface(l, if l.dark { 0.16 } else { 0.5 })).inert())
        .child(el().absolute(0.0, 0.0).w_full().h_full().radius(r).border(1.0, l.text.alpha(if l.dark { 0.14 } else { 0.5 })).inert())
        .child(content.w_full().h_full().min_h(0.0))
}

fn surface(l: &Look, a: f32) -> rune::Color {
    crate::ui::theme::color(l.v.surface).alpha(a)
}

pub const SLOT: f32 = 128.0;
pub const BAR_H: f32 = 58.0;

/// The floating tab bar, centred along the window's bottom edge.
pub fn tab_bar(s: &App, l: &Look, tabs: &[(Tab, &'static str, &'static Icon)]) -> El {
    let pad = 6.0;
    let bw = SLOT * tabs.len() as f32 + pad * 2.0;
    let sel = tabs.iter().position(|(t, _, _)| *t == s.tab).unwrap_or(0) as f32;
    let (dw, dh) = (SLOT, BAR_H - pad * 2.0);
    let spring = Spring::new(0.45, 0.3);

    let mut labels = row().absolute(pad, 0.0).size(SLOT * tabs.len() as f32, BAR_H);
    for (tab, name, ic) in tabs {
        let tab = *tab;
        let on = tab == s.tab;
        let ink = if on { l.text } else { l.muted };
        labels = labels.child(
            row()
                .key(format!("tab-{name}"))
                .size(SLOT, BAR_H)
                .center()
                .gap(8.0)
                .cursor(rune::Cursor::Pointer)
                .role("tab")
                .label(*name)
                .checked(on)
                .focusable(true)
                .on_press(move |s: &mut App| s.tab = tab)
                .child(icon(ic, 16.0).color(if on { l.accent } else { ink }))
                .child(text(*name).font(l.kit.ty.ui_medium).color(ink)),
        );
    }

    let bar = el()
        .key("tab-bar")
        .size(bw, BAR_H)
        .radius(BAR_H / 2.0)
        .shadow(14.0, 40.0, rune::Color::BLACK.alpha(if l.dark { 0.45 } else { 0.18 }))
        .child(rune::effect(LENS).effect_data(gfx::data(l.dark, 0, ROLE_BAR)).effect_backdrop(true).absolute(0.0, 0.0).size(bw, BAR_H).radius(BAR_H / 2.0))
        .child(el().absolute(0.0, 0.0).size(bw, BAR_H).radius(BAR_H / 2.0).fill(surface(l, if l.dark { 0.12 } else { 0.42 })).inert())
        .child(el().absolute(0.0, 0.0).size(bw, BAR_H).radius(BAR_H / 2.0).border(1.0, l.text.alpha(if l.dark { 0.16 } else { 0.55 })).inert())
        // The selection: an accent wash with a drop of clearer glass over it, both on one spring.
        .child(el().key("tab-sel").absolute(pad + SLOT * sel, pad).size(dw, dh).radius(dh / 2.0).fill(l.accent.alpha(if l.dark { 0.22 } else { 0.16 })).motion(spring).inert())
        .child(rune::effect(LENS).key("tab-drop").effect_data(gfx::data(l.dark, 0, ROLE_DROP)).effect_backdrop(true).absolute(pad + SLOT * sel, pad).size(dw, dh).radius(dh / 2.0).motion(spring).inert())
        .child(labels);

    row().key("tab-dock").pin(Some(0.0), None, Some(0.0), Some(18.0)).justify_center().child(bar)
}
