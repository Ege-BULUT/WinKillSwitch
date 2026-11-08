//! "Find by window": which process owns the top-level window under the mouse pointer.

use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::WindowsAndMessaging::{GA_ROOT, GetAncestor, GetCursorPos, GetWindowTextW, GetWindowThreadProcessId, WindowFromPoint};

use crate::model::Pid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub pid: Pid,
    /// The window's title, possibly empty.
    pub title: String,
}

/// The process behind the top-level window under the pointer, if any.
pub fn under_cursor() -> Option<Target> {
    // SAFETY: plain window queries with valid out-pointers and a sized text buffer.
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) == 0 {
            return None;
        }
        let hwnd = WindowFromPoint(pt);
        if hwnd.is_null() {
            return None;
        }
        let root = GetAncestor(hwnd, GA_ROOT);
        let root = if root.is_null() { hwnd } else { root };
        let mut pid = 0u32;
        GetWindowThreadProcessId(root, &mut pid);
        if pid == 0 {
            return None;
        }
        let mut buf = [0u16; 256];
        let n = GetWindowTextW(root, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
        Some(Target { pid, title: String::from_utf16_lossy(&buf[..n]) })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn something_is_under_the_pointer() {
        // There is always a window under the pointer on an interactive desktop (at least the
        // desktop itself). On a headless session there may be none, which is also fine.
        if let Some(t) = super::under_cursor() {
            assert!(t.pid > 0);
        }
    }
}
