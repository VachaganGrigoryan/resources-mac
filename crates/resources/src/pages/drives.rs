use egui::Ui;

use crate::icons;
use crate::sampler::DiskStat;
use crate::theme::color;
use crate::widgets::{Item, boxed_list, card, fmt_bytes, fmt_rate_highest, group};

/// `free_up`: `None` when Disk Sweep isn't found (no button), else the last launch error.
/// Returns true when "Free Up Space…" was clicked.
pub fn show(ui: &mut Ui, d: &DiskStat, free_up: Option<Option<&str>>) -> bool {
    group(ui, "Usage", |ui| {
        boxed_list(
            ui,
            &[
                Item::Graph {
                    caption: "Read Speed",
                    value: fmt_rate_highest(d.read_bps, &d.read_hist),
                    series: &[(&d.read_hist, color::READ)],
                    y_max: None,
                },
                Item::Graph {
                    caption: "Write Speed",
                    value: fmt_rate_highest(d.write_bps, &d.write_hist),
                    series: &[(&d.write_hist, color::WRITE)],
                    y_max: None,
                },
                Item::Row {
                    title: "Total Read",
                    value: fmt_bytes(d.read_total),
                },
                Item::Row {
                    title: "Total Written",
                    value: fmt_bytes(d.write_total),
                },
            ],
        );
    });
    group(ui, "Properties", |ui| {
        let used = d.total.saturating_sub(d.avail);
        let row = |title, value| Item::Row { title, value };
        boxed_list(
            ui,
            &[
                row("Name", d.name.clone()),
                row("Mount Point", d.mount.clone()),
                row("Capacity", fmt_bytes(d.total)),
                row("Used", fmt_bytes(used)),
                row("Available", fmt_bytes(d.avail)),
                row("Removable", if d.removable { "Yes" } else { "No" }.into()),
            ],
        );
    });
    let Some(error) = free_up else {
        return false;
    };
    group(ui, "Free Up Space", |ui| {
        card(ui, |ui| {
            ui.label("Find caches, build output and Docker data to clean up in Disk Sweep.");
            if let Some(e) = error {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            ui.button(format!("{}  Free Up Space…", icons::BROOM))
                .clicked()
        })
    })
}
