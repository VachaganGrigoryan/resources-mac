//! Overview: totals across every watched folder and Docker, and the biggest wins.

use egui::{Align, Layout, RichText, Ui};
use resources_ui::display_name;
use resources_ui::widgets::{card, fmt_bytes, group};

use crate::app::Page;
use crate::docker::Prune;
use crate::scan::Kind;
use crate::widgets::{stacked_bar, stat_tile};
use crate::worker::State;

/// Suggestions listed under "Top suggestions".
const TOP: usize = 10;

pub fn show(ui: &mut Ui, s: &State) -> Option<Page> {
    let mut open = None;
    let sum = |kinds: &[Kind]| -> u64 {
        s.roots
            .iter()
            .filter_map(|r| r.node())
            .map(|n| kinds.iter().map(|k| n.by_kind[k.index()]).sum::<u64>())
            .sum()
    };
    let scanned: u64 = s
        .roots
        .iter()
        .filter_map(|r| r.node())
        .map(|n| n.bytes)
        .sum();
    let (docker_value, docker_hint) = match &s.docker {
        Some(Ok(u)) => {
            // Dangling images are a subset of unused ones.
            let bytes: u64 = Prune::ALL
                .iter()
                .filter(|p| **p != Prune::DanglingImages)
                .map(|p| u.reclaimable(*p).1)
                .sum();
            (
                fmt_bytes(bytes),
                "unused images, volumes, stopped containers".to_owned(),
            )
        }
        Some(Err(_)) => ("—".to_owned(), "Docker isn't running".to_owned()),
        None => ("…".to_owned(), "checking Docker".to_owned()),
    };
    ui.columns(4, |cols| {
        stat_tile(
            &mut cols[0],
            "Scanned",
            &fmt_bytes(scanned),
            &format!("{} folders", s.roots.len()),
        );
        stat_tile(
            &mut cols[1],
            "Reclaimable",
            &fmt_bytes(sum(&[Kind::Cache])),
            "caches & builds",
        );
        stat_tile(
            &mut cols[2],
            "To review",
            &fmt_bytes(sum(&[Kind::Editor, Kind::Ignored])),
            "editor & gitignored files",
        );
        stat_tile(&mut cols[3], "Docker", &docker_value, &docker_hint);
    });

    if s.roots.is_empty() {
        group(ui, "Folders", |ui| {
            card(ui, |ui| {
                ui.label("Add a folder in Settings to start, such as your projects folder.");
                if ui.button("Open Settings").clicked() {
                    open = Some(Page::Settings);
                }
            });
        });
        return open;
    }

    group(ui, "Folders", |ui| {
        for r in &s.roots {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .link(RichText::new(display_name(&r.root)).strong())
                        .clicked()
                    {
                        open = Some(Page::Folder(r.root.clone()));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if let Some(n) = r.node() {
                            ui.label(fmt_bytes(n.bytes));
                        }
                        if r.scanning {
                            ui.spinner();
                        }
                    });
                });
                match &r.result {
                    Some(Ok(n)) => stacked_bar(ui, &n.by_kind, 10.0),
                    Some(Err(e)) => {
                        ui.colored_label(ui.visuals().error_fg_color, e);
                    }
                    None => {
                        ui.label(RichText::new("Scanning…").weak());
                    }
                }
            });
        }
    });

    let mut top: Vec<_> = s
        .roots
        .iter()
        .flat_map(|r| r.suggestions.iter().map(move |sg| (r, sg)))
        .filter(|(_, sg)| sg.kind == Kind::Cache)
        .collect();
    top.sort_by_key(|(_, sg)| std::cmp::Reverse(sg.bytes));
    top.truncate(TOP);
    if !top.is_empty() {
        group(ui, "Top suggestions", |ui| {
            card(ui, |ui| {
                egui::Grid::new("top-suggestions")
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        for (r, sg) in top {
                            let rel = sg.path.strip_prefix(&r.root).unwrap_or(&sg.path);
                            let name = format!("{}/{}", display_name(&r.root), rel.display());
                            if ui.link(name).clicked() {
                                open = Some(Page::Folder(r.root.clone()));
                            }
                            ui.label(fmt_bytes(sg.bytes));
                            ui.label(RichText::new(sg.label).weak());
                            ui.end_row();
                        }
                    });
            });
        });
    }
    open
}
