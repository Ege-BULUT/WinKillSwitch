//! System access. Each submodule has one owner; the UI only calls the functions listed here.
pub mod autostart;
pub mod cleanup;
pub mod elevate;
pub mod gpu;
pub mod guard;
pub mod info;
pub mod kill;
pub mod memory;
pub mod procs;
pub mod related;
pub mod shell;
pub mod verinfo;
