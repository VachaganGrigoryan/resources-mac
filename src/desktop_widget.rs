//! Desktop widget: a small borderless window below all other windows with live cards for the
//! stats chosen in Settings. Drag it anywhere; its position is saved.

use egui::{
    Align2, CornerRadius, FontId, Rect, Sense, Stroke, Ui, ViewportBuilder, ViewportCommand,
    ViewportId, WindowLevel, vec2,
};

use crate::pages::settings::Settings;
use crate::sampler::State;
use crate::stats;
use crate::theme::palette;
use crate::widgets::paint_graph;

pub const TITLE: &str = "Resources Widget";
const WIDTH: f32 = 240.0;
const CARD_H: f32 = 64.0;
const PAD: f32 = 10.0;
/// Where the widget first appears, and returns to on "Reset Position".
const HOME: [f32; 2] = [40.0, 60.0];

/// Shows the widget this frame; call it every frame while it is enabled.
pub fn show(ctx: &egui::Context, settings: &mut Settings, s: &State) {
    let cards: Vec<_> = settings
        .widget_stats
        .iter()
        .filter_map(|&st| Some((st, stats::card(st, s)?)))
        .collect();
    let height = PAD + cards.len().max(1) as f32 * (CARD_H + PAD);
    let builder = ViewportBuilder::default()
        .with_title(TITLE)
        .with_decorations(false)
        .with_transparent(true)
        .with_has_shadow(false)
        .with_resizable(false)
        .with_taskbar(false)
        .with_window_level(WindowLevel::AlwaysOnBottom)
        .with_inner_size([WIDTH, height])
        .with_position(settings.widget_pos.unwrap_or(HOME));
    ctx.show_viewport_immediate(
        ViewportId::from_hash_of(TITLE),
        builder,
        |ui: &mut Ui, _| {
            let rect = ui.max_rect();
            let p = palette(ui.visuals());
            let text = ui.visuals().text_color();
            let painter = ui.painter();
            painter.rect(
                rect,
                CornerRadius::same(16),
                p.sidebar.gamma_multiply(0.92),
                Stroke::new(1.0, p.border),
                egui::StrokeKind::Inside,
            );
            if cards.is_empty() {
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "Choose stats in Settings",
                    FontId::proportional(12.0),
                    text.gamma_multiply(0.6),
                );
            }
            for (i, (stat, card)) in cards.iter().enumerate() {
                let top = rect.top() + PAD + i as f32 * (CARD_H + PAD);
                let r = Rect::from_min_size(
                    egui::pos2(rect.left() + PAD, top),
                    vec2(rect.width() - 2.0 * PAD, CARD_H),
                );
                let graph = Rect::from_min_max(r.left_top() + vec2(0.0, 30.0), r.right_bottom());
                paint_graph(painter, graph, &card.series, card.y_max, 6.0);
                painter.text(
                    r.left_top(),
                    Align2::LEFT_TOP,
                    format!("{}  {}", stat.icon(), stat.label()),
                    FontId::proportional(11.0),
                    text.gamma_multiply(0.6),
                );
                painter.text(
                    r.left_top() + vec2(0.0, 14.0),
                    Align2::LEFT_TOP,
                    &card.value,
                    FontId::proportional(13.0),
                    text,
                );
            }
            // Only adopt the window's position after the user dragged it, so a pending move
            // (first show, "Reset Position") isn't overwritten by the old position.
            let id = ui.scope_id().with("dragged");
            let mut dragged = ui.ctx().data(|d| d.get_temp(id).unwrap_or(false));
            if settings.widget_pos.is_none() {
                settings.widget_pos = Some(HOME);
                dragged = false;
            }
            let resp = ui.interact(rect, ui.scope_id().with("drag"), Sense::drag());
            if resp.drag_started() {
                ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
                dragged = true;
            }
            if let Some(outer) = ui.input(|i| i.viewport().outer_rect).filter(|_| dragged) {
                settings.widget_pos = Some([outer.min.x, outer.min.y]);
            }
            ui.ctx().data_mut(|d| d.insert_temp(id, dragged));
        },
    );
}
