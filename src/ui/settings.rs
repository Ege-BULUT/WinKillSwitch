//! The Settings tab: palette, light/dark, accent and glass. Every change applies at once and is saved.

use rune::{Cx, El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, SegmentedControl, Switch, TextInput};

use crate::app::App;
use crate::model::{GlassStyle, ThemeMode};
use crate::ui::palettes::{self, Palette};
use crate::ui::theme::{Look, color};

fn section(l: &Look, title: &str, hint: &str, body: El) -> El {
    col()
        .w_full()
        .gap(10.0)
        .child(
            col()
                .gap(2.0)
                .child(text(title.to_string()).font(l.kit.ty.body_medium).color(l.text))
                .child(text(hint.to_string()).font(l.kit.ty.small).color(l.muted).wrap()),
        )
        .child(body)
}

fn add_rule(s: &mut App) {
    let rule = s.alert_edit.text.trim().to_string();
    if !rule.is_empty() && !s.alerts.watch.iter().any(|w| w.eq_ignore_ascii_case(&rule)) {
        s.alerts.watch.push(rule);
        s.persist();
    }
    s.alert_edit = rune::TextEdit::new("");
    s.alert_edit.focused = true;
}

fn alerts(s: &App, l: &Look) -> El {
    let k = &l.kit;
    let risky = row()
        .w_full()
        .gap(16.0)
        .center_y()
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(2.0)
                .child(text("Programs started from Downloads or Temp").font(k.ty.ui_medium).color(l.text))
                .child(text("Installers and downloaded tools run from there; so does most malware.").font(k.ty.small).color(l.muted).wrap()),
        )
        .child(
            Switch::new(s.alerts.risky_folders)
                .label("Alert for programs started from Downloads or Temp")
                .hide_label()
                .on_toggle(|s: &mut App| {
                    s.alerts.risky_folders = !s.alerts.risky_folders;
                    s.persist();
                })
                .build(k),
        );
    let input = TextInput::new(&s.alert_edit)
        .placeholder("Process name, like steam* or discord")
        .width(None)
        .on_edit(|s: &mut App| &mut s.alert_edit)
        .on_submit(|s: &mut App, _| add_rule(s))
        .build(k)
        .key("alert-input");
    let add = Button::new("Watch")
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Regular)
        .on_press(add_rule)
        .build(k)
        .key("alert-add");
    let mut chips = row().w_full().gap(8.0).flex_wrap();
    for (i, w) in s.alerts.watch.iter().enumerate() {
        chips = chips.child(
            row()
                .key(format!("alert-rule-{i}"))
                .gap(6.0)
                .center_y()
                .pl(12.0)
                .pr(4.0)
                .py(4.0)
                .radius(999.0)
                .fill(l.accent.alpha(0.14))
                .child(text(w.clone()).font(k.ty.mono).color(l.text))
                .child(
                    row()
                        .key(format!("alert-remove-{i}"))
                        .size(22.0, 22.0)
                        .radius(11.0)
                        .center()
                        .cursor(rune::Cursor::Pointer)
                        .hover(l.text.alpha(0.1))
                        .role("button")
                        .label(format!("Stop watching {w}"))
                        .on_press(move |s: &mut App| {
                            if i < s.alerts.watch.len() {
                                s.alerts.watch.remove(i);
                                s.persist();
                            }
                        })
                        .child(text("×").font(k.ty.ui).color(l.muted)),
                ),
        );
    }
    let watch = col()
        .w_full()
        .gap(10.0)
        .child(
            col()
                .gap(2.0)
                .child(text("Programs to watch").font(k.ty.ui_medium).color(l.text))
                .child(text("Tell me whenever one of these starts. Use * as a wildcard.").font(k.ty.small).color(l.muted).wrap()),
        )
        .child(row().w_full().gap(8.0).center_y().child(el().grow(1.0).min_w(0.0).child(input)).child(add))
        .child(if s.alerts.watch.is_empty() { text("Nothing watched yet.").font(k.ty.small).color(l.muted) } else { chips });
    col()
        .w_full()
        .max_w(560.0)
        .gap(14.0)
        .padding(12.0)
        .radius(12.0)
        .fill(l.card)
        .child(risky)
        .child(el().w_full().h(1.0).fill(l.line))
        .child(watch)
}

/// A palette card: its light and dark sides, each with the variant's accents.
fn palette_card(s: &App, l: &Look, p: &'static Palette) -> El {
    let on = s.appearance.palette == p.id;
    let half = |dark: bool| {
        let v = p.variant(dark);
        let mut dots = row().gap(5.0);
        for (_, hex) in v.accents.iter().take(5) {
            dots = dots.child(el().size(12.0, 12.0).radius(6.0).fill(color(*hex)));
        }
        col()
            .grow(1.0)
            .h(74.0)
            .padding(10.0)
            .gap(8.0)
            .justify_between()
            .fill(color(v.bg))
            .child(text(if dark { p.dark_name } else { p.light_name }).font(l.kit.ty.small_medium).color(color(v.text)))
            .child(dots)
    };
    let id = p.id;
    col()
        .key(format!("pal-{id}"))
        .w(232.0)
        .gap(8.0)
        .padding(8.0)
        .radius(16.0)
        .fill(if on { l.accent.alpha(0.14) } else { l.card })
        .border(if on { 2.0 } else { 1.0 }, if on { l.accent } else { l.line })
        .hover(l.card_hover)
        .cursor(rune::Cursor::Pointer)
        .role("radio")
        .label(p.name)
        .checked(on)
        .focusable(true)
        .press_scale(0.98)
        .on_press(move |s: &mut App| {
            s.appearance.palette = id.to_string();
            s.persist();
        })
        .child(row().w_full().radius(10.0).clip_rounded(10.0).child(half(false)).child(half(true)))
        .child(
            col()
                .w_full()
                .px(4.0)
                .pb(2.0)
                .gap(1.0)
                .child(text(p.name).font(l.kit.ty.ui_medium).color(l.text))
                .child(text(p.source).font(l.kit.ty.text_xsmall).color(l.muted).truncate()),
        )
}

pub fn view(s: &App, _cx: &mut Cx<App>, l: &Look) -> El {
    let k = &l.kit;
    let a = &s.appearance;

    let mut cards = row().w_full().gap(12.0).flex_wrap();
    for p in palettes::ALL.iter() {
        cards = cards.child(palette_card(s, l, p));
    }

    let mode_idx = match a.mode {
        ThemeMode::System => 0,
        ThemeMode::Light => 1,
        ThemeMode::Dark => 2,
    };
    let mode = SegmentedControl::new(&["System", "Light", "Dark"], Some(mode_idx))
        .label("Appearance mode")
        .fit()
        .on_pick(|s: &mut App, i, _| {
            s.appearance.mode = [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark][i.min(2)];
            s.persist();
        })
        .build(k);

    let mut accents = row().gap(10.0).flex_wrap();
    for (i, (name, hex)) in l.v.accents.iter().enumerate() {
        let on = a.accent % l.v.accents.len() == i;
        accents = accents.child(
            row()
                .key(format!("acc-{i}"))
                .gap(8.0)
                .center_y()
                .py(6.0)
                .pl(6.0)
                .pr(12.0)
                .radius(999.0)
                .fill(if on { color(*hex).alpha(0.18) } else { l.card })
                .border(1.0, if on { color(*hex) } else { l.line })
                .hover(l.card_hover)
                .cursor(rune::Cursor::Pointer)
                .role("radio")
                .label(*name)
                .checked(on)
                .focusable(true)
                .on_press(move |s: &mut App| {
                    s.appearance.accent = i;
                    s.persist();
                })
                .child(el().size(20.0, 20.0).radius(10.0).fill(color(*hex)).border(2.0, l.text.alpha(if on { 0.9 } else { 0.0 })))
                .child(text(*name).font(k.ty.small_medium).color(l.text)),
        );
    }

    let glass_idx = match a.glass {
        GlassStyle::Liquid => 0,
        GlassStyle::Fluted => 1,
        GlassStyle::Frosted => 2,
    };
    let glass = SegmentedControl::new(&["Liquid glass", "Fluted glass", "Frosted glass"], Some(glass_idx))
        .label("Glass style")
        .fit()
        .on_pick(|s: &mut App, i, _| {
            s.appearance.glass = [GlassStyle::Liquid, GlassStyle::Fluted, GlassStyle::Frosted][i.min(2)];
            s.persist();
        })
        .build(k);
    let strength = SegmentedControl::new(&["Light", "Medium", "Strong"], Some(a.strength.min(2) as usize))
        .label("Glass strength")
        .fit()
        .on_pick(|s: &mut App, i, _| {
            s.appearance.strength = i.min(2) as u8;
            s.persist();
        })
        .build(k);
    let animated = row()
        .w_full()
        .max_w(560.0)
        .gap(16.0)
        .center_y()
        .padding(12.0)
        .radius(12.0)
        .fill(l.card)
        .child(
            col()
                .grow(1.0)
                .gap(2.0)
                .child(text("Animated background").font(k.ty.ui_medium).color(l.text))
                .child(text("The color gradient behind the glass drifts slowly. When off, the window draws nothing while idle.").font(k.ty.small).color(l.muted).wrap()),
        )
        .child(
            Switch::new(a.animated)
                .label("Animated background")
                .hide_label()
                .on_toggle(|s: &mut App| {
                    s.appearance.animated = !s.appearance.animated;
                    s.persist();
                })
                .build(k),
        );

    let hk = crate::sys::hotkey::status();
    let hotkey = row()
        .w_full()
        .max_w(560.0)
        .gap(16.0)
        .center_y()
        .padding(12.0)
        .radius(12.0)
        .fill(l.card)
        .child(
            col()
                .grow(1.0)
                .gap(2.0)
                .child(text(format!("Quick palette from anywhere ({})", crate::sys::hotkey::LABEL)).font(k.ty.ui_medium).color(l.text))
                .child(
                    text(match hk {
                        crate::sys::hotkey::Status::Taken => "Another app already uses this shortcut, so it is not active. Ctrl+K still works inside WinKillSwitch.",
                        _ => "Brings WinKillSwitch forward with the palette open, even when it is in the background. Ctrl+K works inside the app.",
                    })
                    .font(k.ty.small)
                    .color(if hk == crate::sys::hotkey::Status::Taken { k.c.warning.icon } else { l.muted })
                    .wrap(),
                ),
        )
        .child(
            Switch::new(a.hotkey)
                .label("Quick palette shortcut")
                .hide_label()
                .on_toggle(|s: &mut App| {
                    s.appearance.hotkey = !s.appearance.hotkey;
                    crate::sys::hotkey::set_enabled(s.appearance.hotkey);
                    s.persist();
                })
                .build(k),
        );

    let alerts = alerts(s, l);

    let body = col()
        .key("settings-body")
        .w_full()
        .padding(28.0)
        .gap(30.0)
        .child(
            col()
                .gap(4.0)
                .child(text("Appearance").font(k.ty.body_medium).color(l.text))
                .child(text("Changes apply at once and are saved.").font(k.ty.small).color(l.muted)),
        )
        .child(section(l, "Color palette", "Every palette has an official light and dark variant; the mode picks which one you see.", cards))
        .child(section(l, "Mode", "System follows the Windows light or dark setting.", row().child(mode)))
        .child(section(l, "Accent color", "Taken from the palette itself. Selections, buttons and the background gradient follow it.", accents))
        .child(section(
            l,
            "Glass",
            "Liquid glass bends light at its edges like the tab bar, fluted glass has vertical ribs, frosted glass is the calmest.",
            col().gap(12.0).child(row().gap(12.0).flex_wrap().child(glass).child(strength)).child(animated),
        ))
        .child(section(l, "Shortcuts", "Reach anything by typing its name.", hotkey))
        .child(section(l, "Alerts", "A quiet note in the corner when a new process starts that you care about.", alerts))
        .child(section(l, "Rule packs", "Ready-made sets of limits and alerts. Turning one off removes only its rules.", crate::ui::packs::section(s, l)))
        .child(section(l, "Limits", "Processor and memory ceilings for programs, kept until you remove them.", crate::ui::limits::settings_list(s, l)));

    col().w_full().h_full().min_h(0.0).child(body.scroll(ScrollSpec { stick_bottom: false }).grow(1.0).min_h(0.0))
}
