use egui::Ui;

use crate::collect::battery::BatteryStat;
use crate::theme::color;
use crate::widgets::{graph_box, group, property_rows};

pub fn show(ui: &mut Ui, b: &BatteryStat) {
    group(ui, "Usage", |ui| {
        graph_box(
            ui,
            "Charge",
            &format!("{:.0} %", b.percent),
            &[(&b.charge_hist, color::BATTERY)],
            Some(100.0),
            120.0,
        );
    });
    group(ui, "Properties", |ui| {
        let cycles = b
            .cycles
            .map_or_else(|| "Unknown".to_owned(), |c| c.to_string());
        property_rows(
            ui,
            &[
                ("State", b.state.clone()),
                ("Health", format!("{:.0} %", b.health)),
                ("Cycles", cycles),
            ],
        );
    });
}
