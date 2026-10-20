use egui::Ui;

use crate::sampler::NetStat;
use crate::theme::color;
use crate::widgets::{Item, boxed_list, fmt_bytes, fmt_rate_highest, group};

/// One interface, or the merged VPN entry when `members` lists its tunnels.
pub fn show(ui: &mut Ui, n: &NetStat, members: &[&NetStat]) {
    group(ui, "Usage", |ui| {
        boxed_list(
            ui,
            &[
                Item::Graph {
                    caption: "Receiving",
                    value: fmt_rate_highest(n.rx_bps, &n.rx_hist),
                    series: &[(&n.rx_hist, color::RX)],
                    y_max: None,
                },
                Item::Graph {
                    caption: "Sending",
                    value: fmt_rate_highest(n.tx_bps, &n.tx_hist),
                    series: &[(&n.tx_hist, color::TX)],
                    y_max: None,
                },
                Item::Row {
                    title: "Total Received",
                    value: fmt_bytes(n.rx_total),
                },
                Item::Row {
                    title: "Total Sent",
                    value: fmt_bytes(n.tx_total),
                },
            ],
        );
    });
    group(ui, "Properties", |ui| {
        let rows: Vec<Item> = if members.is_empty() {
            let port = crate::devices::port(n).unwrap_or("Unknown").to_owned();
            vec![
                Item::Row {
                    title: "Device",
                    value: n.name.clone(),
                },
                Item::Row {
                    title: "Hardware Port",
                    value: port,
                },
            ]
        } else {
            members
                .iter()
                .map(|m| Item::Row {
                    title: &m.name,
                    value: format!(
                        "Received {} · Sent {}",
                        fmt_bytes(m.rx_total),
                        fmt_bytes(m.tx_total)
                    ),
                })
                .collect()
        };
        boxed_list(ui, &rows);
    });
}
