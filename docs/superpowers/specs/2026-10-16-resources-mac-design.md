# Plan: macOS system monitor inspired by GNOME Resources (Rust + egui)

## Context
GNOME Resources (github.com/nokyan/resources, GPL-3.0, now on GNOME GitLab) is a Rust/GTK4/libadwaita system monitor. Its data layer is Linux-only (`/proc`, sysfs, systemd cgroups, polkit), so it can't simply be recompiled for macOS. You want an open-source Mac version: a pure-Rust desktop app in its own Git repo, built and released by GitHub CI.

**Decisions you made:** egui with a Mac-like theme · v1 = CPU, Memory, Disk, Network, Processes, Apps, Apple Silicon GPU, Battery · unsigned releases plus a Homebrew cask.

**My calls (change any of them):**
- Working name `resources-mac` (rename freely; avoid "GNOME" in the name for trademark reasons).
- License GPL-3.0, so code or icons from upstream can be borrowed legally.
- One crate (lib + bin), not a workspace.
- Minimum macOS 13. Ship one universal binary (arm64 + x86_64).
- Repo is `github.com/VachaganGrigoryan/resources-mac` and the tap is `github.com/VachaganGrigoryan/homebrew-tap`.
- Packaging is a ~30-line `scripts/bundle.sh` (lipo, Info.plist, ad-hoc codesign, `hdiutil`) instead of a packaging tool. It's fully transparent and adds no dependencies.

## Crates
`eframe`/`egui` (UI) · `egui_plot` (live graphs) · `egui_extras` (TableBuilder for process lists) · `sysinfo` (cpu/mem/disks/networks/processes) · `starship-battery` (battery) · `io-kit-sys` + `core-foundation` (GPU stats) · `objc2-app-kit` (NSWorkspace running apps, app quit) · `libc` (kill signals) · `anyhow`.

## Architecture
```
resources-mac/
├─ Cargo.toml / Cargo.lock / rust-toolchain.toml (stable)
├─ src/main.rs              eframe::run_native; window options; spawns sampler
├─ src/app.rs               App struct: sidebar + selected page; reads latest Snapshot
├─ src/theme.rs             Mac-like Visuals: follows system light/dark, SF Pro loaded at runtime from /System/Library/Fonts (fallback: egui default), 8pt spacing, rounded cards, accent blue
├─ src/sample.rs            Snapshot types + pure math: counter deltas → rates, History (VecDeque ring, 60 pts)  [unit-tested]
├─ src/sampler.rs           thread: every 1 s refresh sysinfo + collectors → Arc<Mutex<State>>, then ctx.request_repaint()
├─ src/collect/{gpu,battery,apps}.rs
├─ src/actions.rs           end/kill (SIGTERM/SIGKILL via libc), app quit via NSRunningApplication.terminate
├─ src/pages/{apps,processes,cpu,memory,gpu,drives,network,battery}.rs
├─ macos/Info.plist, macos/AppIcon.icns
├─ scripts/bundle.sh        lipo two targets → .app → codesign -s - → .dmg
└─ .github/workflows/ci.yml, release.yml
```
**Data flow:** the sampler thread owns `sysinfo::System` and pushes snapshots into the shared `State`, which holds the history rings and the latest process and app lists. The UI thread only reads `State` and draws. It never calls sysinfo itself, so the UI never stutters. The UI repaints only when the sampler calls `request_repaint()`, plus whenever you interact. That keeps idle CPU usage near zero, which matters for a monitor app.

**Apps vs processes:** NSWorkspace provides the apps (pid, name, bundle id). Each app's CPU and memory is the sum over the app's process and all its descendants, using the parent-pid tree from sysinfo. Killing a process owned by another user (e.g. root) fails with a visible error. v1 has no privilege escalation; that is the macOS equivalent of upstream's polkit.

## Implementation steps (TDD for pure logic)
1. **Repo:** `cargo new resources-mac`. Add the GPL-3.0 LICENSE, a README crediting GNOME Resources, `.gitignore`, and the design doc at `docs/superpowers/specs/2026-10-16-resources-mac-design.md`. `git init`, then push to GitHub.
2. **CI first** (`ci.yml`, runs on push and PR, `macos-latest`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, plus `Swatinem/rust-cache`. This keeps the build green from day one.
3. **`sample.rs` (tests first):** rate from counter deltas (including counter reset and zero elapsed time), the History ring cap, and app aggregation over the pid tree.
4. **Sampler + CPU/Memory pages:** a `egui::SidePanel` sidebar, then a page with a graph card (`egui_plot::Plot`, fixed 60 s x-axis, 0–100 % y-axis) and a details grid. Per-core mini graphs go on the CPU page.
5. **Drives + Network pages:** read/write and rx/tx rates, plus capacity bars.
6. **Processes + Apps pages:** a `TableBuilder` with sortable columns and a search field. End/Kill buttons ask for confirmation in an `egui::Modal`. App icons are added later; v1 shows names only.
7. **GPU:** read `IOAccelerator` → `PerformanceStatistics` → `Device Utilization %` and `In use system memory`. The keys are undocumented, so if they're missing the page is hidden, never a crash. A unit test parses a captured fixture dictionary.
8. **Battery:** hide the page when there is no battery (desktop Macs).
9. **Release** (`release.yml`, runs on a `v*` tag, `permissions: contents: write`):
   - `cargo build --release --target aarch64-apple-darwin` and the same for `x86_64-apple-darwin`, then `scripts/bundle.sh <version>` produces `resources-mac-<v>.dmg`. The job fails if the tag doesn't match the `Cargo.toml` version.
   - `softprops/action-gh-release@v2` uploads the dmg and its sha256 to a draft release.
   - A tap job renders `Casks/resources-mac.rb` (url, sha256, `depends_on macos: ">= :ventura"`) and pushes it to `VachaganGrigoryan/homebrew-tap` with a fine-grained PAT secret `HOMEBREW_TAP_TOKEN`.
10. **README:** cover `brew install --cask VachaganGrigoryan/tap/resources-mac`, the first-launch Gatekeeper step (System Settings → Privacy & Security → Open Anyway), and `cargo run --release` for building from source.

## Verification
- Locally: `cargo test`, then `cargo run --release`. Compare CPU%, memory and network rate with Activity Monitor, kill a test `sleep 999` from the Processes page, and confirm idle CPU of the app itself stays under about 2 % in Activity Monitor.
- Bundle: `scripts/bundle.sh 0.0.0`, then `lipo -info` on the binary shows both archs. Open the dmg and launch the app.
- CI: open a PR and confirm `ci.yml` passes. Push tag `v0.1.0` to get a draft release with the dmg and an updated tap. On a clean Mac, `brew install --cask` from the tap, then launch the app.
- Gate before calling it done: `/check` (fmt, clippy, tests).

## Risks
- **Unsigned app:** Gatekeeper blocks the first launch, and macOS Sequoia removed right-click → Open. The ad-hoc signature is required just to run on Apple Silicon. Homebrew is tightening rules on unsigned casks, but a personal tap still works. Signing and notarization ($99/yr Apple Developer account) is the upgrade path.
- **Look and feel:** egui draws its own widgets, so the app will look Mac-like but not native, and screen-reader support (AccessKit) is basic.
- **GPU stats** use undocumented IOKit keys and may change between macOS versions.
