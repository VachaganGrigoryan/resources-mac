use egui::Ui;

use crate::sampler::State;
use crate::theme::color;
use crate::widgets::{Item, boxed_list, fmt_bytes, fmt_used, group};

pub fn show(ui: &mut Ui, s: &State) {
    group(ui, "Usage", |ui| {
        boxed_list(
            ui,
            &[
                Item::Graph {
                    caption: "Memory",
                    value: fmt_used(s.mem_used, s.mem_total),
                    series: &[(&s.mem_hist, color::MEMORY)],
                    y_max: Some(100.0),
                },
                Item::Graph {
                    caption: "Swap",
                    value: fmt_used(s.swap_used, s.swap_total),
                    series: &[(&s.swap_hist, color::SWAP)],
                    y_max: Some(100.0),
                },
            ],
        );
    });
    group(ui, "Properties", |ui| {
        boxed_list(
            ui,
            &[
                Item::Row {
                    title: "Total",
                    value: fmt_bytes(s.mem_total),
                },
                Item::Row {
                    title: "Available",
                    value: fmt_bytes(s.mem_total.saturating_sub(s.mem_used)),
                },
                Item::Row {
                    title: "Swap Total",
                    value: fmt_bytes(s.swap_total),
                },
            ],
        );
    });
}
