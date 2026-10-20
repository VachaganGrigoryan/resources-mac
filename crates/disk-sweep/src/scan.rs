//! Walks a folder into a size tree. Project folders keep their children; caches, editor
//! metadata, ignored files and `.git` are summed into one node each.

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

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

// shortcut: hard links are counted once per link; dedupe by inode if clones skew the totals.
/// Bytes and files under `dir`, without classifying anything.
fn sum_dir(dir: &Path, progress: &Progress) -> (u64, u64) {
    let Ok(entries) = fs::read_dir(dir) else {
        progress.error();
        return (0, 0);
    };
    let (mut bytes, mut files) = (0, 0);
    for entry in entries {
        let Ok(md) = entry.and_then(|e| e.metadata().map(|md| (e.path(), md))) else {
            progress.error();
            continue;
        };
        let (path, md) = md;
        bytes += allocated(&md);
        if md.is_dir() {
            let (b, f) = sum_dir(&path, progress);
            bytes += b;
            files += f;
        } else {
            files += 1;
            progress.add(1, allocated(&md));
        }
    }
    (bytes, files)
}

/// A project folder: classifies each entry and keeps the tree.
fn walk(
    dir: &Path,
    md: &fs::Metadata,
    stack: &mut IgnoreStack,
    progress: &Progress,
) -> io::Result<Node> {
    let entries = fs::read_dir(dir)?;
    let token = stack.enter(dir);
    let own = allocated(md);
    let mut node = Node {
        name: file_name(dir),
        path: dir.to_path_buf(),
        kind: Kind::Source,
        is_dir: true,
        bytes: own,
        files: 0,
        by_kind: [0; 5],
        children: Vec::new(),
        detector: None,
    };
    node.by_kind[Kind::Source.index()] = own;
    for entry in entries {
        // DirEntry::metadata does not follow symlinks.
        let Ok((path, md)) = entry.and_then(|e| e.metadata().map(|md| (e.path(), md))) else {
            progress.error();
            continue;
        };
        let name = file_name(&path);
        let is_dir = md.is_dir();
        let ignored = stack.is_ignored(&path, is_dir);
        let (kind, detector) = classify(&path, &name, is_dir, ignored);
        let child = if kind == Kind::Source && is_dir {
            match walk(&path, &md, stack, progress) {
                Ok(child) => child,
                Err(_) => {
                    progress.error();
                    leaf(path, name, kind, true, allocated(&md), 0, None)
                }
            }
        } else if is_dir {
            let (b, f) = sum_dir(&path, progress);
            leaf(path, name, kind, true, allocated(&md) + b, f, detector)
        } else {
            progress.add(1, allocated(&md));
            leaf(path, name, kind, false, allocated(&md), 1, detector)
        };
        node.bytes += child.bytes;
        node.files += child.files;
        for (sum, b) in node.by_kind.iter_mut().zip(child.by_kind) {
            *sum += b;
        }
        node.children.push(child);
    }
    stack.leave(token);
    node.children.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    Ok(node)
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
    walk(root, &md, &mut IgnoreStack::new(), progress)
}

/// First node named `name`, depth-first, `node` itself included.
pub fn find<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    if node.name == name {
        return Some(node);
    }
    node.children.iter().find_map(|c| find(c, name))
}
