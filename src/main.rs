#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// The data model and `sys` interfaces are deliberately complete; this build leaves the
// process-ending, related-process and autostart paths as stubs, so some fields and helpers
// they are designed for are not yet read.
#![allow(dead_code)]
//! WinKillSwitch: a sleek Windows process manager with kill-all-related, groups and a
//! revertable PauseSwitch, built on the Rune UI framework.

mod actions;
mod app;
mod demo;
mod model;
mod store;
mod sys;
mod ui;
mod worker;

use std::time::Duration;

use rune::{AppSpec, Chrome, Tx};

use app::{App, DiskItem, DiskPhase};
use worker::Msg;

/// Seconds the cleaners' count-up and ring pulse play after a result.
pub const REVEAL: f64 = 1.6;

fn handle(s: &mut App, m: Msg) {
    match m {
        Msg::Sample(snap, mem) => {
            if s.demo {
                return; // the demo machine stays as it was set up
            }
            s.snap = snap;
            if s.diag.is_some() {
                let snap = s.snap.clone();
                ui::diagnose::add_sample(s, &snap, false);
            }
            s.record_mem(mem);
            s.rebuild_rows();
            let hits: Vec<(model::Pid, String, String)> =
                s.alert_watch.check(&s.alerts, &s.alert_roots, &s.snap).into_iter().map(|(p, why)| (p.pid, p.name.clone(), why)).collect();
            for (pid, name, why) in hits {
                s.toast_about(pid, format!("{name} started"), why);
            }
            if !s.limits.is_empty() {
                s.enforce_limits();
            }
            if let Some(pid) = s.pending_info.take() {
                s.open_info(pid);
            }
            let events = s.guards.check(&s.snap);
            for ev in events {
                match ev {
                    sys::guard::GuardEvent::Blocked { name, .. } => s.toast(format!("Blocked relaunch of {name}"), None, false),
                    sys::guard::GuardEvent::UserLaunch { name, .. } => s.toast(format!("{name} started by you, guard released"), None, false),
                }
            }
        }
        Msg::RamStage(st) => s.ram.stage = Some(st),
        Msg::Ram(report) => {
            s.ram.stage = None;
            s.ram.running = false;
            s.record_mem(report.after);
            s.ram.last = Some(report);
            s.ram.shown_at = s.clock;
        }
        Msg::DiskScanning { done, total, current, files, bytes } => s.disk.phase = DiskPhase::Scanning { done, total, current, files, bytes },
        Msg::DiskFound(found) => {
            s.disk.items = found
                .into_iter()
                .filter(|(c, f)| f.bytes > 0 || c.recycle_bin)
                .map(|(cat, found)| {
                    let selected = cat.default_on && found.blocked.is_none() && found.bytes > 0;
                    DiskItem { cat, found, selected }
                })
                .collect();
            s.disk.items.sort_by(|a, b| b.found.bytes.cmp(&a.found.bytes));
            s.disk.phase = DiskPhase::Ready;
        }
        Msg::Startup(items) => {
            if !s.demo {
                s.startup.items = Some(items);
            }
        }
        Msg::Locks(r) => {
            if let Some(st) = s.locks.as_mut().filter(|st| st.target.as_ref() == Some(&r.target)) {
                st.busy = false;
                st.report = Some(r);
            }
        }
        Msg::Detail(d) => {
            if let Some(info) = s.info.as_mut().filter(|i| i.pid == d.pid) {
                info.detail = Some(d);
            }
        }
        Msg::DiskCleaning { done, total, current, freed } => s.disk.phase = DiskPhase::Cleaning { done, total, current, freed },
        Msg::DiskCleaned { results, drive } => {
            let freed = results.iter().map(|(_, r)| r.freed).sum();
            let deleted = results.iter().map(|(_, r)| r.deleted).sum();
            let skipped = results.iter().map(|(_, r)| r.skipped).sum();
            s.disk.results = results;
            s.disk.drive = drive;
            s.disk.phase = DiskPhase::Done { freed, deleted, skipped, at: s.clock };
        }
    }
}

fn poll(s: &mut App, now: f64, tx: &mut Tx) -> bool {
    s.clock = now;
    let mut changed = false;

    let msgs = s.worker.as_ref().map(|w| w.drain()).unwrap_or_default();
    for m in msgs {
        handle(s, m);
        changed = true;
    }

    let before = s.toasts.len();
    s.expire_toasts();
    changed |= s.toasts.len() != before;

    // Ease the memory ring toward the live value; while anything animates, draw every frame.
    let target = s.mem.used_frac();
    let mut animating = false;
    if (s.mem_shown - target).abs() > 0.0005 {
        s.mem_shown += (target - s.mem_shown) * 0.12;
        animating = true;
    }
    let cache_target = if s.mem.total > 0 { s.mem.cache as f32 / s.mem.total as f32 } else { 0.0 };
    if (s.cache_shown - cache_target).abs() > 0.0005 {
        s.cache_shown += (cache_target - s.cache_shown) * 0.08;
        animating = true;
    }
    let disk_reveal = matches!(s.disk.phase, DiskPhase::Done { at, .. } if now - at < REVEAL);
    if s.ram.running || (s.ram.last.is_some() && now - s.ram.shown_at < REVEAL) || disk_reveal {
        animating = true;
    }
    if animating && s.tab == app::Tab::Clean {
        changed = true;
        tx.wake_after(Duration::from_millis(16));
    } else {
        if animating {
            changed = true;
        }
        tx.wake_after(Duration::from_millis(250));
    }
    changed
}

/// `--version` and `--help` for scripts and agents. The release exe is a windowed app, so it
/// attaches to the console that launched it before printing.
fn cli() -> bool {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let want = |f: &str| args.iter().any(|a| a == f);
    if !(want("--version") || want("-V") || want("--help")) {
        return false;
    }
    // SAFETY: attaching to the parent's console has no other effect on this process.
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS);
    }
    if want("--help") {
        println!("WinKillSwitch {}

Usage: WinKillSwitch.exe [--version] [--scene demo --snap out.png --size 1280x800]", env!("CARGO_PKG_VERSION"));
    } else {
        println!("WinKillSwitch {}", env!("CARGO_PKG_VERSION"));
    }
    true
}

fn on_key(s: &mut App, ev: &rune::KeyEvent, _tx: &mut Tx) -> bool {
    if ev.is_command('k') && s.palette.is_none() {
        ui::palette::open(s);
        return true;
    }
    false
}

fn main() {
    if cli() {
        return;
    }
    // Stay responsive when the machine is busy: the UI thread runs above normal priority, the
    // sampler and cleaners below it.
    // SAFETY: changes only this process's own priority class.
    unsafe {
        use windows_sys::Win32::System::Threading::{ABOVE_NORMAL_PRIORITY_CLASS, GetCurrentProcess, SetPriorityClass};
        SetPriorityClass(GetCurrentProcess(), ABOVE_NORMAL_PRIORITY_CLASS);
    }
    rune::prewarm_gpu();
    let chrome = Chrome { full_size_content: true, traffic_lights: None, drag_regions: true };
    let spec = AppSpec {
        poll: Some(poll),
        on_key: Some(on_key),
        // Named states for headless snapshots (`--snap out.png --scene settings`).
        scenes: &[
            ("demo", demo::apply),
            ("settings", |s| s.tab = app::Tab::Settings),
            ("clean", |s| s.tab = app::Tab::Clean),
            ("info-self", |s| s.pending_info = Some(std::process::id())),
            ("palette", |s| ui::palette::open(s)),
            ("tree", |s| s.appearance.tree = true),
            ("diagnose", |s| ui::diagnose::start(s)),
            ("startup", |s| {
                s.tab = app::Tab::PauseSwitch;
                s.pause_view = 1;
                ui::startup::load(s);
            }),
            ("pause", |s| s.tab = app::Tab::PauseSwitch),
            ("catppuccin", |s| s.appearance.palette = "catppuccin".into()),
            ("rose-pine", |s| s.appearance.palette = "rose-pine".into()),
            ("tokyo-night", |s| s.appearance.palette = "tokyo-night".into()),
            ("kanagawa", |s| s.appearance.palette = "kanagawa".into()),
            ("everforest", |s| s.appearance.palette = "everforest".into()),
            ("fluted", |s| s.appearance.glass = model::GlassStyle::Fluted),
        ],
        min_size: Some((820.0, 560.0)),
        ..AppSpec::new("WinKillSwitch", (1080.0, 720.0), init, ui::view)
    };
    let app = rune::App::new(spec).chrome(chrome).compositor(ui::gfx::Gfx::default());
    // Ctrl+Shift+K from anywhere: bring the window forward with the palette open.
    if !rune::is_headless() {
        let handle = app.handle();
        sys::hotkey::start(store::load().appearance.hotkey, move || {
            handle.open();
            handle.update(|s: &mut App, _| {
                if s.palette.is_none() {
                    ui::palette::open(s);
                }
            });
        });
    }
    app.run();
    sys::net::stop();
}

fn init() -> App {
    let mut app = App::new();
    // Test hook: watch one more name for this run only (not saved unless a rule is edited).
    if let Ok(rule) = std::env::var("WKS_WATCH") {
        app.alerts.watch.push(rule);
    }
    app.rebuild_rows();
    app
}
