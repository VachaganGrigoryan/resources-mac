# Disk Sweep (egui disk clean-up app) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A second egui app in the `resources-mac` repo that scans chosen folders (e.g. `~/WS`), shows where the bytes are, tells caches/build output/editor config/gitignored files apart from project files, suggests what to move to the Trash, and does the same for Docker (images, containers, volumes, build cache, Docker Desktop VM disk).

**Architecture:** New workspace member `crates/disk-sweep` depending on the existing `resources-mac` lib for theme, Phosphor icons, widgets (`sidebar_item`, `header_bar`, `card`, `destructive_button`, `fmt_bytes`) and `table::Confirm`. A background thread walks each root with `std::fs`, classifies entries with a detector table + a `.gitignore` matcher stack (`ignore::gitignore`), and publishes a size tree + suggestions into `Arc<Mutex<State>>`; the UI mirrors `resources-mac` (left sidebar of pages, header bar, page column). Docker data comes from `docker system df -v --format '{{json .}}'`; actions are `trash::delete` and `docker … prune -f`, both behind the existing `Confirm` modal.

**Tech Stack:** Rust stable (edition 2024), eframe/egui/egui_extras 0.37, `ignore` 0.4 (gitignore matching only), `trash` 5, `rfd` 0.17 (folder picker), serde/serde_json, egui_kittest 0.37 + tempfile for tests.

**Spec:** the Context section below (the user skipped brainstorming questions; design calls are marked "my call"). Task 1 saves this file into the repo as `docs/superpowers/plans/2026-10-20-disk-sweep.md`.

## Context

**Why:** finding what is safe to delete in a workspace of ~20 projects means running `du`, guessing which `dist/` is build output and which is source, and remembering `docker system prune`. The user wants one window that answers "where is the space, how much can I free, and what do I lose if I do".

**Goals**
- Pick root folders; scan them in the background with a visible progress count.
- Classify every entry into one of five kinds: **Project files**, **Caches & builds**, **Editor & OS metadata** (`.idea`, `.vscode`, `.DS_Store`…), **Other ignored** (gitignored but not a known cache), **Git history** (`.git`).
- Human-readable stats: totals per kind per root, stacked bars, a treemap drill-down, and a ranked suggestion list with "how to get it back" (`pnpm install`, `cargo build`…).
- Move selected suggestions to the Trash (reversible), then rescan.
- Docker page: per-category size vs reclaimable, lists of images/containers/volumes/build cache, prune buttons with confirmation, Docker Desktop VM disk real-vs-apparent size.

**Non-goals (v1)**: global toolchain caches (`~/.npm`, `~/.cargo/registry`, `~/.gradle`, `~/.cache/uv` — together ~45 GB on this Mac, a natural next page), parallel scanning, permanent delete, app bundle/DMG for the new binary, Linux/Windows.

**Anti-goals**: never delete without a confirm that shows path count and size; never suggest `build/`, `dist/`, `out/`, `vendor/`, `tmp/`, `.cache/` unless `.gitignore` says they are ignored (generic names collide with sources); never follow symlinks; never descend into a nested git repo with the parent's ignore rules.

**Decisions (my calls)**: crate/binary name `disk-sweep`, window title "Disk Sweep"; decimal units via `resources_mac::widgets::fmt_bytes`; allocated size (`blocks*512`) rather than apparent; suggestions default-checked only for Caches & builds, Editor/Other-ignored listed unchecked under "Review"; Settings reachable from the sidebar (no popup menu).

## Global Constraints

- Repo: `/Users/vachagan/WS/Projects/resources-mac`, becomes a Cargo workspace; `site/` stays an independent crate (`exclude = ["site"]`).
- Toolchain: `rust-toolchain.toml` channel `stable`; edition `2024`; license `Apache-2.0`; every new dependency's license must be in `deny.toml` `[licenses].allow` (ignore: Unlicense OR MIT; trash: MIT; rfd: MIT; tempfile: MIT OR Apache-2.0).
- Gate (CI `.github/workflows/ci.yml`): `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`.
- Commits: sign off (`git commit -s`), end with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.
- Reuse, don't copy: `resources_mac::{theme, icons, widgets, table::Confirm}`. New Phosphor glyphs go into `resources-mac/src/icons.rs`.
- Shared-state pattern: `Arc<Mutex<State>>` + `ctx.request_repaint()` from the worker, as in `resources-mac/src/sampler.rs`.

## Review Focus

1. Unreadable directory (permission denied) inside a root → skipped, counted in `Progress.errors`, scan still completes. Test in Task 2.
2. Symlink pointing at an ancestor → never followed; file count stays finite. Test in Task 2.
3. Docker CLI missing or daemon down → `docker::df()` returns `Err(message)`; page shows the message, no panic. Test in Task 4.
4. Root folder removed/unmounted since last run → `scan_root` returns `Err`; sidebar entry shows the error. Test in Task 2 and Task 6.
5. One of the selected paths vanished before Trash → the others still move; the error names the missing path and stays in the modal. Test in Task 7.

---

### Task 1: Workspace + empty app window

**Files:**
- Modify: `Cargo.toml` (add `[workspace]`), `.github/workflows/ci.yml`, `README.md`
- Create: `crates/disk-sweep/Cargo.toml`, `crates/disk-sweep/src/main.rs`, `crates/disk-sweep/src/lib.rs`
- Create: `docs/superpowers/plans/2026-10-20-disk-sweep.md` (copy of this plan)

**Interfaces:**
- Produces: crate `disk-sweep` (lib name `disk_sweep`) with dependency `resources-mac = { path = "../.." }`, `eframe`/`egui`/`egui_extras` 0.37 (`persistence` feature), `serde` derive, `serde_json` 1, `ignore` 0.4, `trash` 5, `rfd` 0.17; dev-deps `egui_kittest` 0.37 (`wgpu`, `snapshot` not needed — only default + `wgpu` as resources-mac uses), `tempfile` 3.

- [ ] **Step 1: Make the root a workspace**

In root `Cargo.toml` add
```toml
[workspace]
members = ["crates/disk-sweep"]
exclude = ["site"]
```
and keep `[package]` as is. Create `crates/disk-sweep/Cargo.toml` with the deps above, `[[bin]] name = "disk-sweep"`, and the same `[profile.release]` is inherited from the root (profiles live only in the workspace root — leave the existing one).

- [ ] **Step 2: Minimal binary**

`src/lib.rs`: `//! Disk Sweep: …` + `pub const APP_NAME: &str = "Disk Sweep";`.
`src/main.rs`: copy `resources-mac/src/main.rs` options (window 1000×680, min 720×480, fullsize content view, LowPower wgpu), call `resources_mac::theme::apply(&cc.egui_ctx)`, and run an `eframe::App` that draws `resources_mac::widgets::header_bar(ui, disk_sweep::APP_NAME, None)` inside a `CentralPanel`. Reuse `resources_mac::APP_ICON_PNG` for the window icon for now.

- [ ] **Step 3: Verify**

Run: `cargo run -p disk-sweep` → a window titled "Disk Sweep" opens with the header. Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace && (cd site && cargo check)` → all pass (site still builds standalone thanks to `exclude`).

- [ ] **Step 4: CI, README, plan copy**

`ci.yml` `check` job: add `--workspace` to the clippy and test lines. README: a short "Disk Sweep" section (what it does, `cargo run -p disk-sweep`). Copy this plan to `docs/superpowers/plans/2026-10-20-disk-sweep.md`.

- [ ] **Step 5: Commit**

`git add -A Cargo.toml Cargo.lock crates .github README.md docs/superpowers && git commit -s -m "feat(disk-sweep): workspace and empty app window"` (+ trailer).

---

### Task 2: Scan + classify

**Files:**
- Create: `crates/disk-sweep/src/scan.rs`, `crates/disk-sweep/src/classify.rs`
- Modify: `crates/disk-sweep/src/lib.rs` (`pub mod scan; pub mod classify;`)
- Test: `crates/disk-sweep/tests/scan.rs`

**Interfaces:**
- Produces (`scan.rs`):
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
  pub enum Kind { Source, Cache, Editor, Ignored, Git }
  impl Kind {
      pub const ALL: [Kind; 5];
      pub fn label(self) -> &'static str; // "Project files", "Caches & builds", "Editor & OS metadata", "Other ignored", "Git history"
      pub fn index(self) -> usize;
  }
  pub struct Node {
      pub name: String, pub path: PathBuf, pub kind: Kind, pub is_dir: bool,
      pub bytes: u64, pub files: u64,
      pub by_kind: [u64; 5],            // bytes of the subtree per kind
      pub children: Vec<Node>,          // sorted by bytes desc; empty unless kind == Source && is_dir
      pub detector: Option<&'static classify::Detector>,
  }
  #[derive(Default)]
  pub struct Progress { pub files: AtomicU64, pub bytes: AtomicU64, pub errors: AtomicU64 }
  pub fn scan_root(root: &Path, progress: &Progress) -> std::io::Result<Node>;
  pub fn find<'a>(node: &'a Node, name: &str) -> Option<&'a Node>; // depth-first by file name
  ```
- Produces (`classify.rs`):
  ```rust
  pub enum Marker { Always, Sibling(&'static [&'static str]), Inside(&'static str), IfIgnored }
  pub struct Detector { pub name: &'static str, pub marker: Marker, pub label: &'static str, pub restore: &'static str }
  pub static DETECTORS: &[Detector];
  pub const EDITOR: &[&str]; // ".idea", ".vscode", ".vs", ".fleet", ".zed", ".DS_Store"; plus any "*.iml"
  pub struct IgnoreStack { /* Vec<ignore::gitignore::Gitignore> + repo-root boundaries */ }
  impl IgnoreStack {
      pub fn new() -> Self;
      pub fn enter(&mut self, dir: &Path) -> usize;      // pushes <dir>/.gitignore and, if <dir>/.git exists, a repo boundary + .git/info/exclude; returns a token for leave()
      pub fn leave(&mut self, token: usize);
      pub fn is_ignored(&self, path: &Path, is_dir: bool) -> bool; // deepest matcher wins, only matchers inside the current repo
  }
  pub fn classify(path: &Path, name: &str, is_dir: bool, ignored: bool) -> (Kind, Option<&'static Detector>);
  ```

- [ ] **Step 1: Write the failing tests** (`tests/scan.rs`, fixtures built in a `tempfile::tempdir()` with a small `touch(path, bytes)` helper that writes `bytes` zero bytes)

```rust
#[test] fn classifies_project_fixture() {
    // proj/package.json, proj/src/main.js (10 B), proj/node_modules/a/b.js (4096 B),
    // proj/.gitignore = "dist/\n*.log\n!keep.log\n", proj/dist/x (1 B), proj/notes.log, proj/keep.log,
    // proj/.idea/w.xml, proj/.git/HEAD, proj/build/keep.js (not ignored)
    let root = scan_root(dir.path(), &Progress::default()).unwrap();
    assert_eq!(find(&root, "node_modules").unwrap().kind, Kind::Cache);
    assert_eq!(find(&root, "node_modules").unwrap().detector.unwrap().restore, "npm/pnpm/yarn install");
    assert!(find(&root, "node_modules").unwrap().children.is_empty());
    assert_eq!(find(&root, "dist").unwrap().kind, Kind::Cache);     // generic name + ignored
    assert_eq!(find(&root, "build").unwrap().kind, Kind::Source);   // generic name, not ignored
    assert_eq!(find(&root, "notes.log").unwrap().kind, Kind::Ignored);
    assert_eq!(find(&root, "keep.log").unwrap().kind, Kind::Source); // negation
    assert_eq!(find(&root, ".idea").unwrap().kind, Kind::Editor);
    assert_eq!(find(&root, ".git").unwrap().kind, Kind::Git);
    assert_eq!(root.bytes, root.children.iter().map(|c| c.bytes).sum::<u64>() + own_dir_blocks /* allow: assert root.bytes >= sum */);
    assert_eq!(root.by_kind.iter().sum::<u64>(), root.bytes);
    assert!(root.by_kind[Kind::Cache.index()] >= 4096);
}
#[test] fn nested_gitignore_applies_only_below_its_dir()   // a/.gitignore="tmp/"; a/tmp/f → Ignored, b/tmp/f → Source
#[test] fn nested_repo_ignores_parent_rules()              // root .gitignore="secret/"; sub/.git/ + sub/secret/f → Source
#[test] fn venv_needs_pyvenv_cfg()                         // x/.venv/pyvenv.cfg → Cache; y/.venv/f → Source
#[test] fn unreadable_dir_is_skipped_and_counted()         // chmod 0o000 on proj/locked; scan Ok, progress.errors == 1 (skip when running as root)
#[test] fn symlink_to_ancestor_is_not_followed()           // proj/loop -> ..; root.files < 20
#[test] fn missing_root_is_an_error()                      // scan_root("/nonexistent/…") is Err
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p disk-sweep --test scan` → FAIL: unresolved imports `disk_sweep::scan`.

- [ ] **Step 3: Implement `classify.rs`**

`DETECTORS` (name, marker, label, restore):
`node_modules` Sibling(["package.json"]) "Node packages" "npm/pnpm/yarn install" · `target` Sibling(["Cargo.toml"]) "Cargo build output" "cargo build" · `.venv`/`venv` Inside("pyvenv.cfg") "Python virtualenv" "uv sync" · `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.tox`, `.nox` Always "Python cache" "re-created on next run" · `.next` Always "Next.js build" "next build" · `.turbo`, `.parcel-cache`, `.nuxt`, `.svelte-kit`, `.angular`, `.vite`, `storybook-static` Always "JS build cache" "rebuild" · `.gradle` Always, `build` Sibling(["build.gradle", "build.gradle.kts"]) "Gradle build" "gradle build" · `Pods` Sibling(["Podfile"]) "CocoaPods" "pod install" · `DerivedData` Always "Xcode DerivedData" "rebuild in Xcode" · `.build` Sibling(["Package.swift"]) "SwiftPM build" "swift build" · `.dart_tool` Always "Dart tool cache" "flutter pub get" · `dist`, `build`, `out`, `coverage`, `vendor`, `tmp`, `.tmp`, `.cache` IfIgnored "Build output / cache" "rebuild".
Order matters: specific `build` (Gradle sibling) before generic `build` (IfIgnored).
`classify`: `.git` (dir or file) → Git; name in `EDITOR` or ends with `.iml` → Editor; first detector whose `name` matches and whose marker holds → Cache; else `ignored` → Ignored; else Source.
`IgnoreStack`: `Vec<Gitignore>` + `Vec<usize>` of repo boundaries; `enter` builds `GitignoreBuilder::new(dir)` with `.add(dir/.gitignore)` (and a second matcher from `.git/info/exclude` when `.git` exists); `is_ignored` walks matchers from the top down to the current boundary and returns on the first `Match::Ignore` (true) or `Match::Whitelist` (false). Use `Gitignore::matched(path, is_dir)` (absolute paths under the matcher root are fine).

- [ ] **Step 4: Implement `scan.rs`**

Recursive `walk(dir, stack, progress, inherited: Option<Kind>) -> Node`: `read_dir`, `symlink_metadata` per entry (symlinks counted as their own blocks, never followed), `bytes = md.blocks() * 512` (`std::os::unix::fs::MetadataExt`), errors → `progress.errors += 1` and skip. For each entry: `ignored = stack.is_ignored(..)`, `(kind, det) = classify(..)`; if `kind != Source` or `inherited.is_some()`, recurse in summing mode (same function, `inherited = Some(kind)`, children dropped); otherwise keep the child. Sort children by `bytes` desc. `by_kind[kind]` for leaves; sum children for dirs; a Source dir's own blocks count as Source. `// shortcut: hardlinks are counted once per link; dedupe by inode when clones show up in the stats`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p disk-sweep --test scan` → PASS (7 tests).

- [ ] **Step 6: Commit**

`git commit -s -m "feat(disk-sweep): scan tree with gitignore-aware classification"`.

---

### Task 3: Suggestions

**Files:**
- Create: `crates/disk-sweep/src/suggest.rs`; register in `lib.rs`
- Test: unit tests in `suggest.rs` using a hand-built `Node`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Debug, PartialEq, Eq)]
  pub struct Suggestion { pub path: PathBuf, pub project: String, pub kind: Kind, pub label: &'static str, pub restore: Option<&'static str>, pub bytes: u64, pub files: u64 }
  pub fn suggestions(root: &Node) -> Vec<Suggestion>; // every Cache/Editor/Ignored node, bytes desc
  pub fn default_selected(s: &Suggestion) -> bool;     // s.kind == Kind::Cache
  ```
  `project` = first path component below the root (root's own name when the node sits directly under it). `label` = detector label, or "Editor / OS metadata", or "Ignored by .gitignore"; `restore` = detector restore for Cache, `None` otherwise.

- [ ] **Step 1: Write failing tests**
```rust
#[test] fn lists_non_source_nodes_sorted_by_size()  // tree: root/A/node_modules(5000, Cache) , root/B/.idea(10, Editor), root/A/notes.log(100, Ignored), root/A/src(…, Source) → 3 suggestions, order 5000,100,10, projects "A","A","B"
#[test] fn only_caches_are_preselected()
```
- [ ] **Step 2: Run** `cargo test -p disk-sweep suggest` → FAIL (module missing).
- [ ] **Step 3: Implement** — a depth-first collector over `children`; leaf kinds carry no children so the walk stops at them.
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(disk-sweep): clean-up suggestions from the scan tree`.

---

### Task 4: Docker usage and prune

**Files:**
- Create: `crates/disk-sweep/src/docker.rs`; register in `lib.rs`
- Test: unit tests in `docker.rs` with a JSON fixture string

**Interfaces:**
- Produces:
  ```rust
  pub struct Image { pub repository: String, pub tag: String, pub size: u64, pub unique_size: u64, pub containers: u32, pub created_since: String }
  pub struct Container { pub name: String, pub image: String, pub size: u64, pub status: String }
  pub struct Volume { pub name: String, pub size: u64, pub links: u32 }
  pub struct BuildCache { pub id: String, pub size: u64, pub in_use: bool, pub shared: bool, pub last_used: String }
  #[derive(Default)] pub struct Usage { pub images: Vec<Image>, pub containers: Vec<Container>, pub volumes: Vec<Volume>, pub build_cache: Vec<BuildCache> }
  #[derive(Clone, Copy, PartialEq, Eq, Debug)]
  pub enum Prune { DanglingImages, UnusedImages, StoppedContainers, UnusedVolumes, BuildCache }
  impl Prune { pub const ALL: [Prune; 5]; pub fn label(self) -> &'static str; pub fn args(self) -> &'static [&'static str]; }
  impl Usage { pub fn reclaimable(&self, p: Prune) -> (usize, u64); pub fn total(&self) -> u64; }
  pub fn parse_size(s: &str) -> Option<u64>;               // "6.911GB"→6_911_000_000, "10.94MB", "1.2kB"→1200, "0B"→0, "0B (virtual 161MB)"→0
  pub fn parse_df(json: &str) -> Result<Usage, String>;
  pub fn run_docker(program: &str, args: &[&str]) -> Result<String, String>; // stdout on success, trimmed stderr (or spawn error) on failure
  pub fn df() -> Result<Usage, String>;                     // run_docker("docker", ["system","df","-v","--format","{{json .}}"]) → parse_df
  pub fn prune(p: Prune) -> Result<String, String>;        // runs `docker <args>`; Ok(last stdout line, e.g. "Total reclaimed space: 1.2GB")
  pub struct VmDisk { pub path: PathBuf, pub allocated: u64, pub apparent: u64 }
  pub fn vm_disk() -> Option<VmDisk>;                      // ~/Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw, blocks*512 vs len
  ```
  `Prune::args`: `["image","prune","-f"]`, `["image","prune","-a","-f"]`, `["container","prune","-f"]`, `["volume","prune","-a","-f"]`, `["builder","prune","-a","-f"]`.
  `reclaimable`: DanglingImages = images with `tag == "<none>"` and `containers == 0` (sum `unique_size`); UnusedImages = `containers == 0`; StoppedContainers = status not starting with `"Up"`; UnusedVolumes = `links == 0`; BuildCache = `!in_use`.
  JSON fields (strings) observed from Docker 28: Images `Repository, Tag, Size, UniqueSize, Containers, CreatedSince`; Containers `Names, Image, Size, Status`; Volumes `Name, Size, Links`; BuildCache `ID, Size, InUse, Shared, LastUsedSince`.

- [ ] **Step 1: Write failing tests**
```rust
#[test] fn parses_docker_sizes()            // the five cases above + parse_size("garbage") == None
#[test] fn parses_system_df_verbose_json()  // fixture with 2 images (one <none>, Containers "0"), 1 container "Exited (0) 2 days ago", 1 volume Links "0"; assert counts and reclaimable(UnusedVolumes) == (1, size)
#[test] fn missing_docker_binary_is_an_error() // run_docker("definitely-not-docker-xyz", &["system","df"]) is Err — factor df()/prune() over `fn run_docker(program: &str, args: &[&str]) -> Result<String, String>`
```
- [ ] **Step 2: Run** `cargo test -p disk-sweep docker` → FAIL.
- [ ] **Step 3: Implement** with `std::process::Command` and `serde_json::Value` (or `#[derive(Deserialize)]` raw structs with `String` fields). Non-zero exit → `Err(stderr.trim())`.
- [ ] **Step 4: Run** → PASS. Also manually: `cargo test -p disk-sweep -- --ignored docker_live` optional `#[ignore]` test calling `df()` prints counts.
- [ ] **Step 5: Commit** `feat(disk-sweep): docker disk usage and prune commands`.

---

### Task 5: Treemap layout

**Files:**
- Create: `crates/disk-sweep/src/treemap.rs`; register in `lib.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn squarify(weights: &[f64], rect: egui::Rect) -> Vec<egui::Rect>; // Bruls et al. squarified treemap; weights ≥ 0, output same length/order
  pub fn show(ui: &mut egui::Ui, node: &Node, height: f32) -> Option<usize>; // draws node.children (top 40 by bytes, rest merged into one "… N more" tile), fill = kind colour, label when the tile fits, hover text "name · size · files"; returns the clicked child index (directories only)
  ```

- [ ] **Step 1: Failing test** `squarify_is_proportional_and_disjoint`: weights `[50, 25, 15, 10]` in a 200×100 rect → 4 rects, each area within 0.5 % of its share, all inside the rect, pairwise intersections have zero area. Plus `squarify_handles_zero_and_empty` (empty → empty; a zero weight → zero-area rect, no panic).
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement** `squarify` (sort indices by weight desc, lay rows along the shorter side, accept a row while worst aspect ratio improves). `show` uses `ui.allocate_exact_size`, `painter.rect_filled` with a 1 px gap, `Sense::click`, `on_hover_text`.
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(disk-sweep): squarified treemap`.

---

### Task 6: Worker, settings, app shell

**Files:**
- Create: `crates/disk-sweep/src/worker.rs`, `src/app.rs`, `src/widgets.rs`, `src/pages/mod.rs`, `src/pages/settings.rs`
- Modify: `src/main.rs` (build `App`, load/save `Settings`), `src/lib.rs`
- Modify: `resources-mac/src/icons.rs` (add glyphs)
- Test: `crates/disk-sweep/tests/ui.rs`

**Interfaces:**
- Produces (`worker.rs`):
  ```rust
  pub struct RootScan { pub root: PathBuf, pub progress: Arc<Progress>, pub scanning: bool, pub result: Option<Result<Node, String>>, pub suggestions: Vec<Suggestion> }
  #[derive(Default)] pub struct State { pub roots: Vec<RootScan>, pub docker: Option<Result<Usage, String>>, pub docker_vm: Option<VmDisk>, pub docker_busy: bool, pub log: Vec<String> }
  pub type Shared = Arc<Mutex<State>>;
  pub fn start_scan(shared: &Shared, root: PathBuf, ctx: egui::Context);   // adds/replaces the RootScan, spawns a thread, repaints every 250 ms while scanning and once at the end
  pub fn remove_root(shared: &Shared, root: &Path);
  pub fn refresh_docker(shared: &Shared, ctx: egui::Context);              // df() + vm_disk() on a thread
  pub fn trash_paths(paths: &[PathBuf]) -> Result<(), String>;             // per-path trash::delete; keeps going after a failure; Err lists the failed paths
  pub fn trash_then_rescan(shared: &Shared, root: PathBuf, paths: Vec<PathBuf>, ctx: egui::Context) -> std::sync::mpsc::Receiver<Result<(), String>>; // trash_paths on a thread, then start_scan(root)
  pub fn prune_then_refresh(shared: &Shared, p: Prune, ctx: egui::Context) -> std::sync::mpsc::Receiver<Result<String, String>>;
  ```
- Produces (`pages/settings.rs`): `#[derive(Serialize, Deserialize, Clone, PartialEq)] #[serde(default)] pub struct Settings { pub roots: Vec<PathBuf>, pub theme: ThemePreference }`, `Settings::KEY = "disk-sweep"`, `pub enum SettingsAction { Added(PathBuf), Removed(PathBuf) }`, `pub fn show(ui, &mut Settings) -> Option<SettingsAction>` ("Add folder…" via `rfd::FileDialog::new().pick_folder()`, a row per root with `icons::X` remove button, theme radio copied from `resources-mac/src/pages/settings.rs`).
- Produces (`widgets.rs`): `pub fn kind_color(kind: Kind) -> Color32` (Source = `theme::color::CPU`, Cache = `color::DRIVE`, Editor = `color::MEMORY`, Ignored = `Color32::GRAY`, Git = `color::BATTERY`); `pub fn stacked_bar(ui, by_kind: &[u64; 5], height: f32)`; `pub fn legend(ui, by_kind: &[u64; 5])` (dot, label, `fmt_bytes`); `pub fn stat_tile(ui, title: &str, value: &str, hint: &str)` (uses `resources_mac::widgets::card`).
- Produces (`app.rs`): `#[derive(Clone, PartialEq, Eq)] pub enum Page { Overview, Folder(PathBuf), Docker, Settings }`; `pub struct App { … }`; `App::new(shared: Shared) -> Self`, `with_settings(Settings)`, `pub fn show(&mut self, ui: &mut Ui)`; `impl eframe::App` (`update` → `show`; `save` → `eframe::set_value(storage, Settings::KEY, &self.settings)`). Sidebar: "Overview" (`icons::BROOM`), one `Folder` entry per root (`icons::FOLDER`, label = last path component, "…" suffix while scanning), "Docker" (`icons::SHIPPING_CONTAINER`), "Settings" (`icons::GEAR_SIX`). Copy the panel/scroll structure from `resources-mac/src/app.rs:224-300`. On startup `main.rs` calls `start_scan` for every saved root and `refresh_docker`.
- Produces (`resources-mac/src/icons.rs`): `FOLDER`, `BROOM`, `SHIPPING_CONTAINER`, `TRASH`, `ARROWS_CLOCKWISE`, `PLUS`, `X` — code points from `@phosphor-icons/web@2.1.2` `src/regular/style.css` (the vendored `assets/fonts/Phosphor.ttf` already contains them).

- [ ] **Step 1: Failing UI tests** (`tests/ui.rs`, harness like `resources-mac/tests/ui.rs` with `with_context_setup(resources_mac::theme::apply)`)
```rust
#[test] fn sidebar_lists_pages()             // "Overview", "Docker", "Settings" present
#[test] fn settings_lists_roots()            // Settings{roots:[/tmp/a]} → open "Settings" → label contains "a"
#[test] fn failed_root_shows_error()         // State with RootScan{result: Some(Err("not found"))} → open folder page → shows "not found"
```
- [ ] **Step 2: Run** `cargo test -p disk-sweep --test ui` → FAIL.
- [ ] **Step 3: Implement** worker, settings, widgets, app shell and the Settings page; wire `main.rs`. Settings `Added` → `start_scan`; `Removed` → `remove_root`; both persist via `save`.
- [ ] **Step 4: Run** tests → PASS; `cargo run -p disk-sweep` → add `~/WS` in Settings, sidebar shows the folder with "…" while scanning, then without.
- [ ] **Step 5: Commit** `feat(disk-sweep): background scanner, settings and app shell`.

---

### Task 7: Folder page (summary, treemap, suggestions, Trash)

**Files:**
- Create: `crates/disk-sweep/src/pages/folder.rs`
- Modify: `src/app.rs` (route `Page::Folder`), `tests/ui.rs`

**Interfaces:**
- Produces: `#[derive(Default)] pub struct FolderView { pub selected: HashSet<PathBuf>, pub zoom: Vec<usize>, pub confirm: Confirm<FolderAction>, pub pending: Option<Receiver<Result<(), String>>>, pub filter: String }`, `#[derive(Clone, Copy)] pub enum FolderAction { Trash }`, `pub fn show(ui, view: &mut FolderView, scan: &RootScan, shared: &Shared, ctx: &egui::Context)`.
- Layout: while scanning → progress line "Scanning… 123,456 files · 42.1 GB" (`fmt_bytes`); result → `stacked_bar` + `legend`, stat row (total / reclaimable = Cache bytes / to review = Editor+Ignored), treemap (`treemap::show`, height 260, breadcrumb from `zoom`, click a dir tile to push, breadcrumb click to pop), then an `egui_extras::TableBuilder` of suggestions: checkbox · project · path relative to root · kind label · `fmt_bytes` · restore hint; "Review" rows (Editor/Ignored) after a separator, unchecked by default (`default_selected` applied when a scan result arrives). Bottom `resources_mac::table::action_bar`-style bar: left "N items · X GB selected", right `destructive_button("Move to Trash")` → `confirm.ask(FolderAction::Trash, format!("Move {n} items ({size}) to the Trash?\n{first 5 paths}…"))`; on confirm → `trash_then_rescan`, store receiver in `pending`; poll each frame: `Ok` → `confirm.finish(Ok)`, clear selection; `Err(e)` → `confirm.finish(Err(e))`.

- [ ] **Step 1: Failing tests**
```rust
#[test] fn folder_page_lists_suggestions()        // State with a hand-built RootScan (node_modules 5 GB Cache, .idea Editor) → shows "node_modules", "5.00 GB", "Node packages", "npm/pnpm/yarn install"
#[test] fn trash_asks_for_confirmation_first()    // click "Move to Trash" → label containing "Move 1 item" visible; click "Cancel" → gone
#[test] fn trash_reports_missing_path()           // unit test in worker.rs: trash_paths(&[PathBuf::from("/nonexistent/zzz")]) → Err containing "/nonexistent/zzz"
```
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS; manual: select a small `__pycache__` in a scratch project, confirm, check it is in the Trash and the folder rescans.
- [ ] **Step 5: Commit** `feat(disk-sweep): folder page with treemap, suggestions and Trash`.

---

### Task 8: Overview page

**Files:**
- Create: `crates/disk-sweep/src/pages/overview.rs`; route in `app.rs`; tests in `tests/ui.rs`

**Interfaces:**
- Produces: `pub fn show(ui, s: &State) -> Option<Page>` — four `stat_tile`s ("Scanned", "Reclaimable" = Σ Cache, "To review" = Σ Editor+Ignored, "Docker reclaimable" = Σ `reclaimable(p)` over `Prune::ALL` minus double counting: use UnusedImages + StoppedContainers + UnusedVolumes + BuildCache); one row per root: name, `stacked_bar`, total, "Open" → `Some(Page::Folder(root))`; "Top suggestions": 10 largest across roots, clicking a row opens its folder page. Empty state when `roots` is empty: text "Add a folder in Settings to start" + button → `Some(Page::Settings)`.

- [ ] **Step 1: Failing tests** `overview_shows_totals` (two fake roots → "Reclaimable" tile shows the summed Cache bytes string), `overview_empty_state_links_to_settings`.
- [ ] **Step 2–4:** FAIL → implement → PASS.
- [ ] **Step 5: Commit** `feat(disk-sweep): overview page`.

---

### Task 9: Docker page

**Files:**
- Create: `crates/disk-sweep/src/pages/docker.rs`; route in `app.rs`; tests in `tests/ui.rs`

**Interfaces:**
- Produces: `#[derive(Default)] pub struct DockerView { pub confirm: Confirm<Prune>, pub pending: Option<Receiver<Result<String, String>>>, pub last: Option<String> }`, `pub fn show(ui, view: &mut DockerView, s: &State, shared: &Shared, ctx)`.
- Layout: `Err(msg)` → "Docker isn't available" card with `msg` and a Retry (`refresh_docker`); `None` → "Loading…"; `Ok(usage)` → stat tiles per category (size · reclaimable), VM disk card "Docker Desktop disk: 22.0 GB used of a 275 GB sparse image" when `docker_vm` is `Some` with the hint "space inside the VM is returned to macOS by Docker Desktop in the background after pruning"; then one collapsing section per `Prune::ALL` with the matching rows (images: repo:tag · size · containers · age; containers: name · image · status · size; volumes: name · links · size; build cache: id · in use · size · last used) and a `destructive_button(Prune::label)` enabled when `reclaimable(p).0 > 0` → `confirm.ask(p, "Remove N … (X)? This runs `docker …`.")` → `prune_then_refresh`; result line shown under the header (`view.last`). Header `icons::ARROWS_CLOCKWISE` button → `refresh_docker`.

- [ ] **Step 1: Failing tests** `docker_page_shows_reclaimable` (State.docker = Ok(fixture Usage) → shows "Unused volumes" count/size string), `docker_page_shows_error` (Err("Cannot connect to the Docker daemon") → text visible), `prune_asks_for_confirmation_first`.
- [ ] **Step 2–4:** FAIL → implement → PASS; manual: Retry with Docker Desktop stopped shows the daemon error; with it running, `docker container prune` on a stopped test container reports "Total reclaimed space".
- [ ] **Step 5: Commit** `feat(disk-sweep): docker page with prune actions`.

---

### Task 10: Gate and wrap-up

- [ ] **Step 1:** `cargo fmt --all && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace && (cd site && cargo test --locked)` → all green; `cargo deny check licenses` if `cargo-deny` is installed (CI runs it anyway).
- [ ] **Step 2:** Run `cargo run -p disk-sweep` end-to-end against `~/WS`: scan finishes, suggestions list matches a spot check with `du -sh <path>` for the top 3, Trash one small cache, Docker page numbers match `docker system df`.
- [ ] **Step 3:** README screenshots are optional (skipped). Commit any fixes: `chore(disk-sweep): gate fixes`.

## Verification (end-to-end)

1. `cargo test --workspace` — scan/classify (7), suggest (2), docker (3), treemap (2), worker (1), UI (≈10) all pass.
2. `cargo run -p disk-sweep` → Settings → Add folder `~/WS` → sidebar shows "WS…" then "WS"; Overview tiles populate; Folder page treemap zooms on click; select a `__pycache__`, "Move to Trash" → modal → confirm → item appears in the macOS Trash and the folder rescans with a smaller total.
3. Docker page totals equal `docker system df` output; with Docker Desktop quit, the page shows the daemon error and Retry recovers after relaunch.
4. `cd site && cargo check` still works (workspace exclude).
