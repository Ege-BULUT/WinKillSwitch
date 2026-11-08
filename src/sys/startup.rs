//! Everything that starts with Windows, read only: Run and RunOnce keys (per user, machine and
//! 32-bit), policy Run keys, the Startup folders, scheduled tasks with a logon or boot trigger,
//! automatic services, and the Winlogon and AppInit spots malware likes. Nothing is changed.

use std::path::{Path, PathBuf};

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};

use crate::sys::verinfo::VerCache;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// Winlogon or AppInit changed from the Windows default: worth a look.
    Unusual,
    Registry,
    Folder,
    Task,
    Service,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Unusual => "Unusual",
            Kind::Registry => "Registry",
            Kind::Folder => "Startup folder",
            Kind::Task => "Scheduled task",
            Kind::Service => "Service",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Item {
    pub name: String,
    pub command: String,
    pub exe: Option<PathBuf>,
    pub kind: Kind,
    /// Where it lives ("HKCU\...\Run", a folder, a task path, a service key).
    pub location: String,
    pub enabled: bool,
    pub publisher: String,
    /// From Microsoft (Windows itself, or a Microsoft app).
    pub microsoft: bool,
}

/// Replace `%VAR%` with its value.
pub fn expand(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('%') {
            Some(j) => {
                let var = &after[..j];
                match std::env::var(var) {
                    Ok(v) if !var.is_empty() => out.push_str(&v),
                    _ => out.push_str(&rest[i..i + j + 2]),
                }
                rest = &after[j + 1..];
            }
            None => {
                out.push_str(&rest[i..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// The program a command line runs: a quoted path, else everything up to the first `.exe`
/// (paths with spaces are often unquoted), else the first word. `\??\` and `\SystemRoot\`
/// prefixes from service image paths are resolved.
pub fn exe_of(command: &str) -> Option<PathBuf> {
    let c = expand(command.trim());
    let c = c.strip_prefix(r"\??\").unwrap_or(&c).to_string();
    let c = match c.get(..12) {
        Some(p) if p.eq_ignore_ascii_case(r"\SystemRoot\") => format!("{}\\{}", std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into()), &c[12..]),
        _ => c,
    };
    let path = if let Some(r) = c.strip_prefix('"') {
        r.split('"').next().unwrap_or("").to_string()
    } else if let Some(i) = c.to_lowercase().find(".exe") {
        c[..i + 4].to_string()
    } else {
        c.split_whitespace().next().unwrap_or("").to_string()
    };
    if path.is_empty() {
        return None;
    }
    let p = PathBuf::from(&path);
    if p.is_absolute() {
        return Some(p);
    }
    // A bare name ("rundll32.exe") resolves to System32 when it is there.
    let sys = PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into())).join("System32").join(&p);
    Some(if sys.exists() { sys } else { p })
}

/// StartupApproved: an entry Task Manager disabled has an odd first byte.
fn approved(hive: &RegKey, sub: &str, name: &str) -> bool {
    let key = format!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\{sub}");
    match hive.open_subkey_with_flags(key, KEY_READ).and_then(|k| k.get_raw_value(name)) {
        Ok(v) => v.bytes.first().is_none_or(|b| b & 1 == 0),
        Err(_) => true,
    }
}

fn run_keys(out: &mut Vec<Item>) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let spots: [(&RegKey, &str, &str, &str); 7] = [
        (&hkcu, "HKCU", r"Software\Microsoft\Windows\CurrentVersion\Run", "Run"),
        (&hkcu, "HKCU", r"Software\Microsoft\Windows\CurrentVersion\RunOnce", ""),
        (&hklm, "HKLM", r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run", "Run"),
        (&hklm, "HKLM", r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Run", "Run32"),
        (&hklm, "HKLM", r"SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce", ""),
        (&hkcu, "HKCU", r"Software\Microsoft\Windows\CurrentVersion\Policies\Explorer\Run", ""),
        (&hklm, "HKLM", r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer\Run", ""),
    ];
    for (hive, hname, path, approved_sub) in spots {
        let Ok(key) = hive.open_subkey_with_flags(path, KEY_READ) else { continue };
        for (name, _) in key.enum_values().flatten() {
            let Ok(cmd) = key.get_value::<String, _>(&name) else { continue };
            let enabled = approved_sub.is_empty() || approved(hive, approved_sub, &name);
            out.push(Item {
                name: if name.is_empty() { "(default)".into() } else { name },
                exe: exe_of(&cmd),
                command: cmd,
                kind: Kind::Registry,
                location: format!(r"{hname}\{path}"),
                enabled,
                publisher: String::new(),
                microsoft: false,
            });
        }
    }
}

fn startup_folders(out: &mut Vec<Item>) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let mut dirs = Vec::new();
    if let Ok(a) = std::env::var("APPDATA") {
        dirs.push(PathBuf::from(a).join(r"Microsoft\Windows\Start Menu\Programs\Startup"));
    }
    if let Ok(p) = std::env::var("ProgramData") {
        dirs.push(PathBuf::from(p).join(r"Microsoft\Windows\Start Menu\Programs\StartUp"));
    }
    for d in dirs {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.eq_ignore_ascii_case("desktop.ini") {
                continue;
            }
            let path = e.path();
            out.push(Item {
                name: path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or(name.clone()),
                command: path.display().to_string(),
                exe: Some(path.clone()),
                kind: Kind::Folder,
                location: d.display().to_string(),
                enabled: approved(&hkcu, "StartupFolder", &name),
                publisher: String::new(),
                microsoft: false,
            });
        }
    }
}

fn tag<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let i = xml.find(&open)? + open.len();
    let j = xml[i..].find(&format!("</{name}>"))?;
    Some(xml[i..i + j].trim())
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&apos;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// A task XML file's (command, enabled) when it runs at logon or boot.
pub fn parse_task(xml: &str) -> Option<(String, bool)> {
    let triggers = tag(xml, "Triggers")?;
    if !(triggers.contains("<LogonTrigger") || triggers.contains("<BootTrigger")) {
        return None;
    }
    let exec = tag(xml, "Exec")?;
    let cmd = unescape(tag(exec, "Command")?);
    let args = tag(exec, "Arguments").map(unescape).unwrap_or_default();
    let enabled = tag(xml, "Settings").and_then(|s| tag(s, "Enabled")).is_none_or(|v| !v.eq_ignore_ascii_case("false"));
    let command = if args.is_empty() { cmd } else if cmd.contains(' ') && !cmd.starts_with('"') { format!("\"{cmd}\" {args}") } else { format!("{cmd} {args}") };
    Some((command, enabled))
}

fn read_utf16_or_8(p: &Path) -> Option<String> {
    let b = std::fs::read(p).ok()?;
    if b.len() >= 2 && b[0] == 0xFF && b[1] == 0xFE {
        let w: Vec<u16> = b[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        Some(String::from_utf16_lossy(&w))
    } else {
        Some(String::from_utf8_lossy(&b).into_owned())
    }
}

fn tasks(out: &mut Vec<Item>) {
    let root = PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into())).join(r"System32\Tasks");
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let Ok(md) = e.metadata() else { continue };
            if md.is_dir() {
                stack.push(e.path());
                continue;
            }
            let path = e.path();
            let Some(xml) = read_utf16_or_8(&path) else { continue };
            let Some((command, enabled)) = parse_task(&xml) else { continue };
            let rel = path.strip_prefix(&root).map(|r| format!("\\{}", r.display())).unwrap_or_default();
            out.push(Item {
                name: path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
                exe: exe_of(&command),
                command,
                kind: Kind::Task,
                location: rel,
                enabled,
                publisher: String::new(),
                microsoft: false,
            });
        }
    }
}

fn services(out: &mut Vec<Item>) {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let Ok(root) = hklm.open_subkey_with_flags(r"SYSTEM\CurrentControlSet\Services", KEY_READ) else { return };
    for name in root.enum_keys().flatten() {
        let Ok(k) = root.open_subkey_with_flags(&name, KEY_READ) else { continue };
        let start: u32 = k.get_value("Start").unwrap_or(4);
        let ty: u32 = k.get_value("Type").unwrap_or(0);
        // Automatic (2), and a program rather than a driver (own or shared process).
        if start != 2 || ty & 0x30 == 0 {
            continue;
        }
        let Ok(image) = k.get_value::<String, _>("ImagePath") else { continue };
        let display: String = k.get_value("DisplayName").unwrap_or_default();
        let display = if display.is_empty() || display.starts_with('@') { name.clone() } else { display };
        // A shared svchost service's code is its ServiceDll.
        let dll = k.open_subkey_with_flags("Parameters", KEY_READ).and_then(|p| p.get_value::<String, _>("ServiceDll")).ok();
        let exe = dll.as_deref().map(|d| PathBuf::from(expand(d))).or_else(|| exe_of(&image));
        out.push(Item {
            name: display,
            command: image,
            exe,
            kind: Kind::Service,
            location: format!(r"HKLM\SYSTEM\CurrentControlSet\Services\{name}"),
            enabled: true,
            publisher: String::new(),
            microsoft: false,
        });
    }
}

fn unusual(out: &mut Vec<Item>) {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let wl = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon";
    if let Ok(k) = hklm.open_subkey_with_flags(wl, KEY_READ) {
        let shell: String = k.get_value("Shell").unwrap_or_default();
        if !shell.is_empty() && !shell.trim().eq_ignore_ascii_case("explorer.exe") {
            out.push(Item { name: "Winlogon shell".into(), exe: exe_of(&shell), command: shell, kind: Kind::Unusual, location: format!(r"HKLM\{wl}"), enabled: true, publisher: String::new(), microsoft: false });
        }
        let ui: String = k.get_value("Userinit").unwrap_or_default();
        let norm = ui.trim().trim_end_matches(',').to_lowercase();
        if !ui.is_empty() && !norm.ends_with(r"\system32\userinit.exe") && norm != "userinit.exe" {
            out.push(Item { name: "Winlogon userinit".into(), exe: exe_of(&ui), command: ui, kind: Kind::Unusual, location: format!(r"HKLM\{wl}"), enabled: true, publisher: String::new(), microsoft: false });
        }
    }
    let ai = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Windows";
    if let Ok(k) = hklm.open_subkey_with_flags(ai, KEY_READ) {
        let dlls: String = k.get_value("AppInit_DLLs").unwrap_or_default();
        let on: u32 = k.get_value("LoadAppInit_DLLs").unwrap_or(0);
        if on != 0 && !dlls.trim().is_empty() {
            out.push(Item { name: "AppInit DLLs".into(), exe: exe_of(&dlls), command: dlls, kind: Kind::Unusual, location: format!(r"HKLM\{ai}"), enabled: true, publisher: String::new(), microsoft: false });
        }
    }
}

/// Everything, Unusual first, then by kind and name.
pub fn scan() -> Vec<Item> {
    let mut v = Vec::new();
    unusual(&mut v);
    run_keys(&mut v);
    startup_folders(&mut v);
    tasks(&mut v);
    services(&mut v);
    let mut ver = VerCache::default();
    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into()).to_lowercase();
    for it in &mut v {
        if let Some(exe) = &it.exe {
            it.publisher = ver.get(exe).publisher;
            let in_windows = exe.to_string_lossy().to_lowercase().starts_with(&windir);
            it.microsoft = it.publisher.starts_with("Microsoft") || (in_windows && it.publisher.is_empty());
        }
    }
    v.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_program_in_a_command() {
        assert_eq!(exe_of(r#""C:\Program Files\App\app.exe" --min"#), Some(PathBuf::from(r"C:\Program Files\App\app.exe")));
        assert_eq!(exe_of(r"C:\Program Files\App\app.exe --min"), Some(PathBuf::from(r"C:\Program Files\App\app.exe")));
        let sr = std::env::var("SystemRoot").unwrap();
        assert_eq!(exe_of(r"%SystemRoot%\system32\svchost.exe -k netsvcs"), Some(PathBuf::from(format!(r"{sr}\system32\svchost.exe"))));
        assert_eq!(exe_of(r"\SystemRoot\System32\drivers\x.sys"), Some(PathBuf::from(format!(r"{sr}\System32\drivers\x.sys"))));
        assert_eq!(exe_of(""), None);
    }

    #[test]
    fn expands_and_keeps_unknown_variables() {
        let sr = std::env::var("SystemRoot").unwrap();
        assert_eq!(expand("%SystemRoot%\\a"), format!("{sr}\\a"));
        assert_eq!(expand("100% and %NOPE_WKS%"), "100% and %NOPE_WKS%");
    }

    #[test]
    fn parses_logon_tasks_only() {
        let logon = r#"<Task><Triggers><LogonTrigger><Enabled>true</Enabled></LogonTrigger></Triggers><Settings><Enabled>false</Enabled></Settings><Actions><Exec><Command>C:\a b\up.exe</Command><Arguments>/silent &amp; quiet</Arguments></Exec></Actions></Task>"#;
        assert_eq!(parse_task(logon), Some((r#""C:\a b\up.exe" /silent & quiet"#.to_string(), false)));
        let daily = r#"<Task><Triggers><CalendarTrigger/></Triggers><Actions><Exec><Command>x.exe</Command></Exec></Actions></Task>"#;
        assert_eq!(parse_task(daily), None);
    }

    #[test]
    #[ignore = "reads this machine's startup entries"]
    fn survey() {
        let v = scan();
        for k in [Kind::Unusual, Kind::Registry, Kind::Folder, Kind::Task, Kind::Service] {
            let n = v.iter().filter(|i| i.kind == k).count();
            let ms = v.iter().filter(|i| i.kind == k && i.microsoft).count();
            eprintln!("{:16} {n:4} ({ms} Microsoft)", k.label());
        }
        for i in v.iter().filter(|i| !i.microsoft).take(25) {
            eprintln!("{:14} {:30} {:5} {:28} {}", i.kind.label(), i.name, i.enabled, i.publisher, i.exe.as_ref().map(|p| p.display().to_string()).unwrap_or_default());
        }
    }
}
