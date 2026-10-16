use egui::Ui;
use egui_extras::{Column, TableBuilder};

use crate::actions::{Sig, signal};
use crate::sampler::State;
use crate::table::{Confirm, SortCol, filter_sort, header};
use crate::widgets::fmt_bytes;

#[derive(Default)]
pub struct ProcView {
    query: String,
    col: SortCol,
    asc: bool,
    selected: Option<u32>,
    confirm: Confirm<(u32, Sig)>,
}

impl ProcView {
    pub fn show(&mut self, ui: &mut Ui, s: &State) {
        let rows = filter_sort(&s.procs, &self.query, self.col, self.asc);
        let selected = self
            .selected
            .and_then(|pid| rows.iter().find(|p| p.pid == pid).copied());

        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Search processes")
                    .desired_width(240.0),
            );
            ui.weak(format!("{} processes", rows.len()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                for (label, sig) in [("Kill", Sig::Kill), ("End", Sig::Term)] {
                    if ui
                        .add_enabled(selected.is_some(), egui::Button::new(label))
                        .clicked()
                        && let Some(p) = selected
                    {
                        self.confirm.ask(
                            (p.pid, sig),
                            format!("{label} \"{}\" (pid {})?", p.name, p.pid),
                        );
                    }
                }
            });
        });

        TableBuilder::new(ui)
            .striped(true)
            .column(Column::remainder().at_least(160.0).clip(true))
            .columns(Column::exact(70.0), 2)
            .columns(Column::exact(90.0), 2)
            .header(24.0, |mut h| {
                let (col, asc) = (&mut self.col, &mut self.asc);
                h.col(|ui| header(ui, "Name", SortCol::Name, col, asc));
                h.col(|ui| header(ui, "PID", SortCol::Pid, col, asc));
                h.col(|ui| header(ui, "CPU", SortCol::Cpu, col, asc));
                h.col(|ui| header(ui, "Memory", SortCol::Mem, col, asc));
                h.col(|ui| header(ui, "User", SortCol::User, col, asc));
            })
            .body(|body| {
                body.rows(22.0, rows.len(), |mut row| {
                    let p = rows[row.index()];
                    let is_sel = self.selected == Some(p.pid);
                    row.set_selected(is_sel);
                    row.col(|ui| {
                        if ui
                            .add(
                                egui::Button::selectable(is_sel, &p.name)
                                    .frame_when_inactive(false),
                            )
                            .clicked()
                        {
                            self.selected = Some(p.pid);
                        }
                    });
                    row.col(|ui| {
                        ui.label(p.pid.to_string());
                    });
                    row.col(|ui| {
                        ui.label(format!("{:.1} %", p.cpu));
                    });
                    row.col(|ui| {
                        ui.label(fmt_bytes(p.mem));
                    });
                    row.col(|ui| {
                        ui.label(&p.user);
                    });
                });
            });

        if let Some((pid, sig)) = self.confirm.show(ui, "Confirm") {
            self.confirm.finish(signal(pid, sig));
        }
    }
}
