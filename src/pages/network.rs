use egui::Ui;

use crate::sampler::{NetStat, State};
use crate::widgets::{card, fmt_bytes, fmt_rate, graph};

/// Loopback and interfaces that never carried traffic (idle tunnels, bridges) are noise.
fn visible(n: &NetStat) -> bool {
    n.name != "lo0" && n.rx_total + n.tx_total > 0
}

pub fn show(ui: &mut Ui, s: &State) {
    for n in s.nets.iter().filter(|n| visible(n)) {
        card(ui, |ui| {
            ui.strong(&n.name);
            ui.label(format!(
                "In {}   ·   Out {}   ·   Total in {}, out {}",
                fmt_rate(n.rx_bps),
                fmt_rate(n.tx_bps),
                fmt_bytes(n.rx_total),
                fmt_bytes(n.tx_total)
            ));
            graph(ui, &[&n.rx_hist, &n.tx_hist], None, 80.0);
        });
    }
}
