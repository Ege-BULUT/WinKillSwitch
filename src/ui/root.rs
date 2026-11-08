//! The whole window: a palette gradient behind glass panels (groups, the current tab, and the
//! group inspector), the floating glass tab bar, and the overlay layer.

use rune::{Cx, El, el, row};

use crate::app::{App, Tab};
use crate::ui::{clean, gfx, glass, groups, icons, overlays, pauseswitch, processes, settings, theme};

pub fn view(s: &App, cx: &mut Cx<App>) -> El {
    let l = theme::look(&s.appearance, cx.system_dark());
    let a = &s.appearance;
    // Window width from the last layout; narrow windows get a slimmer sidebar and no inspector.
    let width = cx.layout_size("root").map(|z| z[0]).unwrap_or(1280.0);
    let side_w = if width < 1000.0 { 200.0 } else { 252.0 };

    let backdrop = rune::effect(gfx::MESH).effect_data(gfx::data(l.dark, 0, 0.0)).absolute(0.0, 0.0).w_full().h_full();

    let mut layer: Vec<El> = Vec::new();
    let main = match s.tab {
        Tab::Processes => processes::view(s, cx, &l, &mut layer),
        Tab::Clean => clean::view(s, cx, &l),
        Tab::PauseSwitch => pauseswitch::view(s, cx, &l),
        Tab::Settings => settings::view(s, cx, &l),
    };

    let mut body = row()
        .key("body")
        .w_full()
        .h_full()
        .padding(16.0)
        .pb(16.0 + glass::BAR_H + 22.0)
        .gap(14.0)
        .child(glass::panel(&l, a, "side", groups::sidebar(s, &l)).w(side_w).h_full())
        .child(glass::panel(&l, a, "main", main).grow(1.0).h_full().min_w(0.0));
    if s.tab == Tab::Processes && width >= 1180.0 {
        if let Some(insp) = groups::inspector(s, cx, &l) {
            body = body.child(glass::panel(&l, a, "insp", insp).w(300.0).h_full());
        }
    }

    let all = [(Tab::Processes, "Processes", &icons::PROCESSES), (Tab::Clean, "Clean", &icons::CLEAN), (Tab::PauseSwitch, "PauseSwitch", &icons::PAUSE), (Tab::Settings, "Settings", &icons::SETTINGS)];
    let shown: Vec<_> = all.into_iter().filter(|(t, _, _)| *t != Tab::Clean || crate::ui::MEMORY_CLEANER || crate::ui::DISK_CLEANER).collect();
    let tabs = glass::tab_bar(s, &l, &shown);

    // Drop a file or folder anywhere to see who is using it.
    let base = el()
        .key("root")
        .w_full()
        .h_full()
        .fill(theme::color(l.v.bg))
        .on_drop_hover(|s: &mut App, over, _| s.drop_hover = over)
        .on_drop_files(|s: &mut App, files, _| {
            s.drop_hover = false;
            if let Some(f) = files.first() {
                crate::ui::locks::check(s, f.clone());
            }
        })
        .child(backdrop)
        .child(body)
        .child(tabs);

    let window_h = cx.layout_size("root").map(|z| z[1]).unwrap_or(800.0);
    crate::ui::info::sheet(s, &l, &mut layer, window_h);
    overlays::confirm(s, &l.kit, &mut layer);
    overlays::toasts(s, &l.kit, &mut layer);
    crate::ui::locks::sheet(s, &l, &mut layer);
    crate::ui::palette::overlay(s, &l, &mut layer);
    crate::ui::locks::drop_hint(s, &l, &mut layer);
    rune_kit::root(base, layer)
}
