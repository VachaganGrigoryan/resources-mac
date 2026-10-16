use std::sync::{Arc, Mutex};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use resources_mac::app::App;
use resources_mac::sampler::State;

fn harness(state: State) -> Harness<'static, App> {
    let app = App::new(Arc::new(Mutex::new(state)));
    Harness::builder()
        .with_size([1000.0, 680.0])
        .build_ui_state(|ui, app: &mut App| app.show(ui), app)
}

/// Clicks the sidebar entry: it is drawn first, so it is the first match.
fn open(h: &mut Harness<'static, App>, page: &str) {
    h.query_all_by_label(page).next().expect(page).click();
    h.run();
}

#[test]
fn sidebar_lists_pages() {
    let h = harness(State::default());
    for page in [
        "Apps",
        "Processes",
        "Processor",
        "Memory",
        "Drives",
        "Network",
    ] {
        assert!(h.query_all_by_label(page).count() >= 1, "missing {page}");
    }
}

#[test]
fn clicking_memory_shows_memory_page() {
    let mut h = harness(State::default());
    assert!(h.query_by_label_contains("Swap").is_none());
    open(&mut h, "Memory");
    h.get_by_label_contains("Swap");
}
