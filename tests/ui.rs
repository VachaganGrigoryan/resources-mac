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

fn proc(pid: u32, name: &str) -> resources_mac::sample::ProcInfo {
    resources_mac::sample::ProcInfo {
        pid,
        ppid: None,
        name: name.into(),
        user: "me".into(),
        cpu: 1.0,
        mem: 1 << 20,
        threads: None,
    }
}

#[test]
fn processes_search_filters_rows() {
    let s = State {
        procs: vec![proc(101, "Safari"), proc(202, "Finder")],
        ..Default::default()
    };
    let mut h = harness(s);
    open(&mut h, "Processes");
    h.get_by_label("Finder");
    h.get_by_role(egui::accesskit::Role::TextInput).focus();
    h.run();
    h.get_by_role(egui::accesskit::Role::TextInput)
        .type_text("saf");
    h.run();
    h.get_by_label("Safari");
    assert!(h.query_by_label("Finder").is_none());
}

#[test]
fn ending_a_process_asks_for_confirmation() {
    let s = State {
        procs: vec![proc(101, "Safari")],
        ..Default::default()
    };
    let mut h = harness(s);
    open(&mut h, "Processes");
    h.get_by_label("Safari").click();
    h.run();
    h.get_by_label("End").click();
    h.run();
    h.get_by_label_contains("End \"Safari\"");
}

#[test]
fn apps_page_lists_apps() {
    let s = State {
        apps: vec![resources_mac::sample::AppUsage {
            app: resources_mac::sample::AppInfo {
                pid: 7,
                name: "Notes".into(),
                bundle_id: None,
            },
            cpu: 2.0,
            mem: 1 << 20,
            pids: vec![7],
        }],
        ..Default::default()
    };
    let h = harness(s);
    h.get_by_label("Notes");
}
