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

#[test]
fn drives_page_shows_disk() {
    let mut s = State::default();
    s.disks.push(resources_mac::sampler::DiskStat {
        name: "Macintosh HD".into(),
        mount: "/".into(),
        total: 500 << 30,
        avail: 200 << 30,
        ..Default::default()
    });
    let mut h = harness(s);
    open(&mut h, "Drives");
    h.get_by_label_contains("Macintosh HD");
}

#[test]
fn network_page_shows_iface_and_hides_loopback() {
    let mut s = State::default();
    for name in ["en0", "lo0", "utun9"] {
        s.nets.push(resources_mac::sampler::NetStat {
            name: name.into(),
            rx_total: if name == "utun9" { 0 } else { 1000 },
            ..Default::default()
        });
    }
    let mut h = harness(s);
    open(&mut h, "Network");
    h.get_by_label_contains("en0");
    assert!(h.query_by_label_contains("lo0").is_none());
    assert!(
        h.query_by_label_contains("utun9").is_none(),
        "never-used interface shown"
    );
}
