use egui::Ui;

use crate::sampler::State;
use crate::theme::color;
use crate::widgets::{Item, boxed_list, graph_box, group};

pub fn show(ui: &mut Ui, s: &State) {
    group(ui, "Usage", |ui| {
        let total = s.cpu_total.last().unwrap_or(0.0);
        boxed_list(
            ui,
            &[Item::Graph {
                caption: "Total Usage",
                value: format!("{total:.0} %"),
                series: &[(&s.cpu_total, color::CPU)],
                y_max: Some(100.0),
            }],
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Usage by logical processors")
                .small()
                .weak(),
        );
        // Upstream uses a flow box; three columns at the 768 px clamp, fewer when narrow.
        let cols = ((ui.available_width() / 200.0) as usize).clamp(1, 3);
        for (row, chunk) in s.cpu_cores.chunks(cols).enumerate() {
            ui.columns(cols, |c| {
                for (i, core) in chunk.iter().enumerate() {
                    graph_box(
                        &mut c[i],
                        &format!("CPU {}", row * cols + i + 1),
                        &format!("{:.0} %", core.last().unwrap_or(0.0)),
                        &[(core, color::CPU_CORE)],
                        Some(100.0),
                        70.0,
                    );
                }
            });
        }
    });
    group(ui, "Properties", |ui| {
        boxed_list(
            ui,
            &[
                Item::Row {
                    title: "Model",
                    value: s.cpu_model.clone(),
                },
                Item::Row {
                    title: "Logical Cores",
                    value: s.cpu_cores.len().to_string(),
                },
            ],
        );
    });
}
