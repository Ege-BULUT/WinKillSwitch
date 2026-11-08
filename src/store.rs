//! Loads and saves `Saved` as JSON under `%APPDATA%\winkillswitch\state.json`.
//! Writes go to a temp file first and are renamed over the old one, so a crash
//! mid-write never loses the pause records needed to revert.

use std::fs;
use std::path::PathBuf;

use crate::model::Saved;

pub fn dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    base.join("winkillswitch")
}

pub fn path() -> PathBuf {
    dir().join("state.json")
}

pub fn load() -> Saved {
    match fs::read_to_string(path()) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            // Keep the unreadable file for inspection instead of overwriting it.
            let _ = fs::copy(path(), dir().join("state.corrupt.json"));
            eprintln!("winkillswitch: state.json unreadable ({e}); starting empty");
            Saved::default()
        }),
        Err(_) => Saved::default(),
    }
}

pub fn save(s: &Saved) -> Result<(), String> {
    fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    let tmp = dir().join("state.json.tmp");
    fs::write(&tmp, json).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path()).map_err(|e| e.to_string())
}
