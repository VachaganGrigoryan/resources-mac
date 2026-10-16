use std::collections::HashMap;

use egui::{Align, Layout, Sense, Ui};
use egui_extras::{Column, TableBuilder};

use crate::actions::{Sig, signal};
use crate::icons;
use crate::sample::AppUsage;
use crate::sampler::State;
use crate::table::{
    BarAction, Confirm, IconCache, Search, SortCol, action_bar, filter_sort, header,
};
use crate::widgets::fmt_bytes;

/// Dropdown entries next to "End Process", as upstream.
const MENU: [(&str, &str, Sig); 3] = [
    ("Kill Process", "Kill", Sig::Kill),
    ("Halt Process", "Halt", Sig::Stop),
    ("Continue Process", "Continue", Sig::Cont),
];

pub struct ProcView {
    pub search: Search,
    col: SortCol,
    asc: bool,
    selected: Option<u32>,
    confirm: Confirm<(u32, Sig)>,
    icons: IconCache,
}

impl Default for ProcView {
    /// Upstream lists processes by memory, largest first.
    fn default() -> Self {
        Self {
            search: Search::default(),
            col: SortCol::Mem,
            asc: false,
            selected: None,
            confirm: Confirm::default(),
            icons: IconCache::default(),
        }
    }
}

impl ProcView {
    pub fn show(&mut self, ui: &mut Ui, s: &State) {
        self.search.show(ui, "Search processes");
        let rows = filter_sort(&s.procs, &self.search.query, self.col, self.asc);
        let selected = self
            .selected
            .and_then(|pid| rows.iter().find(|p| p.pid == pid).copied());

        let count = format!("{} processes", rows.len());
        let menu = MENU.map(|(label, ..)| label);
        let ask = match action_bar(ui, &count, "End Process", &menu, selected.is_some()) {
            Some(BarAction::Primary) => Some(("End", Sig::Term)),
            Some(BarAction::Menu(i)) => Some((MENU[i].1, MENU[i].2)),
            None => None,
        };
        if let (Some((verb, sig)), Some(p)) = (ask, selected) {
            self.confirm.ask(
                (p.pid, sig),
                format!("{verb} \"{}\" (pid {})?", p.name, p.pid),
            );
        }

        self.icons.retain(&s.apps);
        // Processes that belong to an app show that app's icon.
        let owner: HashMap<u32, &AppUsage> = s
            .apps
            .iter()
            .filter(|a| a.app.pid != 0)
            .flat_map(|a| a.pids.iter().map(move |&pid| (pid, a)))
            .collect();
        let (col, asc, sel) = (&mut self.col, &mut self.asc, &mut self.selected);
        let cache = &mut self.icons;
        TableBuilder::new(ui)
            .sense(Sense::click())
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::remainder().at_least(160.0).clip(true))
            .column(Column::exact(90.0))
            .column(Column::exact(100.0))
            .column(Column::exact(110.0))
            .column(Column::exact(90.0))
            .header(28.0, |mut h| {
                h.col(|ui| {
                    ui.add_space(8.0);
                    header(ui, "Process", SortCol::Name, col, asc);
                });
                h.col(|ui| header(ui, "Process ID", SortCol::Pid, col, asc));
                h.col(|ui| header(ui, "User", SortCol::User, col, asc));
                h.col(|ui| header(ui, "Memory", SortCol::Mem, col, asc));
                h.col(|ui| header(ui, "Processor", SortCol::Cpu, col, asc));
            })
            .body(|body| {
                body.rows(34.0, rows.len(), |mut row| {
                    let p = rows[row.index()];
                    row.set_selected(*sel == Some(p.pid));
                    let mut clicked = false;
                    row.col(|ui| {
                        ui.add_space(8.0);
                        let app = owner.get(&p.pid).copied();
                        cache.show(ui, app, icons::GEAR_SIX, 20.0);
                        ui.add_space(4.0);
                        clicked |= ui
                            .add(
                                egui::Label::new(&p.name)
                                    .selectable(false)
                                    .sense(Sense::click()),
                            )
                            .clicked();
                    });
                    row.col(|ui| {
                        ui.label(p.pid.to_string());
                    });
                    row.col(|ui| {
                        ui.label(&p.user);
                    });
                    row.col(|ui| {
                        ui.label(fmt_bytes(p.mem));
                    });
                    row.col(|ui| {
                        ui.label(format!("{:.1} %", p.cpu));
                    });
                    if clicked || row.response().clicked() {
                        *sel = Some(p.pid);
                    }
                });
            });

        if let Some((pid, sig)) = self.confirm.show(ui, "Confirm") {
            self.confirm.finish(signal(pid, sig));
        }
    }
}
