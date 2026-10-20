//! The Drives page's link to Disk Sweep, drawn on its own (clicking never launches anything).

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use resources_mac::pages::drives;
use resources_mac::sampler::DiskStat;

struct Page {
    /// `None`: Disk Sweep not found. `Some(err)`: found, with the last launch error.
    free_up: Option<Option<String>>,
    clicks: usize,
}

fn harness(free_up: Option<Option<String>>) -> Harness<'static, Page> {
    let disk = DiskStat {
        name: "Macintosh HD".into(),
        mount: "/".into(),
        total: 500 << 30,
        avail: 20 << 30,
        ..Default::default()
    };
    Harness::builder()
        .with_size([800.0, 1600.0])
        .with_context_setup(resources_mac::theme::apply)
        .build_ui_state(
            move |ui, p: &mut Page| {
                let free_up = p.free_up.as_ref().map(|e| e.as_deref());
                if drives::show(ui, &disk, free_up) {
                    p.clicks += 1;
                }
            },
            Page { free_up, clicks: 0 },
        )
}

fn shows(h: &Harness<'static, Page>, text: &str) -> bool {
    h.query_all_by_label_contains(text).next().is_some()
}

#[test]
fn free_up_hidden_when_disk_sweep_missing() {
    let h = harness(None);
    assert!(shows(&h, "Macintosh HD"));
    assert!(!shows(&h, "Free Up Space"));
}

#[test]
fn free_up_click_is_reported() {
    let mut h = harness(Some(None));
    h.get_by_label_contains("Free Up Space…").click();
    h.run();
    assert_eq!(h.state().clicks, 1);
}

#[test]
fn free_up_shows_launch_error() {
    let h = harness(Some(Some("No such file or directory".into())));
    assert!(shows(&h, "No such file or directory"));
}
