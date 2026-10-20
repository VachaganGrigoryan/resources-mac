//! Squarified treemap of a folder's children: area by size, colour by kind.

use egui::{Align2, Color32, FontId, Rect, Sense, Ui, WidgetInfo, pos2, vec2};
use resources_mac::widgets::fmt_bytes;

use crate::scan::Node;
use crate::widgets::kind_color;

/// Tiles drawn individually; smaller children share one "more" tile.
const MAX_TILES: usize = 40;

/// Worst aspect ratio of a row of `areas` laid along a side of length `side`.
fn worst(areas: &[f64], side: f64) -> f64 {
    let sum: f64 = areas.iter().sum();
    let max = areas.iter().copied().fold(f64::MIN, f64::max);
    let min = areas.iter().copied().fold(f64::MAX, f64::min);
    let (s2, w2) = (sum * sum, side * side);
    f64::max(w2 * max / s2, s2 / (w2 * min))
}

/// Squarified treemap (Bruls, Huizing, van Wijk): one rect per weight, in input order, areas
/// proportional to the weights. Zero or negative weights get an empty rect.
pub fn squarify(weights: &[f64], rect: Rect) -> Vec<Rect> {
    let mut out = vec![Rect::from_min_size(rect.min, egui::Vec2::ZERO); weights.len()];
    let total: f64 = weights.iter().filter(|w| **w > 0.0).sum();
    if total <= 0.0 || rect.area() <= 0.0 {
        return out;
    }
    let mut order: Vec<usize> = (0..weights.len()).filter(|&i| weights[i] > 0.0).collect();
    order.sort_by(|a, b| weights[*b].total_cmp(&weights[*a]));
    let scale = f64::from(rect.area()) / total;
    let areas: Vec<f64> = order.iter().map(|&i| weights[i] * scale).collect();

    let mut free = rect;
    let mut start = 0;
    while start < order.len() {
        let side = f64::from(free.width().min(free.height()));
        if side <= 0.0 {
            break;
        }
        let mut end = start + 1;
        let mut best = worst(&areas[start..end], side);
        while end < order.len() {
            let next = worst(&areas[start..=end], side);
            if next > best {
                break;
            }
            best = next;
            end += 1;
        }
        let row: f64 = areas[start..end].iter().sum();
        if free.width() >= free.height() {
            // A column along the left edge.
            let thick = row / f64::from(free.height());
            let mut y = free.top();
            for k in start..end {
                let h = (areas[k] / thick) as f32;
                out[order[k]] = Rect::from_min_size(pos2(free.left(), y), vec2(thick as f32, h));
                y += h;
            }
            free.min.x += thick as f32;
        } else {
            // A row along the top edge.
            let thick = row / f64::from(free.width());
            let mut x = free.left();
            for k in start..end {
                let w = (areas[k] / thick) as f32;
                out[order[k]] = Rect::from_min_size(pos2(x, free.top()), vec2(w, thick as f32));
                x += w;
            }
            free.min.y += thick as f32;
        }
        start = end;
    }
    out
}

/// Draws `node`'s children as tiles filling the width, `height` tall. Returns the index of the
/// clicked child when it is a folder with something inside to zoom into.
pub fn show(ui: &mut Ui, node: &Node, height: f32) -> Option<usize> {
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let shown = node.children.len().min(MAX_TILES);
    let rest: u64 = node.children[shown..].iter().map(|c| c.bytes).sum();
    let mut weights: Vec<f64> = node.children[..shown]
        .iter()
        .map(|c| c.bytes as f64)
        .collect();
    if rest > 0 {
        weights.push(rest as f64);
    }
    let rects = squarify(&weights, area);
    let mut clicked = None;
    for (i, rect) in rects.iter().enumerate() {
        let tile = rect.shrink(1.0);
        if tile.width() < 1.0 || tile.height() < 1.0 {
            continue;
        }
        let (name, fill, files, zoomable) = match node.children.get(i).filter(|_| i < shown) {
            Some(c) => (
                c.name.clone(),
                kind_color(c.kind),
                c.files,
                !c.children.is_empty(),
            ),
            None => (
                format!("{} more", node.children.len() - shown),
                Color32::GRAY,
                node.children[shown..].iter().map(|c| c.files).sum(),
                false,
            ),
        };
        let bytes = weights[i] as u64;
        let resp = ui.interact(tile, ui.scope_id().with(("tile", i)), Sense::click());
        resp.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, &name));
        let fill = if resp.hovered() {
            fill
        } else {
            fill.gamma_multiply(0.8)
        };
        ui.painter().rect_filled(tile, 4.0, fill);
        if tile.width() > 60.0 && tile.height() > 18.0 {
            let text = if tile.height() > 34.0 {
                format!("{name}\n{}", fmt_bytes(bytes))
            } else {
                name.clone()
            };
            let painter = ui.painter().with_clip_rect(tile);
            painter.text(
                tile.left_top() + vec2(6.0, 4.0),
                Align2::LEFT_TOP,
                text,
                FontId::proportional(11.0),
                Color32::WHITE,
            );
        }
        let resp = resp.on_hover_text(format!("{name} · {} · {files} files", fmt_bytes(bytes)));
        if zoomable && resp.clicked() {
            clicked = Some(i);
        }
    }
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squarify_is_proportional_and_disjoint() {
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(200.0, 100.0));
        let weights = [50.0, 25.0, 15.0, 10.0];
        let rects = squarify(&weights, area);
        assert_eq!(rects.len(), 4);
        for (r, w) in rects.iter().zip(weights) {
            let share = r.area() / area.area();
            assert!(
                (share - w as f32 / 100.0).abs() < 0.005,
                "{r:?} share {share}"
            );
            assert!(area.expand(0.01).contains_rect(*r), "{r:?} outside");
        }
        for (i, a) in rects.iter().enumerate() {
            for b in &rects[i + 1..] {
                let overlap = a.intersect(*b);
                assert!(
                    overlap.width() <= 0.01 || overlap.height() <= 0.01,
                    "{a:?} overlaps {b:?}"
                );
            }
        }
    }

    #[test]
    fn squarify_keeps_input_order() {
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0));
        let rects = squarify(&[10.0, 70.0, 20.0], area);
        assert!(rects[1].area() > rects[2].area() && rects[2].area() > rects[0].area());
    }

    #[test]
    fn squarify_handles_zero_and_empty() {
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 50.0));
        assert!(squarify(&[], area).is_empty());
        let rects = squarify(&[0.0, 5.0], area);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].area(), 0.0);
        assert!((rects[1].area() - area.area()).abs() < 0.5);
        let all_zero = squarify(&[0.0, 0.0], area);
        assert!(all_zero.iter().all(|r| r.area() == 0.0));
    }
}
