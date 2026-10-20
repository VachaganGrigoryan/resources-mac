//! Pieces both apps' Settings pages share.

use egui::{RichText, ThemePreference, Ui};

use crate::icons;
use crate::widgets::{card, group};

/// Small dimmed explanation under a setting.
pub fn hint(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().weak());
}

/// The "Appearance" group: System / Light / Dark chips that switch the theme right away.
pub fn appearance(ui: &mut Ui, theme: &mut ThemePreference) {
    group(ui, "Appearance", |ui| {
        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (choice, icon, label) in [
                    (ThemePreference::System, icons::DESKTOP, "System"),
                    (ThemePreference::Light, icons::SUN, "Light"),
                    (ThemePreference::Dark, icons::MOON, "Dark"),
                ] {
                    let chip =
                        egui::Button::selectable(*theme == choice, format!("{icon}  {label}"))
                            .corner_radius(14);
                    if ui.add(chip).clicked() {
                        *theme = choice;
                        ui.ctx().set_theme(choice);
                    }
                }
            });
        });
    });
}
