//! Small formatters shared across the view.

/// Human-readable byte size, e.g. `812 MB`, `1.4 GB`.
pub fn bytes(n: u64) -> String {
    const KB: f64 = 1024.0;
    let n = n as f64;
    if n >= KB * KB * KB {
        format!("{:.1} GB", n / (KB * KB * KB))
    } else if n >= KB * KB {
        format!("{:.0} MB", n / (KB * KB))
    } else if n >= KB {
        format!("{:.0} KB", n / KB)
    } else {
        format!("{n:.0} B")
    }
}

/// A CPU percent like `3.4%` or `12%`.
pub fn pct(v: f32) -> String {
    if v < 10.0 { format!("{v:.1}%") } else { format!("{v:.0}%") }
}

/// A count with thousands separators, e.g. `12,345`.
pub fn count(n: u64) -> String {
    let d = n.to_string();
    let mut out = String::new();
    for (i, ch) in d.chars().enumerate() {
        if i > 0 && (d.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}
