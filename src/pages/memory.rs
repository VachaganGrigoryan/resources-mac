use egui::Ui;

use crate::sampler::State;
use crate::theme::color;
use crate::widgets::{fmt_bytes, graph_box, group, property_rows};

pub fn show(ui: &mut Ui, s: &State) {
    group(ui, "Usage", |ui| {
        graph_box(
            ui,
            "Memory",
            &format!("{} / {}", fmt_bytes(s.mem_used), fmt_bytes(s.mem_total)),
            &[(&s.mem_hist, color::MEMORY)],
            Some(100.0),
            120.0,
        );
        graph_box(
            ui,
            "Swap",
            &format!("{} / {}", fmt_bytes(s.swap_used), fmt_bytes(s.swap_total)),
            &[(&s.swap_hist, color::SWAP)],
            Some(100.0),
            120.0,
        );
    });
    group(ui, "Properties", |ui| {
        property_rows(
            ui,
            &[
                ("Total", fmt_bytes(s.mem_total)),
                (
                    "Available",
                    fmt_bytes(s.mem_total.saturating_sub(s.mem_used)),
                ),
                ("Swap Total", fmt_bytes(s.swap_total)),
            ],
        );
    });
}
