//! "Why is my PC slow?": watch the machine for a few samples, then explain in plain words what
//! is holding it back, worst first, each with the one action that helps.

use crate::model::{Pid, Snapshot};
use crate::sys::memory::MemStatus;

/// Samples to watch before answering (the sampler ticks every 1.5 s).
pub const SAMPLES: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Good,
    Warn,
    Bad,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Fix {
    None,
    CleanMemory,
    AnalyzeDisk,
    ShowProcess(Pid),
}

#[derive(Clone, Debug)]
pub struct Finding {
    /// The area it is about: "Processor", "Memory", "Storage space", "Disk activity", "Graphics", "Uptime".
    pub area: &'static str,
    pub level: Level,
    pub title: String,
    pub detail: String,
    pub fix: Fix,
}

pub struct Inputs<'a> {
    pub samples: &'a [Snapshot],
    pub mem: MemStatus,
    /// Free and total bytes on the system drive.
    pub drive: (u64, u64),
    pub uptime_secs: u64,
}

const GB: u64 = 1 << 30;
const MB: f64 = (1u64 << 20) as f64;

fn gb(b: u64) -> String {
    format!("{:.1} GB", b as f64 / GB as f64)
}

/// The process with the highest average of `f` over every sample it appears in, weighting a
/// missing sample as zero so a one-tick spike does not dominate.
fn top_by(samples: &[Snapshot], f: impl Fn(&crate::model::ProcInfo) -> f64) -> Option<(Pid, String, f64)> {
    let mut sums: std::collections::HashMap<Pid, (String, f64)> = std::collections::HashMap::new();
    for snap in samples {
        for p in &snap.procs {
            if p.pid == 0 {
                continue; // System Idle Process
            }
            let e = sums.entry(p.pid).or_insert_with(|| (p.name.clone(), 0.0));
            e.1 += f(p);
        }
    }
    let n = samples.len().max(1) as f64;
    sums.into_iter().map(|(pid, (name, sum))| (pid, name, sum / n)).max_by(|a, b| a.2.total_cmp(&b.2))
}

pub fn diagnose(inp: &Inputs) -> Vec<Finding> {
    let mut out = Vec::new();
    let samples = inp.samples;
    let n = samples.len().max(1) as f32;

    // Processor.
    let cpu = samples.iter().map(|s| s.cpu_total).sum::<f32>() / n;
    let top_cpu = top_by(samples, |p| p.cpu as f64);
    let (level, title) = if cpu >= 85.0 {
        (Level::Bad, format!("Your processor is maxed out ({cpu:.0}%)"))
    } else if cpu >= 60.0 {
        (Level::Warn, format!("Your processor is busy ({cpu:.0}%)"))
    } else {
        (Level::Good, format!("Processor is fine ({cpu:.0}%)"))
    };
    let mut f = Finding { area: "Processor", level, title, detail: "Plenty of headroom for what you are doing.".into(), fix: Fix::None };
    if let Some((pid, name, pc)) = top_cpu.filter(|t| t.2 >= 20.0) {
        f.level = f.level.max(Level::Warn);
        f.detail = format!("{name} is using {pc:.0}% of it. If you are not expecting that, open it to see what it is.");
        f.fix = Fix::ShowProcess(pid);
    } else if level != Level::Good {
        f.detail = "Many processes share the load, none stands out. Closing apps you are not using helps.".into();
    }
    out.push(f);

    // Memory.
    let used = inp.mem.used_frac();
    let top_mem = samples.last().and_then(|s| s.procs.iter().max_by_key(|p| p.mem)).map(|p| (p.pid, p.name.clone(), p.mem));
    let (level, title) = if used >= 0.9 {
        (Level::Bad, format!("Memory is almost full ({:.0}%)", used * 100.0))
    } else if used >= 0.8 {
        (Level::Warn, format!("Memory is getting full ({:.0}%)", used * 100.0))
    } else {
        (Level::Good, format!("Memory is fine ({:.0}% used)", used * 100.0))
    };
    let mut f = Finding { area: "Memory", level, title, detail: format!("{} available.", gb(inp.mem.available)), fix: Fix::None };
    if level != Level::Good {
        f.detail = "When memory runs out Windows swaps to disk, which is what makes everything feel sluggish.".into();
        f.fix = Fix::CleanMemory;
        if let Some((_, name, m)) = top_mem.filter(|t| inp.mem.total > 0 && t.2 * 6 >= inp.mem.total) {
            f.detail = format!("{name} alone holds {}. Closing it, or cleaning memory, gives room back.", gb(m));
        }
    }
    out.push(f);

    // Storage space.
    let (free, total) = inp.drive;
    if total > 0 {
        let frac = free as f64 / total as f64;
        let (level, title) = if free < 5 * GB || frac < 0.05 {
            (Level::Bad, format!("Your system drive is nearly full ({} free)", gb(free)))
        } else if free < 15 * GB || frac < 0.10 {
            (Level::Warn, format!("Your system drive is low on space ({} free)", gb(free)))
        } else {
            (Level::Good, format!("Plenty of disk space ({} free)", gb(free)))
        };
        let (detail, fix) = if level == Level::Good {
            ("Updates and the page file have room to work.".into(), Fix::None)
        } else {
            ("Windows needs free space for updates and the page file. Clearing temporary files usually frees several gigabytes.".into(), Fix::AnalyzeDisk)
        };
        out.push(Finding { area: "Storage space", level, title, detail, fix });
    }

    // Disk activity.
    let top_disk = top_by(samples, |p| p.disk as f64);
    match top_disk.filter(|t| t.2 >= 25.0 * MB) {
        Some((pid, name, rate)) => out.push(Finding {
            area: "Disk activity",
            level: Level::Warn,
            title: format!("{name} is keeping the disk busy"),
            detail: format!("{:.0} MB/s of reads and writes. Updates, indexing and antivirus scans do this for a while, then settle.", rate / MB),
            fix: Fix::ShowProcess(pid),
        }),
        None => out.push(Finding { area: "Disk activity", level: Level::Good, title: "Disk activity is calm".into(), detail: "Nothing is hammering the drive.".into(), fix: Fix::None }),
    }

    // Graphics.
    let top_gpu = top_by(samples, |p| p.gpu as f64);
    match top_gpu.filter(|t| t.2 >= 70.0) {
        Some((pid, name, pc)) => out.push(Finding {
            area: "Graphics",
            level: Level::Warn,
            title: format!("{name} is using {pc:.0}% of the graphics card"),
            detail: "Expected for a game or a video export. For anything else, take a look.".into(),
            fix: Fix::ShowProcess(pid),
        }),
        None => out.push(Finding { area: "Graphics", level: Level::Good, title: "Graphics card has headroom".into(), detail: "No app is saturating it.".into(), fix: Fix::None }),
    }

    // Uptime.
    let days = inp.uptime_secs / 86_400;
    if days >= 7 {
        out.push(Finding {
            area: "Uptime",
            level: Level::Warn,
            title: format!("Windows has been running for {days} days"),
            detail: "A restart clears leaked memory and finishes pending updates. Save your work first.".into(),
            fix: Fix::None,
        });
    } else {
        let hours = inp.uptime_secs / 3600;
        let since = if days >= 1 { format!("{days} day{}", if days == 1 { "" } else { "s" }) } else { format!("{hours} hour{}", if hours == 1 { "" } else { "s" }) };
        out.push(Finding { area: "Uptime", level: Level::Good, title: format!("Restarted recently ({since} ago)"), detail: "No restart needed.".into(), fix: Fix::None });
    }

    // Worst first; the order above breaks ties.
    out.sort_by(|a, b| b.level.cmp(&a.level));
    out
}

/// One line summing up the findings.
pub fn verdict(findings: &[Finding]) -> (Level, String) {
    let bad = findings.iter().filter(|f| f.level == Level::Bad).count();
    let warn = findings.iter().filter(|f| f.level == Level::Warn).count();
    match (bad, warn) {
        (0, 0) => (Level::Good, "Everything looks healthy".into()),
        (0, w) => (Level::Warn, format!("{w} thing{} could be slowing you down", if w == 1 { "" } else { "s" })),
        (b, w) => (Level::Bad, format!("{} problem{} found", b + w, if b + w == 1 { "" } else { "s" })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ProcInfo, ProcKind};

    fn proc(pid: Pid, name: &str, cpu: f32, mem: u64) -> ProcInfo {
        ProcInfo {
            pid,
            parent: None,
            name: name.into(),
            exe: None,
            cpu,
            mem,
            start: 0,
            disk: 0,
            gpu: 0.0,
            gpu_engine: String::new(),
            status: "Running",
            kind: ProcKind::App,
            cmd: String::new(),
            session: None,
            publisher: String::new(),
            description: String::new(),
        }
    }

    fn snap(cpu_total: f32, procs: Vec<ProcInfo>) -> Snapshot {
        Snapshot { procs, total_mem: 16 * GB, used_mem: 4 * GB, cpu_total }
    }

    fn mem(used_frac: f64) -> MemStatus {
        let total = 16 * GB;
        MemStatus { total, available: (total as f64 * (1.0 - used_frac)) as u64, cache: 0 }
    }

    #[test]
    fn a_calm_machine_is_healthy() {
        let s = vec![snap(8.0, vec![proc(4, "a.exe", 2.0, GB)]); SAMPLES];
        let f = diagnose(&Inputs { samples: &s, mem: mem(0.4), drive: (200 * GB, 500 * GB), uptime_secs: 3600 * 5 });
        assert!(f.iter().all(|f| f.level == Level::Good), "{f:#?}");
        assert_eq!(verdict(&f).0, Level::Good);
    }

    #[test]
    fn names_the_cpu_hog_and_ranks_worst_first() {
        let s = vec![snap(92.0, vec![proc(4, "a.exe", 2.0, GB), proc(9, "miner.exe", 70.0, GB)]); SAMPLES];
        let f = diagnose(&Inputs { samples: &s, mem: mem(0.95), drive: (200 * GB, 500 * GB), uptime_secs: 3600 });
        assert_eq!(f[0].level, Level::Bad);
        let cpu = f.iter().find(|f| f.area == "Processor").unwrap();
        assert_eq!(cpu.fix, Fix::ShowProcess(9));
        assert!(cpu.detail.contains("miner.exe"));
        assert_eq!(f.iter().find(|f| f.area == "Memory").unwrap().fix, Fix::CleanMemory);
    }

    #[test]
    fn a_one_tick_spike_is_not_blamed() {
        let mut s = vec![snap(10.0, vec![proc(4, "a.exe", 1.0, GB)]); SAMPLES - 1];
        s.push(snap(30.0, vec![proc(4, "a.exe", 1.0, GB), proc(7, "spike.exe", 60.0, GB)]));
        let f = diagnose(&Inputs { samples: &s, mem: mem(0.4), drive: (200 * GB, 500 * GB), uptime_secs: 60 });
        assert_eq!(f.iter().find(|f| f.area == "Processor").unwrap().fix, Fix::None);
    }

    #[test]
    fn low_disk_and_long_uptime() {
        let s = vec![snap(5.0, vec![]); SAMPLES];
        let f = diagnose(&Inputs { samples: &s, mem: mem(0.3), drive: (3 * GB, 250 * GB), uptime_secs: 86_400 * 12 });
        let disk = f.iter().find(|f| f.area == "Storage space").unwrap();
        assert_eq!((disk.level, &disk.fix), (Level::Bad, &Fix::AnalyzeDisk));
        assert_eq!(f.iter().find(|f| f.area == "Uptime").unwrap().level, Level::Warn);
    }
}
