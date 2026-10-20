//! Window layout: sidebar of pages on the left, the selected page on the right under a
//! header bar, as in Resources.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use egui::{Frame, RichText, Ui};
use resources_mac::icons;
use resources_mac::theme::palette;
use resources_mac::widgets::{header_bar, page_column, sidebar_item};

use crate::pages;
use crate::pages::folder::FolderView;
use crate::pages::settings::{Settings, SettingsAction};
use crate::worker::{self, Shared};

const SIDEBAR_W: f32 = 240.0;

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
}

fn folder_label(root: &std::path::Path) -> String {
    root.file_name().map_or_else(
        || root.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

pub struct App {
    shared: Shared,
    page: Page,
    settings: Settings,
    folders: HashMap<PathBuf, FolderView>,
}

impl App {
    pub fn new(shared: Shared) -> Self {
        Self {
            shared,
            page: Page::Overview,
            settings: Settings::default(),
            folders: HashMap::new(),
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
                .map(|r| (r.root.clone(), folder_label(&r.root)))
                .collect();
            if let Page::Folder(root) = &self.page
                && !folders.iter().any(|(r, _)| r == root)
            {
                self.page = Page::Overview;
            }
            if s.roots.iter().any(|r| r.scanning) || s.docker_busy {
                ctx.request_repaint_after(Duration::from_millis(250));
            }
            let p = palette(ui.visuals());

            egui::Panel::left("sidebar")
                .exact_size(SIDEBAR_W)
                .resizable(false)
                .frame(Frame::new().fill(p.sidebar).inner_margin(8))
                .show(ui, |ui| {
                    ui.add_space(6.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new(crate::APP_NAME).heading().strong())
                    });
                    ui.add_space(8.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            let mut item = |ui: &mut Ui, page: Page, icon: &str, label: &str| {
                                if sidebar_item(ui, icon, label, self.page == page, None).clicked()
                                {
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
                });

            egui::CentralPanel::default()
                .frame(Frame::new().fill(p.content))
                .show(ui, |ui| {
                    ui.add_space(14.0);
                    let (title, subtitle) = match &self.page {
                        Page::Overview => ("Overview".to_owned(), None),
                        Page::Folder(root) => {
                            (folder_label(root), Some(root.display().to_string()))
                        }
                        Page::Docker => ("Docker".to_owned(), None),
                        Page::Settings => ("Settings".to_owned(), None),
                    };
                    header_bar(ui, &title, subtitle.as_deref());
                    ui.add_space(6.0);
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
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            page_column(ui, |ui| match &self.page {
                                Page::Folder(_) => {
                                    if let Some(scan) = folder {
                                        let view =
                                            self.folders.entry(scan.root.clone()).or_default();
                                        pages::folder::show(ui, view, scan);
                                    }
                                }
                                Page::Settings => {
                                    action = pages::settings::show(ui, &mut self.settings)
                                        .map(Action::Settings);
                                }
                                Page::Overview => {
                                    action = pages::overview::show(ui, &s).map(Action::Open);
                                }
                                Page::Docker => {}
                            });
                            ui.add_space(24.0);
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
