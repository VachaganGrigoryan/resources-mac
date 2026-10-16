use egui::Ui;

use crate::sampler::State;
use crate::widgets::{card, details, graph, graph_card};

pub fn show(ui: &mut Ui, s: &State) {
    let total = s.cpu_total.last().unwrap_or(0.0);
    graph_card(
        ui,
        "Total usage",
        &format!("{total:.0} %"),
        &[&s.cpu_total],
        Some(100.0),
    );
    card(ui, |ui| {
        details(
            ui,
            "cpu_details",
            &[
                ("Model", s.cpu_model.clone()),
                ("Logical cores", s.cpu_cores.len().to_string()),
            ],
        );
    });
    card(ui, |ui| {
        ui.strong("Per core");
        let cols = 4;
        let width = (ui.available_width() - 8.0 * (cols - 1) as f32) / cols as f32;
        egui::Grid::new("cores").spacing([8.0, 8.0]).show(ui, |ui| {
            for (i, core) in s.cpu_cores.iter().enumerate() {
                ui.vertical(|ui| {
                    ui.set_width(width);
                    ui.small(format!(
                        "Core {} · {:.0} %",
                        i + 1,
                        core.last().unwrap_or(0.0)
                    ));
                    graph(ui, &[core], Some(100.0), 48.0);
                });
                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });
    });
}
