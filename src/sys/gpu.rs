//! Per-process GPU use from the "GPU Engine" performance counters, the same source Task Manager
//! reads. Each counter instance is one engine of one process, named like
//! `pid_1234_luid_0x0_0xD1F8_phys_0_eng_0_engtype_3D`. Like Task Manager, utilisation is summed
//! per engine type and a process shows its busiest type.

use std::collections::HashMap;

use windows_sys::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
};

const PDH_MORE_DATA: u32 = 0x8000_07D2;

pub struct GpuSampler {
    query: isize,
    counter: isize,
}

impl GpuSampler {
    /// None when the counters are not available (no WDDM 2 driver, or counters disabled).
    pub fn new() -> Option<GpuSampler> {
        let path: Vec<u16> = "\\GPU Engine(*)\\Utilization Percentage".encode_utf16().chain(Some(0)).collect();
        let (mut query, mut counter) = (0isize, 0isize);
        // SAFETY: out-pointers are valid; the query is closed in Drop.
        unsafe {
            if PdhOpenQueryW(std::ptr::null(), 0, &mut query) != 0 {
                return None;
            }
            if PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter) != 0 {
                PdhCloseQuery(query);
                return None;
            }
            PdhCollectQueryData(query); // rates need a first sample
        }
        Some(GpuSampler { query, counter })
    }

    /// pid -> (percent, busiest engine type) since the previous call.
    pub fn sample(&mut self) -> HashMap<u32, (f32, String)> {
        let mut per_type: HashMap<(u32, String), f64> = HashMap::new();
        // SAFETY: the buffer is sized by the first call and read only for `count` items; the
        // instance name pointers point into that same buffer.
        unsafe {
            if PdhCollectQueryData(self.query) != 0 {
                return HashMap::new();
            }
            let (mut size, mut count) = (0u32, 0u32);
            if PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut size, &mut count, std::ptr::null_mut()) != PDH_MORE_DATA || size == 0 {
                return HashMap::new();
            }
            let mut buf = vec![0u8; size as usize];
            let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
            if PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut size, &mut count, items) != 0 {
                return HashMap::new();
            }
            for i in 0..count as usize {
                let it = &*items.add(i);
                if it.FmtValue.CStatus > 1 {
                    continue; // not valid or new data
                }
                let v = it.FmtValue.Anonymous.doubleValue;
                if v <= 0.0 {
                    continue;
                }
                let mut n = 0;
                while *it.szName.add(n) != 0 {
                    n += 1;
                }
                let name = String::from_utf16_lossy(std::slice::from_raw_parts(it.szName, n));
                if let Some((pid, ty)) = parse(&name) {
                    *per_type.entry((pid, ty)).or_insert(0.0) += v;
                }
            }
        }
        let mut out: HashMap<u32, (f32, String)> = HashMap::new();
        for ((pid, ty), v) in per_type {
            let v = v.min(100.0) as f32;
            let e = out.entry(pid).or_insert((0.0, String::new()));
            if v > e.0 {
                *e = (v, ty);
            }
        }
        out
    }
}

impl Drop for GpuSampler {
    fn drop(&mut self) {
        // SAFETY: the query was opened in `new`.
        unsafe {
            PdhCloseQuery(self.query);
        }
    }
}

/// `pid_1234_..._engtype_3D` -> (1234, "3D"). Engine type names keep their spaces
/// ("Video Decode").
fn parse(name: &str) -> Option<(u32, String)> {
    let rest = name.strip_prefix("pid_")?;
    let pid: u32 = rest.split('_').next()?.parse().ok()?;
    let ty = name.split("engtype_").nth(1)?.to_string();
    Some((pid, if ty.is_empty() { "Other".into() } else { ty }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_instance_names() {
        assert_eq!(parse("pid_1234_luid_0x00000000_0x0000D1F8_phys_0_eng_0_engtype_3D"), Some((1234, "3D".into())));
        assert_eq!(parse("pid_8_luid_0x0_0x1_phys_0_eng_3_engtype_Video Decode"), Some((8, "Video Decode".into())));
        assert_eq!(parse("_Total"), None);
    }

    #[test]
    fn counters_open_and_sample() {
        // Every machine with a WDDM 2 driver has these counters; a VM without a GPU may not.
        if let Some(mut g) = GpuSampler::new() {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let m = g.sample();
            assert!(m.values().all(|(v, _)| (0.0..=100.0).contains(v)));
        }
    }
}

#[cfg(test)]
mod live {
    /// Prints the busiest GPU processes: `cargo test --release gpu_top -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn gpu_top() {
        let mut g = super::GpuSampler::new().expect("GPU Engine counters");
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let mut v: Vec<_> = g.sample().into_iter().collect();
        v.sort_by(|a, b| b.1.0.partial_cmp(&a.1.0).unwrap());
        println!("{} processes use the GPU", v.len());
        for (pid, (pct, ty)) in v.iter().take(6) {
            println!("pid {pid:>6}  {pct:>5.1}%  {ty}");
        }
    }
}
