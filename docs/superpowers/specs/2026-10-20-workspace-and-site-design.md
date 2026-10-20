# One workspace, a shared UI crate, two released apps, one website

Date: 2026-10-20. Status: implemented.

## Context

The repo grew a second app, Disk Sweep, next to the Resources system monitor. Disk Sweep reused the Resources library for theme, icons and widgets, but:

- the window shell (window options, sidebar, header bar, scrolled page column), the Appearance settings group, the bottom action bar, the poison-tolerant lock, the "first 5 … and N more" text, the path display name and the kittest helpers were copied between the apps;
- depending on the whole Resources library made Disk Sweep compile sysinfo, IOKit and the battery crate;
- Disk Sweep was never bundled or released, so the Free Up Space… button that opens it stayed hidden in shipped builds;
- the website only covered Resources;
- the release workflow read the version of the first package in `cargo metadata`, which became Disk Sweep after the workspace change.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Layout | Virtual workspace root; `crates/ui`, `crates/resources`, `crates/disk-sweep`; `site/` stays a separate Cargo project | Each app is a peer; shared code has one home |
| Shared crate | `crates/ui`, package `resources-ui` | Holds everything both apps draw with; depends only on egui and eframe |
| Resources package name | Stays `resources-mac` | The binary name is in Info.plist, bundle.sh, the cask token and users' installed casks; the lib name keeps every `resources_mac::` path valid |
| Old import paths | Resources re-exports `icons`, `theme`, `widgets`, `History`, `Confirm` and `action_bar` from `resources-ui` | No churn in Resources' pages and tests |
| Versioning | One `[workspace.package] version`; one `v*` tag releases both apps | One release job and one tag check; Disk Sweep 0.1.0 was never released |
| Shell API | `shell::sidebar()` then `shell::content()` with `shell::header()` and `shell::scrolled_column()`; Main Menu entries are data | Sequential calls avoid two closures borrowing the app at once |
| Distribution | `Disk Sweep.app` with its own Info.plist (`io.github.vachagangrigoryan.disk-sweep`), icon, dmg and cask `disk-sweep` | Installs side by side; Resources already finds `/Applications/Disk Sweep.app` |
| Icons | `scripts/icon.swift` draws both icons with the same body | One drawing toolkit, matching icons |
| Website | Home page for both apps, plus `/resources/` and `/disk-sweep/`; headers and footers duplicated; icons in a shared `/icons.svg` sprite | No build step, CSP stays `default-src 'self'` |

## Shared crate contents

| Module | Items |
|---|---|
| `lib` | `lock`, `display_name`, `native_options`, `History`, `HISTORY_LEN` |
| `theme`, `icons` | The GNOME Resources palette, fonts and Phosphor glyphs |
| `widgets` | Graphs, sidebar items, cards, boxed lists, buttons, `fmt_*`, `pct`, `action_bar`, `names_preview` |
| `shell` | `SIDEBAR_W`, `MenuEntry`, `sidebar`, `content`, `header`, `scrolled_column` |
| `confirm` | `Confirm<A>`, the modal before destructive actions |
| `settings` | `appearance` (System / Light / Dark), `hint` |
| `test_support` (feature `test-support`) | `harness`, `shows`, `open` |

Resources keeps its sort/search/icon-cache table code, menu bar, desktop widget, About dialog and all AppKit code.

## Consequences

- A tag builds both apps, attaches `resources-mac-<v>.dmg` and `disk-sweep-<v>.dmg` with checksums to one draft release, and pushes both casks.
- A Resources-only fix also re-releases Disk Sweep under the new version.
- Moving the shell kept Resources pixel-identical: `cargo test -p resources-mac --test screenshots -- --ignored` leaves `docs/screenshots/` unchanged. The Overview keeps its own f32 percentage, because the shared f64 one shifted pixels.
- `cargo tree -p disk-sweep -i sysinfo` finds nothing; Disk Sweep's remaining AppKit bindings come from eframe, winit and the folder picker.
- The site test fails if a page references a screenshot or icon that does not exist.
