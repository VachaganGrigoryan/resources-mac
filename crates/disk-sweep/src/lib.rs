//! Disk Sweep: finds caches, build output, editor metadata and gitignored files in chosen
//! folders, plus Docker disk usage, and moves what you pick to the Trash.

pub const APP_NAME: &str = "Disk Sweep";

pub mod classify;
pub mod scan;
