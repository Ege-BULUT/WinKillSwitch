//! "What is this process?": its signature, who it runs as, whether Windows protects it, and a
//! plain-language verdict on whether WinKillSwitch can manage it (and if not, why).

use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LocalFree};
use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows_sys::Win32::Security::WinTrust::{
    WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0, WINTRUST_FILE_INFO, WTD_CHOICE_FILE, WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE,
    WTD_STATEACTION_VERIFY, WTD_UI_NONE, WinVerifyTrust,
};
use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TOKEN_USER, TokenElevation, TokenUser};
use windows_sys::Win32::System::Threading::{
    GetProcessInformation, OpenProcess, OpenProcessToken, PROCESS_PROTECTION_LEVEL_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, ProcessProtectionLevelInfo,
};

use crate::model::Pid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signature {
    /// Signed, and Windows trusts the signature.
    Valid,
    /// No signature at all.
    Unsigned,
    /// Signed, but the signature is broken or untrusted.
    Invalid,
    /// No file to check.
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Account {
    System,
    LocalService,
    NetworkService,
    User,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// WinKillSwitch can manage it.
    Ok,
    /// Windows itself depends on it.
    Critical,
    /// A protected process: no app can manage it, administrator or not.
    Protected,
    /// Runs above us; restarting WinKillSwitch as administrator helps.
    NeedsAdmin,
    /// Windows refused even a basic query.
    Denied,
}

impl Verdict {
    pub fn headline(self) -> &'static str {
        match self {
            Verdict::Ok => "WinKillSwitch can manage this process",
            Verdict::Critical => "Windows needs this process to run",
            Verdict::Protected => "Windows protects this process",
            Verdict::NeedsAdmin => "Needs administrator",
            Verdict::Denied => "Windows denied access",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Detail {
    pub pid: Pid,
    pub signature: Signature,
    pub account: Account,
    /// The process runs elevated (as administrator).
    pub elevated: bool,
    pub protected: bool,
    pub verdict: Verdict,
    /// Why, in a sentence, and what the user can do.
    pub reason: String,
}

/// Processes Windows cannot run without. Ending one crashes or signs out the session.
const CRITICAL: &[&str] = &["system", "registry", "memory compression", "smss.exe", "csrss.exe", "wininit.exe", "winlogon.exe", "services.exe", "lsass.exe", "lsaiso.exe"];

fn wide_path(p: &Path) -> Vec<u16> {
    p.as_os_str().encode_wide().chain(Some(0)).collect()
}

pub fn signature(exe: &Path) -> Signature {
    if !exe.is_file() {
        return Signature::Unknown;
    }
    let path = wide_path(exe);
    // SAFETY: structures are zeroed then sized; the file path outlives both calls; the second
    // call releases the state the first one allocated.
    unsafe {
        let mut file: WINTRUST_FILE_INFO = std::mem::zeroed();
        file.cbStruct = size_of::<WINTRUST_FILE_INFO>() as u32;
        file.pcwszFilePath = path.as_ptr();
        let mut data: WINTRUST_DATA = std::mem::zeroed();
        data.cbStruct = size_of::<WINTRUST_DATA>() as u32;
        data.dwUIChoice = WTD_UI_NONE;
        data.fdwRevocationChecks = WTD_REVOKE_NONE;
        data.dwUnionChoice = WTD_CHOICE_FILE;
        data.Anonymous = WINTRUST_DATA_0 { pFile: &mut file };
        data.dwStateAction = WTD_STATEACTION_VERIFY;
        let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        let status = WinVerifyTrust(std::ptr::null_mut(), &mut action, (&mut data as *mut WINTRUST_DATA).cast());
        data.dwStateAction = WTD_STATEACTION_CLOSE;
        WinVerifyTrust(std::ptr::null_mut(), &mut action, (&mut data as *mut WINTRUST_DATA).cast());
        match status as u32 {
            0 => Signature::Valid,
            0x800B0100 => Signature::Unsigned, // TRUST_E_NOSIGNATURE
            _ => Signature::Invalid,
        }
    }
}

/// Who the process runs as and whether it is elevated, from its token.
fn token_facts(h: HANDLE) -> (Account, bool) {
    // SAFETY: token opened for query only, buffers sized by the API, everything freed.
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(h, TOKEN_QUERY, &mut token) == 0 {
            return (Account::Unknown, false);
        }
        let mut elev = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut len = 0u32;
        let elevated = GetTokenInformation(token, TokenElevation, (&mut elev as *mut TOKEN_ELEVATION).cast(), size_of::<TOKEN_ELEVATION>() as u32, &mut len) != 0
            && elev.TokenIsElevated != 0;
        let mut buf = vec![0u8; 256];
        let mut account = Account::Unknown;
        if GetTokenInformation(token, TokenUser, buf.as_mut_ptr().cast(), buf.len() as u32, &mut len) != 0 {
            let user = &*(buf.as_ptr() as *const TOKEN_USER);
            let mut s: *mut u16 = std::ptr::null_mut();
            if ConvertSidToStringSidW(user.User.Sid, &mut s) != 0 {
                let mut n = 0;
                while *s.add(n) != 0 {
                    n += 1;
                }
                let sid = String::from_utf16_lossy(std::slice::from_raw_parts(s, n));
                LocalFree(s.cast());
                account = match sid.as_str() {
                    "S-1-5-18" => Account::System,
                    "S-1-5-19" => Account::LocalService,
                    "S-1-5-20" => Account::NetworkService,
                    _ => Account::User,
                };
            }
        }
        CloseHandle(token);
        (account, elevated)
    }
}

fn protected(h: HANDLE) -> bool {
    // SAFETY: plain query into a sized struct.
    unsafe {
        let mut info: PROCESS_PROTECTION_LEVEL_INFORMATION = std::mem::zeroed();
        let ok = GetProcessInformation(h, ProcessProtectionLevelInfo, (&mut info as *mut PROCESS_PROTECTION_LEVEL_INFORMATION).cast(), size_of::<PROCESS_PROTECTION_LEVEL_INFORMATION>() as u32);
        // PROTECTION_LEVEL_NONE
        ok != 0 && info.ProtectionLevel != 0xFFFF_FFFE
    }
}

/// Everything slow about one process: run it off the UI thread.
pub fn inspect(pid: Pid, name: &str, exe: Option<&Path>, we_are_admin: bool) -> Detail {
    let signature = exe.map(signature).unwrap_or(Signature::Unknown);
    let lname = name.to_lowercase();
    // SAFETY: open for limited query only; closed below.
    let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    let (account, elevated, is_protected, opened) = if h.is_null() {
        (Account::Unknown, false, false, false)
    } else {
        let (a, e) = token_facts(h);
        let p = protected(h);
        // SAFETY: handle from OpenProcess above.
        unsafe { CloseHandle(h) };
        (a, e, p, true)
    };

    let service = matches!(account, Account::System | Account::LocalService | Account::NetworkService);
    let (verdict, reason) = if CRITICAL.contains(&lname.as_str()) || pid <= 4 {
        (Verdict::Critical, "Ending it would crash Windows or sign you out, so WinKillSwitch leaves it alone.".to_string())
    } else if is_protected {
        (Verdict::Protected, "It is a protected process. Windows keeps every app from ending or reading it, even apps running as administrator. Security software and DRM often run this way.".to_string())
    } else if !opened && !we_are_admin {
        (Verdict::NeedsAdmin, "It runs with more privileges than WinKillSwitch has right now. Restart WinKillSwitch as administrator to manage it.".to_string())
    } else if !opened {
        (Verdict::Denied, "Windows refused access even to an administrator. It may belong to another signed-in user or be shutting down.".to_string())
    } else if (service || elevated) && !we_are_admin {
        let who = if service { "as a Windows service account" } else { "as administrator" };
        (Verdict::NeedsAdmin, format!("It runs {who}. Restart WinKillSwitch as administrator to manage it."))
    } else {
        (Verdict::Ok, "You have the rights to manage it from here.".to_string())
    };
    Detail { pid, signature, account, elevated, protected: is_protected, verdict, reason }
}

/// A short explanation of well-known Windows processes, and whether ending them is safe.
pub fn known(name: &str) -> Option<(&'static str, &'static str)> {
    let n = name.to_lowercase();
    Some(match n.as_str() {
        "system" => ("The Windows kernel itself: drivers and core system work run here.", "Never end it."),
        "registry" => ("Holds the Windows registry in memory.", "Never end it."),
        "memory compression" => ("Keeps compressed memory pages so Windows can fit more in RAM.", "Never end it."),
        "smss.exe" => ("Session Manager: starts each Windows session at boot and sign in.", "Never end it."),
        "csrss.exe" => ("Client Server Runtime: core support for windows and consoles.", "Never end it; Windows will crash."),
        "wininit.exe" => ("Starts the core Windows services at boot.", "Never end it."),
        "winlogon.exe" => ("Handles signing in and out, and Ctrl+Alt+Del.", "Ending it signs you out."),
        "services.exe" => ("Service Control Manager: starts and stops Windows services.", "Never end it."),
        "lsass.exe" => ("Local Security Authority: checks passwords and enforces security policy.", "Never end it; Windows will restart."),
        "svchost.exe" => ("Hosts one or more Windows services. Many copies running is normal.", "Usually safe to leave; ending one stops the services inside it."),
        "dwm.exe" => ("Desktop Window Manager: draws every window, animation and transparency effect.", "Windows restarts it if it stops; the screen may flicker."),
        "explorer.exe" => ("Windows Explorer: the taskbar, Start menu and File Explorer.", "Safe to restart; the taskbar comes back."),
        "runtimebroker.exe" => ("Checks permissions for Microsoft Store apps.", "Safe; it restarts when an app needs it."),
        "searchhost.exe" => ("Windows Search on the taskbar.", "Safe; it restarts when you search."),
        "searchindexer.exe" => ("Builds the search index of your files.", "Safe to end; indexing resumes later."),
        "startmenuexperiencehost.exe" => ("The Start menu.", "Safe; it restarts when you open Start."),
        "shellexperiencehost.exe" => ("Taskbar flyouts such as the clock, network and volume panels.", "Safe; it restarts on demand."),
        "textinputhost.exe" => ("The touch keyboard, emoji panel and handwriting input.", "Safe; it restarts on demand."),
        "ctfmon.exe" => ("Text input services: keyboard layouts and input methods.", "Safe, but typing in some apps may misbehave until it restarts."),
        "sihost.exe" => ("Shell Infrastructure Host: parts of the desktop shell.", "Ending it can make the taskbar unresponsive."),
        "taskhostw.exe" => ("Runs scheduled tasks and background work for Windows.", "Usually safe."),
        "conhost.exe" => ("Hosts a console window for command line programs.", "Ends the console it belongs to."),
        "dllhost.exe" => ("Runs COM components outside the app that uses them, such as thumbnail generators.", "Usually safe."),
        "audiodg.exe" => ("Windows audio engine, including sound effects and enhancements.", "Sound stops until it restarts."),
        "fontdrvhost.exe" => ("Renders fonts in an isolated process.", "Never end it."),
        "wmiprvse.exe" => ("WMI provider: answers system information queries from apps.", "Safe; it starts again on demand."),
        "spoolsv.exe" => ("Print Spooler: queues documents for printers.", "Safe if you are not printing."),
        "msmpeng.exe" => ("Microsoft Defender Antivirus engine.", "Protected; Windows will not let apps end it."),
        "securityhealthservice.exe" => ("Windows Security status and notifications.", "Leave it running."),
        "msedgewebview2.exe" => ("Edge WebView2: web content inside other apps such as Widgets, Teams or Outlook.", "Ends the web view in whichever app uses it."),
        "msedge.exe" => ("Microsoft Edge. Each tab and extension can run in its own process.", "Safe; you may lose unsaved work in open tabs."),
        "chrome.exe" => ("Google Chrome. Each tab, extension and helper runs in its own process.", "Safe; you may lose unsaved work in open tabs."),
        "onedrive.exe" => ("OneDrive sync client.", "Safe; files stop syncing until it runs again."),
        "phoneexperiencehost.exe" => ("Phone Link: connects your phone to Windows.", "Safe."),
        "widgets.exe" => ("Windows Widgets board.", "Safe."),
        "gamebar.exe" | "gamebarftserver.exe" => ("Xbox Game Bar.", "Safe."),
        "nvdisplay.container.exe" => ("NVIDIA display driver helper and control panel services.", "Usually safe, but some driver features stop."),
        "winkillswitch.exe" => ("This app.", "Close it from its window instead."),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn our_own_process_is_manageable() {
        let me = std::process::id();
        let exe = std::env::current_exe().unwrap();
        let d = inspect(me, "winkillswitch-test.exe", Some(&exe), crate::sys::elevate::is_elevated());
        assert_eq!(d.verdict, Verdict::Ok, "{}", d.reason);
        assert_eq!(d.account, Account::User);
        assert_eq!(d.signature, Signature::Unsigned, "a local test build carries no signature");
    }

    #[test]
    fn the_kernel_is_critical() {
        assert_eq!(inspect(4, "System", None, true).verdict, Verdict::Critical);
        assert_eq!(inspect(1234, "csrss.exe", None, true).verdict, Verdict::Critical);
    }

    #[test]
    fn explorer_is_signed_by_windows() {
        let windir = std::env::var("SystemRoot").unwrap();
        assert_eq!(signature(&Path::new(&windir).join("explorer.exe")), Signature::Valid);
    }

    #[test]
    fn known_names_are_case_insensitive() {
        assert!(known("SvcHost.EXE").is_some());
        assert!(known("definitely-not-a-windows-process.exe").is_none());
    }

    /// Prints the verdict for every running process; `-- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn survey() {
        let mut sys = crate::sys::procs::Sampler::new();
        let snap = sys.sample();
        let admin = crate::sys::elevate::is_elevated();
        let mut counts = std::collections::BTreeMap::new();
        for p in &snap.procs {
            let d = inspect(p.pid, &p.name, p.exe.as_deref(), admin);
            *counts.entry(format!("{:?}", d.verdict)).or_insert(0) += 1;
            if p.name.eq_ignore_ascii_case("MsMpEng.exe") || p.name.eq_ignore_ascii_case("lsass.exe") || p.name.eq_ignore_ascii_case("explorer.exe") {
                println!("{:<14} {:?} {:?} {:?} protected={}", p.name, d.verdict, d.signature, d.account, d.protected);
            }
        }
        println!("{counts:?}");
    }
}
