use std::collections::HashMap;

use egui::{Align, Align2, FontId, Layout, Sense, TextureHandle, TextureOptions, Ui, vec2};
use egui_extras::{Column, TableBuilder};

use crate::actions::{Sig, quit_app, signal};
use crate::icons;
use crate::sample::AppUsage;
use crate::sampler::State;
use crate::table::{BarAction, Confirm, Search, SortCol, action_bar, header};
use crate::widgets::fmt_bytes;

#[derive(Clone, Copy)]
enum AppAction {
    Quit(u32),
    Signal(u32, Sig),
}

/// Dropdown entries next to "End App", as upstream.
const MENU: [(&str, &str, Sig); 3] = [
    ("Kill App", "Kill", Sig::Kill),
    ("Halt App", "Halt", Sig::Stop),
    ("Continue App", "Continue", Sig::Cont),
];

#[derive(Default)]
pub struct AppsView {
    pub search: Search,
    col: SortCol,
    asc: bool,
    selected: Option<u32>,
    confirm: Confirm<AppAction>,
    textures: HashMap<u32, TextureHandle>,
}

impl AppsView {
    pub fn show(&mut self, ui: &mut Ui, s: &State) {
        self.search.show(ui, "Search apps");
        let q = self.search.query.trim().to_lowercase();
        let mut apps: Vec<&AppUsage> = s
            .apps
            .iter()
            .filter(|a| q.is_empty() || a.app.name.to_lowercase().contains(&q))
            .collect();
        apps.sort_by(|a, b| {
            let ord = match self.col {
                SortCol::Name => a.app.name.to_lowercase().cmp(&b.app.name.to_lowercase()),
                SortCol::Mem => a.mem.cmp(&b.mem),
                _ => a.cpu.total_cmp(&b.cpu),
            };
            if self.asc { ord } else { ord.reverse() }
        });
        self.textures
            .retain(|pid, _| s.apps.iter().any(|a| a.app.pid == *pid));

        // pid 0 is the "System Processes" row: selectable, but nothing to end.
        let selected = self
            .selected
            .and_then(|pid| apps.iter().find(|a| a.app.pid == pid).copied())
            .filter(|a| a.app.pid != 0);
        let count = format!("{} apps", s.apps.iter().filter(|a| a.app.pid != 0).count());
        let menu = MENU.map(|(label, ..)| label);
        match (
            action_bar(ui, &count, "End App", &menu, selected.is_some()),
            selected,
        ) {
            (Some(BarAction::Primary), Some(a)) => self.confirm.ask(
                AppAction::Quit(a.app.pid),
                format!("End \"{}\"?", a.app.name),
            ),
            (Some(BarAction::Menu(i)), Some(a)) => {
                let (_, verb, sig) = MENU[i];
                self.confirm.ask(
                    AppAction::Signal(a.app.pid, sig),
                    format!("{verb} \"{}\"?", a.app.name),
                );
            }
            _ => {}
        }

        let (col, asc) = (&mut self.col, &mut self.asc);
        let (sel, textures) = (&mut self.selected, &mut self.textures);
        TableBuilder::new(ui)
            .sense(Sense::click())
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::remainder().at_least(180.0).clip(true))
            .column(Column::exact(110.0))
            .column(Column::exact(100.0))
            .header(28.0, |mut h| {
                h.col(|ui| header(ui, "App", SortCol::Name, col, asc));
                h.col(|ui| header(ui, "Memory", SortCol::Mem, col, asc));
                h.col(|ui| header(ui, "Processor", SortCol::Cpu, col, asc));
            })
            .body(|body| {
                body.rows(44.0, apps.len(), |mut row| {
                    let a = apps[row.index()];
                    row.set_selected(*sel == Some(a.app.pid));
                    let mut clicked = false;
                    row.col(|ui| {
                        ui.add_space(8.0);
                        app_icon(ui, textures, a);
                        ui.add_space(6.0);
                        clicked |= ui
                            .add(
                                egui::Label::new(&a.app.name)
                                    .selectable(false)
                                    .sense(Sense::click()),
                            )
                            .clicked();
                    });
                    row.col(|ui| {
                        ui.label(fmt_bytes(a.mem));
                    });
                    row.col(|ui| {
                        ui.label(format!("{:.1} %", a.cpu));
                    });
                    if clicked || row.response().clicked() {
                        *sel = Some(a.app.pid);
                    }
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
                // Pausing only the main process would leave its helpers running.
                AppAction::Signal(pid, sig @ (Sig::Stop | Sig::Cont)) => s
                    .apps
                    .iter()
                    .find(|a| a.app.pid == pid)
                    .map_or(Ok(()), |a| a.pids.iter().try_for_each(|&p| signal(p, sig))),
                AppAction::Signal(pid, sig) => signal(pid, sig),
            };
            self.confirm.finish(result);
        }
    }
}

/// 28 px Dock icon, or a Phosphor glyph in a faint circle (upstream's `.bubble`).
fn app_icon(ui: &mut Ui, textures: &mut HashMap<u32, TextureHandle>, a: &AppUsage) {
    let size = vec2(28.0, 28.0);
    if let Some(img) = &a.app.icon {
        let tex = textures.entry(a.app.pid).or_insert_with(|| {
            ui.ctx().load_texture(
                format!("app-icon-{}", a.app.pid),
                (**img).clone(),
                TextureOptions::LINEAR,
            )
        });
        ui.add(egui::Image::new(&*tex).fit_to_exact_size(size));
        return;
    }
    let glyph = if a.app.pid == 0 {
        icons::PUZZLE_PIECE
    } else {
        icons::APP_WINDOW
    };
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let color = ui.visuals().text_color();
    ui.painter()
        .circle_filled(rect.center(), 14.0, color.gamma_multiply(0.08));
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(16.0),
        color,
    );
}
