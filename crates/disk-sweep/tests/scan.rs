use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::atomic::Ordering;

use disk_sweep::scan::{Kind, Progress, find, scan_root};

/// Writes `bytes` zero bytes at `path`, creating parent folders.
fn touch(path: &Path, bytes: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, vec![0u8; bytes]).unwrap();
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn kind_of(root: &disk_sweep::scan::Node, name: &str) -> Kind {
    find(root, name)
        .unwrap_or_else(|| panic!("{name} not in tree"))
        .kind
}

#[test]
fn classifies_project_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("proj");
    write(&p.join("package.json"), "{}");
    touch(&p.join("src/main.js"), 10);
    touch(&p.join("node_modules/a/b.js"), 4096);
    write(&p.join(".gitignore"), "dist/\n*.log\n!keep.log\n");
    touch(&p.join("dist/x"), 1);
    touch(&p.join("notes.log"), 1);
    touch(&p.join("keep.log"), 1);
    touch(&p.join(".idea/w.xml"), 1);
    touch(&p.join(".git/HEAD"), 1);
    touch(&p.join("build/keep.js"), 1);

    let root = scan_root(dir.path(), &Progress::default()).unwrap();

    let nm = find(&root, "node_modules").unwrap();
    assert_eq!(nm.kind, Kind::Cache);
    assert_eq!(nm.detector.unwrap().restore, "npm/pnpm/yarn install");
    assert!(nm.children.is_empty());
    assert!(nm.files >= 1);
    assert_eq!(kind_of(&root, "dist"), Kind::Cache);
    assert_eq!(kind_of(&root, "build"), Kind::Source);
    assert_eq!(kind_of(&root, "notes.log"), Kind::Ignored);
    assert_eq!(kind_of(&root, "keep.log"), Kind::Source);
    assert_eq!(kind_of(&root, ".idea"), Kind::Editor);
    assert_eq!(kind_of(&root, ".git"), Kind::Git);
    assert_eq!(kind_of(&root, "main.js"), Kind::Source);

    assert!(root.bytes >= root.children.iter().map(|c| c.bytes).sum::<u64>());
    assert_eq!(root.by_kind.iter().sum::<u64>(), root.bytes);
    assert!(root.by_kind[Kind::Cache.index()] >= 4096);
    let proj = find(&root, "proj").unwrap();
    assert!(
        proj.children.windows(2).all(|w| w[0].bytes >= w[1].bytes),
        "children sorted by size"
    );
}

#[test]
fn nested_gitignore_applies_only_below_its_dir() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("a/.gitignore"), "scratch/\n");
    touch(&dir.path().join("a/scratch/f"), 1);
    touch(&dir.path().join("b/scratch/f"), 1);
    let root = scan_root(dir.path(), &Progress::default()).unwrap();
    let a = find(&root, "a").unwrap();
    let b = find(&root, "b").unwrap();
    assert_eq!(find(a, "scratch").unwrap().kind, Kind::Ignored);
    assert_eq!(find(b, "scratch").unwrap().kind, Kind::Source);
}

#[test]
fn nested_repo_ignores_parent_rules() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join(".gitignore"), "secret/\n");
    touch(&dir.path().join("sub/.git/HEAD"), 1);
    touch(&dir.path().join("sub/secret/f"), 1);
    touch(&dir.path().join("secret/f"), 1);
    let root = scan_root(dir.path(), &Progress::default()).unwrap();
    let sub = find(&root, "sub").unwrap();
    assert_eq!(find(sub, "secret").unwrap().kind, Kind::Source);
    let top = root.children.iter().find(|c| c.name == "secret").unwrap();
    assert_eq!(top.kind, Kind::Ignored);
}

#[test]
fn venv_needs_pyvenv_cfg() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("x/.venv/pyvenv.cfg"), 1);
    touch(&dir.path().join("y/.venv/f"), 1);
    let root = scan_root(dir.path(), &Progress::default()).unwrap();
    assert_eq!(
        find(find(&root, "x").unwrap(), ".venv").unwrap().kind,
        Kind::Cache
    );
    assert_eq!(
        find(find(&root, "y").unwrap(), ".venv").unwrap().kind,
        Kind::Source
    );
}

#[test]
fn unreadable_dir_is_skipped_and_counted() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("proj/ok.txt"), 1);
    touch(&dir.path().join("proj/locked/f"), 1);
    let locked = dir.path().join("proj/locked");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    // Root can read anything; the case can't happen there.
    if fs::read_dir(&locked).is_ok() {
        return;
    }
    let progress = Progress::default();
    let result = scan_root(dir.path(), &progress);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    let root = result.unwrap();
    assert_eq!(progress.errors.load(Ordering::Relaxed), 1);
    assert!(find(&root, "ok.txt").is_some());
}

#[test]
fn symlink_to_ancestor_is_not_followed() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("proj/f"), 1);
    std::os::unix::fs::symlink("..", dir.path().join("proj/loop")).unwrap();
    let root = scan_root(dir.path(), &Progress::default()).unwrap();
    assert!(
        root.files < 20,
        "followed the symlink: {} files",
        root.files
    );
    assert!(!find(&root, "loop").unwrap().is_dir);
}

#[test]
fn missing_root_is_an_error() {
    assert!(
        scan_root(
            Path::new("/nonexistent/disk-sweep-test"),
            &Progress::default()
        )
        .is_err()
    );
}

/// Scans a real folder and prints what the app would suggest:
/// `DISK_SWEEP_ROOT=~/WS cargo test -p disk-sweep --release --test scan real_folder -- --ignored --nocapture`
#[test]
#[ignore]
fn real_folder() {
    let root = std::env::var("DISK_SWEEP_ROOT").expect("set DISK_SWEEP_ROOT");
    let progress = Progress::default();
    let start = std::time::Instant::now();
    let node = scan_root(Path::new(&root), &progress).unwrap();
    println!(
        "{} files, {} B in {:?}, {} unreadable",
        node.files,
        node.bytes,
        start.elapsed(),
        progress.errors.load(Ordering::Relaxed)
    );
    for k in Kind::ALL {
        println!("{:>22}: {} B", k.label(), node.by_kind[k.index()]);
    }
    for s in disk_sweep::suggest::suggestions(&node).iter().take(10) {
        println!(
            "{:>14} B  {:?}  {}  {}",
            s.bytes,
            s.kind,
            s.label,
            s.path.display()
        );
    }
}

#[test]
fn nested_repo_ignored_by_parent_is_walked_not_suggested() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("proj");
    write(&p.join(".gitignore"), "vendor/\nthird_party/\n");
    touch(&p.join("vendor/lib/.git/HEAD"), 1);
    touch(&p.join("vendor/lib/src.c"), 1);
    touch(&p.join("vendor/.git/HEAD"), 1);
    touch(&p.join("vendor/main.c"), 1);
    touch(&p.join("third_party/.git/HEAD"), 1);
    touch(&p.join("third_party/x.c"), 1);
    let root = scan_root(dir.path(), &Progress::default()).unwrap();
    // Clones with their own history may hold unpushed work: never a cache.
    assert_eq!(kind_of(&root, "vendor"), Kind::Source);
    assert_eq!(kind_of(&root, "third_party"), Kind::Source);
    assert_eq!(kind_of(&root, "main.c"), Kind::Source);
}

#[test]
fn hard_links_are_counted_once() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("a/big"), 64 * 1024);
    fs::create_dir_all(dir.path().join("b")).unwrap();
    fs::hard_link(dir.path().join("a/big"), dir.path().join("b/big")).unwrap();
    let root = scan_root(dir.path(), &Progress::default()).unwrap();
    let a = find(&root, "a").unwrap().bytes;
    let b = find(&root, "b").unwrap().bytes;
    assert!(a.max(b) >= 64 * 1024);
    assert!(
        a.min(b) < 64 * 1024,
        "second link counted again: a={a} b={b}"
    );
}

#[test]
fn hard_links_inside_a_cache_are_counted_once() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("proj");
    write(&p.join("Cargo.toml"), "");
    touch(&p.join("target/debug/deps/app"), 64 * 1024);
    fs::hard_link(p.join("target/debug/deps/app"), p.join("target/debug/app")).unwrap();
    let root = scan_root(dir.path(), &Progress::default()).unwrap();
    let target = find(&root, "target").unwrap().bytes;
    assert!(
        target < 2 * 64 * 1024,
        "target counted both links: {target}"
    );
}
