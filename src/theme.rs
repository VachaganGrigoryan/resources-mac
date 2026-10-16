//! GNOME Resources look on macOS: system light/dark, SF Pro text, Phosphor icons,
//! Adwaita-like surfaces and the upstream per-resource graph colours.

use std::sync::Arc;

use egui::epaint::text::{FontTweak, VariationCoords};
use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, TextStyle, Theme, Visuals};

use crate::icons;

const SF_PRO: &str = "/System/Library/Fonts/SFNS.ttf";

pub const ACCENT: Color32 = Color32::from_rgb(0x35, 0x84, 0xE4);
/// Font family for titles: SF Pro at weight 600 (regular-weight fallback without SF Pro).
pub const SEMIBOLD: &str = "semibold";

pub fn semibold() -> FontFamily {
    FontFamily::Name(SEMIBOLD.into())
}

pub const DESTRUCTIVE: Color32 = Color32::from_rgb(0xE0, 0x1B, 0x24);

/// Graph colours, copied from upstream `src/ui/pages/*.rs` (`MAIN_GRAPH_COLOR` and friends).
pub mod color {
    use egui::Color32;

    pub const CPU: Color32 = Color32::from_rgb(0x35, 0x84, 0xE4);
    pub const CPU_CORE: Color32 = Color32::from_rgb(0x1C, 0x71, 0xD8);
    pub const MEMORY: Color32 = Color32::from_rgb(0xC5, 0x2F, 0x90);
    pub const SWAP: Color32 = Color32::from_rgb(0x94, 0x29, 0x7C);
    pub const GPU: Color32 = Color32::from_rgb(0xED, 0x33, 0x3B);
    pub const DRIVE: Color32 = Color32::from_rgb(0xFF, 0x78, 0x00);
    pub const READ: Color32 = Color32::from_rgb(0xE6, 0x61, 0x00);
    pub const WRITE: Color32 = Color32::from_rgb(0xC6, 0x46, 0x00);
    pub const NETWORK: Color32 = Color32::from_rgb(0x25, 0x9A, 0xAB);
    pub const RX: Color32 = Color32::from_rgb(0x34, 0xAB, 0xAF);
    pub const TX: Color32 = Color32::from_rgb(0x20, 0x81, 0x8F);
    pub const BATTERY: Color32 = Color32::from_rgb(0x33, 0xD1, 0x7A);
    pub const POWER: Color32 = Color32::from_rgb(0x26, 0xA2, 0x69);
}

/// Surface colours for the current light/dark mode.
pub struct Palette {
    pub sidebar: Color32,
    pub content: Color32,
    pub card: Color32,
    pub border: Color32,
    pub selected: Color32,
}

pub fn palette(v: &Visuals) -> Palette {
    if v.dark_mode {
        Palette {
            sidebar: Color32::from_rgb(0x2E, 0x2E, 0x32),
            content: Color32::from_rgb(0x22, 0x22, 0x26),
            card: Color32::from_rgb(0x36, 0x36, 0x3A),
            border: Color32::from_white_alpha(18),
            selected: Color32::from_white_alpha(20),
        }
    } else {
        Palette {
            sidebar: Color32::from_rgb(0xEB, 0xEB, 0xED),
            content: Color32::from_rgb(0xFA, 0xFA, 0xFA),
            card: Color32::WHITE,
            border: Color32::from_black_alpha(18),
            selected: Color32::from_black_alpha(20),
        }
    }
}

pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::System);

    let mut fonts = FontDefinitions::default();
    // SF Pro is read from the system at runtime (not bundled); egui's default font is the fallback.
    let semibold = semibold();
    let mut semibold_list = fonts.families[&FontFamily::Proportional].clone();
    if let Ok(bytes) = std::fs::read(SF_PRO) {
        let bold = FontData::from_owned(bytes.clone()).tweak(FontTweak {
            coords: VariationCoords::new([(b"wght", 600.0)]),
            ..Default::default()
        });
        fonts
            .font_data
            .insert("sf".into(), Arc::new(FontData::from_owned(bytes)));
        fonts.font_data.insert("sf-semibold".into(), Arc::new(bold));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "sf".into());
        semibold_list.insert(0, "sf-semibold".into());
    }
    fonts.families.insert(semibold.clone(), semibold_list);
    fonts.font_data.insert(
        "phosphor".into(),
        Arc::new(FontData::from_static(icons::FONT)),
    );
    for family in [
        FontFamily::Proportional,
        FontFamily::Monospace,
        semibold.clone(),
    ] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("phosphor".into());
    }
    ctx.set_fonts(fonts);

    for theme in [Theme::Light, Theme::Dark] {
        ctx.style_mut_of(theme, |style| {
            // macOS HIG scale: 11 caption, 13 body, 15 semibold title.
            style.text_styles = [
                (TextStyle::Small, FontId::proportional(11.0)),
                (TextStyle::Body, FontId::proportional(13.0)),
                (TextStyle::Button, FontId::proportional(13.0)),
                (TextStyle::Monospace, FontId::monospace(12.0)),
                (TextStyle::Heading, FontId::new(15.0, semibold.clone())),
            ]
            .into();
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            style.spacing.button_padding = egui::vec2(12.0, 6.0);
            let v = &mut style.visuals;
            let p = palette(v);
            v.panel_fill = p.content;
            v.window_fill = p.card;
            v.selection.bg_fill = ACCENT.gamma_multiply(0.25);
            v.selection.stroke.color = ACCENT;
            v.hyperlink_color = ACCENT;
            v.widgets.noninteractive.bg_stroke.color = p.border;
            for w in [
                &mut v.widgets.inactive,
                &mut v.widgets.hovered,
                &mut v.widgets.active,
                &mut v.widgets.noninteractive,
            ] {
                w.corner_radius = 8.into();
            }
        });
    }
}
