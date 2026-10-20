//! The folder page's Trash flow, driven through its bottom bar.

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;

use disk_sweep::pages::folder::{FolderView, bar};
use disk_sweep::scan::{Progress, scan_root};
use disk_sweep::suggest::suggestions;
use disk_sweep::worker::RootScan;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

struct Page {
    view: FolderView,
    scan: RootScan,
    /// Path lists the bar asked to trash, in order.
    trashed: Vec<Vec<PathBuf>>,
}

/// A real folder with a Node project: `node_modules` (a cache) and `.idea` (to review).
fn page(dir: &tempfile::TempDir) -> (Harness<'static, Page>, PathBuf, PathBuf) {
    let proj = dir.path().join("proj");
    fs::create_dir_all(proj.join("node_modules/a")).unwrap();
    fs::create_dir_all(proj.join(".idea")).unwrap();
    fs::write(proj.join("package.json"), "{}").unwrap();
    fs::write(proj.join("node_modules/a/i.js"), "x").unwrap();
    let node = scan_root(dir.path(), &Progress::default()).unwrap();
    let mut scan = RootScan::new(dir.path().to_path_buf());
    scan.scanning = false;
    scan.generation = 1;
    scan.suggestions = suggestions(&node);
    scan.result = Some(Ok(node));
    let mut state = Page {
        view: FolderView::default(),
        scan,
        trashed: Vec::new(),
    };
    state.view.sync(&state.scan);
    let h = Harness::builder()
        .with_size([800.0, 600.0])
        .with_context_setup(resources_ui::theme::apply)
        .build_ui_state(
            |ui, p: &mut Page| {
                if let Some(paths) = bar(ui, &mut p.view, &p.scan) {
                    p.trashed.push(paths);
                }
            },
            state,
        );
    (h, proj.join("node_modules"), proj.join(".idea"))
}

fn shows(h: &Harness<'static, Page>, text: &str) -> bool {
    h.query_all_by_label_contains(text).next().is_some()
}

/// The bar's button opens the dialog; the dialog's own button (drawn later) confirms.
fn click_move(h: &mut Harness<'static, Page>, nth: usize) {
    h.query_all_by_label("Move to Trash")
        .nth(nth)
        .unwrap()
        .click();
    h.run();
}

#[test]
fn confirm_trashes_the_listed_paths_even_if_selection_changes() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, node_modules, idea) = page(&dir);
    click_move(&mut h, 0);
    assert!(shows(&h, "proj/node_modules"));
    // A rescan finishing behind the open dialog resets the selection.
    h.state_mut().view.selected = [idea].into();
    h.run();
    click_move(&mut h, 1);
    assert_eq!(h.state().trashed, [vec![node_modules]]);
}

#[test]
fn failed_trash_keeps_the_dialog_with_the_error_and_offers_a_retry() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, node_modules, _) = page(&dir);
    click_move(&mut h, 0);
    click_move(&mut h, 1);
    let (tx, rx) = mpsc::channel();
    tx.send(Err(format!("{}: not permitted", node_modules.display())))
        .unwrap();
    h.state_mut().view.pending = Some(rx);
    h.run();
    assert!(shows(&h, "not permitted"), "error shown in the dialog");
    assert!(shows(&h, "Try again"), "dialog offers a retry");
    click_move(&mut h, 1);
    assert_eq!(h.state().trashed.len(), 2);
    assert_eq!(h.state().trashed[1], [node_modules]);
}

#[test]
fn failed_trash_of_vanished_items_closes_the_dialog_and_keeps_the_error() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, node_modules, _) = page(&dir);
    click_move(&mut h, 0);
    click_move(&mut h, 1);
    fs::remove_dir_all(&node_modules).unwrap();
    let (tx, rx) = mpsc::channel();
    tx.send(Err(format!("{}: no longer exists", node_modules.display())))
        .unwrap();
    h.state_mut().view.pending = Some(rx);
    h.run();
    assert!(!shows(&h, "Try again"), "nothing left to retry");
    assert!(shows(&h, "no longer exists"), "error still visible");
}
