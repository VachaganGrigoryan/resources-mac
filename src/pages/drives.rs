use egui::Ui;

use crate::sampler::DiskStat;
use crate::theme::color;
use crate::widgets::{fmt_bytes, fmt_rate, graph_box, group, property_rows};

pub fn show(ui: &mut Ui, d: &DiskStat) {
    group(ui, "Usage", |ui| {
        graph_box(
            ui,
            "Read Speed",
            &fmt_rate(d.read_bps),
            &[(&d.read_hist, color::READ)],
            None,
            120.0,
        );
        graph_box(
            ui,
            "Write Speed",
            &fmt_rate(d.write_bps),
            &[(&d.write_hist, color::WRITE)],
            None,
            120.0,
        );
        property_rows(
            ui,
            &[
                ("Total Read", fmt_bytes(d.read_total)),
                ("Total Written", fmt_bytes(d.write_total)),
            ],
        );
    });
    group(ui, "Properties", |ui| {
        let used = d.total.saturating_sub(d.avail);
        property_rows(
            ui,
            &[
                ("Name", d.name.clone()),
                ("Mount Point", d.mount.clone()),
                ("Capacity", fmt_bytes(d.total)),
                ("Used", fmt_bytes(used)),
                ("Available", fmt_bytes(d.avail)),
                (
                    "Removable",
                    if d.removable { "Yes" } else { "No" }.to_owned(),
                ),
            ],
        );
    });
}
