//! Renders every page offscreen with sample data for visual review.
//! Run: `cargo test --test screenshots -- --ignored`; images land in `docs/screenshots/`
//! (committed, shown in the README).

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

fn tunnel(name: &str, rx_total: u64, tx_total: u64) -> NetStat {
    NetStat {
        name: name.into(),
        rx_total,
        tx_total,
        ..Default::default()
    }
}

fn sample_state() -> State {
    let procs: Vec<ProcInfo> = [
        (1, 0, "launchd", "root", 0.1, 30),
        (412, 1, "WindowServer", "_windowserver", 7.2, 410),
        (900, 1, "Safari", "me", 3.4, 820),
        (901, 900, "Safari Web Content", "me", 5.1, 640),
        (902, 900, "Safari Web Content", "me", 2.3, 380),
        (903, 900, "Safari Networking", "me", 0.6, 90),
        (904, 900, "Safari Graphics", "me", 1.2, 160),
        (950, 1, "Calculator", "me", 0.2, 45),
        (960, 1, "Notes", "me", 1.1, 210),
        (970, 0, "kernel_task", "root", 4.0, 1200),
        (980, 1, "mds_stores", "root", 2.2, 160),
        (981, 1, "Spotlight", "me", 0.4, 120),
        (1000, 1, "Terminal", "me", 0.9, 150),
        (1001, 1000, "zsh", "me", 0.0, 8),
        (1002, 1001, "cargo", "me", 62.0, 900),
        (1003, 1002, "rustc", "me", 88.0, 1400),
        (1100, 1001, "TextEdit", "me", 0.3, 70),
        (1101, 1100, "TextEdit Helper", "me", 0.1, 30),
    ]
    .into_iter()
    .map(|(pid, ppid, name, user, cpu, mb)| ProcInfo {
        pid,
        ppid: (ppid != 0).then_some(ppid),
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
        app(
            1000,
            "Terminal",
            "/System/Applications/Utilities/Terminal.app",
        ),
        app(1100, "TextEdit", "/System/Applications/TextEdit.app"),
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
        nets: vec![
            NetStat {
                name: "en0".into(),
                port: "Wi-Fi".into(),
                rx_bps: 120_000.0,
                tx_bps: 18_000.0,
                rx_total: 8_400_000_000,
                tx_total: 950_000_000,
                rx_hist: trace(0.0, 3e5, 4),
                tx_hist: trace(0.0, 6e4, 13),
            },
            tunnel("utun3", 210_000_000, 48_000_000),
            tunnel("utun4", 2_400_000, 900_000),
        ],
        vpn: Some(NetStat {
            name: "VPN".into(),
            rx_bps: 3_400.0,
            tx_bps: 1_100.0,
            rx_total: 212_400_000,
            tx_total: 48_900_000,
            rx_hist: trace(0.0, 4e3, 7),
            tx_hist: trace(0.0, 1.5e3, 17),
            ..Default::default()
        }),
        vpn_members: vec!["utun3".into(), "utun4".into()],
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
            design_wh: Some(72.4),
            power_w: 18.6,
            model: Some("bq40z651".into()),
            charge_hist: trace(78.0, 2.0, 1),
            power_hist: trace(12.0, 8.0, 5),
        }),
        ticks: 60,
    }
}

fn shoot(theme: egui::Theme, name: &str) {
    let state = Arc::new(Mutex::new(sample_state()));
    let mut h = Harness::builder()
        .with_size([1000.0, 680.0])
        .with_pixels_per_point(2.0)
        .with_theme(theme)
        .with_render_cursor(false)
        .with_context_setup(move |ctx| {
            resources_mac::theme::apply(ctx);
            ctx.set_theme(theme);
        })
        .wgpu()
        .build_ui_state(|ui, app: &mut App| app.show(ui), App::new(state));
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/screenshots");
    std::fs::create_dir_all(&dir).unwrap();
    for (label, slug) in [
        ("Overview", "overview"),
        ("Apps", "apps"),
        ("Processes", "processes"),
        ("Processor", "processor"),
        ("Memory", "memory"),
        ("GPU", "gpu"),
        ("995 GB Drive", "drive"),
        ("Wi-Fi Connection", "network"),
        ("VPN Connection", "vpn"),
        ("72 Wh Battery", "battery"),
        ("Settings", "settings"),
    ] {
        // Settings sits in the Main Menu rather than the sidebar.
        if label == "Settings" {
            h.get_by_label("Main Menu").click();
            h.run_steps(4);
        }
        // Lower sidebar entries start scrolled out of the 680 px window.
        h.query_all_by_label(label)
            .next()
            .expect(label)
            .scroll_to_me();
        h.run_steps(4);
        h.query_all_by_label(label).next().expect(label).click();
        // The Overview's bubbles animate into place (about 30 frames).
        h.run_steps(60);
        let img = h.render().expect("render");
        img.save(dir.join(format!("{name}-{slug}.png"))).unwrap();
    }
}

#[test]
#[ignore = "rewrites docs/screenshots"]
fn screenshots_light() {
    shoot(egui::Theme::Light, "light");
}

#[test]
#[ignore = "rewrites docs/screenshots"]
fn screenshots_dark() {
    shoot(egui::Theme::Dark, "dark");
}

const FPS: f32 = 15.0;

/// Shift a full history by one sample so its graph scrolls in a seamless loop.
fn spin(h: &mut History) {
    let first = h.values().next();
    if let Some(v) = first {
        h.push(v);
    }
}

fn spin_all(s: &mut State) {
    spin(&mut s.cpu_total);
    s.cpu_cores.iter_mut().for_each(spin);
    spin(&mut s.mem_hist);
    spin(&mut s.swap_hist);
    for d in &mut s.disks {
        spin(&mut d.read_hist);
        spin(&mut d.write_hist);
    }
    for n in s.nets.iter_mut().chain(s.vpn.as_mut()) {
        spin(&mut n.rx_hist);
        spin(&mut n.tx_hist);
    }
    if let Some(g) = &mut s.gpu {
        spin(&mut g.util_hist);
    }
    if let Some(b) = &mut s.battery {
        spin(&mut b.charge_hist);
        spin(&mut b.power_hist);
    }
    s.ticks += 1;
}

/// Records a walkthrough as numbered PNG frames, with a gliding cursor and graphs that keep moving.
struct Demo {
    h: Harness<'static, App>,
    state: Arc<Mutex<State>>,
    dir: std::path::PathBuf,
    frame: usize,
    cursor: egui::Pos2,
}

impl Demo {
    fn step(&mut self) {
        // About 4x real time: a new sample every 4 frames.
        if self.frame.is_multiple_of(4) {
            spin_all(&mut self.state.lock().unwrap());
        }
        self.h.step();
        let img = self.h.render().expect("render");
        img.save(self.dir.join(format!("{:04}.png", self.frame)))
            .unwrap();
        self.frame += 1;
    }

    fn hold(&mut self, secs: f32) {
        for _ in 0..(secs * FPS) as usize {
            self.step();
        }
    }

    /// Glide to the first widget labelled `label` (in the sidebar if `side`) and click it.
    fn click(&mut self, label: &str, side: Option<bool>) {
        let to = self
            .h
            .query_all_by_label_contains(label)
            .map(|n| n.rect().center())
            .find(|p| side.is_none_or(|s| (p.x < 250.0) == s))
            .expect(label);
        let from = self.cursor;
        for i in 1..=10 {
            let t = i as f32 / 10.0;
            let pos = from.lerp(to, t * t * (3.0 - 2.0 * t));
            self.h.event(egui::Event::PointerMoved(pos));
            self.step();
        }
        for pressed in [true, false] {
            self.h.event(egui::Event::PointerButton {
                pos: to,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::default(),
            });
            self.step();
        }
        self.cursor = to;
    }
}

#[test]
#[ignore = "writes target/demo frames; encode with ffmpeg"]
fn demo_frames() {
    let state = Arc::new(Mutex::new(sample_state()));
    let h = Harness::builder()
        .with_size([1000.0, 680.0])
        .with_pixels_per_point(2.0)
        .with_theme(egui::Theme::Dark)
        .with_step_dt(1.0 / FPS)
        .with_context_setup(|ctx| {
            resources_mac::theme::apply(ctx);
            ctx.set_theme(egui::Theme::Dark);
        })
        .wgpu()
        .build_ui_state(|ui, app: &mut App| app.show(ui), App::new(state.clone()));
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/demo");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let cursor = egui::pos2(700.0, 450.0);
    h.event(egui::Event::PointerMoved(cursor));
    let mut d = Demo {
        h,
        state,
        dir,
        frame: 0,
        cursor,
    };
    d.hold(2.5);
    d.click("Processor", Some(false));
    d.hold(2.0);
    for page in ["Apps", "Processes", "Processor", "Memory"] {
        d.click(page, Some(true));
        d.hold(1.8);
    }
    d.click("Main Menu", None);
    d.hold(0.5);
    d.click("Settings", None);
    d.hold(0.8);
    d.click("Light", Some(false));
    d.hold(1.5);
    d.click("Overview", Some(true));
    d.hold(3.0);
}
