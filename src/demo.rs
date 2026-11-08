//! A fixed, made-up machine for screenshots and recordings, so published images never show the
//! real process list of whoever builds them. Enabled by the `demo` scene; the look and tab come
//! from WKS_PALETTE, WKS_MODE-independent `--theme`, WKS_TAB, WKS_GLASS and WKS_ACCENT.

use std::path::PathBuf;

use crate::app::{App, Tab};
use crate::model::{GlassStyle, Group, Member, PauseRecord, ProcInfo, ProcKind, Snapshot};

const PROCS: [(&str, &str, f32, u64, &str); 30] = [
    ("chrome.exe", r"C:\Program Files\Google\Chrome\Application\chrome.exe", 6.4, 612, "Google LLC"),
    ("Code.exe", r"C:\Users\demo\AppData\Local\Programs\Microsoft VS Code\Code.exe", 4.1, 488, "Microsoft Corporation"),
    ("Spotify.exe", r"C:\Users\demo\AppData\Roaming\Spotify\Spotify.exe", 2.7, 301, "Spotify AB"),
    ("Discord.exe", r"C:\Users\demo\AppData\Local\Discord\app-1.0.9200\Discord.exe", 2.2, 356, "Discord Inc."),
    ("chrome.exe", r"C:\Program Files\Google\Chrome\Application\chrome.exe", 1.9, 244, "Google LLC"),
    ("dwm.exe", r"C:\Windows\System32\dwm.exe", 1.6, 118, "Microsoft Corporation"),
    ("steamwebhelper.exe", r"C:\Program Files (x86)\Steam\bin\cef\cef.win7x64\steamwebhelper.exe", 1.3, 214, "Valve"),
    ("explorer.exe", r"C:\Windows\explorer.exe", 1.1, 176, "Microsoft Corporation"),
    ("Figma.exe", r"C:\Users\demo\AppData\Local\Figma\app-125.4.9\Figma.exe", 0.9, 402, "Figma, Inc."),
    ("chrome.exe", r"C:\Program Files\Google\Chrome\Application\chrome.exe", 0.8, 167, "Google LLC"),
    ("OneDrive.exe", r"C:\Program Files\Microsoft OneDrive\OneDrive.exe", 0.6, 92, "Microsoft Corporation"),
    ("steam.exe", r"C:\Program Files (x86)\Steam\steam.exe", 0.5, 141, "Valve"),
    ("MsMpEng.exe", r"C:\ProgramData\Microsoft\Windows Defender\Platform\MsMpEng.exe", 0.5, 233, "Microsoft Corporation"),
    ("GoogleUpdater.exe", r"C:\Program Files (x86)\Google\GoogleUpdater\GoogleUpdater.exe", 0.4, 21, "Google LLC"),
    ("Notion.exe", r"C:\Users\demo\AppData\Local\Programs\Notion\Notion.exe", 0.4, 268, "Notion Labs, Inc."),
    ("svchost.exe", r"C:\Windows\System32\svchost.exe", 0.3, 38, "Microsoft Corporation"),
    ("EpicGamesLauncher.exe", r"C:\Program Files (x86)\Epic Games\Launcher\Portal\Binaries\Win64\EpicGamesLauncher.exe", 0.3, 188, "Epic Games, Inc."),
    ("Teams.exe", r"C:\Program Files\WindowsApps\MSTeams\ms-teams.exe", 0.3, 274, "Microsoft Corporation"),
    ("SearchHost.exe", r"C:\Windows\SystemApps\MicrosoftWindows.Client.CBS\SearchHost.exe", 0.2, 96, "Microsoft Corporation"),
    ("Docker Desktop.exe", r"C:\Program Files\Docker\Docker\Docker Desktop.exe", 0.2, 197, "Docker Inc."),
    ("chrome.exe", r"C:\Program Files\Google\Chrome\Application\chrome.exe", 0.2, 88, "Google LLC"),
    ("RuntimeBroker.exe", r"C:\Windows\System32\RuntimeBroker.exe", 0.1, 24, "Microsoft Corporation"),
    ("Slack.exe", r"C:\Users\demo\AppData\Local\slack\app-4.41.105\slack.exe", 0.1, 231, "Slack Technologies"),
    ("AdobeCollabSync.exe", r"C:\Program Files\Adobe\Acrobat DC\Acrobat\AdobeCollabSync.exe", 0.1, 31, "Adobe Inc."),
    ("ctfmon.exe", r"C:\Windows\System32\ctfmon.exe", 0.1, 17, "Microsoft Corporation"),
    ("winkillswitch.exe", r"C:\Program Files\WinKillSwitch\winkillswitch.exe", 0.1, 64, "WinKillSwitch"),
    ("csrss.exe", r"C:\Windows\System32\csrss.exe", 0.0, 6, "Microsoft Corporation"),
    ("NVDisplay.Container.exe", r"C:\Windows\System32\DriverStore\NVDisplay.Container.exe", 0.0, 18, "NVIDIA Corporation"),
    ("Zoom.exe", r"C:\Users\demo\AppData\Roaming\Zoom\bin\Zoom.exe", 0.0, 112, "Zoom Video Communications"),
    ("lsass.exe", r"C:\Windows\System32\lsass.exe", 0.0, 21, "Microsoft Corporation"),
];

fn member(name: &str, exe: &str) -> Member {
    Member { name: name.to_string(), exe: Some(exe.to_lowercase()) }
}

pub fn apply(s: &mut App) {
    s.demo = true;
    let procs = PROCS
        .iter()
        .enumerate()
        .map(|(i, (name, exe, cpu, mb, publisher))| {
            let kind = if exe.starts_with(r"C:\Windows") { ProcKind::Windows } else { ProcKind::App };
            ProcInfo {
                pid: 1200 + (i as u32 * 1373) % 30000,
                parent: None,
                name: name.to_string(),
                exe: Some(PathBuf::from(exe)),
                cpu: *cpu,
                mem: mb * 1024 * 1024,
                start: 0,
                disk: 0,
                status: "Running",
                kind,
                cmd: String::new(),
                session: Some(1),
                publisher: publisher.to_string(),
                description: name.trim_end_matches(".exe").to_string(),
            }
        })
        .collect();
    let mut procs: Vec<ProcInfo> = procs;
    // Chrome's helpers are children of its first process, which started a few hours ago.
    let chrome_pid = procs[0].pid;
    for p in procs.iter_mut().skip(1).filter(|p| p.name == "chrome.exe") {
        p.parent = Some(chrome_pid);
    }
    procs[0].start = crate::model::now_secs().saturating_sub(3 * 3600 + 25 * 60);
    procs[0].cmd = r#""C:\Program Files\Google\Chrome\Application\chrome.exe" --profile-directory=Default --restore-last-session"#.into();
    procs[0].description = "Google Chrome".into();
    s.snap = Snapshot { procs, total_mem: 32 * 1024 * 1024 * 1024, used_mem: 15 * 1024 * 1024 * 1024, cpu_total: 18.0 };

    let chrome = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
    s.groups = vec![
        Group {
            id: 1,
            name: "Browsers".into(),
            members: vec![member("chrome.exe", chrome), member("GoogleUpdater.exe", r"C:\Program Files (x86)\Google\GoogleUpdater\GoogleUpdater.exe")],
            pause: None,
        },
        Group {
            id: 2,
            name: "Game launchers".into(),
            members: vec![
                member("steam.exe", r"C:\Program Files (x86)\Steam\steam.exe"),
                member("steamwebhelper.exe", r"C:\Program Files (x86)\Steam\bin\cef\cef.win7x64\steamwebhelper.exe"),
                member("EpicGamesLauncher.exe", r"C:\Program Files (x86)\Epic Games\Launcher\Portal\Binaries\Win64\EpicGamesLauncher.exe"),
            ],
            pause: Some(PauseRecord::default()),
        },
        Group {
            id: 3,
            name: "Chat".into(),
            members: vec![
                member("Discord.exe", r"C:\Users\demo\AppData\Local\Discord\app-1.0.9200\Discord.exe"),
                member("Slack.exe", r"C:\Users\demo\AppData\Local\slack\app-4.41.105\slack.exe"),
                member("Teams.exe", r"C:\Program Files\WindowsApps\MSTeams\ms-teams.exe"),
                member("Zoom.exe", r"C:\Users\demo\AppData\Roaming\Zoom\bin\Zoom.exe"),
            ],
            pause: None,
        },
    ];
    s.next_id = 4;
    s.selected_group = Some(1);
    s.active_pid = s.snap.procs.first().map(|p| p.pid);
    s.elevated = true;

    let env = |k: &str| std::env::var(k).ok();
    if let Some(p) = env("WKS_PALETTE") {
        s.appearance.palette = p;
    }
    if let Some(a) = env("WKS_ACCENT").and_then(|a| a.parse().ok()) {
        s.appearance.accent = a;
    }
    s.appearance.glass = match env("WKS_GLASS").as_deref() {
        Some("fluted") => GlassStyle::Fluted,
        Some("frosted") => GlassStyle::Frosted,
        _ => GlassStyle::Liquid,
    };
    s.appearance.mode = crate::model::ThemeMode::System;
    s.tab = match env("WKS_TAB").as_deref() {
        Some("settings") => Tab::Settings,
        Some("pause") => Tab::PauseSwitch,
        Some("clean") => Tab::Clean,
        _ => Tab::Processes,
    };
    // A 32 GB machine, just after a clean: the history dips and the result shows.
    use crate::sys::memory::{MemStatus, RamReport};
    let gb = 1024 * 1024 * 1024u64;
    let before = MemStatus { total: 32 * gb, available: 13 * gb, cache: 4 * gb };
    let after = MemStatus { total: 32 * gb, available: 13 * gb + gb * 6 / 10, cache: gb * 12 / 10 };
    s.mem = after;
    s.mem_shown = after.used_frac();
    s.cache_shown = after.cache as f32 / after.total as f32;
    s.mem_hist = (0..60).map(|i| if i < 44 { 0.595 + 0.006 * ((i as f32) * 0.7).sin() } else { 0.575 + 0.002 * ((i as f32) * 0.9).sin() }).collect();
    s.ram.last = Some(RamReport { before, after, trimmed: 214, skipped: 9, purged_cache: true });
    s.ram.shown_at = -100.0;

    // The disk cleaner: WKS_DISK=ready shows a reviewed list, WKS_DISK=done a finished clean.
    use crate::app::{DiskItem, DiskPhase};
    use crate::sys::cleanup::{Cleaned, Found};
    let mb = 1024 * 1024u64;
    let sizes = [("temp", 4_812 * mb, true), ("chrome", 1_903 * mb, true), ("edge", 344 * mb, true), ("wintemp", 391 * mb, true),
        ("thumbs", 212 * mb, true), ("crash", 486 * mb, true), ("shaders", 702 * mb, false), ("bin", 2_640 * mb, false)];
    let cats = crate::sys::cleanup::categories();
    s.disk.items = sizes
        .iter()
        .filter_map(|(id, bytes, on)| {
            let cat = cats.iter().find(|c| c.id == *id)?.clone();
            Some(DiskItem { cat, found: Found { bytes: *bytes, files: bytes / (180 * 1024), blocked: None }, selected: *on })
        })
        .collect();
    s.disk.drive = (212 * gb, 952 * gb);
    match std::env::var("WKS_DISK").as_deref() {
        Ok("ready") => s.disk.phase = DiskPhase::Ready,
        Ok("done") => {
            s.disk.results = s.disk.items.iter().filter(|i| i.selected).map(|i| (i.cat.id.to_string(), Cleaned { freed: i.found.bytes * 97 / 100, deleted: i.found.files, skipped: 41 })).collect();
            let freed = s.disk.results.iter().map(|(_, r)| r.freed).sum();
            s.disk.drive = (212 * gb + freed, 952 * gb);
            s.disk.phase = DiskPhase::Done { freed, deleted: 48_210, skipped: 41, at: -100.0 };
        }
        _ => {}
    }
    if std::env::var("WKS_INFO").is_ok() {
        use crate::sys::info::{Account, Detail, Signature, Verdict};
        s.info = Some(crate::app::InfoState {
            pid: chrome_pid,
            detail: Some(Detail {
                pid: chrome_pid,
                signature: Signature::Valid,
                account: Account::User,
                elevated: false,
                protected: false,
                verdict: Verdict::Ok,
                reason: "You have the rights to manage it from here.".into(),
            }),
        });
    }
    s.rebuild_rows();
}
