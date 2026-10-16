//! resources-mac: a macOS system monitor inspired by GNOME Resources.

pub mod app;
pub mod pages;
pub mod sample;
pub mod sampler;
pub mod theme;
pub mod widgets;

/// Points kept per graph series (one per second).
pub const HISTORY_LEN: usize = 60;
