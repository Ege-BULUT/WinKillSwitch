//! Process sampling with sysinfo. CPU needs two samples, so keep one `Sampler` alive.
use std::path::Path;
use std::time::Instant;

use sysinfo::{DiskUsage, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System, UpdateKind};

use crate::model::{ProcInfo, ProcKind, Snapshot};
use crate::sys::verinfo::VerCache;

pub struct Sampler {
    sys: System,
    cores: f32,
    ver: VerCache,
    last: Option<Instant>,
    windir: String,
}

impl Sampler {
    pub fn new() -> Sampler {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        let cores = sys.cpus().len().max(1) as f32;
        let windir = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into()).to_lowercase();
        Sampler { sys, cores, ver: VerCache::default(), last: None, windir }
    }

    /// Refresh and return the current process list (dead processes removed).
    pub fn sample(&mut self) -> Snapshot {
        let kind = ProcessRefreshKind::nothing()
            .with_memory()
            .with_cpu()
            .with_disk_usage()
            .with_cmd(UpdateKind::OnlyIfNotSet)
            .with_exe(UpdateKind::OnlyIfNotSet);
        self.sys.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

        let elapsed = self.last.replace(Instant::now()).map(|t| t.elapsed().as_secs_f64()).unwrap_or(1.0).max(0.1);

        let procs = self
            .sys
            .processes()
            .values()
            .filter(|p| p.pid().as_u32() != 0)
            .map(|p| {
                let exe = p.exe().map(|e| e.to_path_buf());
                let name = p.name().to_string_lossy().into_owned();
                let session = p.session_id().map(|s| s.as_u32());
                let kind = classify(exe.as_deref(), &self.windir, session);
                let DiskUsage { read_bytes, written_bytes, .. } = p.disk_usage();
                let disk = (((read_bytes + written_bytes) as f64) / elapsed) as u64;
                let ver = exe.as_deref().map(|e| self.ver.get(e)).unwrap_or_default();
                let description = if ver.description.is_empty() { name.clone() } else { ver.description };
                let cmd = p.cmd().iter().map(|a| a.to_string_lossy()).collect::<Vec<_>>().join(" ");
                ProcInfo {
                    pid: p.pid().as_u32(),
                    parent: p.parent().map(|x| x.as_u32()),
                    name,
                    exe,
                    cpu: (p.cpu_usage() / self.cores).clamp(0.0, 100.0),
                    mem: p.memory(),
                    start: p.start_time(),
                    disk,
                    status: status_label(p.status()),
                    kind,
                    cmd,
                    session,
                    publisher: ver.publisher,
                    description,
                }
            })
            .collect();

        Snapshot {
            procs,
            total_mem: self.sys.total_memory(),
            used_mem: self.sys.used_memory(),
            cpu_total: self.sys.global_cpu_usage(),
        }
    }
}

fn status_label(s: ProcessStatus) -> &'static str {
    match s {
        ProcessStatus::Run | ProcessStatus::Idle => "Running",
        ProcessStatus::Stop => "Suspended",
        ProcessStatus::Zombie => "Not responding",
        _ => "Running",
    }
}

fn classify(exe: Option<&Path>, windir: &str, session: Option<u32>) -> ProcKind {
    match exe {
        None => ProcKind::Background,
        Some(e) => {
            let p = e.to_string_lossy().to_lowercase();
            if p.starts_with(windir) {
                ProcKind::Windows
            } else if session == Some(0) {
                ProcKind::Background
            } else {
                ProcKind::App
            }
        }
    }
}
