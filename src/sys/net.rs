//! Per-process network traffic from an ETW real-time session on the Microsoft-Windows-Kernel-
//! Network provider, the source Resource Monitor uses. Every TCP and UDP send and receive (IPv4
//! and IPv6) carries the process id and byte count; this adds them up per process.
//!
//! Needs administrator rights. Without them `start` returns false and the column stays empty.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use windows_sys::Win32::System::Diagnostics::Etw::{
    CONTROLTRACE_HANDLE, ControlTraceW, EVENT_CONTROL_CODE_ENABLE_PROVIDER, EVENT_RECORD, EVENT_TRACE_CONTROL_STOP, EVENT_TRACE_LOGFILEW, EVENT_TRACE_PROPERTIES,
    EVENT_TRACE_REAL_TIME_MODE, EnableTraceEx2, OpenTraceW, PROCESS_TRACE_MODE_EVENT_RECORD, PROCESS_TRACE_MODE_REAL_TIME, PROCESSTRACE_HANDLE, ProcessTrace,
    StartTraceW, TRACE_LEVEL_INFORMATION, WNODE_FLAG_TRACED_GUID,
};
use windows_sys::core::GUID;

use crate::model::Pid;

const SESSION: &str = "WinKillSwitch Network";
/// Microsoft-Windows-Kernel-Network.
const PROVIDER: GUID = GUID::from_u128(0x7dd42a49_5329_4832_8dfd_43d979153a88);
/// KERNEL_NETWORK_KEYWORD_IPV4 | KERNEL_NETWORK_KEYWORD_IPV6.
const KEYWORDS: u64 = 0x10 | 0x20;

static TOTALS: OnceLock<Mutex<HashMap<Pid, (u64, u64)>>> = OnceLock::new();
static RUNNING: AtomicBool = AtomicBool::new(false);
/// Every event the session delivered, for diagnostics.
pub static EVENTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn totals() -> &'static Mutex<HashMap<Pid, (u64, u64)>> {
    TOTALS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Which direction an event id counts as: TCP send/recv v4 (10/11) and v6 (26/27), UDP send/recv
/// v4 (42/43) and v6 (58/59). Retransmits and connection events are not traffic.
pub fn direction(id: u16) -> Option<bool> {
    match id {
        10 | 26 | 42 | 58 => Some(true),
        11 | 27 | 43 | 59 => Some(false),
        _ => None,
    }
}

unsafe extern "system" fn on_event(rec: *mut EVENT_RECORD) {
    // SAFETY: ETW hands a valid record for the duration of the callback; the payload is at least
    // PID (u32) then size (u32) for the events we count, which we check by length.
    unsafe {
        let r = &*rec;
        EVENTS.fetch_add(1, Ordering::Relaxed);
        let Some(sent) = direction(r.EventHeader.EventDescriptor.Id) else { return };
        if r.UserDataLength < 8 || r.UserData.is_null() {
            return;
        }
        let p = r.UserData as *const u8;
        let pid = u32::from_le_bytes(std::ptr::read_unaligned(p as *const [u8; 4]));
        let size = u32::from_le_bytes(std::ptr::read_unaligned(p.add(4) as *const [u8; 4])) as u64;
        if let Ok(mut t) = totals().lock() {
            let e = t.entry(pid).or_default();
            if sent {
                e.0 += size;
            } else {
                e.1 += size;
            }
        }
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// A properties block with room for the session name after it, as ETW wants.
fn properties() -> Vec<u8> {
    let head = std::mem::size_of::<EVENT_TRACE_PROPERTIES>();
    let mut buf = vec![0u8; head + (SESSION.len() + 1) * 2 + 16];
    // SAFETY: the buffer is larger than the struct and zeroed; the struct is plain data.
    unsafe {
        let p = &mut *(buf.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES);
        p.Wnode.BufferSize = buf.len() as u32;
        p.Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        p.Wnode.ClientContext = 1; // QPC timestamps
        p.LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
        p.FlushTimer = 1;
        p.LoggerNameOffset = head as u32;
    }
    buf
}

/// Stop a session left behind by an earlier run (or this one, on exit).
pub fn stop() {
    let name = wide(SESSION);
    let mut props = properties();
    // SAFETY: a valid name and a properties block sized as above.
    unsafe {
        ControlTraceW(CONTROLTRACE_HANDLE { Value: 0 }, name.as_ptr(), props.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES, EVENT_TRACE_CONTROL_STOP);
    }
    RUNNING.store(false, Ordering::SeqCst);
}

/// Start counting. Returns false without administrator rights or if ETW refuses.
pub fn start() -> bool {
    if RUNNING.load(Ordering::SeqCst) {
        return true;
    }
    stop();
    let name = wide(SESSION);
    let mut props = properties();
    let mut handle = CONTROLTRACE_HANDLE { Value: 0 };
    // SAFETY: as in `stop`; `handle` receives the session.
    unsafe {
        if StartTraceW(&mut handle, name.as_ptr(), props.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES) != 0 {
            return false;
        }
        if EnableTraceEx2(handle, &PROVIDER, EVENT_CONTROL_CODE_ENABLE_PROVIDER, TRACE_LEVEL_INFORMATION as u8, KEYWORDS, 0, 0, std::ptr::null()) != 0 {
            stop();
            return false;
        }
    }
    RUNNING.store(true, Ordering::SeqCst);
    std::thread::Builder::new()
        .name("wks-net".into())
        .spawn(move || {
            let mut name = wide(SESSION);
            // SAFETY: the logfile struct is zeroed plain data with the fields ProcessTrace needs;
            // `name` outlives OpenTraceW. ProcessTrace blocks until the session stops.
            unsafe {
                let mut lf: EVENT_TRACE_LOGFILEW = std::mem::zeroed();
                lf.LoggerName = name.as_mut_ptr();
                lf.Anonymous1.ProcessTraceMode = PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
                lf.Anonymous2.EventRecordCallback = Some(on_event);
                let h: PROCESSTRACE_HANDLE = OpenTraceW(&mut lf);
                if h.Value == u64::MAX {
                    RUNNING.store(false, Ordering::SeqCst);
                    return;
                }
                ProcessTrace(&h, 1, std::ptr::null(), std::ptr::null());
            }
            RUNNING.store(false, Ordering::SeqCst);
        })
        .is_ok()
}

pub fn running() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

/// Bytes per second per process since the last call.
#[derive(Default)]
pub struct NetMeter {
    last: HashMap<Pid, (u64, u64)>,
    at: Option<std::time::Instant>,
}

impl NetMeter {
    pub fn rates(&mut self) -> HashMap<Pid, u64> {
        let now = totals().lock().map(|t| t.clone()).unwrap_or_default();
        let secs = self.at.replace(std::time::Instant::now()).map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);
        let mut out = HashMap::new();
        if secs > 0.05 {
            for (pid, (s, r)) in &now {
                let (ps, pr) = self.last.get(pid).copied().unwrap_or((0, 0));
                let d = s.saturating_sub(ps) + r.saturating_sub(pr);
                if d > 0 {
                    out.insert(*pid, (d as f64 / secs) as u64);
                }
            }
        }
        self.last = now;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_only_traffic_events() {
        assert_eq!(direction(10), Some(true));
        assert_eq!(direction(59), Some(false));
        assert_eq!(direction(12), None, "connect is not traffic");
        assert_eq!(direction(14), None, "retransmit is not counted twice");
    }

    /// Needs administrator rights: sends a few small UDP packets to TEST-NET-1 (192.0.2.1,
    /// reserved for documentation, dropped harmlessly) and expects to see them under our PID.
    /// Loopback traffic is not attributed to processes by this provider.
    #[test]
    #[ignore = "needs administrator rights and sends 20 small packets"]
    fn sees_our_own_traffic() {
        assert!(start(), "ETW session (run elevated)");
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let mut meter = NetMeter::default();
        meter.rates();
        let sock = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
        for _ in 0..20 {
            let _ = sock.send_to(&[0u8; 1200], "192.0.2.1:9");
        }
        std::thread::sleep(std::time::Duration::from_millis(2500));
        let r = meter.rates();
        stop();
        let ours = r.get(&std::process::id()).copied().unwrap_or(0);
        eprintln!("our rate: {ours} B/s; {} processes with traffic, {} events", r.len(), EVENTS.load(Ordering::Relaxed));
        assert!(ours > 0);
    }
}
