//! Settings page: which folders to watch, and the theme.

use std::path::PathBuf;

use egui::{Align, Layout, RichText, ThemePreference, Ui};
use resources_ui::icons;
use resources_ui::settings::{appearance, hint};
use resources_ui::widgets::{card, group, icon_button};
use serde::{Deserialize, Serialize};

/// Saved with eframe's app storage under [`Settings::KEY`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub roots: Vec<PathBuf>,
    pub theme: ThemePreference,
}

impl Settings {
    pub const KEY: &str = "disk-sweep";
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            theme: ThemePreference::System,
        }
    }
}

pub enum SettingsAction {
    Added(PathBuf),
    Removed(PathBuf),
}

pub fn show(ui: &mut Ui, settings: &mut Settings) -> Option<SettingsAction> {
    let mut action = None;
    group(ui, "Folders", |ui| {
        card(ui, |ui| {
            if settings.roots.is_empty() {
                ui.label(RichText::new("No folders yet.").weak());
            }
            for root in &settings.roots {
                ui.horizontal(|ui| {
                    ui.label(format!("{}  {}", icons::FOLDER, root.display()));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let label = format!("Remove {}", root.display());
                        if icon_button(ui, icons::X, &label).clicked() {
                            action = Some(SettingsAction::Removed(root.clone()));
                        }
                    });
                });
            }
            ui.add_space(4.0);
            if ui.button(format!("{}  Add Folder…", icons::PLUS)).clicked()
                && let Some(dir) = rfd::FileDialog::new().pick_folder()
                && !settings.roots.contains(&dir)
            {
                action = Some(SettingsAction::Added(dir));
            }
        });
        hint(ui, "Folders are scanned at start and after every clean-up.");
    });
    match &action {
        Some(SettingsAction::Added(dir)) => settings.roots.push(dir.clone()),
        Some(SettingsAction::Removed(dir)) => settings.roots.retain(|r| r != dir),
        None => {}
    }
    appearance(ui, &mut settings.theme);
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_load_as_defaults() {
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s, Settings::default());
    }
}
