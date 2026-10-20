use egui::{FontFamily, TextStyle};
use egui_kittest::Harness;
use resources_ui::theme;

fn ctx() -> egui::Context {
    let h = Harness::builder()
        .with_context_setup(theme::apply)
        .build_ui(|ui| {
            ui.heading("Title");
        });
    h.ctx.clone()
}

#[test]
fn text_styles_are_readable() {
    let ctx = ctx();
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let styles = ctx.style_of(theme).text_styles.clone();
        for (style, font) in &styles {
            assert!(font.size >= 11.0, "{style:?} is {} pt", font.size);
        }
        let size = |s: TextStyle| styles[&s].size;
        assert!(size(TextStyle::Heading) > size(TextStyle::Body));
        assert!(size(TextStyle::Body) > size(TextStyle::Small));
    }
}

#[test]
fn heading_uses_semibold_family() {
    let ctx = ctx();
    let semibold = FontFamily::Name(theme::SEMIBOLD.into());
    assert_eq!(
        ctx.style_of(egui::Theme::Light).text_styles[&TextStyle::Heading].family,
        semibold
    );
    assert!(ctx.fonts(|f| f.families()).contains(&semibold));
}
