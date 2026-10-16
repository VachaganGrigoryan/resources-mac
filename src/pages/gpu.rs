use egui::Ui;

use crate::collect::gpu::GpuStat;
use crate::theme::color;
use crate::widgets::{fmt_bytes, graph_box, group, property_rows};

pub fn show(ui: &mut Ui, g: &GpuStat) {
    group(ui, "Usage", |ui| {
        let util = g.util_hist.last().unwrap_or(0.0);
        graph_box(
            ui,
            "GPU Usage",
            &format!("{util:.0} %"),
            &[(&g.util_hist, color::GPU)],
            Some(100.0),
            120.0,
        );
    });
    group(ui, "Properties", |ui| {
        let mem = g.mem_used.map_or_else(|| "Unknown".to_owned(), fmt_bytes);
        property_rows(ui, &[("Model", g.name.clone()), ("Memory in Use", mem)]);
    });
}
