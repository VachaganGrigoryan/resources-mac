# Resources for Mac

A macOS system monitor written in Rust with [egui](https://github.com/emilk/egui), inspired by [GNOME Resources](https://apps.gnome.org/Resources/).

- **Apps:** running apps with combined CPU and memory across their processes; Quit / Force Quit
- **Processes:** searchable, sortable list; End (SIGTERM) / Kill (SIGKILL)
- **Processor:** total and per-core usage
- **Memory:** RAM and swap
- **GPU:** Apple Silicon usage and memory in use
- **Drives:** capacity, read/write rates
- **Network:** per-interface traffic
- **Battery:** charge, health, cycle count

The GPU and Battery pages appear only on Macs that report them.

## Install

```sh
brew install --cask VachaganGrigoryan/tap/resources-mac
```

Or download the `.dmg` from [Releases](https://github.com/VachaganGrigoryan/resources-mac/releases) and drag **Resources** to Applications.

Requires macOS 13 or later. The app is universal (Apple Silicon and Intel).

**First launch:** releases are not notarized, so macOS blocks the app the first time. Open **System Settings → Privacy & Security**, scroll down and click **Open Anyway**. Alternatively, run:

```sh
xattr -dr com.apple.quarantine /Applications/Resources.app
```

## Build from source

```sh
cargo run --release
```

Quality gate (same as CI):

```sh
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

To build the universal `.app` and `.dmg` locally:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
scripts/bundle.sh 0.1.0   # → dist/Resources.app, dist/resources-mac-0.1.0.dmg
```

## Releasing

1. Bump `version` in `Cargo.toml` and commit.
2. `git tag v0.1.0 && git push --tags`.
3. The **Release** workflow builds the universal dmg, attaches it to a draft GitHub release and updates the cask in [`homebrew-tap`](https://github.com/VachaganGrigoryan/homebrew-tap). This needs the repository secret `HOMEBREW_TAP_TOKEN`: a fine-grained token with *Contents: read and write* on `homebrew-tap`.
4. Publish the draft release. The cask URL works only after the release is published.

## Limitations

- Processes owned by other users (e.g. root) can't be ended. The app does not ask for administrator rights.
- GPU statistics come from undocumented IOKit keys and may change in future macOS versions.

## Credits and license

Inspired by [GNOME Resources](https://gitlab.gnome.org/GNOME/Incubator/resources) by nokyan and contributors. This is an independent project and is not affiliated with GNOME.

Licensed under the [GNU GPL v3.0 or later](LICENSE).
