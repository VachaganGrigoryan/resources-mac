use egui::Ui;

use crate::sampler::State;
use crate::widgets::{card, details, fmt_bytes, graph_card};

pub fn show(ui: &mut Ui, s: &State) {
    let used = format!("{} / {}", fmt_bytes(s.mem_used), fmt_bytes(s.mem_total));
    graph_card(ui, "Memory", &used, &[&s.mem_hist], Some(100.0));
    card(ui, |ui| {
        details(
            ui,
            "mem_details",
            &[
                ("Used", fmt_bytes(s.mem_used)),
                (
                    "Available",
                    fmt_bytes(s.mem_total.saturating_sub(s.mem_used)),
                ),
                ("Total", fmt_bytes(s.mem_total)),
                (
                    "Swap used",
                    format!("{} / {}", fmt_bytes(s.swap_used), fmt_bytes(s.swap_total)),
                ),
            ],
        );
    });
}
