//! Process list filtering, sorting, search and icons shared by the Apps and Processes pages.

use std::collections::HashMap;

pub use resources_ui::confirm::Confirm;
pub use resources_ui::widgets::{BarAction, action_bar};

use crate::icons;
use crate::sample::{AppUsage, ProcInfo};

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

/// Dock icon textures by app pid, shared shape for the Apps and Processes lists.
#[derive(Default)]
pub struct IconCache(HashMap<u32, egui::TextureHandle>);

impl IconCache {
    /// Drops textures of apps that quit.
    pub fn retain(&mut self, apps: &[AppUsage]) {
        self.0
            .retain(|pid, _| apps.iter().any(|a| a.app.pid == *pid));
    }

    /// `app`'s Dock icon, uploaded on first use; `None` when it has none.
    pub fn texture(&mut self, ctx: &egui::Context, a: &AppUsage) -> Option<&egui::TextureHandle> {
        let img = a.app.icon.as_ref()?;
        Some(self.0.entry(a.app.pid).or_insert_with(|| {
            ctx.load_texture(
                format!("app-icon-{}", a.app.pid),
                (**img).clone(),
                egui::TextureOptions::LINEAR,
            )
        }))
    }

    /// `size` px icon for `app`, or `glyph` in a faint circle (upstream's `.bubble`)
    /// when it has none.
    pub fn show(&mut self, ui: &mut egui::Ui, app: Option<&AppUsage>, glyph: &str, size: f32) {
        let side = egui::vec2(size, size);
        if let Some(tex) = app.and_then(|a| self.texture(ui.ctx(), a)) {
            ui.add(egui::Image::new(tex).fit_to_exact_size(side));
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
