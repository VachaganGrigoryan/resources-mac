//! Background work: folder scans, Docker queries, Trash and prune. Results land in the shared
//! [`State`]; the UI reads it every frame.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use crate::docker::{self, Prune, Usage, VmDisk};
use crate::scan::{Node, Progress, scan_root};
use crate::suggest::{Suggestion, suggestions};

/// One chosen folder and its latest scan.
pub struct RootScan {
    pub root: PathBuf,
    pub progress: Arc<Progress>,
    pub scanning: bool,
    /// The last finished scan; kept while a rescan runs so the page doesn't go blank.
    pub result: Option<Result<Node, String>>,
    pub suggestions: Vec<Suggestion>,
    /// Which scan may write here: a newer one makes an older thread's result stale.
    pub generation: u64,
}

impl RootScan {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            progress: Arc::default(),
            scanning: true,
            result: None,
            suggestions: Vec::new(),
            generation: 0,
        }
    }

    /// The scan tree once a scan has succeeded.
    pub fn node(&self) -> Option<&Node> {
        self.result.as_ref().and_then(|r| r.as_ref().ok())
    }
}

#[derive(Default)]
pub struct State {
    pub roots: Vec<RootScan>,
    /// `None` until the first query returns.
    pub docker: Option<Result<Usage, String>>,
    pub docker_vm: Option<VmDisk>,
    pub docker_busy: bool,
}

pub type Shared = Arc<Mutex<State>>;

/// The state, even if a worker panicked while holding it.
pub fn lock(shared: &Shared) -> MutexGuard<'_, State> {
    shared.lock().unwrap_or_else(|e| e.into_inner())
}

static GENERATION: AtomicU64 = AtomicU64::new(1);

/// Scans `root` on a thread, adding it to the state if it is new.
pub fn start_scan(shared: &Shared, root: PathBuf, ctx: egui::Context) {
    let progress = Arc::new(Progress::default());
    let generation = GENERATION.fetch_add(1, Ordering::Relaxed);
    {
        let mut s = lock(shared);
        let i = match s.roots.iter().position(|r| r.root == root) {
            Some(i) => i,
            None => {
                s.roots.push(RootScan::new(root.clone()));
                s.roots.len() - 1
            }
        };
        let entry = &mut s.roots[i];
        entry.progress = Arc::clone(&progress);
        entry.scanning = true;
        entry.generation = generation;
    }
    let shared = Arc::clone(shared);
    thread::spawn(move || {
        let result = scan_root(&root, &progress).map_err(|e| format!("{}: {e}", root.display()));
        let found = result.as_ref().map(suggestions).unwrap_or_default();
        let mut s = lock(&shared);
        if let Some(entry) = s
            .roots
            .iter_mut()
            .find(|r| r.root == root && r.generation == generation)
        {
            entry.result = Some(result);
            entry.suggestions = found;
            entry.scanning = false;
        }
        drop(s);
        ctx.request_repaint();
    });
}

pub fn remove_root(shared: &Shared, root: &Path) {
    lock(shared).roots.retain(|r| r.root != root);
}

/// Queries Docker on a thread.
pub fn refresh_docker(shared: &Shared, ctx: egui::Context) {
    lock(shared).docker_busy = true;
    let shared = Arc::clone(shared);
    thread::spawn(move || {
        let usage = docker::df();
        let vm = docker::vm_disk();
        let mut s = lock(&shared);
        s.docker = Some(usage);
        s.docker_vm = vm;
        s.docker_busy = false;
        drop(s);
        ctx.request_repaint();
    });
}

/// Moves each path to the Trash, carrying on past failures; the error lists every path that
/// didn't move and why.
pub fn trash_paths(paths: &[PathBuf]) -> Result<(), String> {
    let failed: Vec<String> = paths
        .iter()
        .filter_map(|p| {
            if p.symlink_metadata().is_err() {
                return Some(format!("{}: no longer exists", p.display()));
            }
            trash::delete(p)
                .err()
                .map(|e| format!("{}: {e}", p.display()))
        })
        .collect();
    if failed.is_empty() {
        Ok(())
    } else {
        Err(failed.join("\n"))
    }
}

/// Trashes `paths` on a thread, then rescans `root`.
pub fn trash_then_rescan(
    shared: &Shared,
    root: PathBuf,
    paths: Vec<PathBuf>,
    ctx: egui::Context,
) -> Receiver<Result<(), String>> {
    let (tx, rx) = mpsc::channel();
    let shared = Arc::clone(shared);
    thread::spawn(move || {
        let _ = tx.send(trash_paths(&paths));
        start_scan(&shared, root, ctx.clone());
        ctx.request_repaint();
    });
    rx
}

/// Runs a prune on a thread, then refreshes the Docker numbers.
pub fn prune_then_refresh(
    shared: &Shared,
    p: Prune,
    ctx: egui::Context,
) -> Receiver<Result<String, String>> {
    let (tx, rx) = mpsc::channel();
    let shared = Arc::clone(shared);
    lock(&shared).docker_busy = true;
    thread::spawn(move || {
        let _ = tx.send(docker::prune(p));
        refresh_docker(&shared, ctx.clone());
        ctx.request_repaint();
    });
    rx
}
