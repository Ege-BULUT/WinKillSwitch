//! Reads an executable's version resource for its publisher (CompanyName) and friendly
//! description (FileDescription), the strings Task Manager shows in those columns. Results are
//! read once per path and cached, since the resource never changes while a file sits on disk.

use std::collections::HashMap;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};

#[derive(Clone, Default)]
pub struct VerInfo {
    pub publisher: String,
    pub description: String,
}

#[derive(Default)]
pub struct VerCache {
    map: HashMap<String, VerInfo>,
}

impl VerCache {
    pub fn get(&mut self, exe: &Path) -> VerInfo {
        let key = exe.to_string_lossy().to_lowercase();
        if let Some(v) = self.map.get(&key) {
            return v.clone();
        }
        let v = read(exe).unwrap_or_default();
        self.map.insert(key, v.clone());
        v
    }
}

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

fn read(exe: &Path) -> Option<VerInfo> {
    let path: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: `path` is a NUL-terminated UTF-16 buffer living across the call.
    let size = unsafe { GetFileVersionInfoSizeW(path.as_ptr(), std::ptr::null_mut()) };
    if size == 0 {
        return None;
    }
    let mut buf = vec![0u8; size as usize];
    // SAFETY: buffer is `size` bytes, exactly what the previous call reported.
    let ok = unsafe { GetFileVersionInfoW(path.as_ptr(), 0, size, buf.as_mut_ptr().cast()) };
    if ok == 0 {
        return None;
    }

    // The translation table gives the (language, codepage) pairs the strings are keyed under.
    let (lang, cp) = translation(&buf).unwrap_or((0x0409, 0x04B0));
    let query = |field: &str| -> String {
        let sub = format!("\\StringFileInfo\\{lang:04x}{cp:04x}\\{field}");
        string_value(&buf, &sub).unwrap_or_default()
    };

    Some(VerInfo { publisher: query("CompanyName"), description: query("FileDescription") })
}

fn translation(buf: &[u8]) -> Option<(u16, u16)> {
    let sub = wide("\\VarFileInfo\\Translation");
    let mut ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    let mut len: u32 = 0;
    // SAFETY: `buf` outlives the call; `ptr`/`len` receive a pointer into `buf` and its length.
    let ok = unsafe { VerQueryValueW(buf.as_ptr().cast(), sub.as_ptr(), &mut ptr, &mut len) };
    if ok == 0 || ptr.is_null() || len < 4 {
        return None;
    }
    // SAFETY: the block is at least one u16 pair (lang, codepage).
    let pair = unsafe { std::slice::from_raw_parts(ptr as *const u16, 2) };
    Some((pair[0], pair[1]))
}

fn string_value(buf: &[u8], sub_block: &str) -> Option<String> {
    let sub = wide(sub_block);
    let mut ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    let mut len: u32 = 0;
    // SAFETY: `buf` outlives the call; on success `ptr`/`len` describe a UTF-16 string in `buf`.
    let ok = unsafe { VerQueryValueW(buf.as_ptr().cast(), sub.as_ptr(), &mut ptr, &mut len) };
    if ok == 0 || ptr.is_null() || len == 0 {
        return None;
    }
    // `len` is in characters and includes the trailing NUL.
    let n = (len as usize).saturating_sub(1);
    // SAFETY: the resource holds `len` UTF-16 units at `ptr`.
    let s = unsafe { std::slice::from_raw_parts(ptr as *const u16, n) };
    let out = String::from_utf16_lossy(s).trim().to_string();
    (!out.is_empty()).then_some(out)
}
