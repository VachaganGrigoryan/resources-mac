//! Disk Sweep: finds caches, build output, editor metadata and gitignored files in chosen
//! folders, plus Docker disk usage, and moves what you pick to the Trash.

pub mod app;
pub mod classify;
pub mod docker;
pub mod pages;
pub mod scan;
pub mod suggest;
pub mod treemap;
pub mod widgets;
pub mod worker;

pub const APP_NAME: &str = "Disk Sweep";
