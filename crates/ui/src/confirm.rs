//! Modal that asks before a destructive action and keeps the error if it fails.

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
