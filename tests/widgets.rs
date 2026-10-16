use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use resources_mac::sample::History;
use resources_mac::theme::{self, color};
use resources_mac::{icons, widgets};

fn harness<'a>(f: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a> {
    Harness::builder()
        .with_size([400.0, 400.0])
        .with_context_setup(theme::apply)
        .build_ui(f)
}

#[test]
fn sidebar_item_is_found_by_plain_label() {
    let mut hist = History::default();
    hist.push(40.0);
    let h = harness(|ui| {
        widgets::sidebar_item(
            ui,
            icons::CPU,
            "Processor",
            true,
            Some((&[(&hist, color::CPU)], Some(100.0))),
        );
    });
    h.get_by_label("Processor");
}

#[test]
fn icon_button_is_found_by_label_not_glyph() {
    let h = harness(|ui| {
        widgets::icon_button(ui, icons::MAGNIFYING_GLASS, "Search");
    });
    h.get_by_label("Search");
}

#[test]
fn boxed_list_shows_graph_and_rows() {
    let mut hist = History::default();
    hist.push(1e6);
    let h = harness(|ui| {
        widgets::boxed_list(
            ui,
            &[
                widgets::Item::Graph {
                    caption: "Read Speed",
                    value: "1.00 MB/s".into(),
                    series: &[(&hist, color::READ)],
                    y_max: None,
                },
                widgets::Item::Row {
                    title: "Cycles",
                    value: "123".into(),
                },
            ],
        );
    });
    for label in ["Read Speed", "1.00 MB/s", "Cycles", "123"] {
        h.get_by_label(label);
    }
}
