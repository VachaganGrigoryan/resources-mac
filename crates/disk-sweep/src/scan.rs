//! Walks a folder into a size tree. Project folders keep their children; caches, editor
//! metadata, ignored files and `.git` are summed into one node each.

use resources_ui::display_name;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::classify::{Detector, IgnoreStack, classify};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Source,
    Cache,
    Editor,
    Ignored,
    Git,
}

impl Kind {
    pub const ALL: [Kind; 5] = [
        Kind::Source,
        Kind::Cache,
        Kind::Editor,
        Kind::Ignored,
        Kind::Git,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Kind::Source => "Project files",
            Kind::Cache => "Caches & builds",
            Kind::Editor => "Editor & OS metadata",
            Kind::Ignored => "Other ignored",
            Kind::Git => "Git history",
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub path: PathBuf,
    pub kind: Kind,
    pub is_dir: bool,
    /// Space taken on disk (allocated blocks), this entry and everything below it.
    pub bytes: u64,
    /// Files (anything but folders) below it, or 1 for a file.
    pub files: u64,
    /// `bytes` split by kind.
    pub by_kind: [u64; 5],
    /// Largest first; only project folders keep children.
    pub children: Vec<Node>,
    pub detector: Option<&'static Detector>,
}

/// Running totals for the progress line, shared with the UI thread.
#[derive(Debug, Default)]
pub struct Progress {
    pub files: AtomicU64,
    pub bytes: AtomicU64,
    /// Entries that couldn't be read (permission denied, vanished mid-scan).
    pub errors: AtomicU64,
}

impl Progress {
    fn add(&self, files: u64, bytes: u64) {
        self.files.fetch_add(files, Ordering::Relaxed);
        self.bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    fn error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }
}

fn allocated(md: &fs::Metadata) -> u64 {
    md.blocks() * 512
}

/// What one scan carries down the tree.
struct Walk<'a> {
    stack: IgnoreStack,
    progress: &'a Progress,
    /// Multi-link files already counted, by (device, inode): `du` counts them once too.
    linked: HashSet<(u64, u64)>,
}

impl Walk<'_> {
    /// Space `md` takes, or 0 for another link to a file already counted.
    // shortcut: APFS clones still count in full; they share blocks the API doesn't expose.
    fn size(&mut self, md: &fs::Metadata) -> u64 {
        if !md.is_dir() && md.nlink() > 1 && !self.linked.insert((md.dev(), md.ino())) {
            return 0;
        }
        allocated(md)
    }

    /// Bytes and files under `dir`, without classifying anything.
    fn sum_dir(&mut self, dir: &Path) -> (u64, u64) {
        let Ok(entries) = fs::read_dir(dir) else {
            self.progress.error();
            return (0, 0);
        };
        let (mut bytes, mut files) = (0, 0);
        for entry in entries {
            let Ok((path, md)) = entry.and_then(|e| e.metadata().map(|md| (e.path(), md))) else {
                self.progress.error();
                continue;
            };
            let size = self.size(&md);
            bytes += size;
            if md.is_dir() {
                let (b, f) = self.sum_dir(&path);
                bytes += b;
                files += f;
            } else {
                files += 1;
                self.progress.add(1, size);
            }
        }
        (bytes, files)
    }

    /// A project folder: classifies each entry and keeps the tree.
    fn walk(&mut self, dir: &Path, md: &fs::Metadata) -> io::Result<Node> {
        let entries = fs::read_dir(dir)?;
        let token = self.stack.enter(dir);
        let own = allocated(md);
        let mut node = leaf(
            dir.to_path_buf(),
            display_name(dir),
            Kind::Source,
            true,
            own,
            0,
            None,
        );
        for entry in entries {
            // DirEntry::metadata does not follow symlinks.
            let Ok((path, md)) = entry.and_then(|e| e.metadata().map(|md| (e.path(), md))) else {
                self.progress.error();
                continue;
            };
            let name = display_name(&path);
            let is_dir = md.is_dir();
            let ignored = self.stack.is_ignored(&path, is_dir);
            let (mut kind, mut detector) = classify(&path, &name, is_dir, ignored);
            // A clone with its own history may hold unpushed work, whatever the parent ignores.
            if is_dir && matches!(kind, Kind::Cache | Kind::Ignored) && path.join(".git").exists() {
                (kind, detector) = (Kind::Source, None);
            }
            let child = if kind == Kind::Source && is_dir {
                match self.walk(&path, &md) {
                    Ok(child) => child,
                    Err(_) => {
                        self.progress.error();
                        leaf(path, name, kind, true, allocated(&md), 0, None)
                    }
                }
            } else if is_dir {
                let (b, f) = self.sum_dir(&path);
                leaf(path, name, kind, true, allocated(&md) + b, f, detector)
            } else {
                let size = self.size(&md);
                self.progress.add(1, size);
                leaf(path, name, kind, false, size, 1, detector)
            };
            node.bytes += child.bytes;
            node.files += child.files;
            for (sum, b) in node.by_kind.iter_mut().zip(child.by_kind) {
                *sum += b;
            }
            node.children.push(child);
        }
        self.stack.leave(token);
        node.children.sort_by_key(|c| std::cmp::Reverse(c.bytes));
        Ok(node)
    }
}

fn leaf(
    path: PathBuf,
    name: String,
    kind: Kind,
    is_dir: bool,
    bytes: u64,
    files: u64,
    detector: Option<&'static Detector>,
) -> Node {
    let mut by_kind = [0; 5];
    by_kind[kind.index()] = bytes;
    Node {
        name,
        path,
        kind,
        is_dir,
        bytes,
        files,
        by_kind,
        children: Vec::new(),
        detector,
    }
}

/// Scans `root` (a folder). Unreadable entries below it are skipped and counted in
/// `progress.errors`; an unreadable or missing root is an error.
pub fn scan_root(root: &Path, progress: &Progress) -> io::Result<Node> {
    let md = fs::metadata(root)?;
    if !md.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotADirectory, "not a folder"));
    }
    Walk {
        stack: IgnoreStack::new(),
        progress,
        linked: HashSet::new(),
    }
    .walk(root, &md)
}

/// First node named `name`, depth-first, `node` itself included.
pub fn find<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    if node.name == name {
        return Some(node);
    }
    node.children.iter().find_map(|c| find(c, name))
}
