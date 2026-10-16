use egui::Ui;

use crate::sampler::NetStat;
use crate::theme::color;
use crate::widgets::{fmt_bytes, fmt_rate, graph_box, group, property_rows};

/// Loopback, Apple's peer-to-peer Wi-Fi links and interfaces that never carried traffic
/// (idle tunnels, bridges) are noise.
pub fn visible(n: &NetStat) -> bool {
    n.name != "lo0"
        && !n.name.starts_with("awdl")
        && !n.name.starts_with("llw")
        && n.rx_total + n.tx_total > 0
}

pub fn show(ui: &mut Ui, n: &NetStat) {
    group(ui, "Usage", |ui| {
        graph_box(
            ui,
            "Receiving",
            &fmt_rate(n.rx_bps),
            &[(&n.rx_hist, color::RX)],
            None,
            120.0,
        );
        graph_box(
            ui,
            "Sending",
            &fmt_rate(n.tx_bps),
            &[(&n.tx_hist, color::TX)],
            None,
            120.0,
        );
        property_rows(
            ui,
            &[
                ("Total Received", fmt_bytes(n.rx_total)),
                ("Total Sent", fmt_bytes(n.tx_total)),
            ],
        );
    });
    group(ui, "Properties", |ui| {
        let port = if n.port.is_empty() {
            "Unknown".to_owned()
        } else {
            n.port.clone()
        };
        property_rows(ui, &[("Device", n.name.clone()), ("Hardware Port", port)]);
    });
}
