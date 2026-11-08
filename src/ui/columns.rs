//! The process table's columns: what exists, what shows by default, and how each one draws and
//! sorts. Right click the header to choose; the choice is saved.

use std::cmp::Ordering;

use rune::Align;

use crate::model::ProcInfo;
use crate::ui::fmt;

pub struct Col {
    pub id: &'static str,
    pub title: &'static str,
    /// Fixed width, or None for a flexible column.
    pub width: Option<f32>,
    pub min: f32,
    pub end: bool,
}

const fn fixed(id: &'static str, title: &'static str, w: f32, min: f32, end: bool) -> Col {
    Col { id, title, width: Some(w), min, end }
}
const fn flex(id: &'static str, title: &'static str, min: f32) -> Col {
    Col { id, title, width: None, min, end: false }
}

/// Every column, in display order. "name" is always shown.
pub const ALL: [Col; 13] = [
    flex("name", "Name", 180.0),
    fixed("type", "Type", 130.0, 90.0, false),
    fixed("status", "Status", 90.0, 70.0, false),
    fixed("publisher", "Publisher", 160.0, 90.0, false),
    fixed("desc", "Description", 180.0, 100.0, false),
    fixed("pid", "PID", 72.0, 56.0, true),
    fixed("cmd", "Command line", 260.0, 120.0, false),
    fixed("cpu", "CPU", 72.0, 56.0, true),
    fixed("mem", "Memory", 92.0, 72.0, true),
    fixed("disk", "Disk", 92.0, 72.0, true),
    fixed("net", "Network", 96.0, 72.0, true),
    fixed("gpu", "GPU", 68.0, 56.0, true),
    fixed("engine", "GPU engine", 110.0, 80.0, false),
];

pub const DEFAULT: [&str; 7] = ["name", "pid", "cpu", "mem", "disk", "net", "gpu"];

/// Which columns go first when the table is too narrow for all of them.
const DROP_ORDER: [&str; 11] = ["engine", "desc", "cmd", "publisher", "status", "type", "gpu", "net", "disk", "pid", "mem"];

/// Keep the columns that fit `width`, dropping by `DROP_ORDER`. Name always stays.
pub fn fit(mut cols: Vec<&'static Col>, width: f32) -> Vec<&'static Col> {
    let need = |cs: &[&Col]| cs.iter().map(|c| c.width.unwrap_or(c.min)).sum::<f32>() + 24.0;
    for id in DROP_ORDER {
        if need(&cols) <= width {
            break;
        }
        cols.retain(|c| c.id != id);
    }
    cols
}

pub fn align(c: &Col) -> Align {
    if c.end { Align::End } else { Align::Start }
}

/// Disk rate, compact: "1.2 MB/s", or "0 MB/s" when idle.
pub fn rate(b: u64) -> String {
    if b < 1024 * 100 { "0 MB/s".into() } else { format!("{:.1} MB/s", b as f64 / 1_048_576.0) }
}

/// Network rate in megabits per second, the unit connections are sold in.
pub fn net(b: u64) -> String {
    let mbps = b as f64 * 8.0 / 1_000_000.0;
    if b == 0 && !crate::sys::net::running() {
        // Not measured: per-process network needs administrator rights.
        "\u{2014}".into()
    } else if b == 0 { "0 Mbps".into() } else if mbps < 0.1 { "0.1 Mbps".into() } else { format!("{mbps:.1} Mbps") }
}

/// The text a cell shows for a column (the name column draws its own element).
pub fn text(p: &ProcInfo, id: &str) -> String {
    match id {
        "type" => p.kind.label().to_string(),
        "status" => p.status.to_string(),
        "publisher" => p.publisher.clone(),
        "desc" => p.description.clone(),
        "pid" => p.pid.to_string(),
        "cmd" => p.cmd.clone(),
        "cpu" => fmt::pct(p.cpu),
        "mem" => fmt::bytes(p.mem),
        "disk" => rate(p.disk),
        "net" => net(p.net),
        "gpu" => if p.gpu < 0.05 { "0%".into() } else { fmt::pct(p.gpu) },
        "engine" => if p.gpu < 0.05 { String::new() } else { p.gpu_engine.clone() },
        _ => p.name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_tables_drop_the_least_useful_columns() {
        let all: Vec<&Col> = ALL.iter().filter(|c| DEFAULT.contains(&c.id)).collect();
        assert_eq!(fit(all.clone(), f32::MAX).len(), DEFAULT.len());
        let ids = |v: Vec<&Col>| v.iter().map(|c| c.id).collect::<Vec<_>>();
        assert_eq!(ids(fit(all.clone(), 560.0)), ["name", "pid", "cpu", "mem", "disk"]);
        assert_eq!(ids(fit(all, 10.0)), ["name", "cpu"]);
    }
}

/// Numbers sort as numbers, text without case.
pub fn cmp(a: &ProcInfo, b: &ProcInfo, id: &str) -> Ordering {
    let f = |x: f32, y: f32| x.partial_cmp(&y).unwrap_or(Ordering::Equal);
    match id {
        "pid" => a.pid.cmp(&b.pid),
        "cpu" => f(a.cpu, b.cpu),
        "mem" => a.mem.cmp(&b.mem),
        "disk" => a.disk.cmp(&b.disk),
        "net" => a.net.cmp(&b.net),
        "gpu" => f(a.gpu, b.gpu),
        _ => text(a, id).to_lowercase().cmp(&text(b, id).to_lowercase()),
    }
}
