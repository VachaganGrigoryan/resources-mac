use egui::Ui;

use crate::collect::gpu::GpuStat;
use crate::theme::color;
use crate::widgets::{Item, boxed_list, fmt_bytes, group};

pub fn show(ui: &mut Ui, g: &GpuStat) {
    let mem = g.mem_used.map_or_else(|| "Unknown".to_owned(), fmt_bytes);
    group(ui, "Usage", |ui| {
        let util = g.util_hist.last().unwrap_or(0.0);
        boxed_list(
            ui,
            &[
                Item::Graph {
                    caption: "Total Usage",
                    value: format!("{util:.0} %"),
                    series: &[(&g.util_hist, color::GPU)],
                    y_max: Some(100.0),
                },
                Item::Row {
                    title: "Video Memory in Use",
                    value: mem,
                },
            ],
        );
    });
    group(ui, "Properties", |ui| {
        boxed_list(
            ui,
            &[Item::Row {
                title: "Model",
                value: g.name.clone(),
            }],
        );
    });
}
