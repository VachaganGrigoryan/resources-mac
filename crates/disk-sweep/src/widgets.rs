//! Disk Sweep drawing helpers on top of `resources_mac::widgets`.

use egui::Color32;
use resources_mac::theme::color;

use crate::scan::Kind;

pub fn kind_color(kind: Kind) -> Color32 {
    match kind {
        Kind::Source => color::CPU,
        Kind::Cache => color::DRIVE,
        Kind::Editor => color::MEMORY,
        Kind::Ignored => Color32::from_rgb(0x8E, 0x8E, 0x93),
        Kind::Git => color::POWER,
    }
}
