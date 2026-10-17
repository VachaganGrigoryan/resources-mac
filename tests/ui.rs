use std::sync::{Arc, Mutex};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use resources_mac::app::App;
use resources_mac::icons;
use resources_mac::sampler::State;

fn harness(state: State) -> Harness<'static, App> {
    let app = App::new(Arc::new(Mutex::new(state)));
    Harness::builder()
        .with_size([1000.0, 680.0])
        .with_context_setup(resources_mac::theme::apply)
        .build_ui_state(|ui, app: &mut App| app.show(ui), app)
}

/// True if any widget's label contains `text` (headers repeat names shown in Properties).
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
    let h = harness(State::default());
    for page in ["Apps", "Processes", "Processor", "Memory"] {
        assert!(h.query_all_by_label(page).count() >= 1, "missing {page}");
    }
}

#[test]
fn clicking_memory_shows_memory_page() {
    let mut h = harness(State::default());
    assert!(!shows(&h, "Swap"), "Swap");
    open(&mut h, "Memory");
    assert!(shows(&h, "Swap"), "Swap");
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
    // Sidebar name follows upstream: capacity in decimal units.
    open(&mut h, "537 GB Drive");
    assert!(shows(&h, "Macintosh HD"), "Macintosh HD");
}

#[test]
fn removed_device_falls_back_to_overview() {
    let state = Arc::new(Mutex::new(State::default()));
    state
        .lock()
        .unwrap()
        .disks
        .push(resources_mac::sampler::DiskStat {
            name: "USB Stick".into(),
            mount: "/Volumes/USB".into(),
            total: 64_000_000_000,
            removable: true,
            ..Default::default()
        });
    let mut h = Harness::builder()
        .with_size([1000.0, 680.0])
        .with_context_setup(resources_mac::theme::apply)
        .build_ui_state(
            |ui, app: &mut App| app.show(ui),
            App::new(Arc::clone(&state)),
        );
    open(&mut h, "64 GB Drive");
    assert!(shows(&h, "USB Stick"), "USB Stick");
    state.lock().unwrap().disks.clear();
    h.run();
    assert!(!shows(&h, "USB Stick"), "USB Stick");
    assert!(h.query_by_label("64 GB Drive").is_none());
    h.get_by_role_and_label(egui::accesskit::Role::Canvas, "Overview");
}

#[test]
fn network_page_shows_iface_and_hides_loopback() {
    let mut s = State::default();
    for name in ["en0", "lo0", "utun9", "awdl0"] {
        s.nets.push(resources_mac::sampler::NetStat {
            name: name.into(),
            port: if name == "en0" {
                "Wi-Fi".into()
            } else {
                String::new()
            },
            rx_total: if name == "utun9" { 0 } else { 1000 },
            ..Default::default()
        });
    }
    let mut h = harness(s);
    open(&mut h, "Wi-Fi Connection");
    assert!(shows(&h, "en0"), "en0");
    assert!(!shows(&h, "lo0"), "lo0");
    assert!(!shows(&h, "awdl0"), "awdl0");
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
    // Search is hidden until the header's Search button (or Cmd+F) opens it, as in GNOME.
    assert!(h.query_by_role(egui::accesskit::Role::TextInput).is_none());
    h.get_by_label("Search").click();
    h.run();
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
    h.get_by_label("End Process").click();
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
                icon: None,
            },
            cpu: 2.0,
            mem: 1 << 20,
            pids: vec![7],
        }],
        ..Default::default()
    };
    let mut h = harness(s);
    open(&mut h, "Apps");
    h.get_by_label("Notes").click();
    h.run();
    h.get_by_label("End App").click();
    h.run();
    assert!(shows(&h, "End \"Notes\""));
}

#[test]
fn gpu_page_shown_only_with_stats() {
    let h = harness(State::default());
    assert!(h.query_by_label("GPU").is_none());

    let s = State {
        gpu: Some(resources_mac::collect::gpu::GpuStat {
            name: "Apple M4 Max".into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut h = harness(s);
    open(&mut h, "GPU");
    assert!(shows(&h, "Apple M4 Max"), "Apple M4 Max");
}

#[test]
fn battery_page_shown_only_with_battery() {
    let h = harness(State::default());
    assert!(h.query_by_label("Battery").is_none());

    let s = State {
        battery: Some(resources_mac::collect::battery::BatteryStat {
            percent: 81.0,
            health: 97.0,
            cycles: Some(123),
            state: "Charging".into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut h = harness(s);
    open(&mut h, "Battery");
    assert!(shows(&h, "Charging"), "Charging");
    h.get_by_label("123");
}

#[test]
fn duplicate_sidebar_names_get_device_suffix() {
    let mut s = State::default();
    for (name, mount) in [("Backup A", "/Volumes/A"), ("Backup B", "/Volumes/B")] {
        s.disks.push(resources_mac::sampler::DiskStat {
            name: name.into(),
            mount: mount.into(),
            total: 1_000_000_000_000,
            ..Default::default()
        });
    }
    let h = harness(s);
    h.get_by_label("1 TB Drive (/Volumes/A)");
    h.get_by_label("1 TB Drive (/Volumes/B)");
}

#[test]
fn tunnels_merge_into_one_vpn_entry() {
    let net = |name: &str, port: &str| resources_mac::sampler::NetStat {
        name: name.into(),
        port: port.into(),
        rx_total: 10,
        ..Default::default()
    };
    let s = State {
        nets: vec![net("en0", "Wi-Fi"), net("utun3", ""), net("utun4", "")],
        vpn: Some(net("VPN", "")),
        vpn_members: vec!["utun3".into(), "utun4".into()],
        ..Default::default()
    };
    let mut h = harness(s);
    assert_eq!(h.query_all_by_label("VPN Connection").count(), 1);
    h.get_by_label("Wi-Fi Connection");
    open(&mut h, "VPN Connection");
    assert!(shows(&h, "utun3"));
    assert!(shows(&h, "utun4"));
}

#[test]
fn battery_sidebar_shows_design_capacity() {
    let s = State {
        battery: Some(resources_mac::collect::battery::BatteryStat {
            percent: 76.0,
            state: "Charging".into(),
            design_wh: Some(59.0),
            power_w: 40.9,
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut h = harness(s);
    open(&mut h, "59 Wh Battery");
    assert!(shows(&h, "76 % (Charging)"));
    assert!(shows(&h, "40.9 W"));
}

#[test]
fn processes_page_has_gnome_columns() {
    let mut h = harness(State::default());
    open(&mut h, "Processes");
    for col in ["Process", "Process ID", "User", "Memory", "Processor"] {
        assert!(h.query_all_by_label(col).next().is_some(), "missing {col}");
    }
}

#[test]
fn about_dialog_shows_version_and_license() {
    let mut h = harness(State::default());
    h.get_by_label("Main Menu").click();
    h.run();
    h.get_by_label("About Resources").click();
    h.run();
    assert!(shows(&h, env!("CARGO_PKG_VERSION")));
    assert!(shows(&h, "Apache-2.0"));
}

fn notes_with_helper() -> State {
    let mut helper = proc(8, "Notes Helper");
    helper.ppid = Some(7);
    State {
        procs: vec![proc(7, "Notes"), helper],
        apps: vec![resources_mac::sample::AppUsage {
            app: resources_mac::sample::AppInfo {
                pid: 7,
                name: "Notes".into(),
                bundle_id: None,
                icon: None,
            },
            cpu: 2.0,
            mem: 2 << 20,
            pids: vec![7, 8],
        }],
        ..Default::default()
    }
}

#[test]
fn overview_is_the_main_page_and_ends_apps() {
    let mut h = harness(notes_with_helper());
    h.get_by_role_and_label(egui::accesskit::Role::Canvas, "Overview");
    h.get_by_label("Notes").click();
    h.run();
    h.get_by_label("End App").click();
    h.run();
    assert!(shows(&h, "End \"Notes\""));
}

#[test]
fn double_clicking_a_bubble_shows_its_process_tree() {
    let mut h = harness(notes_with_helper());
    // Let the bubble settle: a double-click needs both clicks in one spot.
    for _ in 0..200 {
        h.step();
    }
    // Kittest delivers each click in its own 0.25 s frame; widen the double-click window.
    h.ctx
        .options_mut(|o| o.input_options.max_double_click_delay = 2.0);
    let bubble = h.get_by_label("Notes");
    bubble.click();
    bubble.click();
    h.run();
    h.get_by_label("Notes Helper");
    h.get_by_label("Back to Overview").click();
    h.run();
    assert!(h.query_by_label("Notes Helper").is_none());
}

/// Settings lives in the ☰ Main Menu, not the sidebar.
fn open_settings(h: &mut Harness<'static, App>) {
    h.get_by_label("Main Menu").click();
    h.run();
    h.get_by_label("Settings").click();
    h.run();
}

#[test]
fn settings_open_from_main_menu_not_sidebar() {
    let mut h = harness(State::default());
    assert!(h.query_by_label("Settings").is_none());
    open_settings(&mut h);
    h.get_by_label("Show stats in the menu bar");
}

#[test]
fn appearance_chips_switch_theme() {
    let mut h = harness(State::default());
    open_settings(&mut h);
    h.get_by_label_contains("Dark").click();
    h.run();
    assert_eq!(h.ctx.theme(), egui::Theme::Dark);
    h.get_by_label_contains("Light").click();
    h.run();
    assert_eq!(h.ctx.theme(), egui::Theme::Light);
}

#[test]
fn settings_choose_menu_bar_stats_with_preview() {
    let mut h = harness(State::default());
    open_settings(&mut h);
    h.get_by_label("Show stats in the menu bar");
    assert!(shows(
        &h,
        &format!("{} 0%   {} 0%", icons::CPU, icons::MEMORY)
    ));
    // First Swap chip is the menu bar's; the second belongs to the widget.
    h.query_all_by_label_contains("Swap")
        .next()
        .unwrap()
        .click();
    h.run();
    assert!(shows(
        &h,
        &format!("{} 0%   {} 0 B", icons::MEMORY, icons::MEMORY)
    ));
}
