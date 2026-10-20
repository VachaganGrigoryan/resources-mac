//! Docker: what images, containers, volumes and build cache take, and the prunes that free it.

use std::sync::mpsc::Receiver;

use egui::{Align, Layout, RichText, Ui};
use resources_ui::confirm::Confirm;
use resources_ui::icons;
use resources_ui::widgets::{
    card, destructive_button, fmt_bytes, group, icon_button, names_preview,
};

use crate::docker::{Prune, Usage};
use crate::widgets::stat_tile;
use crate::worker::State;

#[derive(Default)]
pub struct DockerView {
    pub confirm: Confirm<Prune>,
    /// The command the open dialog shows; confirming runs exactly this.
    pub asked: Vec<String>,
    /// The removal in flight.
    pub pending: Option<Receiver<Result<String, String>>>,
    /// Outcome of the last prune ("Total reclaimed space: 1.2GB").
    pub last: Option<String>,
}

pub enum DockerAction {
    Refresh,
    /// Arguments after `docker`.
    Run(Vec<String>),
}

/// Name, detail and size of every item `p` would remove.
fn items(u: &Usage, p: Prune) -> Vec<(String, String, u64)> {
    match p {
        Prune::DanglingImages | Prune::UnusedImages => u
            .images_for(p)
            .map(|i| {
                (
                    format!("{}:{}", i.repository, i.tag),
                    i.created_since.clone(),
                    i.unique_size,
                )
            })
            .collect(),
        Prune::StoppedContainers => u
            .stopped_containers()
            .map(|c| {
                (
                    c.name.clone(),
                    format!("{} · {}", c.image, c.status),
                    c.size,
                )
            })
            .collect(),
        Prune::UnusedVolumes => u
            .unused_volumes()
            .map(|v| {
                (
                    v.name.clone(),
                    "not used by any container".to_owned(),
                    v.size,
                )
            })
            .collect(),
        Prune::BuildCache => u
            .idle_build_cache()
            .map(|b| (b.id.clone(), format!("last used {}", b.last_used), b.size))
            .collect(),
    }
}

/// The confirm question: what goes, that it is permanent, and the command it runs.
fn question(p: Prune, u: &Usage, command: &[String]) -> String {
    let (n, bytes) = u.reclaimable(p);
    let mut q = format!("{} ({n}, {})?\n", button_label(p), fmt_bytes(bytes));
    let rows = items(u, p);
    let names: Vec<&str> = rows.iter().map(|(name, _, _)| name.as_str()).collect();
    q.push_str(&names_preview(&names));
    let shown: Vec<&str> = command.iter().take(3).map(String::as_str).collect();
    let more = if command.len() > 3 { " …" } else { "" };
    q.push_str(&format!(
        "\n\nThis permanently deletes them; it can't be undone.\nRuns `docker {}{more}`.",
        shown.join(" ")
    ));
    q
}

fn button_label(p: Prune) -> String {
    format!("Remove {}", p.label().to_lowercase())
}

pub fn show(ui: &mut Ui, view: &mut DockerView, s: &State) -> Option<DockerAction> {
    let mut action = None;
    if let Some(rx) = &view.pending
        && let Ok(result) = rx.try_recv()
    {
        view.last = Some(match &result {
            Ok(line) if line.contains("reclaimed") => line.clone(),
            Ok(_) => "Removed.".to_owned(),
            Err(e) => format!("Docker couldn't remove everything: {e}"),
        });
        view.confirm.finish(result.map(|_| ()));
        view.pending = None;
    }

    ui.horizontal(|ui| {
        if let Some(last) = &view.last {
            ui.label(RichText::new(last).weak());
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if icon_button(ui, icons::ARROWS_CLOCKWISE, "Refresh").clicked() {
                action = Some(DockerAction::Refresh);
            }
            if s.docker_busy || view.pending.is_some() {
                ui.spinner();
            }
        });
    });

    let usage = match &s.docker {
        None => {
            ui.label(RichText::new("Checking Docker…").weak());
            return action;
        }
        Some(Err(e)) => {
            card(ui, |ui| {
                ui.label(RichText::new("Docker isn't available").strong());
                ui.colored_label(ui.visuals().error_fg_color, e);
                ui.label(
                    RichText::new("Start Docker Desktop (or the daemon) and retry.")
                        .small()
                        .weak(),
                );
                if ui.button("Retry").clicked() {
                    action = Some(DockerAction::Refresh);
                }
            });
            return action;
        }
        Some(Ok(u)) => u,
    };

    if !usage.summary.is_empty() {
        ui.columns(usage.summary.len(), |cols| {
            for (col, c) in cols.iter_mut().zip(&usage.summary) {
                stat_tile(
                    col,
                    &c.kind,
                    &fmt_bytes(c.size),
                    &format!(
                        "{} reclaimable · {} of {} in use",
                        fmt_bytes(c.reclaimable),
                        c.active,
                        c.count
                    ),
                );
            }
        });
    }

    if let Some(vm) = &s.docker_vm {
        group(ui, "Docker Desktop disk", |ui| {
            card(ui, |ui| {
                ui.label(format!(
                    "{} used on this Mac; the disk image can grow to {}.",
                    fmt_bytes(vm.allocated),
                    fmt_bytes(vm.apparent)
                ));
                ui.label(
                    RichText::new(
                        "Space freed by a prune goes back to macOS once Docker Desktop trims \
                         the disk image, which it does in the background.",
                    )
                    .small()
                    .weak(),
                );
            });
        });
    }

    group(ui, "Clean up", |ui| {
        for p in Prune::ALL {
            let (n, bytes) = usage.reclaimable(p);
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(p.label()).strong());
                    ui.label(RichText::new(format!("{n} · {}", fmt_bytes(bytes))).weak());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let enabled = n > 0 && view.pending.is_none();
                        if destructive_button(ui, &button_label(p), enabled).clicked() {
                            view.asked = usage.command(p);
                            view.confirm.ask(p, question(p, usage, &view.asked));
                        }
                    });
                });
                let rows = items(usage, p);
                if !rows.is_empty() {
                    egui::CollapsingHeader::new(format!("Show {n}"))
                        .id_salt(p.label())
                        .show(ui, |ui| {
                            egui::Grid::new(p.label())
                                .num_columns(3)
                                .striped(true)
                                .show(ui, |ui| {
                                    for (name, detail, size) in rows {
                                        ui.add(egui::Label::new(name).truncate());
                                        ui.label(RichText::new(detail).weak());
                                        ui.label(fmt_bytes(size));
                                        ui.end_row();
                                    }
                                });
                        });
                }
            });
        }
    });

    if view.confirm.show(ui, "Remove").is_some() && view.pending.is_none() {
        action = Some(DockerAction::Run(view.asked.clone()));
    }
    action
}
