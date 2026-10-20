//! One watched folder: scan progress or error, summary, treemap and suggestions.

use std::sync::atomic::Ordering;

use egui::{RichText, Ui};
use resources_mac::widgets::{card, fmt_bytes};

use crate::worker::RootScan;

pub fn show(ui: &mut Ui, scan: &RootScan) {
    if scan.scanning {
        let files = scan.progress.files.load(Ordering::Relaxed);
        let bytes = scan.progress.bytes.load(Ordering::Relaxed);
        ui.label(RichText::new(format!("Scanning… {files} files · {}", fmt_bytes(bytes))).weak());
    }
    if let Some(Err(e)) = &scan.result {
        card(ui, |ui| {
            ui.label(RichText::new("Couldn't scan this folder").strong());
            ui.colored_label(ui.visuals().error_fg_color, e);
        });
    }
}
