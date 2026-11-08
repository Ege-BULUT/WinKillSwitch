//! The Processes tab: a search field, a live table drawn on glass, and the action bar.

use std::borrow::Cow;

use rune::{Cx, El, Phase, col, el, icon, row, text};
use rune_kit::{Button, ButtonVariant, Cell, ControlSize, MenuItem, SegmentedControl, Table, TableColumn, TextInput, context_menu};

use crate::app::App;
use crate::ui::theme::Look;
use crate::ui::{columns, fmt, icons};

pub fn view(s: &App, _cx: &mut Cx<App>, l: &Look, layer: &mut Vec<El>) -> El {
    let k = &l.kit;
    let ok = k.c.success.icon;
    let (ink, muted) = (l.text, l.muted);
    let ui = k.ty.ui;
    let tree = s.appearance.tree;
    let faint = l.muted.alpha(0.6);

    let mut table = Table::new(
        |s: &App| &s.table,
        |s: &mut App| &mut s.table,
        |s: &App| s.view_rows.len(),
        |s: &App, i| Cow::Owned(s.view_rows.get(i).copied().unwrap_or(0).to_string()),
    )
    .name("Processes");
    for c in columns::ALL.iter().filter(|c| s.column_shown(c.id)) {
        let mut tc = TableColumn::new(c.id, c.title).min(c.min).align(columns::align(c)).sortable();
        tc = match c.width {
            Some(w) => tc.width(w),
            None => tc.flex(2.0),
        };
        table = table.column(tc);
    }
    let table = table
    .cell(move |s: &App, i, col| {
        let Some(&pid) = s.view_rows.get(i) else { return Cell::text("") };
        let Some(p) = s.snap.get(pid) else { return Cell::text("") };
        match col {
            "name" if tree => {
                // Indent by depth; a chevron folds and unfolds the branch.
                let (depth, kids) = s.row_meta.get(i).copied().unwrap_or((0, false));
                let folded = s.collapsed.contains(&pid);
                let chevron: El = if kids {
                    row()
                        .key(format!("fold-{pid}"))
                        .size(16.0, 16.0)
                        .center()
                        .cursor(rune::Cursor::Pointer)
                        .role("button")
                        .label(if folded { "Expand" } else { "Collapse" })
                        .on_press(move |s: &mut App| {
                            if !s.collapsed.remove(&pid) {
                                s.collapsed.insert(pid);
                            }
                            s.rebuild_rows();
                        })
                        .child(icon(&icons::CHEVRON, 10.0).color(muted).rotate(if folded { 0.0 } else { std::f32::consts::FRAC_PI_2 }))
                } else {
                    el().size(16.0, 16.0)
                };
                Cell::el(
                    row()
                        .gap(6.0)
                        .center_y()
                        .child(el().w(depth as f32 * 16.0).h(1.0))
                        .child(chevron)
                        .child(el().size(7.0, 7.0).radius(3.5).fill(if depth == 0 { ok } else { faint }))
                        .child(text(p.name.clone()).font(ui).color(ink)),
                )
            }
            "name" => Cell::el(
                row()
                    .gap(9.0)
                    .center_y()
                    .child(el().size(7.0, 7.0).radius(3.5).fill(ok))
                    .child(text(p.name.clone()).font(ui).color(ink)),
            ),
            "pid" => Cell::text(p.pid.to_string()).mono().color(muted),
            "cpu" | "mem" | "disk" | "gpu" => Cell::text(columns::text(p, col)).mono(),
            "type" | "status" | "engine" => Cell::text(columns::text(p, col)).color(muted),
            _ => Cell::text(columns::text(p, col)),
        }
    })
    .on_sort(|s: &mut App, col, dir, _| {
        s.table.set_sort(col, dir);
        s.rebuild_rows();
    })
    .on_select(|s: &mut App, id, _| {
        if let Ok(pid) = id.parse::<u32>() {
            s.active_pid = Some(pid);
        }
    })
    // Double click or Return: what is this process?
    .on_open(|s: &mut App, id, _| {
        if let Ok(pid) = id.parse::<u32>() {
            s.active_pid = Some(pid);
            s.open_info(pid);
        }
    })
    .zebra(true)
    .style(l.table_style())
    .build(s, k)
    .grow(1.0)
    .min_h(0.0);

    let search = TextInput::new(&s.search_edit)
        .placeholder("Search by name, PID or path")
        .leading_icon(k.glyph.search)
        .width(None)
        .on_edit(|s: &mut App| &mut s.search_edit)
        .on_change(|s: &mut App, _| {
            s.search = s.search_edit.text.clone();
            s.rebuild_rows();
        })
        .build(k)
        .key("search");

    let used = if s.snap.total_mem > 0 { s.snap.used_mem as f32 / s.snap.total_mem as f32 } else { 0.0 };
    let stat = |label: &str, value: String| {
        col().gap(1.0).child(text(label.to_string()).font(k.ty.text_xsmall_medium).color(muted)).child(text(value).font(k.ty.ui_medium).color(ink))
    };
    let stats = row()
        .gap(20.0)
        .child(stat("PROCESSES", s.snap.procs.len().to_string()))
        .child(stat("CPU", fmt::pct(s.snap.cpu_total)))
        .child(stat("MEMORY", format!("{:.0}%", used * 100.0)))
        .child(stat("SELECTED", s.table.selected_len().to_string()));
    // Search and stats share a row while there is room and stack when the window is narrow.
    let top = row()
        .w_full()
        .flex_wrap()
        .center_y()
        .justify_between()
        .gaps(10.0, 20.0)
        .child(
            row()
                .grow(1.0)
                .min_w(240.0)
                .max_w(640.0)
                .gap(8.0)
                .center_y()
                .child(finder(s, l))
                .child(row().grow(1.0).min_w(160.0).child(search.w_full()))
                .child(
                    SegmentedControl::new(&["List", "Tree"], Some(usize::from(s.appearance.tree)))
                        .label("Process view")
                        .fit()
                        .on_pick(|s: &mut App, i, _| {
                            s.appearance.tree = i == 1;
                            s.rebuild_rows();
                            s.persist();
                        })
                        .build(k),
                ),
        )
        .child(stats);

    col()
        .w_full()
        .h_full()
        .padding(16.0)
        .gap(12.0)
        .child(top)
        .child(header_menu(s, l, layer, col().key("table-frame").w_full().grow(1.0).min_h(0.0).radius(12.0).clip_rounded(12.0).child(table)))
        .child(action_bar(s, l))
}

fn action_bar(s: &App, l: &Look) -> El {
    let k = &l.kit;
    let has_active = s.active_pid.is_some();
    let has_sel = s.table.selected_len() > 0;

    let details = Button::new("Details")
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Regular)
        .disabled(!has_active)
        .on_press(|s: &mut App| {
            if let Some(pid) = s.active_pid {
                s.open_info(pid);
            }
        })
        .build(k)
        .key("details");
    let kill = Button::new("End process")
        .variant(ButtonVariant::Danger)
        .control_size(ControlSize::Regular)
        .disabled(!has_active)
        .on_press(|s: &mut App| {
            if let Some(pid) = s.active_pid {
                crate::actions::kill_one(s, pid);
            }
        })
        .build(k)
        .key("kill-one");
    let kill_rel = Button::new("End all related")
        .variant(ButtonVariant::Danger)
        .control_size(ControlSize::Regular)
        .icon(&icons::SKULL)
        .disabled(!has_active)
        .on_press(|s: &mut App| {
            if let Some(pid) = s.active_pid {
                crate::actions::kill_all_related(s, pid);
            }
        })
        .build(k)
        .key("kill-rel");
    let add = Button::new("Add to group")
        .variant(ButtonVariant::Outline)
        .control_size(ControlSize::Regular)
        .icon(&icons::PLUS)
        .disabled(!(has_sel || has_active) || s.selected_group.is_none())
        .on_press(|s: &mut App| {
            if let Some(id) = s.selected_group {
                if s.table.selected_len() > 0 {
                    crate::actions::add_selected_to_group(s, id);
                } else if let Some(pid) = s.active_pid {
                    crate::actions::add_pid_to_group(s, id, pid);
                }
            }
        })
        .build(k)
        .key("add-group");
    let hint = text(if s.selected_group.is_none() { "Pick a group on the left to add to it." } else { "" })
        .font(k.ty.small)
        .color(l.muted);

    row()
        .w_full()
        .flex_wrap()
        .center_y()
        .gaps(8.0, 8.0)
        .child(row().gap(8.0).center_y().grow(1.0).min_w(0.0).child(add).child(hint.truncate().min_w(0.0)))
        .child(row().gap(8.0).child(details).child(kill_rel).child(kill))
}

/// Right click on the table header opens the column picker.
fn header_menu(s: &App, l: &Look, layer: &mut Vec<El>, frame: El) -> El {
    let header_h = l.table_style().header_height;
    let trigger = frame.on_secondary_press_local(move |s: &mut App, e: rune::PointerEvent, _| {
        if e.y <= header_h + 2.0 {
            s.column_menu = Some(e.window());
        }
    });
    let mut menu = context_menu("columns", trigger, s.column_menu).group_label("Columns").min_width(200.0);
    for c in columns::ALL.iter().filter(|c| c.id != "name") {
        let id = c.id;
        menu = menu.item(MenuItem::new(c.title).checked(s.column_shown(id)).on_select(move |s: &mut App| {
            s.toggle_column(id);
        }));
    }
    menu.separator()
        .item(MenuItem::new("Reset to default").on_select(|s: &mut App| {
            s.columns.clear();
            s.column_menu = None;
            s.persist();
        }))
        .on_close(|s: &mut App| s.column_menu = None)
        .build(&l.kit, layer)
        // The menu wraps its trigger; the wrapper has to grow like the table frame it holds.
        .w_full()
        .grow(1.0)
        .min_h(0.0)
}

/// The find-by-window target: press it, drag onto any window, release to select its process.
fn finder(s: &App, l: &Look) -> El {
    let on = s.finding;
    let target = row()
        .key("find-window")
        .size(38.0, 38.0)
        .radius(12.0)
        .center()
        .fill(if on { l.accent.alpha(0.25) } else { l.text.alpha(0.06) })
        .border(1.0, if on { l.accent } else { l.line })
        .cursor(rune::Cursor::Crosshair)
        .role("button")
        .label("Find a process by dragging onto its window")
        .help("Drag onto any window to find its process")
        .child(icon(&icons::TARGET, 18.0).color(if on { l.accent } else { l.muted }))
        .on_drag(|s: &mut App, d: &mut rune::Drag, tx: &mut rune::Tx| match d.phase {
            Phase::Down => {
                s.finding = true;
                s.find_hover = None;
            }
            Phase::Move => s.find_hover = crate::sys::finder::under_cursor(),
            Phase::Up => {
                s.finding = false;
                let hit = crate::sys::finder::under_cursor();
                s.find_hover = None;
                if let Some(t) = hit {
                    crate::actions::found_window(s, t, tx);
                }
            }
        });
    // The target keeps the same parent whether or not the label shows: the drag's pointer
    // capture follows the element's place in the tree, so moving it would drop the release.
    let wrap = row().key("find-wrap").gap(8.0).center_y().child(target);
    if !on {
        return wrap;
    }
    // While dragging, say what is under the pointer.
    let label = match &s.find_hover {
        Some(t) if t.pid == std::process::id() => "Over WinKillSwitch".to_string(),
        Some(t) => {
            let name = s.snap.get(t.pid).map(|p| p.name.clone()).unwrap_or_else(|| format!("PID {}", t.pid));
            if t.title.is_empty() { format!("Over {name}") } else { format!("Over {name} \u{00b7} {}", t.title) }
        }
        None => "Drag onto any window".to_string(),
    };
    wrap.child(
            text(label)
                .font(l.kit.ty.small_medium)
                .color(l.text)
                .truncate()
                .max_w(230.0)
                .px(10.0)
                .py(6.0)
                .radius(10.0)
                .fill(l.accent.alpha(0.16)),
        )
}
