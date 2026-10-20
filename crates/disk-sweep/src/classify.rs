//! What a file or folder is: a known cache or build output, editor/OS metadata, git history,
//! something `.gitignore` excludes, or a project file.

use std::path::Path;

use ignore::Match;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::scan::Kind;

/// When a folder with a detector's name really is that cache.
#[derive(Debug)]
pub enum Marker {
    Always,
    /// One of these files sits next to it (`node_modules` beside `package.json`).
    Sibling(&'static [&'static str]),
    /// This file is inside it (`.venv/pyvenv.cfg`).
    Inside(&'static str),
    /// Only when `.gitignore` excludes it: generic names like `build` are often sources.
    IfIgnored,
}

/// A kind of re-creatable folder and how to get it back.
#[derive(Debug)]
pub struct Detector {
    pub names: &'static [&'static str],
    pub marker: Marker,
    pub label: &'static str,
    pub restore: &'static str,
}

const fn d(
    names: &'static [&'static str],
    marker: Marker,
    label: &'static str,
    restore: &'static str,
) -> Detector {
    Detector {
        names,
        marker,
        label,
        restore,
    }
}

/// First match wins, so specific entries go before generic ones with the same name.
pub static DETECTORS: &[Detector] = &[
    d(
        &["node_modules"],
        Marker::Sibling(&["package.json"]),
        "Node packages",
        "npm/pnpm/yarn install",
    ),
    d(
        &["target"],
        Marker::Sibling(&["Cargo.toml"]),
        "Cargo build output",
        "cargo build",
    ),
    d(
        &[".venv", "venv"],
        Marker::Inside("pyvenv.cfg"),
        "Python virtualenv",
        "uv sync",
    ),
    d(
        &[
            "__pycache__",
            ".pytest_cache",
            ".mypy_cache",
            ".ruff_cache",
            ".tox",
            ".nox",
        ],
        Marker::Always,
        "Python cache",
        "re-created on next run",
    ),
    d(&[".next"], Marker::Always, "Next.js build", "next build"),
    d(
        &[
            ".turbo",
            ".parcel-cache",
            ".nuxt",
            ".svelte-kit",
            ".angular",
            ".vite",
            "storybook-static",
        ],
        Marker::Always,
        "JS build cache",
        "rebuild",
    ),
    d(&[".gradle"], Marker::Always, "Gradle cache", "gradle build"),
    d(
        &["build"],
        Marker::Sibling(&["build.gradle", "build.gradle.kts"]),
        "Gradle build",
        "gradle build",
    ),
    d(
        &["Pods"],
        Marker::Sibling(&["Podfile"]),
        "CocoaPods",
        "pod install",
    ),
    d(
        &["DerivedData"],
        Marker::Always,
        "Xcode DerivedData",
        "rebuild in Xcode",
    ),
    d(
        &[".build"],
        Marker::Sibling(&["Package.swift"]),
        "SwiftPM build",
        "swift build",
    ),
    d(
        &[".dart_tool"],
        Marker::Always,
        "Dart tool cache",
        "flutter pub get",
    ),
    d(
        &[
            "dist", "build", "out", "coverage", "vendor", "tmp", ".tmp", ".cache",
        ],
        Marker::IfIgnored,
        "Build output / cache",
        "rebuild",
    ),
];

/// IDE, editor and Finder files; any `*.iml` too.
pub const EDITOR: &[&str] = &[".idea", ".vscode", ".vs", ".fleet", ".zed", ".DS_Store"];

struct Layer {
    matcher: Gitignore,
    /// Top of a git repository: rules of folders above it don't apply inside.
    repo_root: bool,
}

/// The `.gitignore` files that apply to the folder being walked, outermost first.
// shortcut: rules from folders above the scan root and the global excludes file are not read;
// add them if scanning inside a repo (not at or above it) turns out to be common.
#[derive(Default)]
pub struct IgnoreStack {
    layers: Vec<Layer>,
}

fn matcher(root: &Path, file: &Path) -> Gitignore {
    let mut b = GitignoreBuilder::new(root);
    // A missing or unreadable file just means no rules.
    let _ = b.add(file);
    b.build().unwrap_or_else(|_| Gitignore::empty())
}

impl IgnoreStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts applying `dir`'s ignore files; pass the token to [`IgnoreStack::leave`].
    pub fn enter(&mut self, dir: &Path) -> usize {
        let token = self.layers.len();
        let git = dir.join(".git");
        if git.exists() {
            self.layers.push(Layer {
                matcher: matcher(dir, &git.join("info/exclude")),
                repo_root: true,
            });
        }
        let file = dir.join(".gitignore");
        if file.is_file() {
            self.layers.push(Layer {
                matcher: matcher(dir, &file),
                repo_root: false,
            });
        }
        token
    }

    pub fn leave(&mut self, token: usize) {
        self.layers.truncate(token);
    }

    /// Deepest rule wins; `!pattern` re-includes.
    pub fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        for layer in self.layers.iter().rev() {
            match layer.matcher.matched(path, is_dir) {
                Match::Ignore(_) => return true,
                Match::Whitelist(_) => return false,
                Match::None => {}
            }
            if layer.repo_root {
                break;
            }
        }
        false
    }
}

fn holds(marker: &Marker, path: &Path, ignored: bool) -> bool {
    match marker {
        Marker::Always => true,
        Marker::Sibling(files) => path
            .parent()
            .is_some_and(|p| files.iter().any(|f| p.join(f).exists())),
        Marker::Inside(file) => path.join(file).exists(),
        Marker::IfIgnored => ignored,
    }
}

/// Kind of the entry at `path` (`name` is its file name), and the cache detector that matched.
pub fn classify(
    path: &Path,
    name: &str,
    is_dir: bool,
    ignored: bool,
) -> (Kind, Option<&'static Detector>) {
    if name == ".git" {
        return (Kind::Git, None);
    }
    if EDITOR.contains(&name) || name.ends_with(".iml") {
        return (Kind::Editor, None);
    }
    if is_dir
        && let Some(det) = DETECTORS
            .iter()
            .find(|det| det.names.contains(&name) && holds(&det.marker, path, ignored))
    {
        return (Kind::Cache, Some(det));
    }
    if ignored {
        (Kind::Ignored, None)
    } else {
        (Kind::Source, None)
    }
}
