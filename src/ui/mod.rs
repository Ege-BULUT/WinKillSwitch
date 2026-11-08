//! The Rune view, split by screen area.
pub mod clean;
pub mod columns;
pub mod fmt;
pub mod gfx;
pub mod glass;
pub mod groups;
pub mod icons;
pub mod diagnose;
pub mod info;
pub mod limits;
pub mod locks;
pub mod overlays;
pub mod palette;
pub mod palettes;
pub mod pauseswitch;
pub mod processes;
pub mod root;
pub mod settings;
pub mod startup;
pub mod theme;
pub mod viz;

pub use root::view;

/// Release gates: a feature ships (and its tab or card appears) once it has been tested.
/// v0.2.0 turns on the memory cleaner, v0.3.0 the disk cleaner.
pub const MEMORY_CLEANER: bool = true;
pub const DISK_CLEANER: bool = true;
