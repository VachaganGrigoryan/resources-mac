use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use resources_ui::{icons, theme};

#[test]
fn phosphor_glyphs_render_in_harness() {
    // Without the vendored font, egui panics on the private-use glyph.
    let h = Harness::builder()
        .with_context_setup(theme::apply)
        .build_ui(|ui| {
            ui.label(format!("{} Processor", icons::CPU));
        });
    h.get_by_label_contains("Processor");
}
