//! CPU and memory ceilings for a program, by name, kept across restarts of that program.
//!
//! - CPU: every process of the program joins one Job Object with a hard CPU rate cap, so all of
//!   them together never use more than that share of the processor. The program only runs slower.
//! - Memory: each process gets a hard working set maximum. Windows keeps at most that much of it
//!   in RAM and pages the rest out, so allocations never fail; the program can get slower.
//!
//! Removing a limit lifts both. Windows' own processes are never limited.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_CPU_RATE_CONTROL_ENABLE, JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP, JOBOBJECT_CPU_RATE_CONTROL_INFORMATION,
    JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0, JobObjectCpuRateControlInformation, SetInformationJobObject,
};
use windows_sys::Win32::System::Memory::{QUOTA_LIMITS_HARDWS_MAX_DISABLE, QUOTA_LIMITS_HARDWS_MAX_ENABLE, QUOTA_LIMITS_HARDWS_MIN_DISABLE, SetProcessWorkingSetSizeEx};
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

use crate::model::{Pid, ProcKind, Snapshot};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limit {
    /// Image name, matched case-insensitively ("chrome.exe").
    pub name: String,
    /// Percent of the whole processor, 1..=100.
    pub cpu: Option<u8>,
    /// Megabytes of RAM per process.
    pub mem_mb: Option<u64>,
}

/// The choices the limits dialog offers.
pub const CPU_STEPS: [Option<u8>; 4] = [None, Some(50), Some(25), Some(10)];
pub const MEM_STEPS: [Option<u64>; 5] = [None, Some(4096), Some(2048), Some(1024), Some(512)];

/// Default working set bounds Windows gives a process; used to lift a memory limit.
const WS_MIN_DEFAULT: usize = 200 * 1024;
const WS_MAX_DEFAULT: usize = 1380 * 1024;

struct Job {
    handle: isize,
    cpu: Option<u8>,
}

impl Drop for Job {
    fn drop(&mut self) {
        // SAFETY: the handle came from CreateJobObjectW and is closed once.
        unsafe {
            CloseHandle(self.handle as HANDLE);
        }
    }
}

fn set_job_cpu(job: isize, cpu: Option<u8>) -> bool {
    let info = JOBOBJECT_CPU_RATE_CONTROL_INFORMATION {
        ControlFlags: if cpu.is_some() { JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP } else { 0 },
        // CpuRate is in hundredths of a percent of the whole machine.
        Anonymous: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0 { CpuRate: cpu.map(|c| c.clamp(1, 100) as u32 * 100).unwrap_or(0) },
    };
    // SAFETY: a valid job handle and a correctly sized struct.
    unsafe {
        SetInformationJobObject(job as HANDLE, JobObjectCpuRateControlInformation, &info as *const _ as *const _, std::mem::size_of_val(&info) as u32) != 0
    }
}

fn open(pid: Pid) -> Option<HANDLE> {
    // SAFETY: plain OpenProcess; the caller closes the handle.
    let h = unsafe { OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    (!h.is_null()).then_some(h)
}

/// The opened process really is `name` (process ids get reused) and is not part of Windows.
fn is_expected(h: HANDLE, name: &str) -> bool {
    use windows_sys::Win32::System::Threading::QueryFullProcessImageNameW;
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    // SAFETY: a valid handle with PROCESS_QUERY_LIMITED_INFORMATION and a sized buffer.
    if unsafe { QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len) } == 0 {
        return false;
    }
    let path = String::from_utf16_lossy(&buf[..len as usize]).to_lowercase();
    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into()).to_lowercase();
    let file = path.rsplit('\\').next().unwrap_or("");
    file == name.to_lowercase() && !path.starts_with(&(windir + "\\"))
}

fn set_working_set(h: HANDLE, mem_mb: Option<u64>) -> bool {
    // SAFETY: a valid process handle opened with PROCESS_SET_QUOTA.
    unsafe {
        match mem_mb {
            Some(mb) => {
                let max = (mb as usize) << 20;
                SetProcessWorkingSetSizeEx(h, (1usize << 20).min(max / 2), max, QUOTA_LIMITS_HARDWS_MAX_ENABLE | QUOTA_LIMITS_HARDWS_MIN_DISABLE) != 0
            }
            None => SetProcessWorkingSetSizeEx(h, WS_MIN_DEFAULT, WS_MAX_DEFAULT, QUOTA_LIMITS_HARDWS_MAX_DISABLE | QUOTA_LIMITS_HARDWS_MIN_DISABLE) != 0,
        }
    }
}

/// Short text for a limit: "CPU 25% · RAM 1 GB".
pub fn describe(l: &Limit) -> String {
    let mut parts = Vec::new();
    if let Some(c) = l.cpu {
        parts.push(format!("CPU {c}%"));
    }
    if let Some(m) = l.mem_mb {
        parts.push(if m >= 1024 { format!("RAM {} GB", m / 1024) } else { format!("RAM {m} MB") });
    }
    parts.join(" \u{00b7} ")
}

/// Keeps limits applied to the processes that match them, as processes come and go.
#[derive(Default)]
pub struct Enforcer {
    jobs: HashMap<String, Job>,
    /// pid -> (limit name, memory setting applied). Failures are recorded too, so they are not
    /// retried every tick.
    applied: HashMap<Pid, (String, Option<u64>)>,
    /// Processes skipped for good: not what their name says, part of Windows, or access denied.
    refused: std::collections::HashSet<Pid>,
}

impl Enforcer {
    /// Bring every running process in line with `limits`. Returns one message per process a
    /// limit could not be applied to.
    pub fn sync(&mut self, limits: &[Limit], snap: &Snapshot) -> Vec<String> {
        let mut errors = Vec::new();
        let by_name: HashMap<String, &Limit> = limits.iter().filter(|l| l.cpu.is_some() || l.mem_mb.is_some()).map(|l| (l.name.to_lowercase(), l)).collect();

        // Lift limits that were removed or changed away from a process.
        let alive: std::collections::HashSet<Pid> = snap.procs.iter().map(|p| p.pid).collect();
        self.applied.retain(|pid, _| alive.contains(pid));
        self.refused.retain(|pid| alive.contains(pid));
        let stale: Vec<Pid> = self
            .applied
            .iter()
            .filter(|(_, (name, _))| !by_name.contains_key(name))
            .map(|(pid, _)| *pid)
            .collect();
        for pid in stale {
            if let Some(h) = open(pid) {
                set_working_set(h, None);
                // SAFETY: opened above.
                unsafe { CloseHandle(h) };
            }
            self.applied.remove(&pid);
        }
        self.jobs.retain(|name, job| {
            if by_name.contains_key(name) {
                true
            } else {
                set_job_cpu(job.handle, None);
                false
            }
        });

        for p in &snap.procs {
            let key = p.name.to_lowercase();
            let Some(limit) = by_name.get(&key) else { continue };
            if p.kind == ProcKind::Windows || p.pid == std::process::id() || self.refused.contains(&p.pid) {
                continue;
            }
            let job = self.jobs.entry(key.clone()).or_insert_with(|| {
                // SAFETY: an unnamed job with default security.
                let h = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
                Job { handle: h as isize, cpu: None }
            });
            if job.handle == 0 {
                continue;
            }
            if job.cpu != limit.cpu {
                set_job_cpu(job.handle, limit.cpu);
                job.cpu = limit.cpu;
            }
            match self.applied.get(&p.pid) {
                Some((n, mem)) if *n == key && *mem == limit.mem_mb => continue,
                _ => {}
            }
            let prev = self.applied.get(&p.pid).map(|(_, m)| *m);
            let first = prev.is_none();
            let Some(h) = open(p.pid) else {
                self.refused.insert(p.pid);
                errors.push(format!("{} (PID {}): access denied, try running as administrator", p.name, p.pid));
                continue;
            };
            if !is_expected(h, &p.name) {
                // SAFETY: opened above.
                unsafe { CloseHandle(h) };
                self.refused.insert(p.pid);
                continue;
            }
            let mut ok = true;
            if first {
                // SAFETY: both handles are valid; a process can be in several (nested) jobs.
                ok &= unsafe { AssignProcessToJobObject(job.handle as HANDLE, h) } != 0;
            }
            if limit.mem_mb.is_some() || prev.flatten().is_some() {
                ok &= set_working_set(h, limit.mem_mb);
            }
            // SAFETY: opened above.
            unsafe { CloseHandle(h) };
            self.applied.insert(p.pid, (key.clone(), limit.mem_mb));
            if !ok {
                errors.push(format!("{} (PID {}): Windows refused the limit", p.name, p.pid));
            }
        }
        errors
    }

    /// How many running processes a limit is applied to.
    pub fn count(&self, name: &str) -> usize {
        let key = name.to_lowercase();
        self.applied.values().filter(|(n, _)| *n == key).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ProcInfo;

    #[test]
    fn describes() {
        assert_eq!(describe(&Limit { name: "a".into(), cpu: Some(25), mem_mb: Some(1024) }), "CPU 25% \u{00b7} RAM 1 GB");
        assert_eq!(describe(&Limit { name: "a".into(), cpu: None, mem_mb: Some(512) }), "RAM 512 MB");
    }

    fn cpu_time(pid: Pid) -> f64 {
        use windows_sys::Win32::Foundation::FILETIME;
        use windows_sys::Win32::System::Threading::GetProcessTimes;
        let z = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let (mut c, mut e, mut k, mut u) = (z, z, z, z);
        let h = open(pid).unwrap();
        // SAFETY: valid handle and out-pointers.
        unsafe {
            GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u);
            CloseHandle(h);
        }
        let t = |f: FILETIME| ((f.dwHighDateTime as u64) << 32 | f.dwLowDateTime as u64) as f64 / 1e7;
        t(k) + t(u)
    }

    fn proc_info(pid: Pid, name: &str) -> ProcInfo {
        ProcInfo {
            pid,
            parent: None,
            name: name.into(),
            exe: None,
            cpu: 0.0,
            mem: 0,
            start: 0,
            disk: 0,
            net: 0,
            gpu: 0.0,
            gpu_engine: String::new(),
            status: "Running",
            kind: ProcKind::Background,
            cmd: String::new(),
            session: None,
            publisher: String::new(),
            description: String::new(),
        }
    }

    /// PowerShell copied out of the Windows folder, which the enforcer rightly refuses to touch.
    fn outside_powershell(tag: &str) -> std::path::PathBuf {
        let src = std::path::PathBuf::from(std::env::var("SystemRoot").unwrap()).join("System32").join("WindowsPowerShell").join("v1.0").join("powershell.exe");
        let dst = std::env::temp_dir().join(format!("wks-{tag}-{}.exe", std::process::id()));
        std::fs::copy(src, &dst).unwrap();
        dst
    }

    /// A busy child uses a whole core; capped at 2% of the machine it uses about that.
    #[test]
    #[ignore = "burns a core for a few seconds"]
    fn cpu_cap_slows_a_busy_child() {
        let exe = outside_powershell("burn");
        let name = exe.file_name().unwrap().to_string_lossy().into_owned();
        let mut child = std::process::Command::new(&exe).args(["-NoProfile", "-Command", "while($true){}"]).spawn().unwrap();
        let pid = child.id();
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f64;
        let measure = || {
            let a = cpu_time(pid);
            std::thread::sleep(std::time::Duration::from_secs(3));
            (cpu_time(pid) - a) / 3.0 / cores * 100.0
        };
        let before = measure();
        let snap = Snapshot { procs: vec![proc_info(pid, &name)], ..Default::default() };
        let mut e = Enforcer::default();
        let limits = vec![Limit { name: name.clone(), cpu: Some(2), mem_mb: None }];
        assert!(e.sync(&limits, &snap).is_empty());
        let capped = measure();
        e.sync(&[], &snap);
        let after = measure();
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_file(&exe);
        eprintln!("machine share: before {before:.1}%, capped {capped:.1}%, lifted {after:.1}%");
        assert!(capped <= 3.0, "capped at {capped}");
        assert!(after > capped * 1.5, "lifted: {after}");
    }

    fn working_set(pid: Pid) -> u64 {
        use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
        let h = open(pid).unwrap();
        // SAFETY: valid handle; the struct is zeroed and sized.
        unsafe {
            let mut c: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
            c.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            GetProcessMemoryInfo(h, &mut c, c.cb);
            CloseHandle(h);
            c.WorkingSetSize as u64
        }
    }

    /// A child that keeps touching 400 MB stays near a 128 MB ceiling, and keeps running.
    #[test]
    #[ignore = "allocates 400 MB for a few seconds"]
    fn memory_ceiling_holds_a_child_down() {
        let script = "$a = New-Object byte[] 400MB; while($true){ for($i=0;$i -lt $a.Length;$i+=4096){ $a[$i]=1 } }";
        let exe = outside_powershell("mem");
        let name = exe.file_name().unwrap().to_string_lossy().into_owned();
        let mut child = std::process::Command::new(&exe).args(["-NoProfile", "-Command", script]).spawn().unwrap();
        let pid = child.id();
        std::thread::sleep(std::time::Duration::from_secs(4));
        let before = working_set(pid) >> 20;
        let snap = Snapshot { procs: vec![proc_info(pid, &name)], ..Default::default() };
        let mut e = Enforcer::default();
        assert!(e.sync(&[Limit { name: name.clone(), cpu: None, mem_mb: Some(128) }], &snap).is_empty());
        std::thread::sleep(std::time::Duration::from_secs(3));
        let capped = working_set(pid) >> 20;
        let alive = child.try_wait().unwrap().is_none();
        e.sync(&[], &snap);
        std::thread::sleep(std::time::Duration::from_secs(3));
        let lifted = working_set(pid) >> 20;
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_file(&exe);
        eprintln!("working set: before {before} MB, capped {capped} MB, lifted {lifted} MB, alive {alive}");
        assert!(before > 300 && capped <= 136 && alive && lifted > 300);
    }

    /// A Windows program is refused even when a rule names it, so a reused or made-up process id
    /// can never cap part of Windows.
    #[test]
    fn refuses_windows_programs() {
        let mut child = std::process::Command::new("cmd").args(["/c", "ping -n 30 127.0.0.1 >nul"]).spawn().unwrap();
        let mut p = proc_info(child.id(), "cmd.exe");
        p.kind = ProcKind::App; // even if it were misclassified
        let snap = Snapshot { procs: vec![p], ..Default::default() };
        let mut e = Enforcer::default();
        assert!(e.sync(&[Limit { name: "cmd.exe".into(), cpu: Some(10), mem_mb: Some(64) }], &snap).is_empty());
        assert_eq!(e.count("cmd.exe"), 0);
        let _ = child.kill();
    }

    /// A process id whose real program differs from the snapshot's name is left alone.
    #[test]
    fn refuses_a_mismatched_pid() {
        let exe = outside_powershell("pid");
        let mut child = std::process::Command::new(&exe).args(["-NoProfile", "-Command", "Start-Sleep 20"]).spawn().unwrap();
        let snap = Snapshot { procs: vec![proc_info(child.id(), "Spotify.exe")], ..Default::default() };
        let mut e = Enforcer::default();
        e.sync(&[Limit { name: "Spotify.exe".into(), cpu: Some(10), mem_mb: None }], &snap);
        assert_eq!(e.count("Spotify.exe"), 0);
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_file(&exe);
    }

    /// Caps a child we start ourselves (from outside the Windows folder), then lifts it.
    #[test]
    fn limits_a_child_and_lifts_it() {
        let exe = outside_powershell("lift");
        let name = exe.file_name().unwrap().to_string_lossy().into_owned();
        let mut child = std::process::Command::new(&exe).args(["-NoProfile", "-Command", "Start-Sleep 20"]).spawn().unwrap();
        let snap = Snapshot { procs: vec![proc_info(child.id(), &name)], ..Default::default() };
        let mut e = Enforcer::default();
        let limits = vec![Limit { name: name.to_uppercase(), cpu: Some(10), mem_mb: Some(64) }];
        assert!(e.sync(&limits, &snap).is_empty());
        assert_eq!(e.count(&name), 1);
        assert!(e.sync(&limits, &snap).is_empty(), "idempotent");
        assert!(e.sync(&[], &snap).is_empty());
        assert_eq!(e.count(&name), 0);
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_file(&exe);
    }
}
