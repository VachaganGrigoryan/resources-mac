//! Kittest helpers for the apps' UI tests (feature `test-support`).

use egui::Ui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

/// A 1000×680 window with the shared theme, drawing `app` with `show` every frame.
pub fn harness<A: 'static>(app: A, show: fn(&mut A, &mut Ui)) -> Harness<'static, A> {
    Harness::builder()
        .with_size([1000.0, 680.0])
        .with_context_setup(crate::theme::apply)
        .build_ui_state(move |ui, app: &mut A| show(app, ui), app)
}

/// True if any widget's label contains `text` (headers repeat names shown elsewhere).
pub fn shows<A>(h: &Harness<'_, A>, text: &str) -> bool {
    h.query_all_by_label_contains(text).next().is_some()
}

/// Clicks the sidebar entry: it is drawn first, so it is the first match.
pub fn open<A>(h: &mut Harness<'_, A>, page: &str) {
    h.query_all_by_label(page).next().expect(page).click();
    h.run();
}
