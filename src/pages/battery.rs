use egui::Ui;

use crate::collect::battery::BatteryStat;
use crate::widgets::{card, details, graph_card};

pub fn show(ui: &mut Ui, b: &BatteryStat) {
    graph_card(
        ui,
        "Charge",
        &format!("{:.0} %", b.percent),
        &[&b.charge_hist],
        Some(100.0),
    );
    card(ui, |ui| {
        let cycles = b
            .cycles
            .map_or_else(|| "Unknown".to_owned(), |c| c.to_string());
        details(
            ui,
            "battery_details",
            &[
                ("State", b.state.clone()),
                ("Health", format!("{:.0} %", b.health)),
                ("Cycles", cycles),
            ],
        );
    });
}
