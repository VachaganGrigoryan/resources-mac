//! Disk Sweep drawing helpers on top of `resources_ui::widgets`.

use egui::Color32;
use resources_ui::theme::color;
use resources_ui::widgets::{card, fmt_bytes};

use crate::scan::Kind;

pub fn kind_color(kind: Kind) -> Color32 {
    match kind {
        Kind::Source => color::CPU,
        Kind::Cache => color::DRIVE,
        Kind::Editor => color::MEMORY,
        Kind::Ignored => Color32::from_rgb(0x8E, 0x8E, 0x93),
        Kind::Git => color::POWER,
    }
}

/// One bar split by kind, widths proportional to bytes; hovering lists the parts.
pub fn stacked_bar(ui: &mut egui::Ui, by_kind: &[u64; 5], height: f32) {
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let total: u64 = by_kind.iter().sum();
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, ui.visuals().faint_bg_color);
    if total == 0 {
        return;
    }
    let mut x = rect.left();
    for kind in Kind::ALL {
        let bytes = by_kind[kind.index()];
        if bytes == 0 {
            continue;
        }
        let w = rect.width() * (bytes as f64 / total as f64) as f32;
        let seg = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(w, height));
        painter.rect_filled(seg, 2.0, kind_color(kind));
        x += w;
    }
    resp.on_hover_text(
        Kind::ALL
            .iter()
            .filter(|k| by_kind[k.index()] > 0)
            .map(|k| format!("{}: {}", k.label(), fmt_bytes(by_kind[k.index()])))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// Colour dot, kind and size for every kind present.
pub fn legend(ui: &mut egui::Ui, by_kind: &[u64; 5]) {
    ui.horizontal_wrapped(|ui| {
        for kind in Kind::ALL {
            let bytes = by_kind[kind.index()];
            if bytes == 0 {
                continue;
            }
            let (dot, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
            ui.painter()
                .circle_filled(dot.center(), 5.0, kind_color(kind));
            ui.label(format!("{}  {}", kind.label(), fmt_bytes(bytes)));
            ui.add_space(8.0);
        }
    });
}

/// Card with a small title, a big value and a dimmed hint.
pub fn stat_tile(ui: &mut egui::Ui, title: &str, value: &str, hint: &str) {
    card(ui, |ui| {
        ui.label(egui::RichText::new(title).small().weak());
        ui.label(
            egui::RichText::new(value)
                .size(22.0)
                .family(resources_ui::theme::semibold()),
        );
        ui.label(egui::RichText::new(hint).small().weak());
    });
}
