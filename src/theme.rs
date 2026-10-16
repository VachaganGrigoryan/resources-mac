//! Mac-like look: system light/dark, SF Pro when available, roomy spacing, rounded corners.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};

use crate::widgets::ACCENT;

const SF_PRO: &str = "/System/Library/Fonts/SFNS.ttf";

pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::System);

    // Loaded from the system at runtime (not bundled); egui's default font is the fallback.
    if let Ok(bytes) = std::fs::read(SF_PRO) {
        let mut fonts = FontDefinitions::default();
        fonts
            .font_data
            .insert("sf".into(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "sf".into());
        ctx.set_fonts(fonts);
    }

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.visuals.selection.bg_fill = ACCENT;
        style.visuals.hyperlink_color = ACCENT;
        for w in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.noninteractive,
        ] {
            w.corner_radius = 6.into();
        }
    });
}
