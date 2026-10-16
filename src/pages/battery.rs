use egui::Ui;

use crate::collect::battery::BatteryStat;
use crate::theme::color;
use crate::widgets::{Item, boxed_list, group};

pub fn show(ui: &mut Ui, b: &BatteryStat) {
    group(ui, "Usage", |ui| {
        boxed_list(
            ui,
            &[
                Item::Graph {
                    caption: "Battery Charge",
                    value: format!("{:.0} % ({})", b.percent, b.state),
                    series: &[(&b.charge_hist, color::BATTERY)],
                    y_max: Some(100.0),
                },
                Item::Graph {
                    caption: "Power Usage",
                    value: format!("{:.1} W · Highest: {:.1} W", b.power_w, b.power_hist.max()),
                    series: &[(&b.power_hist, color::POWER)],
                    y_max: None,
                },
            ],
        );
    });
    group(ui, "Properties", |ui| {
        let unknown = || "Unknown".to_owned();
        boxed_list(
            ui,
            &[
                Item::Row {
                    title: "Battery Health",
                    value: format!("{:.0} %", b.health),
                },
                Item::Row {
                    title: "Design Capacity",
                    value: b.design_wh.map_or_else(unknown, |wh| format!("{wh:.1} Wh")),
                },
                Item::Row {
                    title: "Charge Cycles",
                    value: b.cycles.map_or_else(unknown, |c| c.to_string()),
                },
            ],
        );
    });
}
