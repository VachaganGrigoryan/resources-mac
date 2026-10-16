use egui::Ui;

use crate::sampler::State;
use crate::theme::color;
use crate::widgets::{graph_box, group, property_rows};

pub fn show(ui: &mut Ui, s: &State) {
    group(ui, "Usage", |ui| {
        let total = s.cpu_total.last().unwrap_or(0.0);
        graph_box(
            ui,
            "Total usage",
            &format!("{total:.0} %"),
            &[(&s.cpu_total, color::CPU)],
            Some(100.0),
            120.0,
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
        property_rows(
            ui,
            &[
                ("Model", s.cpu_model.clone()),
                ("Logical Cores", s.cpu_cores.len().to_string()),
            ],
        );
    });
}
