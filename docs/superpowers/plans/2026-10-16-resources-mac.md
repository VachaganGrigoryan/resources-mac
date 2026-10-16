# resources-mac Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A pure-Rust macOS system monitor inspired by GNOME Resources, built and released (unsigned dmg + Homebrew cask) by GitHub Actions.

**Architecture:** A single crate (lib + bin). A sampler thread refreshes `sysinfo` and the macOS collectors once per second into `Arc<Mutex<State>>`, then calls `ctx.request_repaint()`. The eframe/egui UI only reads `State` and draws a sidebar plus one page.

**Tech Stack:** Rust stable (1.99), eframe/egui/egui_plot/egui_extras 0.37, egui_kittest 0.37 (dev), sysinfo 0.39, starship-battery 0.12, io-kit-sys 0.5, core-foundation 0.10, objc2-app-kit/objc2-foundation 0.3, libc 0.2, anyhow 1.

**Spec:** `docs/superpowers/specs/2026-10-16-resources-mac-design.md`

## Global Constraints
- Name `resources-mac`, license GPL-3.0-or-later, credit GNOME Resources in the README.
- Minimum macOS 13 (`LSMinimumSystemVersion` 13.0, cask `depends_on macos: ">= :ventura"`).
- Release artifact is one universal (arm64 + x86_64) `.app`, ad-hoc signed (`codesign -s -`), inside `resources-mac-<version>.dmg`.
- Sample interval is 1 s and history is 60 points per series.
- The UI thread never calls sysinfo or IOKit.
- No privilege escalation. Failed kills show an error.
- Gate: `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test`.
- GitHub owner `VachaganGrigoryan`. The tap repo is `VachaganGrigoryan/homebrew-tap` and its secret is `HOMEBREW_TAP_TOKEN`.

## Review Focus
1. **Network counter reset** (an interface goes down and up, or the machine wakes from sleep): rates must read 0, not a huge spike. Task 2 `rate_counter_reset_is_zero`.
2. **Broken process tree** (a ppid cycle or a missing parent): aggregation must terminate and count each pid once. Task 2 `aggregate_handles_ppid_cycle`.
3. **Killing a process that already exited or belongs to root:** show an `Err` message, never panic. Task 6 `signal_dead_pid_errors`.
4. **Mac with no battery, or no IOAccelerator stats** (desktop Macs, Intel, CI VMs): the page is hidden and nothing crashes. Tasks 7/8 `read_*_does_not_panic` plus the hidden-page test in Task 8.
5. **Thousands of processes, mixed-case search:** filtering is case-insensitive and the table stays virtualized. Task 6 `filter_is_case_insensitive`.

---

### Task 1: Scaffold, window, CI

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml` (`channel = "stable"`), `.gitignore` (`/target`, `/dist`), `LICENSE` (GPL-3.0 text from https://www.gnu.org/licenses/gpl-3.0.txt), `README.md` (one paragraph + credit), `src/lib.rs`, `src/main.rs`, `.github/workflows/ci.yml`

**Interfaces:**
- Produces: `resources_mac` lib crate; bin `resources-mac`.

- [ ] **Step 1:** `Cargo.toml` with `edition = "2024"`, `license = "GPL-3.0-or-later"`, and the deps listed in Tech Stack. `[profile.release] lto = true, strip = true`.
- [ ] **Step 2:** `main.rs` opens a 1000×680 window titled "Resources" via `eframe::run_native` with an empty `CentralPanel`.
- [ ] **Step 3:** Run `cargo run`. Expected: the window opens and closes cleanly.
- [ ] **Step 4:** `ci.yml` runs on `push` to main and on `pull_request`, with `runs-on: macos-latest`. Steps: `actions/checkout@v4`, `dtolnay/rust-toolchain@stable` (components rustfmt, clippy), `Swatinem/rust-cache@v2`, then the gate from Global Constraints.
- [ ] **Step 5:** Run the gate locally. Expected: all pass.
- [ ] **Step 6:** Commit with message `chore: scaffold egui app and CI`.

### Task 2: Pure sampling math (`sample.rs`)

**Files:** Create `src/sample.rs` (tests inline `#[cfg(test)]`)

**Interfaces:**
- Produces:
  - `pub fn rate(prev: u64, cur: u64, elapsed: Duration) -> f64` returns bytes/s. It returns 0.0 when `cur < prev` or `elapsed` is zero.
  - `pub struct History { cap: usize, buf: VecDeque<f64> }` with `new(cap)`, `push(v)`, `values() -> impl Iterator<Item=f64>`, `last() -> Option<f64>`, `len()`. Push drops the oldest value once `cap` is reached.
  - `pub struct ProcInfo { pid: u32, ppid: Option<u32>, name: String, user: String, cpu: f32, mem: u64, threads: Option<usize> }`
  - `pub struct AppInfo { pid: u32, name: String, bundle_id: Option<String> }`
  - `pub struct AppUsage { app: AppInfo, cpu: f32, mem: u64, pids: Vec<u32> }`
  - `pub fn aggregate_apps(apps: &[AppInfo], procs: &[ProcInfo]) -> Vec<AppUsage>` sums the app pid plus all descendants. Each pid is visited once, so a cycle can't loop forever.

- [ ] **Step 1: Failing tests.** `rate_basic` (`rate(1000, 3000, 2s) == 1000.0`), `rate_counter_reset_is_zero` (`rate(5000, 10, 1s) == 0.0`), `rate_zero_elapsed_is_zero`, `history_caps_at_60` (push 61, `len()==60`, first value == 1.0), `aggregate_sums_descendants` (app 10 with children 11→12: cpu and mem are summed, pids `[10,11,12]`), `aggregate_handles_ppid_cycle` (11.ppid=12, 12.ppid=11, app 10 unaffected, test completes), `aggregate_app_without_process` (app pid missing from procs → cpu 0, mem 0).
- [ ] **Step 2:** Run `cargo test sample`. Expected: compile FAIL.
- [ ] **Step 3:** Implement. Build a `HashMap<u32, Vec<u32>>` of children once, then do a BFS with a `HashSet` of visited pids.
- [ ] **Step 4:** Run `cargo test sample`. Expected: 7 passed.
- [ ] **Step 5:** Commit with message `feat: rate, history and app aggregation`.

### Task 3: Sampler and State (`sampler.rs`)

**Files:** Create `src/sampler.rs`. Test: `tests/sampler.rs`.

**Interfaces:**
- Consumes: Task 2 types.
- Produces:
  - `pub struct DiskStat { name, mount: String, total, avail: u64, read_bps, write_bps: f64 }`
  - `pub struct NetStat { name: String, rx_bps, tx_bps: f64, rx_hist, tx_hist: History }`
  - `pub struct State { cpu_model: String, cpu_total: History, cpu_cores: Vec<History>, mem_total, mem_used, swap_total, swap_used: u64, mem_hist: History, disks: Vec<DiskStat>, nets: Vec<NetStat>, procs: Vec<ProcInfo>, apps: Vec<AppUsage>, gpu: Option<GpuStat>, battery: Option<BatteryStat> }` with `impl Default`
  - `pub struct Sampler` with `new() -> Self` and `tick(&mut self, s: &mut State)`
  - `pub fn spawn(state: Arc<Mutex<State>>, ctx: egui::Context)` loops: `tick`, `request_repaint`, sleep 1 s.
  - `GpuStat`/`BatteryStat` are defined in Tasks 7/8. Until then they are empty placeholder structs in `collect/mod.rs`, and the gpu/battery fields stay `None`.

- [ ] **Step 1: Failing test.** `two_ticks_fill_history`: after `Sampler::new()` and two `tick`s with a 300 ms sleep between them, `cpu_total.len()==2`, every value is in `0.0..=100.0`, `mem_total > 0`, `!procs.is_empty()`, and `cpu_cores.len() == available_parallelism`.
- [ ] **Step 2:** Run `cargo test --test sampler`. Expected: FAIL.
- [ ] **Step 3:** Implement with sysinfo 0.39 (`refresh_cpu_usage`, `refresh_memory`, `Processes::refresh`, `Disks`, `Networks`). Use `rate()` on cumulative totals together with the measured `Instant` elapsed time. Disk rates come from `Disk::usage()` if sysinfo exposes it; otherwise use 0.0 and add a `shortcut:` comment. Apps come from Task 6 later; until then `apps` stays empty.
- [ ] **Step 4:** Run `cargo test --test sampler`. Expected: PASS.
- [ ] **Step 5:** Commit with message `feat: background sampler`.

### Task 4: Theme, shell, CPU and Memory pages

**Files:** Create `src/theme.rs`, `src/app.rs`, `src/pages/{mod,cpu,memory}.rs`, `src/widgets.rs` (graph card). Modify `src/main.rs`. Test: `tests/ui.rs` (egui_kittest).

**Interfaces:**
- Consumes: `State`, `spawn`.
- Produces:
  - `pub enum Page { Apps, Processes, Cpu, Memory, Gpu, Drives, Network, Battery }`
  - `pub struct App { state: Arc<Mutex<State>>, page: Page, … }` with `App::new(state)` and `impl eframe::App`.
  - `pub fn ui_for_test(ui: &mut egui::Ui, app: &mut App)` draws one frame for kittest.
  - `widgets::graph_card(ui, title: &str, value_label: &str, series: &[&History], y_max: Option<f64>)`. A `y_max` of `None` means autoscale. The x-axis is fixed at 60.
  - `theme::apply(ctx)` sets `ThemePreference::System`, loads `/System/Library/Fonts/SFNS.ttf` if present (falls back to the default font), uses item spacing 8, corner radius 8, and accent `#007AFF`.

- [ ] **Step 1: Failing test.** `sidebar_lists_pages`: a kittest harness with `State::default()` finds the labels "Apps", "Processes", "Processor", "Memory", "Drives", "Network". `clicking_memory_shows_memory_page` clicks "Memory" and expects to find "Swap".
- [ ] **Step 2:** Run `cargo test --test ui`. Expected: FAIL.
- [ ] **Step 3:** Implement. The sidebar is a `SidePanel::left` with `selectable_label`s. The CPU page has the model name, a total graph (0–100) and a grid of per-core mini graphs. The Memory page has a RAM graph plus used/total and swap used/total. Format bytes with a small `fmt_bytes(u64) -> String` in `widgets.rs` using 1024 units, and test it (`fmt_bytes(1536) == "1.5 KiB"`).
- [ ] **Step 4:** Run `cargo test`. Expected: PASS. Then `cargo run --release`: the numbers match Activity Monitor within a few %, and the app's own idle CPU is under about 2 %.
- [ ] **Step 5:** Commit with message `feat: mac theme, sidebar, cpu and memory pages`.

### Task 5: Drives and Network pages

**Files:** Create `src/pages/{drives,network}.rs`. Extend `tests/ui.rs`.

- [ ] **Step 1: Failing test.** `drives_page_shows_disk` puts a `DiskStat{name:"Macintosh HD",…}` in State, clicks "Drives" and finds "Macintosh HD". `network_page_shows_iface` does the same with `en0`.
- [ ] **Step 2:** Run it. Expected: FAIL.
- [ ] **Step 3:** Implement. Each disk card has a capacity `ProgressBar` and a read/write graph; each interface card has an rx/tx graph and `fmt_bytes(rate)/s`. Hide loopback (`lo0`) and interfaces that have never had traffic.
- [ ] **Step 4:** Run `cargo test`. Expected: PASS.
- [ ] **Step 5:** Commit with message `feat: drives and network pages`.

### Task 6: Processes, Apps, actions

**Files:** Create `src/actions.rs`, `src/collect/apps.rs`, `src/pages/{processes,apps}.rs`, `src/table.rs`. Modify `sampler.rs` (fill `apps`). Tests: inline in `table.rs` and `tests/actions.rs`.

**Interfaces:**
- Produces:
  - `pub enum Sig { Term, Kill }`
  - `pub fn signal(pid: u32, sig: Sig) -> Result<(), String>` wraps `libc::kill` and returns the errno text on failure (`"No such process"`, `"Operation not permitted"`).
  - `pub fn quit_app(pid: u32) -> bool` uses `NSRunningApplication::runningApplicationWithProcessIdentifier` and then `terminate`.
  - `pub fn running_apps() -> Vec<AppInfo>` lists `NSWorkspace::sharedWorkspace().runningApplications()` with `activationPolicy == Regular`.
  - `pub enum SortCol { Name, Pid, User, Cpu, Mem }`
  - `pub fn filter_sort<'a>(procs: &'a [ProcInfo], query: &str, col: SortCol, asc: bool) -> Vec<&'a ProcInfo>`

- [ ] **Step 1: Failing tests.** `filter_is_case_insensitive` ("SAF" matches "Safari"), `sort_by_cpu_desc`, `signal_term_kills_child` (spawn `sleep 30`, `signal(pid, Sig::Term)`, `child.wait()` reports a signal), `signal_dead_pid_errors` (spawn `true` and wait, then `signal` returns `Err`), `running_apps_does_not_panic`.
- [ ] **Step 2:** Run `cargo test`. Expected: FAIL.
- [ ] **Step 3:** Implement. Pages use `egui_extras::TableBuilder` with `.body(|b| b.rows(...))` (virtualized), clickable header sort, a search `TextEdit` and row selection. "End"/"Kill" buttons open an `egui::Modal` confirm. A failure shows the error text in the modal. Apps rows use `aggregate_apps`; "Quit" calls `quit_app` and falls back to `signal(Term)`.
- [ ] **Step 4:** Run `cargo test`. Expected: PASS. Manually: kill a `sleep 999` from the UI. Killing a root process (e.g. `launchd` children) shows "Operation not permitted".
- [ ] **Step 5:** Commit with message `feat: processes and apps with end/kill`.

### Task 7: GPU (Apple Silicon)

**Files:** Create `src/collect/gpu.rs`, `src/pages/gpu.rs`. Modify `sampler.rs`.

**Interfaces:**
- Produces:
  - `pub struct GpuStat { name: String, util_hist: History, mem_used: Option<u64> }`
  - `pub fn from_perf_stats(m: &HashMap<String, i64>) -> Option<(f64, Option<u64>)>` reads keys `"Device Utilization %"` and `"In use system memory"`. It returns None if utilization is missing.
  - `pub fn read_gpu() -> Option<(String, f64, Option<u64>)>` iterates `IOServiceMatching("IOAccelerator")`, reads the `PerformanceStatistics` dictionary and the `model` property, and converts numeric CFNumbers to a HashMap.

- [ ] **Step 1: Failing tests.** `from_perf_stats_reads_util` (`{"Device Utilization %":37, "In use system memory":1048576}` → `(37.0, Some(1048576))`), `from_perf_stats_missing_is_none`, `read_gpu_does_not_panic`.
- [ ] **Step 2:** Run it. Expected: FAIL.
- [ ] **Step 3:** Implement. Release every IOKit object (`IOObjectRelease`). The sidebar hides "GPU" while `state.gpu.is_none()`.
- [ ] **Step 4:** Run `cargo test`. Expected: PASS. On this Mac, the GPU page tracks Activity Monitor's GPU History (⌘4).
- [ ] **Step 5:** Commit with message `feat: apple silicon gpu usage`.

### Task 8: Battery

**Files:** Create `src/collect/battery.rs`, `src/pages/battery.rs`. Modify `sampler.rs`. Extend `tests/ui.rs`.

**Interfaces:**
- Produces: `pub struct BatteryStat { percent: f32, health: f32, cycles: Option<u32>, state: String, charge_hist: History }` and `pub fn read_battery() -> Option<(f32, f32, Option<u32>, String)>` via `starship_battery::Manager`.

- [ ] **Step 1: Failing tests.** `read_battery_does_not_panic`, and `battery_hidden_when_none` (the kittest sidebar does not contain "Battery" with `State::default()`).
- [ ] **Step 2:** Run it. Expected: FAIL.
- [ ] **Step 3:** Implement. The page shows charge %, health %, cycles, state and a charge graph (0–100).
- [ ] **Step 4:** Run `cargo test`. Expected: PASS.
- [ ] **Step 5:** Commit with message `feat: battery page`.

### Task 9: Bundle, release workflow, Homebrew tap

**Files:** Create `macos/Info.plist`, `scripts/bundle.sh`, `scripts/cask.sh`, `.github/workflows/release.yml`.

**Interfaces:**
- `scripts/bundle.sh <version>` expects both `target/{aarch64,x86_64}-apple-darwin/release/resources-mac` to exist. It produces `dist/Resources.app` and `dist/resources-mac-<version>.dmg`, prints the dmg sha256, and runs under `set -euo pipefail`.
- `scripts/cask.sh <version> <sha256>` prints the cask Ruby to stdout.

- [ ] **Step 1:** `Info.plist` sets `CFBundleIdentifier io.github.vachagangrigoryan.resources-mac`, `CFBundleName Resources`, `CFBundleExecutable resources-mac`, `CFBundleShortVersionString __VERSION__` (replaced by sed), `LSMinimumSystemVersion 13.0`, `NSHighResolutionCapable true`.
- [ ] **Step 2:** `bundle.sh` runs `lipo -create` into `Contents/MacOS/`, copies the plist with sed, runs `codesign --force -s - dist/Resources.app`, then `hdiutil create -volname Resources -srcfolder dist/Resources.app -ov -format UDZO`.
- [ ] **Step 3:** Run `rustup target add x86_64-apple-darwin aarch64-apple-darwin`, build both targets, then `scripts/bundle.sh 0.0.0`. Expected: `lipo -info dist/Resources.app/Contents/MacOS/resources-mac` lists `x86_64 arm64`, `codesign -v` passes, and the app opens from the dmg.
- [ ] **Step 4:** `cask.sh` emits `cask "resources-mac"` with `version`, `sha256`, `url "https://github.com/VachaganGrigoryan/resources-mac/releases/download/v#{version}/resources-mac-#{version}.dmg"`, `name "Resources"`, `depends_on macos: ">= :ventura"`, `app "Resources.app"`. Verify with `scripts/cask.sh 0.0.0 abc | ruby -c`, expecting `Syntax OK`.
- [ ] **Step 5:** `release.yml` runs on push of tag `v*` with `permissions: contents: write`.
  - Job `build`: on macos-latest, check that the tag equals `v$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')`, build both targets, run `bundle.sh`, then `softprops/action-gh-release@v2` with `files: dist/*.dmg`, `draft: true`. It outputs the sha256.
  - Job `tap`: needs build. Check out `VachaganGrigoryan/homebrew-tap` with `token: ${{ secrets.HOMEBREW_TAP_TOKEN }}`, write `Casks/resources-mac.rb` via `cask.sh`, then commit and push.
  - Because the release is a draft, the cask URL 404s until the release is published. Either publish before announcing, or set `draft: false`.
- [ ] **Step 6:** Commit with message `ci: universal dmg release and homebrew cask`.

### Task 10: Publish (needs user confirmation: creates public repos)

- [ ] **Step 1:** README: features, a screenshot, install (`brew install --cask VachaganGrigoryan/tap/resources-mac`), first launch (System Settings → Privacy & Security → Open Anyway), build from source (`cargo run --release`), release process (`git tag vX.Y.Z && git push --tags`), and a GPL-3.0 credit to GNOME Resources.
- [ ] **Step 2:** `gh repo create VachaganGrigoryan/resources-mac --public --source . --push`. Then `gh repo create VachaganGrigoryan/homebrew-tap --public` with a `Casks/` folder.
- [ ] **Step 3:** The user creates a fine-grained PAT (contents read/write on homebrew-tap) and runs `gh secret set HOMEBREW_TAP_TOKEN -R VachaganGrigoryan/resources-mac`.
- [ ] **Step 4:** Confirm `ci.yml` is green on main (`gh run list`). Tag `v0.1.0`, then confirm the draft release has the dmg and the tap has the cask. Publish the release, then `brew install --cask VachaganGrigoryan/tap/resources-mac` launches the app.
