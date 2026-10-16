//! Renders every page offscreen with sample data for visual review.
//! Run: `cargo test --test screenshots -- --ignored`; images land in `target/screenshots/`.

use std::sync::{Arc, Mutex};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use objc2_app_kit::NSWorkspace;
use objc2_foundation::NSString;
use resources_mac::app::App;
use resources_mac::collect::apps::render_icon;
use resources_mac::collect::battery::BatteryStat;
use resources_mac::collect::gpu::GpuStat;
use resources_mac::sample::{AppInfo, History, ProcInfo, system_processes};
use resources_mac::sampler::{DiskStat, NetStat, State};

/// A plausible 60 s trace: a slow wave with a few spikes.
fn trace(base: f64, amp: f64, seed: u32) -> History {
    let mut h = History::default();
    for i in 0..60u32 {
        let t = f64::from(i + seed);
        let spike = if (i * 7 + seed).is_multiple_of(23) {
            amp * 1.5
        } else {
            0.0
        };
        h.push((base + amp * (t / 5.0).sin().abs() + spike).max(0.0));
    }
    h
}

fn icon(path: &str) -> Option<Arc<egui::ColorImage>> {
    objc2::rc::autoreleasepool(|_| {
        let img = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(path));
        render_icon(&img, 64).map(Arc::new)
    })
}

fn sample_state() -> State {
    let procs: Vec<ProcInfo> = [
        (1, "launchd", "root", 0.1, 30),
        (412, "WindowServer", "_windowserver", 7.2, 410),
        (900, "Safari", "me", 3.4, 820),
        (901, "Safari Web Content", "me", 5.1, 640),
        (950, "Calculator", "me", 0.2, 45),
        (960, "Notes", "me", 1.1, 210),
        (970, "kernel_task", "root", 4.0, 1200),
    ]
    .into_iter()
    .map(|(pid, name, user, cpu, mb)| ProcInfo {
        pid,
        ppid: Some(if pid == 901 { 900 } else { 1 }),
        name: name.into(),
        user: user.into(),
        cpu,
        mem: mb * 1_000_000,
        threads: None,
    })
    .collect();
    let app = |pid, name: &str, path: &str| AppInfo {
        pid,
        name: name.into(),
        bundle_id: None,
        icon: icon(path),
    };
    let apps_info = [
        app(900, "Safari", "/Applications/Safari.app"),
        app(950, "Calculator", "/System/Applications/Calculator.app"),
        app(960, "Notes", "/System/Applications/Notes.app"),
    ];
    let mut apps = resources_mac::sample::aggregate_apps(&apps_info, &procs);
    apps.push(system_processes(&apps, &procs));

    State {
        cpu_model: "Apple M3 Pro".into(),
        cpu_total: trace(18.0, 20.0, 0),
        cpu_cores: (0..12).map(|i| trace(5.0, 30.0, i * 5)).collect(),
        mem_total: 36_000_000_000,
        mem_used: 21_500_000_000,
        swap_total: 2_000_000_000,
        swap_used: 400_000_000,
        mem_hist: trace(58.0, 4.0, 3),
        swap_hist: trace(20.0, 1.0, 9),
        disks: vec![DiskStat {
            name: "Macintosh HD".into(),
            mount: "/".into(),
            total: 994_662_584_320,
            avail: 412_000_000_000,
            read_total: 2_940_000_000_000,
            write_total: 2_930_000_000_000,
            read_bps: 19_490.0,
            write_bps: 38_970.0,
            read_hist: trace(0.0, 4e6, 2),
            write_hist: trace(0.0, 2e6, 11),
            ..Default::default()
        }],
        nets: vec![NetStat {
            name: "en0".into(),
            port: "Wi-Fi".into(),
            rx_bps: 120_000.0,
            tx_bps: 18_000.0,
            rx_total: 8_400_000_000,
            tx_total: 950_000_000,
            rx_hist: trace(0.0, 3e5, 4),
            tx_hist: trace(0.0, 6e4, 13),
        }],
        procs,
        apps,
        gpu: Some(GpuStat {
            name: "Apple M3 Pro".into(),
            util_hist: trace(4.0, 12.0, 6),
            mem_used: Some(1_800_000_000),
        }),
        battery: Some(BatteryStat {
            percent: 81.0,
            health: 97.0,
            cycles: Some(123),
            state: "Charging".into(),
            charge_hist: trace(78.0, 2.0, 1),
        }),
    }
}

fn shoot(theme: egui::Theme, name: &str) {
    let state = Arc::new(Mutex::new(sample_state()));
    let mut h = Harness::builder()
        .with_size([1000.0, 680.0])
        .with_pixels_per_point(2.0)
        .with_theme(theme)
        .with_context_setup(move |ctx| {
            resources_mac::theme::apply(ctx);
            ctx.set_theme(theme);
        })
        .wgpu()
        .build_ui_state(|ui, app: &mut App| app.show(ui), App::new(state));
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/screenshots");
    std::fs::create_dir_all(&dir).unwrap();
    for page in [
        "Apps",
        "Processes",
        "Processor",
        "Memory",
        "995 GB Drive",
        "Wi-Fi Connection",
    ] {
        h.query_all_by_label(page).next().expect(page).click();
        h.run();
        let img = h.render().expect("render");
        let file = format!("{name}-{}.png", page.to_lowercase().replace(' ', "-"));
        img.save(dir.join(file)).unwrap();
    }
}

#[test]
#[ignore = "writes PNGs for manual review"]
fn screenshots_light() {
    shoot(egui::Theme::Light, "light");
}

#[test]
#[ignore = "writes PNGs for manual review"]
fn screenshots_dark() {
    shoot(egui::Theme::Dark, "dark");
}
