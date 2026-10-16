use egui::Ui;
use egui_extras::{Column, TableBuilder};

use crate::actions::{Sig, quit_app, signal};
use crate::sampler::State;
use crate::table::Confirm;
use crate::widgets::fmt_bytes;

#[derive(Clone, Copy)]
enum AppAction {
    Quit(u32),
    ForceQuit(u32),
}

#[derive(Default)]
pub struct AppsView {
    selected: Option<u32>,
    confirm: Confirm<AppAction>,
}

impl AppsView {
    pub fn show(&mut self, ui: &mut Ui, s: &State) {
        let mut apps: Vec<_> = s.apps.iter().collect();
        apps.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
        let selected = self
            .selected
            .and_then(|pid| apps.iter().find(|a| a.app.pid == pid).copied());

        ui.horizontal(|ui| {
            ui.weak(format!("{} apps", apps.len()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let enabled = selected.is_some();
                if ui
                    .add_enabled(enabled, egui::Button::new("Force Quit"))
                    .clicked()
                    && let Some(a) = selected
                {
                    self.confirm.ask(
                        AppAction::ForceQuit(a.app.pid),
                        format!("Force quit \"{}\"?", a.app.name),
                    );
                }
                if ui.add_enabled(enabled, egui::Button::new("Quit")).clicked()
                    && let Some(a) = selected
                {
                    self.confirm.ask(
                        AppAction::Quit(a.app.pid),
                        format!("Quit \"{}\"?", a.app.name),
                    );
                }
            });
        });

        TableBuilder::new(ui)
            .striped(true)
            .column(Column::remainder().at_least(160.0).clip(true))
            .columns(Column::exact(80.0), 3)
            .header(24.0, |mut h| {
                for title in ["Name", "CPU", "Memory", "Processes"] {
                    h.col(|ui| {
                        ui.strong(title);
                    });
                }
            })
            .body(|body| {
                body.rows(22.0, apps.len(), |mut row| {
                    let a = apps[row.index()];
                    let is_sel = self.selected == Some(a.app.pid);
                    row.set_selected(is_sel);
                    row.col(|ui| {
                        if ui
                            .add(
                                egui::Button::selectable(is_sel, &a.app.name)
                                    .frame_when_inactive(false),
                            )
                            .clicked()
                        {
                            self.selected = Some(a.app.pid);
                        }
                    });
                    row.col(|ui| {
                        ui.label(format!("{:.1} %", a.cpu));
                    });
                    row.col(|ui| {
                        ui.label(fmt_bytes(a.mem));
                    });
                    row.col(|ui| {
                        ui.label(a.pids.len().to_string());
                    });
                });
            });

        if let Some(action) = self.confirm.show(ui, "Confirm") {
            let result = match action {
                // Fall back to SIGTERM when the app is not reachable through AppKit.
                AppAction::Quit(pid) => {
                    if quit_app(pid) {
                        Ok(())
                    } else {
                        signal(pid, Sig::Term)
                    }
                }
                AppAction::ForceQuit(pid) => signal(pid, Sig::Kill),
            };
            self.confirm.finish(result);
        }
    }
}
