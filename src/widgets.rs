//! Shared drawing helpers: cards, live graphs and number formatting.

use egui::{Color32, Mesh, Pos2, Sense, Shape, Stroke, Ui, pos2, vec2};

use crate::HISTORY_LEN;
use crate::sample::History;

pub const ACCENT: Color32 = Color32::from_rgb(0x00, 0x7A, 0xFF);
pub const SECOND: Color32 = Color32::from_rgb(0xFF, 0x95, 0x00);

/// Human-readable bytes in decimal units with two decimals: `1536 → "1.54 kB"`.
pub fn fmt_bytes(n: u64) -> String {
    const UNITS: [&str; 6] = ["B", "kB", "MB", "GB", "TB", "PB"];
    let mut v = n as f64;
    let mut unit = 0;
    while v >= 1000.0 && unit < UNITS.len() - 1 {
        v /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} B")
    } else {
        format!("{v:.2} {}", UNITS[unit])
    }
}

pub fn fmt_rate(bps: f64) -> String {
    format!("{}/s", fmt_bytes(bps as u64))
}

/// Rounded card used for every block of content.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::group(ui.style())
        .corner_radius(10.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// Live line graph over the last [`HISTORY_LEN`] seconds, newest on the right.
/// `y_max: None` scales to the largest visible value.
pub fn graph(ui: &mut Ui, series: &[&History], y_max: Option<f64>, height: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, ui.visuals().extreme_bg_color);
    for i in 1..4 {
        let y = rect.top() + rect.height() * i as f32 / 4.0;
        painter.hline(
            rect.x_range(),
            y,
            Stroke::new(1.0, ui.visuals().faint_bg_color),
        );
    }
    let max = y_max
        .unwrap_or_else(|| series.iter().flat_map(|h| h.values()).fold(0.0, f64::max))
        .max(f64::EPSILON);
    let step = rect.width() / (HISTORY_LEN - 1) as f32;
    for (i, h) in series.iter().enumerate() {
        let color = if i == 0 { ACCENT } else { SECOND };
        let offset = HISTORY_LEN - h.len();
        let pts: Vec<Pos2> = h
            .values()
            .enumerate()
            .map(|(x, y)| {
                let frac = (y / max).clamp(0.0, 1.0) as f32;
                pos2(
                    rect.left() + (offset + x) as f32 * step,
                    rect.bottom() - frac * rect.height(),
                )
            })
            .collect();
        let fill = color.gamma_multiply(0.15);
        let mut mesh = Mesh::default();
        for w in pts.windows(2) {
            let base = mesh.vertices.len() as u32;
            for p in [
                w[0],
                w[1],
                pos2(w[1].x, rect.bottom()),
                pos2(w[0].x, rect.bottom()),
            ] {
                mesh.colored_vertex(p, fill);
            }
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base, base + 2, base + 3);
        }
        painter.add(Shape::mesh(mesh));
        painter.add(Shape::line(pts, Stroke::new(1.5, color)));
    }
}

/// Card with a title, the current value on the right and a graph below.
pub fn graph_card(
    ui: &mut Ui,
    title: &str,
    value_label: &str,
    series: &[&History],
    y_max: Option<f64>,
) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.strong(title);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(value_label)
            });
        });
        graph(ui, series, y_max, 140.0);
    });
}

/// Two-column label/value grid.
pub fn details(ui: &mut Ui, id: &str, rows: &[(&str, String)]) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([24.0, 6.0])
        .show(ui, |ui| {
            for (k, v) in rows {
                ui.weak(*k);
                ui.label(v);
                ui.end_row();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_bytes_units() {
        // Decimal units with two decimals, like GNOME Resources and Finder.
        assert_eq!(fmt_bytes(512), "512 B");
        assert_eq!(fmt_bytes(1536), "1.54 kB");
        assert_eq!(fmt_bytes(962_560), "962.56 kB");
        assert_eq!(fmt_bytes(3_000_000_000), "3.00 GB");
    }
}
