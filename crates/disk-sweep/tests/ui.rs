use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use disk_sweep::app::App;
use disk_sweep::classify::DETECTORS;
use disk_sweep::pages::settings::Settings;
use disk_sweep::scan::{Kind, Node};
use disk_sweep::suggest::suggestions;
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

fn node(path: &str, kind: Kind, bytes: u64, children: Vec<Node>) -> Node {
    let mut by_kind = [0; 5];
    if children.is_empty() {
        by_kind[kind.index()] = bytes;
    } else {
        for c in &children {
            for (s, b) in by_kind.iter_mut().zip(c.by_kind) {
                *s += b;
            }
        }
    }
    Node {
        name: path.rsplit('/').next().unwrap().into(),
        path: PathBuf::from(path),
        kind,
        is_dir: true,
        bytes,
        files: 3,
        by_kind,
        detector: (kind == Kind::Cache).then(|| &DETECTORS[0]),
        children,
    }
}

/// A scanned folder holding one project with 5 GB of `node_modules` and an `.idea` folder.
fn scanned(root: &str) -> RootScan {
    let tree = node(
        root,
        Kind::Source,
        5_000_010_000,
        vec![node(
            &format!("{root}/proj"),
            Kind::Source,
            5_000_010_000,
            vec![
                node(
                    &format!("{root}/proj/node_modules"),
                    Kind::Cache,
                    5_000_000_000,
                    vec![],
                ),
                node(&format!("{root}/proj/.idea"), Kind::Editor, 10_000, vec![]),
            ],
        )],
    );
    let mut scan = RootScan::new(PathBuf::from(root));
    scan.scanning = false;
    scan.generation = 1;
    scan.suggestions = suggestions(&tree);
    scan.result = Some(Ok(tree));
    scan
}

fn with_folder(root: &str) -> Harness<'static, App> {
    let state = State {
        roots: vec![scanned(root)],
        ..State::default()
    };
    let mut h = harness(state, Settings::default());
    open(&mut h, root.rsplit('/').next().unwrap());
    h
}

#[test]
fn folder_page_lists_suggestions() {
    let h = with_folder("/tmp/sweep-ws");
    for text in [
        "proj/node_modules",
        "5.00 GB",
        "Node packages",
        "npm/pnpm/yarn install",
        "proj/.idea",
        "Editor / OS metadata",
    ] {
        assert!(shows(&h, text), "missing {text}");
    }
}

#[test]
fn trash_asks_for_confirmation_first() {
    let mut h = with_folder("/tmp/sweep-ws2");
    assert!(!shows(&h, "Move 1 item"));
    h.query_all_by_label("Move to Trash")
        .next()
        .unwrap()
        .click();
    h.run();
    // Only the cache is checked by default; the editor folder waits for review.
    assert!(shows(&h, "Move 1 item (5.00 GB) to the Trash?"));
    h.get_by_label("Cancel").click();
    h.run();
    assert!(!shows(&h, "Move 1 item"));
}

#[test]
fn overview_shows_totals() {
    let state = State {
        roots: vec![scanned("/tmp/sweep-o1"), scanned("/tmp/sweep-o2")],
        ..State::default()
    };
    let h = harness(state, Settings::default());
    // Overview is the first page; two folders with 5 GB of caches each.
    assert!(shows(&h, "Reclaimable"));
    assert!(shows(&h, "10.00 GB"));
    assert!(shows(&h, "sweep-o1"));
    assert!(shows(&h, "Top suggestions"));
}

#[test]
fn overview_empty_state_links_to_settings() {
    let mut h = harness(State::default(), Settings::default());
    assert!(shows(&h, "Add a folder in Settings to start"));
    h.get_by_label("Open Settings").click();
    h.run();
    assert!(shows(&h, "Add Folder"));
}
