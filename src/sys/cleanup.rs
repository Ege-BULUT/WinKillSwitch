//! The disk cleaner: finds regenerable junk (temporary files, caches, crash dumps, update
//! downloads) and removes it.
//!
//! Rules that keep it safe:
//! - Only files under the fixed folders listed in `categories` are touched, never the folders
//!   themselves, and roots come from Windows' own environment variables.
//! - Symbolic links and junctions are never followed or deleted through.
//! - Temporary folders skip anything changed in the last 7 days (the same rule as Windows Disk
//!   Cleanup), so installers and apps that keep working files there are not disturbed. Files in
//!   use simply fail to delete and are counted as skipped.
//! - A browser's cache is skipped while that browser runs.
//! - The Recycle Bin and anything that makes the next launch slower start unselected.

use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use windows_sys::Win32::UI::Shell::{SHEmptyRecycleBinW, SHQueryRecycleBinW, SHQUERYRBINFO, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND};

const REPARSE_POINT: u32 = 0x400;
const DAY: Duration = Duration::from_secs(24 * 3600);
const WEEK: Duration = Duration::from_secs(7 * 24 * 3600);
/// How often scans and cleans report progress, in files.
const TICK_EVERY: u64 = 1500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Temp,
    Windows,
    Crash,
    Thumbs,
    Browser,
    Shader,
    Update,
    Bin,
}

#[derive(Clone, Debug)]
pub struct Category {
    pub id: &'static str,
    pub name: String,
    pub detail: &'static str,
    pub icon: Icon,
    /// Folders whose contents are cleaned.
    pub roots: Vec<PathBuf>,
    /// Only files whose names start with one of these (empty: every file).
    pub prefixes: &'static [&'static str],
    /// Skip files modified more recently than this.
    pub min_age: Option<Duration>,
    pub needs_admin: bool,
    pub default_on: bool,
    /// Skip while any of these processes run (lowercase image names).
    pub blockers: &'static [&'static str],
    pub recycle_bin: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Found {
    pub bytes: u64,
    pub files: u64,
    /// Why this category cannot be cleaned right now, if it cannot.
    pub blocked: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Cleaned {
    pub freed: u64,
    pub deleted: u64,
    pub skipped: u64,
}

fn env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from).filter(|p| p.is_absolute())
}

fn existing(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.into_iter().filter(|p| p.is_dir()).collect()
}

/// Chromium profile folders under `user_data` (Default, Profile 1, ...), each cache inside.
fn chromium_caches(user_data: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(user_data) else { return out };
    for e in rd.flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if n == "Default" || n.starts_with("Profile ") || n == "Guest Profile" {
            for sub in [r"Cache\Cache_Data", "Code Cache", "GPUCache"] {
                out.push(e.path().join(sub));
            }
        }
    }
    out
}

pub fn categories() -> Vec<Category> {
    let local = env("LOCALAPPDATA");
    let windir = env("SystemRoot").unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let pdata = env("ProgramData");
    let l = |p: &str| local.as_ref().map(|b| b.join(p));
    let mut cats = vec![
        Category {
            id: "temp",
            name: "Temporary files".into(),
            detail: "Files apps left in your temp folder. Anything changed in the last week is kept.",
            icon: Icon::Temp,
            roots: existing(env("TEMP").into_iter().collect()),
            prefixes: &[],
            min_age: Some(WEEK),
            needs_admin: false,
            default_on: true,
            blockers: &[],
            recycle_bin: false,
        },
        Category {
            id: "wintemp",
            name: "Windows temporary files".into(),
            detail: "The system temp folder. Needs administrator.",
            icon: Icon::Windows,
            roots: existing(vec![windir.join("Temp")]),
            prefixes: &[],
            min_age: Some(WEEK),
            needs_admin: true,
            default_on: true,
            blockers: &[],
            recycle_bin: false,
        },
        Category {
            id: "crash",
            name: "Crash dumps and error reports".into(),
            detail: "Memory dumps and Windows Error Reporting files from past crashes.",
            icon: Icon::Crash,
            roots: existing(
                [
                    l("CrashDumps"),
                    l(r"Microsoft\Windows\WER\ReportArchive"),
                    l(r"Microsoft\Windows\WER\ReportQueue"),
                    pdata.as_ref().map(|p| p.join(r"Microsoft\Windows\WER\ReportArchive")),
                    pdata.as_ref().map(|p| p.join(r"Microsoft\Windows\WER\ReportQueue")),
                ]
                .into_iter()
                .flatten()
                .collect(),
            ),
            prefixes: &[],
            min_age: None,
            needs_admin: false,
            default_on: true,
            blockers: &[],
            recycle_bin: false,
        },
        Category {
            id: "thumbs",
            name: "Thumbnail cache".into(),
            detail: "Explorer rebuilds these previews when you open a folder. Files Explorer holds open are skipped.",
            icon: Icon::Thumbs,
            roots: existing(l(r"Microsoft\Windows\Explorer").into_iter().collect()),
            prefixes: &["thumbcache_", "iconcache_"],
            min_age: None,
            needs_admin: false,
            default_on: true,
            blockers: &[],
            recycle_bin: false,
        },
    ];
    for (id, name, data, procs) in [
        ("chrome", "Chrome cache", r"Google\Chrome\User Data", &["chrome.exe"][..]),
        ("edge", "Edge cache", r"Microsoft\Edge\User Data", &["msedge.exe"][..]),
        ("brave", "Brave cache", r"BraveSoftware\Brave-Browser\User Data", &["brave.exe"][..]),
        ("vivaldi", "Vivaldi cache", r"Vivaldi\User Data", &["vivaldi.exe"][..]),
    ] {
        let Some(user_data) = l(data).filter(|p| p.is_dir()) else { continue };
        cats.push(Category {
            id,
            name: name.into(),
            detail: "Cached web pages and scripts. Sites load a little slower the first time after cleaning.",
            icon: Icon::Browser,
            roots: existing(chromium_caches(&user_data)),
            prefixes: &[],
            min_age: None,
            needs_admin: false,
            default_on: true,
            blockers: procs,
            recycle_bin: false,
        });
    }
    if let Some(profiles) = l(r"Mozilla\Firefox\Profiles").filter(|p| p.is_dir()) {
        let roots = std::fs::read_dir(&profiles).map(|rd| rd.flatten().map(|e| e.path().join("cache2")).collect()).unwrap_or_default();
        cats.push(Category {
            id: "firefox",
            name: "Firefox cache".into(),
            detail: "Cached web pages and scripts. Sites load a little slower the first time after cleaning.",
            icon: Icon::Browser,
            roots: existing(roots),
            prefixes: &[],
            min_age: None,
            needs_admin: false,
            default_on: true,
            blockers: &["firefox.exe"],
            recycle_bin: false,
        });
    }
    cats.extend([
        Category {
            id: "shaders",
            name: "Graphics shader caches".into(),
            detail: "DirectX, NVIDIA and AMD shader caches. Games rebuild them, which can stutter for a while.",
            icon: Icon::Shader,
            roots: existing(
                [l("D3DSCache"), l(r"NVIDIA\DXCache"), l(r"NVIDIA\GLCache"), l(r"AMD\DxCache"), l(r"AMD\DxcCache")].into_iter().flatten().collect(),
            ),
            prefixes: &[],
            min_age: None,
            needs_admin: false,
            default_on: false,
            blockers: &[],
            recycle_bin: false,
        },
        Category {
            id: "updates",
            name: "Windows Update downloads".into(),
            detail: "Installers Windows Update already used. Needs administrator.",
            icon: Icon::Update,
            roots: existing(vec![
                windir.join(r"SoftwareDistribution\Download"),
                windir.join(r"ServiceProfiles\NetworkService\AppData\Local\Microsoft\Windows\DeliveryOptimization\Cache"),
            ]),
            prefixes: &[],
            min_age: Some(DAY),
            needs_admin: true,
            default_on: false,
            blockers: &[],
            recycle_bin: false,
        },
        Category {
            id: "bin",
            name: "Recycle Bin".into(),
            detail: "Files you deleted. Emptying it is permanent.",
            icon: Icon::Bin,
            roots: Vec::new(),
            prefixes: &[],
            min_age: None,
            needs_admin: false,
            default_on: false,
            blockers: &[],
            recycle_bin: true,
        },
    ]);
    cats
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn recycle_bin_size() -> (u64, u64) {
    let mut info: SHQUERYRBINFO = unsafe { std::mem::zeroed() };
    info.cbSize = size_of::<SHQUERYRBINFO>() as u32;
    // SAFETY: a null root queries every drive; info is sized as the API requires.
    let hr = unsafe { SHQueryRecycleBinW(std::ptr::null(), &mut info) };
    if hr < 0 { (0, 0) } else { (info.i64Size.max(0) as u64, info.i64NumItems.max(0) as u64) }
}

/// Walk a folder without following links, calling `f` for each eligible regular file.
fn walk(root: &Path, cat: &Category, now: SystemTime, f: &mut dyn FnMut(&Path, u64)) {
    let Ok(rd) = std::fs::read_dir(root) else { return };
    for e in rd.flatten() {
        // On Windows a DirEntry's metadata comes from the directory listing itself: no extra
        // system call, and it describes a link rather than following it.
        let Ok(md) = e.metadata() else { continue };
        if md.file_type().is_symlink() || md.file_attributes() & REPARSE_POINT != 0 {
            continue;
        }
        if md.is_dir() {
            walk(&e.path(), cat, now, f);
            continue;
        }
        if !cat.prefixes.is_empty() {
            let n = e.file_name().to_string_lossy().to_lowercase();
            if !cat.prefixes.iter().any(|p| n.starts_with(p)) {
                continue;
            }
        }
        if let (Some(age), Ok(m)) = (cat.min_age, md.modified()) {
            if now.duration_since(m).unwrap_or_default() < age {
                continue;
            }
        }
        f(&e.path(), md.len());
    }
}

/// Remove now-empty folders under `root`, never `root` itself.
fn prune(root: &Path) {
    let Ok(rd) = std::fs::read_dir(root) else { return };
    for e in rd.flatten() {
        let Ok(md) = std::fs::symlink_metadata(e.path()) else { continue };
        if md.is_dir() && !md.file_type().is_symlink() && md.file_attributes() & REPARSE_POINT == 0 {
            prune(&e.path());
            let _ = std::fs::remove_dir(e.path()); // fails harmlessly unless empty
        }
    }
}

fn blocked(cat: &Category, running: &[String], elevated: bool) -> Option<String> {
    if cat.needs_admin && !elevated {
        return Some("Needs administrator".into());
    }
    if let Some(p) = cat.blockers.iter().find(|b| running.iter().any(|r| r == *b)) {
        let app = p.trim_end_matches(".exe");
        return Some(format!("Close {} to include", app[..1].to_uppercase() + &app[1..]));
    }
    None
}

pub fn scan(cat: &Category, running: &[String], elevated: bool) -> Found {
    scan_with(cat, running, elevated, &mut |_, _| {})
}

/// `scan`, calling `tick(files, bytes)` every so often while it walks.
pub fn scan_with(cat: &Category, running: &[String], elevated: bool, tick: &mut dyn FnMut(u64, u64)) -> Found {
    if cat.recycle_bin {
        let (bytes, files) = recycle_bin_size();
        return Found { bytes, files, blocked: None };
    }
    let now = SystemTime::now();
    let (mut bytes, mut files) = (0, 0);
    for r in &cat.roots {
        walk(r, cat, now, &mut |_, len| {
            bytes += len;
            files += 1;
            if files % TICK_EVERY == 0 {
                tick(files, bytes);
            }
        });
    }
    Found { bytes, files, blocked: blocked(cat, running, elevated) }
}

pub fn clean(cat: &Category, running: &[String], elevated: bool) -> Cleaned {
    clean_with(cat, running, elevated, &mut |_| {})
}

/// `clean`, calling `tick(freed)` every so often while it deletes.
pub fn clean_with(cat: &Category, running: &[String], elevated: bool, tick: &mut dyn FnMut(u64)) -> Cleaned {
    if blocked(cat, running, elevated).is_some() {
        return Cleaned::default();
    }
    if cat.recycle_bin {
        let (bytes, files) = recycle_bin_size();
        let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;
        // SAFETY: empties the bin on every drive, without UI.
        let hr = unsafe { SHEmptyRecycleBinW(std::ptr::null_mut(), wide("").as_ptr(), flags) };
        return if hr >= 0 { Cleaned { freed: bytes, deleted: files, skipped: 0 } } else { Cleaned { freed: 0, deleted: 0, skipped: files } };
    }
    let now = SystemTime::now();
    let mut out = Cleaned::default();
    for r in &cat.roots {
        walk(r, cat, now, &mut |path, len| {
            match std::fs::remove_file(path) {
                Ok(()) => {
                    out.freed += len;
                    out.deleted += 1;
                }
                Err(_) => out.skipped += 1,
            }
            if (out.deleted + out.skipped) % TICK_EVERY == 0 {
                tick(out.freed);
            }
        });
        if cat.prefixes.is_empty() {
            prune(r);
        }
    }
    out
}

/// Free and total bytes on the system drive.
pub fn system_drive() -> (u64, u64) {
    let root = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into()) + "\\";
    let (mut free, mut total) = (0u64, 0u64);
    // SAFETY: valid NUL-terminated path and out-pointers.
    unsafe {
        windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(wide(&root).as_ptr(), std::ptr::null_mut(), &mut total, &mut free);
    }
    (free, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::time::{Duration, SystemTime};

    fn sandbox(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("wks-cleanup-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn file(path: &Path, bytes: usize, age: Duration) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![7u8; bytes]).unwrap();
        File::options().write(true).open(path).unwrap().set_modified(SystemTime::now() - age).unwrap();
    }

    fn cat(roots: Vec<PathBuf>, min_age: Option<Duration>, prefixes: &'static [&'static str]) -> Category {
        Category {
            id: "test",
            name: "Test".into(),
            detail: "",
            icon: Icon::Temp,
            roots,
            prefixes,
            min_age,
            needs_admin: false,
            default_on: true,
            blockers: &["blocker.exe"],
            recycle_bin: false,
        }
    }

    #[test]
    fn keeps_recent_files_and_the_root() {
        let root = sandbox("age");
        file(&root.join("old.tmp"), 100, 3 * DAY);
        file(&root.join(r"sub\deeper\old2.tmp"), 50, 3 * DAY);
        file(&root.join("fresh.tmp"), 30, Duration::from_secs(60));
        let c = cat(vec![root.clone()], Some(DAY), &[]);

        let found = scan(&c, &[], false);
        assert_eq!((found.files, found.bytes), (2, 150), "only the two old files count");

        let done = clean(&c, &[], false);
        assert_eq!((done.deleted, done.freed, done.skipped), (2, 150, 0));
        assert!(root.join("fresh.tmp").exists(), "a file changed in the last day is kept");
        assert!(!root.join("sub").exists(), "emptied folders are pruned");
        assert!(root.exists(), "the root folder itself is never removed");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn never_follows_a_junction_out_of_the_root() {
        let root = sandbox("junction-root");
        let outside = sandbox("junction-outside");
        file(&outside.join("precious.txt"), 10, 10 * DAY);
        file(&root.join("junk.tmp"), 10, 10 * DAY);
        let link = root.join("link");
        let ok = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J", &link.to_string_lossy(), &outside.to_string_lossy()])
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "creating a junction needs no admin rights");

        let c = cat(vec![root.clone()], None, &[]);
        assert_eq!(scan(&c, &[], false).files, 1, "the junction's contents are not counted");
        clean(&c, &[], false);
        assert!(outside.join("precious.txt").exists(), "nothing behind the junction is deleted");
        assert!(link.exists(), "the junction itself is left alone");
        let _ = fs::remove_dir(&link);
        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&outside).unwrap();
    }

    #[test]
    fn prefix_filter_only_touches_matching_files() {
        let root = sandbox("prefix");
        file(&root.join("thumbcache_96.db"), 40, DAY);
        file(&root.join("ExplorerStartupLog.etl"), 40, DAY);
        let c = cat(vec![root.clone()], None, &["thumbcache_"]);
        assert_eq!(clean(&c, &[], false).deleted, 1);
        assert!(root.join("ExplorerStartupLog.etl").exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn blocked_categories_report_why_and_clean_nothing() {
        let root = sandbox("blocked");
        file(&root.join("cache.bin"), 10, DAY);
        let c = cat(vec![root.clone()], None, &[]);
        let running = vec!["blocker.exe".to_string()];
        assert_eq!(scan(&c, &running, false).blocked.as_deref(), Some("Close Blocker to include"));
        assert_eq!(clean(&c, &running, false).deleted, 0);
        assert!(root.join("cache.bin").exists());

        let mut admin = cat(vec![root.clone()], None, &[]);
        admin.needs_admin = true;
        assert_eq!(scan(&admin, &[], false).blocked.as_deref(), Some("Needs administrator"));
        assert_eq!(clean(&admin, &[], false).deleted, 0);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn every_category_root_is_absolute_and_never_a_drive_root() {
        for c in categories() {
            for r in &c.roots {
                assert!(r.is_absolute(), "{} has a relative root {}", c.id, r.display());
                assert!(r.parent().is_some() && r.components().count() > 2, "{} would clean a drive root: {}", c.id, r.display());
            }
        }
    }
}

#[cfg(test)]
mod live {
    /// Read-only: scans every real category and prints what it would clean and how long each
    /// took. `cargo test --release scan_report -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn scan_report() {
        let elevated = crate::sys::elevate::is_elevated();
        for c in super::categories() {
            let t = std::time::Instant::now();
            let f = super::scan(&c, &[], elevated);
            println!("{:<32} {:>10} files {:>8} MB {:>7.2?} {:?}", c.name, f.files, f.bytes / 1048576, t.elapsed(), f.blocked);
        }
    }
}
