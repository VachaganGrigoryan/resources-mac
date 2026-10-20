use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use disk_sweep::app::App;
use disk_sweep::pages::settings::Settings;
use disk_sweep::worker::{RootScan, State};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn harness(state: State, settings: Settings) -> Harness<'static, App> {
    let app = App::new(Arc::new(Mutex::new(state))).with_settings(settings);
    Harness::builder()
        .with_size([1000.0, 680.0])
        .with_context_setup(resources_mac::theme::apply)
        .build_ui_state(|ui, app: &mut App| app.show(ui), app)
}

fn shows(h: &Harness<'static, App>, text: &str) -> bool {
    h.query_all_by_label_contains(text).next().is_some()
}

/// Clicks the sidebar entry: it is drawn first, so it is the first match.
fn open(h: &mut Harness<'static, App>, page: &str) {
    h.query_all_by_label(page).next().expect(page).click();
    h.run();
}

#[test]
fn sidebar_lists_pages() {
    let h = harness(State::default(), Settings::default());
    for page in ["Overview", "Docker", "Settings"] {
        assert!(h.query_all_by_label(page).count() >= 1, "missing {page}");
    }
}

#[test]
fn settings_lists_roots() {
    let settings = Settings {
        roots: vec![PathBuf::from("/tmp/sweep-a")],
        ..Settings::default()
    };
    let mut h = harness(State::default(), settings);
    open(&mut h, "Settings");
    assert!(shows(&h, "/tmp/sweep-a"));
    assert!(shows(&h, "Add Folder"));
}

#[test]
fn failed_root_shows_error() {
    let mut scan = RootScan::new(PathBuf::from("/tmp/sweep-gone"));
    scan.scanning = false;
    scan.result = Some(Err("not found".into()));
    let state = State {
        roots: vec![scan],
        ..State::default()
    };
    let mut h = harness(state, Settings::default());
    open(&mut h, "sweep-gone");
    assert!(shows(&h, "not found"));
}
