//! "Who is using this file?" through the Restart Manager, the API installers and PowerToys File
//! Locksmith use. It lists the processes holding a file (or any file in a folder) open.

use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use windows_sys::Win32::System::RestartManager::{
    CCH_RM_SESSION_KEY, RM_PROCESS_INFO, RmConsole, RmCritical, RmEndSession, RmExplorer, RmGetList, RmMainWindow, RmOtherWindow, RmRegisterResources,
    RmService, RmStartSession,
};

use crate::model::Pid;

const ERROR_MORE_DATA: u32 = 234;
/// A folder is checked file by file; this caps how many.
const MAX_FILES: usize = 2000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Locker {
    pub pid: Pid,
    /// The friendly app name Windows reports ("Microsoft Word", "Windows Explorer").
    pub app: String,
    /// "App", "Service", "Explorer", "Console" or "Critical".
    pub kind: &'static str,
    /// For a service, its short name.
    pub service: String,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub target: PathBuf,
    pub lockers: Vec<Locker>,
    /// Files checked (1 for a file).
    pub files: usize,
    /// The folder had more files than were checked.
    pub truncated: bool,
    pub error: Option<String>,
}

fn wide(p: &Path) -> Vec<u16> {
    p.as_os_str().encode_wide().chain(Some(0)).collect()
}

fn files_in(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        if out.len() >= MAX_FILES {
            return;
        }
        let Ok(md) = e.metadata() else { continue };
        if md.file_type().is_symlink() {
            continue;
        }
        if md.is_dir() {
            files_in(&e.path(), out);
        } else {
            out.push(e.path());
        }
    }
}

fn utf16(buf: &[u16]) -> String {
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..n])
}

/// Every process holding `target` open (or any file under it, for a folder).
pub fn who(target: &Path) -> Report {
    let mut report = Report { target: target.to_path_buf(), ..Default::default() };
    let mut files = Vec::new();
    if target.is_dir() {
        files_in(target, &mut files);
        report.truncated = files.len() >= MAX_FILES;
    } else if target.exists() {
        files.push(target.to_path_buf());
    } else {
        report.error = Some("That file no longer exists.".into());
        return report;
    }
    report.files = files.len();
    if files.is_empty() {
        return report;
    }
    let wides: Vec<Vec<u16>> = files.iter().map(|f| wide(f)).collect();
    let ptrs: Vec<*const u16> = wides.iter().map(|w| w.as_ptr()).collect();

    // SAFETY: the session is ended on every path; the name pointers outlive registration; the
    // info buffer is sized by RmGetList's own count and read only for the entries it filled.
    unsafe {
        let mut session = 0u32;
        let mut key = [0u16; CCH_RM_SESSION_KEY as usize + 1];
        if RmStartSession(&mut session, 0, key.as_mut_ptr()) != 0 {
            report.error = Some("Windows could not start a Restart Manager session.".into());
            return report;
        }
        let end = |s| {
            RmEndSession(s);
        };
        if RmRegisterResources(session, ptrs.len() as u32, ptrs.as_ptr(), 0, std::ptr::null(), 0, std::ptr::null()) != 0 {
            end(session);
            report.error = Some("Windows could not check this file.".into());
            return report;
        }
        let mut needed = 0u32;
        let mut count = 0u32;
        let mut reasons = 0u32;
        let mut infos: Vec<RM_PROCESS_INFO> = Vec::new();
        let mut rc = RmGetList(session, &mut needed, &mut count, std::ptr::null_mut(), &mut reasons);
        if rc == ERROR_MORE_DATA {
            infos = vec![std::mem::zeroed(); needed as usize];
            count = needed;
            rc = RmGetList(session, &mut needed, &mut count, infos.as_mut_ptr(), &mut reasons);
        }
        end(session);
        if rc != 0 {
            report.error = Some(format!("Windows could not list the processes (error {rc})."));
            return report;
        }
        for i in infos.iter().take(count as usize) {
            #[allow(non_upper_case_globals)]
            let kind = match i.ApplicationType {
                RmService => "Service",
                RmExplorer => "Explorer",
                RmConsole => "Console",
                RmCritical => "Critical",
                RmMainWindow | RmOtherWindow => "App",
                _ => "App",
            };
            let l = Locker { pid: i.Process.dwProcessId, app: utf16(&i.strAppName), kind, service: utf16(&i.strServiceShortName) };
            if !report.lockers.iter().any(|x| x.pid == l.pid) {
                report.lockers.push(l);
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;

    #[test]
    fn finds_ourselves_holding_a_file() {
        let path = std::env::temp_dir().join(format!("wks-lock-{}.txt", std::process::id()));
        std::fs::write(&path, b"held").unwrap();
        // Keep it open without sharing, the way an app that "locks" a file does.
        let held = std::fs::OpenOptions::new().read(true).share_mode(0).open(&path).unwrap();
        let r = who(&path);
        assert!(r.error.is_none(), "{:?}", r.error);
        assert!(r.lockers.iter().any(|l| l.pid == std::process::id()), "{:?}", r.lockers);
        drop(held);
        let r = who(&path);
        assert!(r.lockers.iter().all(|l| l.pid != std::process::id()), "released after closing");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn checks_files_inside_a_folder() {
        let dir = std::env::temp_dir().join(format!("wks-lockdir-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        let path = dir.join("sub").join("inner.txt");
        std::fs::write(&path, b"x").unwrap();
        let held = std::fs::OpenOptions::new().read(true).share_mode(0).open(&path).unwrap();
        let r = who(&dir);
        assert_eq!(r.files, 1);
        assert!(r.lockers.iter().any(|l| l.pid == std::process::id()));
        drop(held);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_file_says_so() {
        assert!(who(Path::new(r"C:\definitely\not\here.txt")).error.is_some());
    }
}
