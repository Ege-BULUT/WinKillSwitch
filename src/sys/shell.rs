//! Handing things to Windows: open a folder with a file selected, or a web page.
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
    s.encode_wide().chain(Some(0)).collect()
}

/// Open File Explorer with `file` selected.
pub fn reveal(file: &Path) {
    let _ = std::process::Command::new("explorer.exe").arg(format!("/select,{}", file.display())).spawn();
}

/// Open a web address in the default browser. Only `https://` addresses are accepted.
pub fn open_url(url: &str) {
    if !url.starts_with("https://") {
        return;
    }
    let (verb, target) = (wide("open".as_ref()), wide(url.as_ref()));
    // SAFETY: NUL-terminated buffers that outlive the call.
    unsafe {
        ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), target.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
    }
}

/// Percent-encode a search query for a URL.
pub fn query(q: &str) -> String {
    q.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            b' ' => "+".into(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}
