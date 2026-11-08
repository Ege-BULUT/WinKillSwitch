//! Memory status and the one-tap memory cleaner.
//!
//! Cleaning never closes anything. It asks Windows to trim every process's working set (pages
//! move to the standby list and come back the moment an app touches them), and, when running as
//! administrator, flushes the modified list and purges the standby cache, the same operations
//! RAMMap and Mem Reduct offer. The gain is real but temporary: Windows refills memory as apps
//! need it, which is how it is supposed to work.

use std::time::Duration;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LUID};
use windows_sys::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows_sys::Win32::System::ProcessStatus::{EmptyWorkingSet, EnumProcesses, GetPerformanceInfo, PERFORMANCE_INFORMATION};
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA};

/// A moment of physical memory, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MemStatus {
    pub total: u64,
    /// Free plus standby: what apps can get without paging anything out.
    pub available: u64,
    /// The standby/file cache part of `available`.
    pub cache: u64,
}

impl MemStatus {
    pub fn in_use(&self) -> u64 {
        self.total.saturating_sub(self.available)
    }
    pub fn free(&self) -> u64 {
        self.available.saturating_sub(self.cache)
    }
    pub fn used_frac(&self) -> f32 {
        if self.total == 0 { 0.0 } else { self.in_use() as f32 / self.total as f32 }
    }
}

pub fn status() -> MemStatus {
    // SAFETY: both structs are plain data, sized before the call as the APIs require.
    unsafe {
        let mut m: MEMORYSTATUSEX = std::mem::zeroed();
        m.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut m) == 0 {
            return MemStatus::default();
        }
        let mut p: PERFORMANCE_INFORMATION = std::mem::zeroed();
        let cache = if GetPerformanceInfo(&mut p, size_of::<PERFORMANCE_INFORMATION>() as u32) != 0 {
            (p.SystemCache as u64 * p.PageSize as u64).min(m.ullAvailPhys)
        } else {
            0
        };
        MemStatus { total: m.ullTotalPhys, available: m.ullAvailPhys, cache }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RamReport {
    pub before: MemStatus,
    pub after: MemStatus,
    /// Processes whose working set was trimmed.
    pub trimmed: u32,
    /// Processes Windows did not let us trim (protected, or another user's without admin).
    pub skipped: u32,
    /// Whether the standby cache was purged (administrator only).
    pub purged_cache: bool,
}

impl RamReport {
    pub fn freed(&self) -> u64 {
        self.after.available.saturating_sub(self.before.available)
    }
}

// ntdll's memory list command, documented by Sysinternals RAMMap's behaviour.
const SYSTEM_MEMORY_LIST_INFORMATION: i32 = 80;
const MEMORY_FLUSH_MODIFIED_LIST: u32 = 3;
const MEMORY_PURGE_STANDBY_LIST: u32 = 4;

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtSetSystemInformation(class: i32, info: *const core::ffi::c_void, len: u32) -> i32;
}

fn enable_privilege(name: &str) -> bool {
    let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    // SAFETY: our own token, valid out-pointers, handle closed before returning.
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut luid = LUID { LowPart: 0, HighPart: 0 };
        let mut ok = LookupPrivilegeValueW(std::ptr::null(), wide.as_ptr(), &mut luid) != 0;
        if ok {
            let mut tp: TOKEN_PRIVILEGES = std::mem::zeroed();
            tp.PrivilegeCount = 1;
            tp.Privileges[0].Luid = luid;
            tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
            ok = AdjustTokenPrivileges(token, 0, &tp, 0, std::ptr::null_mut(), std::ptr::null_mut()) != 0
                && windows_sys::Win32::Foundation::GetLastError() == 0;
        }
        CloseHandle(token);
        ok
    }
}

fn memory_list_command(cmd: u32) -> bool {
    // SAFETY: the command is a 4-byte value as the information class expects.
    unsafe { NtSetSystemInformation(SYSTEM_MEMORY_LIST_INFORMATION, (&cmd as *const u32).cast(), 4) >= 0 }
}

/// Trim every working set and, as administrator, purge the standby cache. Blocks for a second or
/// two; call it off the UI thread.
pub fn clean(elevated: bool) -> RamReport {
    let before = status();
    let mut pids = vec![0u32; 4096];
    let mut needed = 0u32;
    // SAFETY: the buffer's byte size is passed; `needed` reports how much was written.
    let ok = unsafe { EnumProcesses(pids.as_mut_ptr(), (pids.len() * 4) as u32, &mut needed) } != 0;
    let count = if ok { (needed / 4) as usize } else { 0 };
    let (mut trimmed, mut skipped) = (0, 0);
    for &pid in &pids[..count] {
        if pid == 0 || pid == 4 {
            continue; // the idle process and System
        }
        // SAFETY: plain open, trim and close of a process handle.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA, 0, pid);
            if h.is_null() {
                skipped += 1;
                continue;
            }
            if EmptyWorkingSet(h) != 0 { trimmed += 1 } else { skipped += 1 }
            CloseHandle(h);
        }
    }
    let mut purged_cache = false;
    if elevated && enable_privilege("SeProfileSingleProcessPrivilege") {
        memory_list_command(MEMORY_FLUSH_MODIFIED_LIST);
        purged_cache = memory_list_command(MEMORY_PURGE_STANDBY_LIST);
    }
    // Give the memory manager a moment to move the pages before measuring.
    std::thread::sleep(Duration::from_millis(400));
    RamReport { before, after: status(), trimmed, skipped, purged_cache }
}
