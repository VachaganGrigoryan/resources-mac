use egui::Ui;

use crate::collect::gpu::GpuStat;
use crate::widgets::{card, details, fmt_bytes, graph_card};

pub fn show(ui: &mut Ui, g: &GpuStat) {
    let util = g.util_hist.last().unwrap_or(0.0);
    graph_card(
        ui,
        "Usage",
        &format!("{util:.0} %"),
        &[&g.util_hist],
        Some(100.0),
    );
    card(ui, |ui| {
        let mem = g.mem_used.map_or_else(|| "Unknown".to_owned(), fmt_bytes);
        details(
            ui,
            "gpu_details",
            &[("Model", g.name.clone()), ("Memory in use", mem)],
        );
    });
}
