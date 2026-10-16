//! Process list filtering/sorting and the confirm dialog shared by the Apps and Processes pages.

use crate::sample::ProcInfo;

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

/// Clickable column header that toggles sort direction on repeat clicks.
pub fn header(ui: &mut egui::Ui, label: &str, this: SortCol, col: &mut SortCol, asc: &mut bool) {
    let text = if *col == this {
        format!("{label} {}", if *asc { "^" } else { "v" })
    } else {
        label.to_owned()
    };
    if ui
        .add(egui::Button::new(egui::RichText::new(text).strong()).frame(false))
        .clicked()
    {
        if *col == this {
            *asc = !*asc;
        } else {
            *col = this;
            // Names read naturally A→Z; numbers are most useful largest-first.
            *asc = matches!(this, SortCol::Name | SortCol::User);
        }
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
