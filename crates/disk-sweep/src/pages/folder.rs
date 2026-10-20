//! One watched folder: scan progress or error, where the space is, and what can go to the
//! Trash.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;

use egui::{Align, Layout, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use resources_mac::table::Confirm;
use resources_mac::widgets::{card, destructive_button, fmt_bytes, group};

use crate::scan::{Kind, Node};
use crate::suggest::{Suggestion, default_selected};
use crate::treemap;
use crate::widgets::{legend, stacked_bar, stat_tile};
use crate::worker::RootScan;

#[derive(Clone, Copy)]
pub enum FolderAction {
    Trash,
}

#[derive(Default)]
pub struct FolderView {
    pub selected: HashSet<PathBuf>,
    /// Child indices from the root down to the folder the treemap shows.
    pub zoom: Vec<usize>,
    pub confirm: Confirm<FolderAction>,
    /// The Trash job in flight.
    pub pending: Option<Receiver<Result<(), String>>>,
    /// Scan generation the selection was made for.
    seen: u64,
}

fn plural(n: usize, one: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {one}s")
    }
}

impl FolderView {
    /// Resets the selection to the defaults when a new scan result arrives.
    pub fn sync(&mut self, scan: &RootScan) {
        if scan.scanning || scan.node().is_none() || scan.generation == self.seen {
            return;
        }
        self.seen = scan.generation;
        self.zoom.clear();
        self.selected = scan
            .suggestions
            .iter()
            .filter(|s| default_selected(s))
            .map(|s| s.path.clone())
            .collect();
    }

    fn picked<'a>(&self, scan: &'a RootScan) -> Vec<&'a Suggestion> {
        scan.suggestions
            .iter()
            .filter(|s| self.selected.contains(&s.path))
            .collect()
    }
}

/// Bottom bar with the selection total and "Move to Trash"; returns the paths to trash once
/// the user confirms.
pub fn bar(ui: &mut Ui, view: &mut FolderView, scan: &RootScan) -> Option<Vec<PathBuf>> {
    if let Some(rx) = &view.pending
        && let Ok(result) = rx.try_recv()
    {
        if result.is_ok() {
            view.selected.clear();
        }
        view.confirm.finish(result);
        view.pending = None;
    }
    let picked = view.picked(scan);
    let bytes: u64 = picked.iter().map(|s| s.bytes).sum();
    egui::Panel::bottom(ui.scope_id().with("folder-bar"))
        .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(16, 10)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let busy = view.pending.is_some();
                let count = if busy {
                    "Moving to the Trash…".to_owned()
                } else {
                    format!(
                        "{} · {} selected",
                        plural(picked.len(), "item"),
                        fmt_bytes(bytes)
                    )
                };
                ui.label(RichText::new(count).weak());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let enabled = !picked.is_empty() && !busy && !scan.scanning;
                    if destructive_button(ui, "Move to Trash", enabled).clicked() {
                        let mut question = format!(
                            "Move {} ({}) to the Trash?\n",
                            plural(picked.len(), "item"),
                            fmt_bytes(bytes)
                        );
                        for s in picked.iter().take(5) {
                            question.push_str(&format!("\n{}", relative(scan, s)));
                        }
                        if picked.len() > 5 {
                            question.push_str(&format!("\n… and {} more", picked.len() - 5));
                        }
                        view.confirm.ask(FolderAction::Trash, question);
                    }
                });
            });
        });
    let confirmed = view.confirm.show(ui, "Move to Trash").is_some() && view.pending.is_none();
    confirmed.then(|| picked.iter().map(|s| s.path.clone()).collect())
}

fn relative(scan: &RootScan, s: &Suggestion) -> String {
    s.path
        .strip_prefix(&scan.root)
        .unwrap_or(&s.path)
        .display()
        .to_string()
}

/// The folder the treemap shows; drops zoom steps that no longer exist after a rescan.
fn zoomed<'a>(root: &'a Node, zoom: &mut Vec<usize>) -> &'a Node {
    let mut node = root;
    for (depth, &i) in zoom.iter().enumerate() {
        match node.children.get(i) {
            Some(child) => node = child,
            None => {
                zoom.truncate(depth);
                break;
            }
        }
    }
    node
}

pub fn show(ui: &mut Ui, view: &mut FolderView, scan: &RootScan) {
    if scan.scanning {
        let files = scan.progress.files.load(Ordering::Relaxed);
        let bytes = scan.progress.bytes.load(Ordering::Relaxed);
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(
                RichText::new(format!("Scanning… {files} files · {}", fmt_bytes(bytes))).weak(),
            );
        });
    }
    if let Some(Err(e)) = &scan.result {
        card(ui, |ui| {
            ui.label(RichText::new("Couldn't scan this folder").strong());
            ui.colored_label(ui.visuals().error_fg_color, e);
        });
    }
    let Some(root) = scan.node() else {
        return;
    };
    let errors = scan.progress.errors.load(Ordering::Relaxed);

    stacked_bar(ui, &root.by_kind, 14.0);
    legend(ui, &root.by_kind);
    let cache = root.by_kind[Kind::Cache.index()];
    let review = root.by_kind[Kind::Editor.index()] + root.by_kind[Kind::Ignored.index()];
    ui.columns(3, |cols| {
        let skipped = if errors > 0 {
            format!(" · {errors} unreadable")
        } else {
            String::new()
        };
        stat_tile(
            &mut cols[0],
            "Total",
            &fmt_bytes(root.bytes),
            &format!("{} files{skipped}", root.files),
        );
        stat_tile(
            &mut cols[1],
            "Caches & builds",
            &fmt_bytes(cache),
            "re-created by installs and builds",
        );
        stat_tile(
            &mut cols[2],
            "To review",
            &fmt_bytes(review),
            "editor files and other gitignored files",
        );
    });

    group(ui, "Where the space is", |ui| {
        let node = zoomed(root, &mut view.zoom);
        ui.horizontal_wrapped(|ui| {
            if ui.link(&root.name).clicked() {
                view.zoom.clear();
            }
            let mut at = root;
            for depth in 0..view.zoom.len() {
                at = &at.children[view.zoom[depth]];
                ui.label("›");
                if ui.link(&at.name).clicked() {
                    view.zoom.truncate(depth + 1);
                    break;
                }
            }
        });
        if let Some(i) = treemap::show(ui, node, 260.0) {
            view.zoom.push(i);
        }
    });

    let (safe, other): (Vec<&Suggestion>, Vec<&Suggestion>) =
        scan.suggestions.iter().partition(|s| s.kind == Kind::Cache);
    group(ui, "Safe to delete", |ui| {
        table(ui, "safe", &safe, view, scan);
    });
    group(ui, "Review before deleting", |ui| {
        ui.label(
            RichText::new("Editor settings and files your .gitignore excludes: these may hold local config or data.")
                .small()
                .weak(),
        );
        table(ui, "review", &other, view, scan);
    });
}

/// Label-less checkbox that screen readers and tests still know as `name`.
fn check(ui: &mut Ui, on: &mut bool, name: &str) -> bool {
    let resp = ui.checkbox(on, "");
    let now = *on;
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, now, name));
    resp.changed()
}

fn table(ui: &mut Ui, id: &str, rows: &[&Suggestion], view: &mut FolderView, scan: &RootScan) {
    if rows.is_empty() {
        card(ui, |ui| ui.label(RichText::new("Nothing here.").weak()));
        return;
    }
    card(ui, |ui| {
        let all = rows.iter().all(|s| view.selected.contains(&s.path));
        TableBuilder::new(ui)
            .id_salt(id)
            .vscroll(false)
            .striped(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::exact(24.0))
            .column(Column::remainder().at_least(160.0).clip(true))
            .column(Column::exact(80.0))
            .column(Column::initial(150.0).clip(true))
            .column(Column::initial(170.0).clip(true))
            .header(22.0, |mut h| {
                h.col(|ui| {
                    let mut on = all;
                    if check(ui, &mut on, "Select all") {
                        for s in rows {
                            if on {
                                view.selected.insert(s.path.clone());
                            } else {
                                view.selected.remove(&s.path);
                            }
                        }
                    }
                });
                for title in ["Item", "Size", "What it is", "How to get it back"] {
                    h.col(|ui| {
                        ui.label(RichText::new(title).small().weak());
                    });
                }
            })
            .body(|body| {
                body.rows(24.0, rows.len(), |mut row| {
                    let s = rows[row.index()];
                    row.col(|ui| {
                        let mut on = view.selected.contains(&s.path);
                        if check(ui, &mut on, &format!("Select {}", relative(scan, s))) {
                            if on {
                                view.selected.insert(s.path.clone());
                            } else {
                                view.selected.remove(&s.path);
                            }
                        }
                    });
                    row.col(|ui| {
                        ui.label(relative(scan, s)).on_hover_text(format!(
                            "{}\n{} files",
                            s.path.display(),
                            s.files
                        ));
                    });
                    row.col(|ui| {
                        ui.label(fmt_bytes(s.bytes));
                    });
                    row.col(|ui| {
                        ui.label(s.label);
                    });
                    row.col(|ui| {
                        ui.label(RichText::new(s.restore.unwrap_or("—")).weak());
                    });
                });
            });
    });
}
