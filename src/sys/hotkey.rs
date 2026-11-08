//! The global shortcut (Ctrl+Shift+K) that brings WinKillSwitch forward with the quick palette
//! open, from anywhere. A small thread owns the registration and its message loop; the app can
//! turn it on and off at runtime.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, RegisterHotKey, UnregisterHotKey};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, PostThreadMessageW, WM_APP, WM_HOTKEY};

pub const LABEL: &str = "Ctrl+Shift+K";
const ID: i32 = 0x574B; // "WK"
const WM_SET: u32 = WM_APP + 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Off,
    Active,
    /// Another app already registered this shortcut.
    Taken,
}

static STATUS: AtomicU8 = AtomicU8::new(0);
static THREAD: AtomicU32 = AtomicU32::new(0);
static STARTED: OnceLock<()> = OnceLock::new();

pub fn status() -> Status {
    match STATUS.load(Ordering::Relaxed) {
        1 => Status::Active,
        2 => Status::Taken,
        _ => Status::Off,
    }
}

fn register() {
    // SAFETY: registers a thread hotkey (no window) for the calling thread.
    let ok = unsafe { RegisterHotKey(std::ptr::null_mut(), ID, MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT, u32::from(b'K')) } != 0;
    STATUS.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
}

fn unregister() {
    // SAFETY: removes this thread's registration, harmless if there is none.
    unsafe {
        UnregisterHotKey(std::ptr::null_mut(), ID);
    }
    STATUS.store(0, Ordering::Relaxed);
}

/// Start the shortcut thread once; `on_press` runs on that thread for every press.
pub fn start(enabled: bool, on_press: impl Fn() + Send + 'static) {
    if STARTED.set(()).is_err() {
        return;
    }
    std::thread::Builder::new()
        .name("wks-hotkey".into())
        .spawn(move || {
            // SAFETY: the id of the current thread, used to post it messages.
            THREAD.store(unsafe { GetCurrentThreadId() }, Ordering::Relaxed);
            if enabled {
                register();
            }
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            // SAFETY: a standard message loop on this thread's own queue.
            while unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) } > 0 {
                match msg.message {
                    WM_HOTKEY if msg.wParam == ID as usize => on_press(),
                    WM_SET => {
                        if msg.wParam == 1 { register() } else { unregister() }
                    }
                    _ => {}
                }
            }
        })
        .expect("spawn hotkey thread");
}

/// Turn the shortcut on or off at runtime.
pub fn set_enabled(on: bool) {
    let tid = THREAD.load(Ordering::Relaxed);
    if tid != 0 {
        // SAFETY: posts to the hotkey thread's queue; it exists for the life of the app.
        unsafe {
            PostThreadMessageW(tid, WM_SET, usize::from(on), 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_and_can_be_switched_off() {
        start(true, || {});
        std::thread::sleep(std::time::Duration::from_millis(300));
        // Active, or Taken when a running WinKillSwitch already holds the shortcut.
        assert_ne!(status(), Status::Off);
        set_enabled(false);
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert_eq!(status(), Status::Off);
    }
}
