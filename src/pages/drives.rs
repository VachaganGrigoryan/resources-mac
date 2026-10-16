use egui::Ui;

use crate::sampler::State;
use crate::widgets::{card, fmt_bytes, fmt_rate, graph};

pub fn show(ui: &mut Ui, s: &State) {
    for d in &s.disks {
        card(ui, |ui| {
            let used = d.total.saturating_sub(d.avail);
            ui.horizontal(|ui| {
                ui.strong(&d.name);
                ui.weak(&d.mount);
            });
            let frac = if d.total == 0 {
                0.0
            } else {
                used as f32 / d.total as f32
            };
            ui.add(egui::ProgressBar::new(frac).text(format!(
                "{} used of {}",
                fmt_bytes(used),
                fmt_bytes(d.total)
            )));
            ui.label(format!(
                "Read {}   ·   Write {}",
                fmt_rate(d.read_bps),
                fmt_rate(d.write_bps)
            ));
            graph(ui, &[&d.read_hist, &d.write_hist], None, 80.0);
        });
    }
}
