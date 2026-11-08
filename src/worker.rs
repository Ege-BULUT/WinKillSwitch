//! Everything slow runs here, off the UI thread: process sampling, memory status, the memory
//! cleaner and the disk cleaner. The UI sends `Cmd`s and drains `Msg`s in its poll, so the window
//! keeps drawing at full rate even when the machine is struggling.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

use crate::model::Snapshot;
use crate::sys::cleanup::{self, Category, Cleaned, Found};
use crate::sys::memory::{self, MemStatus, RamReport};
use crate::sys::procs::Sampler;

pub const SAMPLE_EVERY: Duration = Duration::from_millis(1500);

pub enum Cmd {
    /// Sample now instead of waiting for the next tick.
    Refresh,
    CleanRam { elevated: bool },
    ScanDisk { running: Vec<String>, elevated: bool },
    CleanDisk { cats: Vec<Category>, running: Vec<String>, elevated: bool },
}

pub enum Msg {
    Sample(Snapshot, MemStatus),
    RamStage(&'static str),
    Ram(RamReport),
    DiskScanning { done: usize, total: usize, current: String, files: u64, bytes: u64 },
    DiskFound(Vec<(Category, Found)>),
    DiskCleaning { done: usize, total: usize, current: String, freed: u64 },
    DiskCleaned { results: Vec<(String, Cleaned)>, drive: (u64, u64) },
}

pub struct Worker {
    tx: Sender<Cmd>,
    rx: Receiver<Msg>,
}

impl Worker {
    pub fn start() -> Worker {
        let (tx, cmd_rx) = channel::<Cmd>();
        let (msg_tx, rx) = channel::<Msg>();
        std::thread::Builder::new()
            .name("wks-sampler".into())
            .spawn(move || run(cmd_rx, msg_tx))
            .expect("spawn sampler thread");
        Worker { tx, rx }
    }

    pub fn send(&self, c: Cmd) {
        let _ = self.tx.send(c);
    }

    /// Everything that has arrived since the last call.
    pub fn drain(&self) -> Vec<Msg> {
        self.rx.try_iter().collect()
    }
}

fn lower_priority() {
    use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL};
    // SAFETY: adjusts only this thread's own priority.
    unsafe {
        SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
    }
}

fn run(cmds: Receiver<Cmd>, out: Sender<Msg>) {
    lower_priority();
    let mut sampler = Sampler::new();
    let mut next = Instant::now();
    loop {
        let wait = next.saturating_duration_since(Instant::now());
        match cmds.recv_timeout(wait) {
            Ok(Cmd::Refresh) | Err(RecvTimeoutError::Timeout) => {
                if out.send(Msg::Sample(sampler.sample(), memory::status())).is_err() {
                    return; // the window is gone
                }
                next = Instant::now() + SAMPLE_EVERY;
            }
            Ok(Cmd::CleanRam { elevated }) => {
                let out = out.clone();
                std::thread::spawn(move || {
                    let stage_out = out.clone();
                    let r = memory::clean(elevated, &move |st| {
                        let _ = stage_out.send(Msg::RamStage(st));
                    });
                    let _ = out.send(Msg::Ram(r));
                });
            }
            Ok(Cmd::ScanDisk { running, elevated }) => {
                let out = out.clone();
                std::thread::spawn(move || {
                    lower_priority();
                    let cats = cleanup::categories();
                    let total = cats.len();
                    let mut found = Vec::with_capacity(total);
                    let (mut files, mut bytes) = (0u64, 0u64);
                    for (i, c) in cats.into_iter().enumerate() {
                        let _ = out.send(Msg::DiskScanning { done: i, total, current: c.name.clone(), files, bytes });
                        let name = c.name.clone();
                        let f = cleanup::scan_with(&c, &running, elevated, &mut |f, b| {
                            let _ = out.send(Msg::DiskScanning { done: i, total, current: name.clone(), files: files + f, bytes: bytes + b });
                        });
                        files += f.files;
                        bytes += f.bytes;
                        found.push((c, f));
                    }
                    let _ = out.send(Msg::DiskFound(found));
                });
            }
            Ok(Cmd::CleanDisk { cats, running, elevated }) => {
                let out = out.clone();
                std::thread::spawn(move || {
                    lower_priority();
                    let total = cats.len();
                    let mut results = Vec::with_capacity(total);
                    let mut freed = 0;
                    for (i, c) in cats.iter().enumerate() {
                        let _ = out.send(Msg::DiskCleaning { done: i, total, current: c.name.clone(), freed });
                        let name = c.name.clone();
                        let r = cleanup::clean_with(c, &running, elevated, &mut |f| {
                            let _ = out.send(Msg::DiskCleaning { done: i, total, current: name.clone(), freed: freed + f });
                        });
                        freed += r.freed;
                        results.push((c.id.to_string(), r));
                    }
                    let _ = out.send(Msg::DiskCleaned { results, drive: cleanup::system_drive() });
                });
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}
