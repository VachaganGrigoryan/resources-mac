//! Shared drawing helpers in the GNOME Resources style: graphs, sidebar items, cards,
//! property rows and number formatting.

use egui::{
    Align, Align2, Color32, CornerRadius, Frame, Layout, Mesh, Painter, Pos2, Rect, Response,
    RichText, Sense, Shape, Stroke, TextStyle, TextWrapMode, Ui, UiBuilder, WidgetInfo, WidgetText,
    pos2, vec2,
};

use crate::HISTORY_LEN;
use crate::sample::History;
use crate::theme::palette;

/// One graph line and the colour it is drawn in.
pub type Series<'a> = (&'a History, Color32);

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

/// Current rate plus the highest one still on the graph: `"8.37 MB/s · Highest: 247.76 MB/s"`.
pub fn fmt_rate_highest(bps: f64, hist: &History) -> String {
    format!("{} · Highest: {}", fmt_rate(bps), fmt_rate(hist.max()))
}

/// `"5.05 GB / 14.47 GB · 35 %"`.
pub fn fmt_used(used: u64, total: u64) -> String {
    let pct = if total == 0 {
        0.0
    } else {
        used as f64 / total as f64 * 100.0
    };
    format!("{} / {} · {pct:.0} %", fmt_bytes(used), fmt_bytes(total))
}

/// Paints a live graph over the last [`HISTORY_LEN`] seconds, newest on the right, the way
/// upstream's `ResGraph` does: tinted background, 40% area fill, full-colour outline.
/// `y_max: None` scales to the largest visible value.
pub fn paint_graph(
    painter: &Painter,
    rect: Rect,
    series: &[Series],
    y_max: Option<f64>,
    radius: f32,
) {
    let Some(&(_, main)) = series.first() else {
        return;
    };
    painter.rect_filled(rect, radius, main.gamma_multiply(0.1));
    let max = y_max
        .unwrap_or_else(|| {
            series
                .iter()
                .flat_map(|(h, _)| h.values())
                .fold(0.0, f64::max)
        })
        .max(f64::EPSILON);
    let step = rect.width() / (HISTORY_LEN - 1) as f32;
    let painter = painter.with_clip_rect(rect);
    for &(h, color) in series {
        let offset = HISTORY_LEN.saturating_sub(h.len());
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
        let fill = color.gamma_multiply(0.4);
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
        painter.add(Shape::line(pts, Stroke::new(1.0, color)));
    }
}

/// A graph that takes the full available width.
pub fn graph(ui: &mut Ui, series: &[Series], y_max: Option<f64>, height: f32, radius: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    paint_graph(ui.painter(), rect, series, y_max, radius);
}

/// Sidebar entry: icon and label, optionally with an 80 px mini graph underneath.
/// Upstream `stack_sidebar_item.ui`: 16 px icon, 12 px gap, label ellipsized.
pub fn sidebar_item(
    ui: &mut Ui,
    icon: &str,
    label: &str,
    selected: bool,
    graph: Option<(&[Series], Option<f64>)>,
) -> Response {
    const PAD: f32 = 10.0;
    const GRAPH_H: f32 = 80.0;
    let width = ui.available_width();
    let row_h = ui.text_style_height(&TextStyle::Body);
    let height = PAD * 2.0 + row_h + graph.map_or(0.0, |_| 6.0 + GRAPH_H);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(egui::Role::Button, true, selected, label));

    let p = palette(ui.visuals());
    if selected || resp.hovered() {
        let fill = if selected {
            p.selected
        } else {
            p.selected.gamma_multiply(0.5)
        };
        ui.painter().rect_filled(rect, 10.0, fill);
    }
    let text = ui.visuals().text_color();
    let inner = rect.shrink2(vec2(12.0, PAD));
    let icon_rect = Rect::from_min_size(inner.min, vec2(16.0, row_h));
    ui.painter().text(
        icon_rect.center(),
        Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(16.0),
        text,
    );
    let galley = WidgetText::from(label).into_galley(
        ui,
        Some(TextWrapMode::Truncate),
        inner.width() - 28.0,
        TextStyle::Body,
    );
    ui.painter()
        .galley(pos2(inner.left() + 28.0, inner.top()), galley, text);
    if let Some((series, y_max)) = graph {
        let g = Rect::from_min_size(
            pos2(inner.left(), inner.top() + row_h + 6.0),
            vec2(inner.width(), GRAPH_H),
        );
        paint_graph(ui.painter(), g, series, y_max, 4.0);
    }
    resp
}

/// Window title area: bold centred title with an optional dimmed subtitle.
pub fn header_bar(ui: &mut Ui, title: &str, subtitle: Option<&str>) {
    ui.vertical_centered(|ui| {
        ui.add_space(if subtitle.is_some() { 4.0 } else { 12.0 });
        ui.label(RichText::new(title).heading().strong());
        if let Some(sub) = subtitle {
            ui.label(RichText::new(sub).small().weak());
        }
    });
}

/// Centred content column at most 768 px wide (upstream `AdwClamp maximum-size`).
pub fn page_column<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let full = ui.available_rect_before_wrap();
    let width = (full.width() - 48.0).clamp(0.0, 768.0);
    let rect = Rect::from_min_size(
        pos2(full.center().x - width / 2.0, full.top() + 8.0),
        vec2(width, full.height()),
    );
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
        add,
    )
    .inner
}

/// Bold group title followed by its content (upstream `AdwPreferencesGroup`).
pub fn group<R>(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.add_space(12.0);
    ui.label(RichText::new(title).heading().strong());
    add(ui)
}

/// White rounded card with a hairline border and a soft shadow.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    card_frame(ui, 12)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn card_frame(ui: &Ui, margin: i8) -> Frame {
    let p = palette(ui.visuals());
    Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(12))
        .shadow(egui::Shadow {
            offset: [0, 1],
            blur: 4,
            spread: 0,
            color: Color32::from_black_alpha(if ui.visuals().dark_mode { 40 } else { 14 }),
        })
        .inner_margin(margin)
}

/// One section of a [`boxed_list`].
pub enum Item<'a> {
    /// 120 px graph with a small dimmed caption and the current value under it.
    Graph {
        caption: &'a str,
        value: String,
        series: &'a [Series<'a>],
        y_max: Option<f64>,
    },
    /// Small dimmed title with the value under it (upstream `AdwActionRow` with subtitle).
    Row { title: &'a str, value: String },
}

/// One card holding graphs and rows separated by hairlines, as upstream's Usage and
/// Properties groups.
pub fn boxed_list(ui: &mut Ui, items: &[Item]) {
    let border = palette(ui.visuals()).border;
    card_frame(ui, 0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 0.0;
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                let y = ui.cursor().top();
                ui.painter()
                    .hline(ui.max_rect().x_range(), y, Stroke::new(1.0, border));
            }
            Frame::new()
                .inner_margin(egui::Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 2.0;
                    match item {
                        Item::Graph {
                            caption,
                            value,
                            series,
                            y_max,
                        } => {
                            graph(ui, series, *y_max, 120.0, 8.0);
                            ui.add_space(8.0);
                            ui.label(RichText::new(*caption).small().weak());
                            ui.label(value);
                        }
                        Item::Row { title, value } => {
                            ui.label(RichText::new(*title).small().weak());
                            ui.add(egui::Label::new(value.as_str()).truncate());
                        }
                    }
                });
        }
    });
}

/// Card with a graph on top and a caption plus value underneath (upstream `ResGraphBox`).
pub fn graph_box(
    ui: &mut Ui,
    caption: &str,
    value: &str,
    series: &[Series],
    y_max: Option<f64>,
    height: f32,
) {
    card(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        graph(ui, series, y_max, height, 8.0);
        ui.add_space(4.0);
        ui.label(RichText::new(caption).small().weak());
        ui.label(value);
    });
}

/// Frameless icon-only button; screen readers and tests see `label`, hovering shows it.
pub fn icon_button(ui: &mut Ui, icon: &str, label: &str) -> Response {
    let resp = ui
        .add(
            egui::Button::new(RichText::new(icon).size(16.0))
                .frame_when_inactive(false)
                .min_size(vec2(32.0, 32.0))
                .corner_radius(16),
        )
        .on_hover_text(label);
    resp.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, label));
    resp
}

/// Full-width popup menu row: icon then label; screen readers and tests see `label`.
pub fn menu_item(ui: &mut Ui, icon: &str, label: &str) -> Response {
    let resp = ui.add(
        egui::Button::new((RichText::new(icon).size(16.0), label))
            .frame_when_inactive(false)
            .min_size(vec2(ui.available_width(), 28.0))
            .corner_radius(6),
    );
    resp.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, label));
    resp
}

/// Red-tinted button for destructive actions such as "End App".
pub fn destructive_button(ui: &mut Ui, label: &str, enabled: bool) -> Response {
    let red = crate::theme::DESTRUCTIVE;
    ui.add_enabled(
        enabled,
        egui::Button::new(
            RichText::new(label)
                .family(crate::theme::semibold())
                .color(red),
        )
        .fill(red.gamma_multiply(0.15))
        .corner_radius(8),
    )
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
