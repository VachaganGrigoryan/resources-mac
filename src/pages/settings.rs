//! Settings page: which stats go into the menu bar and the desktop widget, with suggestions
//! based on what this Mac is doing right now.

use egui::{Align, Layout, RichText, Ui};
use serde::{Deserialize, Serialize};

use crate::sampler::State;
use crate::stats::{self, Stat};
use crate::widgets::{card, group};

/// Saved with eframe's app storage under [`Settings::KEY`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Show stats in the menu bar; closing the window then keeps the app running there.
    pub menu_bar: bool,
    pub menu_bar_stats: Vec<Stat>,
    pub widget: bool,
    pub widget_stats: Vec<Stat>,
    /// Desktop widget's top-left corner in screen points; `None` until it is first moved.
    pub widget_pos: Option<[f32; 2]>,
}

impl Settings {
    pub const KEY: &str = "settings";
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            menu_bar: true,
            menu_bar_stats: vec![Stat::Cpu, Stat::Memory],
            widget: false,
            widget_stats: vec![Stat::Cpu, Stat::Memory, Stat::Network],
            widget_pos: None,
        }
    }
}

/// Adds `stat` at the end, or removes it.
fn toggle(list: &mut Vec<Stat>, stat: Stat) {
    if let Some(i) = list.iter().position(|s| *s == stat) {
        list.remove(i);
    } else {
        list.push(stat);
    }
}

/// One toggle chip per stat; stats this Mac doesn't report are disabled.
fn chips(ui: &mut Ui, list: &mut Vec<Stat>, s: &State) {
    ui.horizontal_wrapped(|ui| {
        for stat in Stat::ALL {
            let on = list.contains(&stat);
            let available = stats::short(stat, s).is_some();
            let chip = egui::Button::selectable(on, format!("{}  {}", stat.icon(), stat.label()))
                .corner_radius(14);
            if ui.add_enabled(available || on, chip).clicked() {
                toggle(list, stat);
            }
        }
    });
}

fn hint(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().weak());
}

pub fn show(ui: &mut Ui, settings: &mut Settings, s: &State) {
    group(ui, "Menu Bar", |ui| {
        card(ui, |ui| {
            ui.checkbox(&mut settings.menu_bar, "Show stats in the menu bar");
            hint(
                ui,
                "Closing the window keeps Resources running in the menu bar.",
            );
            ui.add_enabled_ui(settings.menu_bar, |ui| {
                chips(ui, &mut settings.menu_bar_stats, s);
                ui.label(RichText::new(stats::title(&settings.menu_bar_stats, s)).monospace())
                    .on_hover_text("Preview");
            });
        });
    });
    group(ui, "Desktop Widget", |ui| {
        card(ui, |ui| {
            ui.checkbox(&mut settings.widget, "Show a widget on the desktop");
            hint(
                ui,
                "Sits below your windows; drag it anywhere. It stays while Resources runs.",
            );
            ui.add_enabled_ui(settings.widget, |ui| {
                chips(ui, &mut settings.widget_stats, s);
                if ui.button("Reset Position").clicked() {
                    settings.widget_pos = None;
                }
            });
        });
    });
    group(ui, "Suggested for This Mac", |ui| {
        card(ui, |ui| {
            for (stat, reason) in stats::suggest(s) {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(stat.icon()).size(16.0));
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(stat.label());
                        hint(ui, reason);
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        for (label, list) in [
                            ("Add to Widget", &mut settings.widget_stats),
                            ("Add to Menu Bar", &mut settings.menu_bar_stats),
                        ] {
                            if ui
                                .add_enabled(!list.contains(&stat), egui::Button::new(label))
                                .clicked()
                            {
                                list.push(stat);
                            }
                        }
                    });
                });
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_adds_then_removes() {
        let mut l = vec![Stat::Cpu];
        toggle(&mut l, Stat::Gpu);
        assert_eq!(l, [Stat::Cpu, Stat::Gpu]);
        toggle(&mut l, Stat::Cpu);
        assert_eq!(l, [Stat::Gpu]);
    }

    #[test]
    fn missing_fields_load_as_defaults() {
        // Older saves (or a new field) must not reset everything.
        let s: Settings = ron::from_str("(widget: true)").unwrap();
        assert!(s.widget);
        assert_eq!(s.menu_bar_stats, Settings::default().menu_bar_stats);
    }
}
