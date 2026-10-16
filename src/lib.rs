//! resources-mac: a macOS system monitor inspired by GNOME Resources.

pub mod sample;

/// Points kept per graph series (one per second).
pub const HISTORY_LEN: usize = 60;
