//! Process list filtering/sorting and the confirm dialog shared by the Apps and Processes pages.

use std::collections::HashMap;

use crate::sample::{AppUsage, ProcInfo};
use crate::{icons, widgets};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortCol {
    Name,
    Pid,
    User,
    #[default]
    Cpu,
    Mem,
}

/// Rows whose name or pid contains `query` (case-insensitive), sorted by `col`.
pub fn filter_sort<'a>(
    procs: &'a [ProcInfo],
    query: &str,
    col: SortCol,
    asc: bool,
) -> Vec<&'a ProcInfo> {
    let q = query.trim().to_lowercase();
    let mut rows: Vec<&ProcInfo> = procs
        .iter()
        .filter(|p| {
            q.is_empty() || p.name.to_lowercase().contains(&q) || p.pid.to_string().contains(&q)
        })
        .collect();
    rows.sort_by(|a, b| {
        let ord = match col {
            SortCol::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortCol::Pid => a.pid.cmp(&b.pid),
            SortCol::User => a.user.cmp(&b.user),
            SortCol::Cpu => a.cpu.total_cmp(&b.cpu),
            SortCol::Mem => a.mem.cmp(&b.mem),
        };
        if asc { ord } else { ord.reverse() }
    });
    rows
}

/// Clickable column header with a Phosphor sort caret; repeat clicks flip the direction.
pub fn header(ui: &mut egui::Ui, label: &str, this: SortCol, col: &mut SortCol, asc: &mut bool) {
    let text = if *col == this {
        let caret = if *asc {
            icons::CARET_UP
        } else {
            icons::CARET_DOWN
        };
        format!("{label}  {caret}")
    } else {
        label.to_owned()
    };
    let resp = ui.add(
        egui::Button::new(
            egui::RichText::new(text)
                .small()
                .family(crate::theme::semibold())
                .weak(),
        )
        .frame(false),
    );
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ColumnHeader, true, label));
    if resp.clicked() {
        if *col == this {
            *asc = !*asc;
        } else {
            *col = this;
            // Names read naturally A→Z; numbers are most useful largest-first.
            *asc = matches!(this, SortCol::Name | SortCol::User);
        }
    }
}

/// GNOME-style search bar under the header: hidden until toggled (Search button or ⌘F),
/// Escape closes it and clears the query.
#[derive(Default)]
pub struct Search {
    pub open: bool,
    pub query: String,
    focus: bool,
}

impl Search {
    pub fn toggle(&mut self) {
        self.open = !self.open;
        self.focus = self.open;
        if !self.open {
            self.query.clear();
        }
    }

    /// Handles the keyboard shortcuts and draws the bar when open.
    pub fn show(&mut self, ui: &mut egui::Ui, hint: &str) {
        let (cmd_f, esc) = ui.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::F),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if cmd_f && !self.open || esc && self.open {
            self.toggle();
        }
        if !self.open {
            return;
        }
        ui.vertical_centered(|ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text(format!("{}  {hint}", icons::MAGNIFYING_GLASS))
                    .desired_width(ui.available_width().min(420.0))
                    .margin(egui::vec2(10.0, 6.0)),
            );
            if std::mem::take(&mut self.focus) {
                resp.request_focus();
            }
        });
        ui.add_space(4.0);
    }
}

/// What the bottom action bar's buttons asked for.
pub enum BarAction {
    /// The red primary button ("End App").
    Primary,
    /// An entry from the dropdown, by index.
    Menu(usize),
}

/// Bottom bar: item count on the left; info button and a red split button on the right
/// (upstream's `AdwSplitButton`).
pub fn action_bar(
    ui: &mut egui::Ui,
    count: &str,
    primary: &str,
    menu: &[&str],
    enabled: bool,
) -> Option<BarAction> {
    let mut action = None;
    egui::Panel::bottom(ui.scope_id().with("actions"))
        .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(16, 10)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(count).weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.add_enabled_ui(enabled, |ui| {
                        ui.menu_button(icons::CARET_DOWN, |ui| {
                            for (i, item) in menu.iter().enumerate() {
                                if ui.button(*item).clicked() {
                                    action = Some(BarAction::Menu(i));
                                }
                            }
                        })
                        .response
                        .on_hover_text("More actions");
                    });
                    if widgets::destructive_button(ui, primary, enabled).clicked() {
                        action = Some(BarAction::Primary);
                    }
                    ui.add_space(8.0);
                    // Upstream opens an information dialog; not built yet.
                    ui.add_enabled_ui(false, |ui| {
                        widgets::icon_button(ui, icons::INFO, "Information");
                    });
                });
            });
        });
    action
}

/// Dock icon textures by app pid, shared shape for the Apps and Processes lists.
#[derive(Default)]
pub struct IconCache(HashMap<u32, egui::TextureHandle>);

impl IconCache {
    /// Drops textures of apps that quit.
    pub fn retain(&mut self, apps: &[AppUsage]) {
        self.0
            .retain(|pid, _| apps.iter().any(|a| a.app.pid == *pid));
    }

    /// `size` px icon for `app`, or `glyph` in a faint circle (upstream's `.bubble`)
    /// when it has none.
    pub fn show(&mut self, ui: &mut egui::Ui, app: Option<&AppUsage>, glyph: &str, size: f32) {
        let side = egui::vec2(size, size);
        if let Some((a, img)) = app.and_then(|a| Some((a, a.app.icon.as_ref()?))) {
            let tex = self.0.entry(a.app.pid).or_insert_with(|| {
                ui.ctx().load_texture(
                    format!("app-icon-{}", a.app.pid),
                    (**img).clone(),
                    egui::TextureOptions::LINEAR,
                )
            });
            ui.add(egui::Image::new(&*tex).fit_to_exact_size(side));
            return;
        }
        let (rect, _) = ui.allocate_exact_size(side, egui::Sense::hover());
        let color = ui.visuals().text_color();
        ui.painter()
            .circle_filled(rect.center(), size / 2.0, color.gamma_multiply(0.08));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            glyph,
            egui::FontId::proportional(size * 0.57),
            color,
        );
    }
}

/// A pending destructive action awaiting confirmation, plus any error from the last attempt.
pub struct Confirm<A> {
    pub pending: Option<(A, String)>,
    pub error: Option<String>,
}

impl<A> Default for Confirm<A> {
    fn default() -> Self {
        Self {
            pending: None,
            error: None,
        }
    }
}

impl<A: Copy> Confirm<A> {
    pub fn ask(&mut self, action: A, question: String) {
        self.pending = Some((action, question));
        self.error = None;
    }

    /// Shows the dialog; returns the action once the user confirms it.
    pub fn show(&mut self, ui: &egui::Ui, confirm_label: &str) -> Option<A> {
        let (action, question) = self.pending.clone()?;
        let mut chosen = None;
        let resp = egui::Modal::new(egui::Id::unique("confirm")).show(ui.ctx(), |ui| {
            ui.set_width(320.0);
            ui.label(&question);
            if let Some(err) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, err);
            }
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    self.pending = None;
                }
                if ui.button(confirm_label).clicked() {
                    chosen = Some(action);
                }
            });
        });
        if resp.should_close() {
            self.pending = None;
        }
        chosen
    }

    /// Records the outcome: success closes the dialog, failure keeps it open with the error.
    pub fn finish(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => self.pending = None,
            Err(e) => self.error = Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::ProcInfo;

    fn p(pid: u32, name: &str, cpu: f32) -> ProcInfo {
        ProcInfo {
            pid,
            ppid: None,
            name: name.into(),
            user: "me".into(),
            cpu,
            mem: 0,
            threads: None,
        }
    }

    #[test]
    fn filter_is_case_insensitive() {
        let procs = [p(1, "Safari", 0.0), p(2, "Finder", 0.0)];
        let out = filter_sort(&procs, "SAF", SortCol::Name, true);
        assert_eq!(out.iter().map(|p| p.pid).collect::<Vec<_>>(), [1]);
    }

    #[test]
    fn filter_matches_pid() {
        let procs = [p(123, "a", 0.0), p(456, "b", 0.0)];
        let out = filter_sort(&procs, "45", SortCol::Pid, true);
        assert_eq!(out.iter().map(|p| p.pid).collect::<Vec<_>>(), [456]);
    }

    #[test]
    fn sort_by_cpu_desc() {
        let procs = [p(1, "a", 5.0), p(2, "b", 50.0), p(3, "c", 1.0)];
        let out = filter_sort(&procs, "", SortCol::Cpu, false);
        assert_eq!(out.iter().map(|p| p.pid).collect::<Vec<_>>(), [2, 1, 3]);
    }
}
