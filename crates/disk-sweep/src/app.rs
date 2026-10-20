//! Window layout: sidebar of pages on the left, the selected page on the right under a
//! header bar, as in Resources.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use egui::Ui;
use resources_ui::widgets::sidebar_item;
use resources_ui::{display_name, icons, shell};

use crate::pages;
use crate::pages::docker::{DockerAction, DockerView};
use crate::pages::folder::FolderView;
use crate::pages::settings::{Settings, SettingsAction};
use crate::worker::{self, Shared};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Overview,
    /// Keyed by the watched folder.
    Folder(PathBuf),
    Docker,
    Settings,
}

/// Something a page asked for, run after the state lock is released.
pub enum Action {
    Open(Page),
    Settings(SettingsAction),
    Trash(PathBuf, Vec<PathBuf>),
    Docker(DockerAction),
}

pub struct App {
    shared: Shared,
    page: Page,
    settings: Settings,
    folders: HashMap<PathBuf, FolderView>,
    docker: DockerView,
}

impl App {
    pub fn new(shared: Shared) -> Self {
        Self {
            shared,
            page: Page::Overview,
            settings: Settings::default(),
            folders: HashMap::new(),
            docker: DockerView::default(),
        }
    }

    pub fn with_settings(mut self, settings: Settings) -> Self {
        self.settings = settings;
        self
    }

    /// Draws one frame. Public so UI tests can drive it without a window.
    pub fn show(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let mut action = None;
        {
            let s = worker::lock(&self.shared);
            let folders: Vec<(PathBuf, String)> = s
                .roots
                .iter()
                .map(|r| (r.root.clone(), display_name(&r.root)))
                .collect();
            if let Page::Folder(root) = &self.page
                && !folders.iter().any(|(r, _)| r == root)
            {
                self.page = Page::Overview;
            }
            if s.roots.iter().any(|r| r.scanning) || s.docker_busy {
                ctx.request_repaint_after(Duration::from_millis(250));
            }

            shell::sidebar(ui, crate::APP_NAME, &[], |ui| {
                let mut item = |ui: &mut Ui, page: Page, icon: &str, label: &str| {
                    if sidebar_item(ui, icon, label, self.page == page, None).clicked() {
                        self.page = page;
                    }
                };
                item(ui, Page::Overview, icons::BROOM, "Overview");
                for (root, label) in &folders {
                    item(ui, Page::Folder(root.clone()), icons::FOLDER, label);
                }
                item(ui, Page::Docker, icons::SHIPPING_CONTAINER, "Docker");
                item(ui, Page::Settings, icons::GEAR_SIX, "Settings");
            });

            shell::content(ui, |ui| {
                let (title, subtitle) = match &self.page {
                    Page::Overview => ("Overview".to_owned(), None),
                    Page::Folder(root) => (display_name(root), Some(root.display().to_string())),
                    Page::Docker => ("Docker".to_owned(), None),
                    Page::Settings => ("Settings".to_owned(), None),
                };
                shell::header(ui, &title, subtitle.as_deref(), |_, _| {});
                let folder = match &self.page {
                    Page::Folder(root) => s.roots.iter().find(|r| &r.root == root),
                    _ => None,
                };
                if let Some(scan) = folder {
                    let view = self.folders.entry(scan.root.clone()).or_default();
                    view.sync(scan);
                    if let Some(paths) = pages::folder::bar(ui, view, scan) {
                        action = Some(Action::Trash(scan.root.clone(), paths));
                    }
                }
                shell::scrolled_column(ui, |ui| match &self.page {
                    Page::Folder(_) => {
                        if let Some(scan) = folder {
                            let view = self.folders.entry(scan.root.clone()).or_default();
                            pages::folder::show(ui, view, scan);
                        }
                    }
                    Page::Settings => {
                        action =
                            pages::settings::show(ui, &mut self.settings).map(Action::Settings);
                    }
                    Page::Overview => {
                        action = pages::overview::show(ui, &s).map(Action::Open);
                    }
                    Page::Docker => {
                        action = pages::docker::show(ui, &mut self.docker, &s).map(Action::Docker);
                    }
                });
            });
        }
        match action {
            Some(Action::Open(page)) => self.page = page,
            Some(Action::Settings(SettingsAction::Added(root))) => {
                worker::start_scan(&self.shared, root, ctx);
            }
            Some(Action::Settings(SettingsAction::Removed(root))) => {
                worker::remove_root(&self.shared, &root);
                self.folders.remove(&root);
            }
            Some(Action::Docker(DockerAction::Refresh)) => {
                worker::refresh_docker(&self.shared, ctx);
            }
            Some(Action::Docker(DockerAction::Run(args))) => {
                self.docker.pending = Some(worker::prune_then_refresh(&self.shared, args, ctx));
            }
            Some(Action::Trash(root, paths)) => {
                let rx = worker::trash_then_rescan(&self.shared, root.clone(), paths, ctx);
                self.folders.entry(root).or_default().pending = Some(rx);
            }
            None => {}
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, Settings::KEY, &self.settings);
    }
}
