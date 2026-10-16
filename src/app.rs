//! Window layout: sidebar of pages on the left, the selected page on the right.

use std::sync::{Arc, Mutex};

use egui::Ui;

use crate::pages;
use crate::sampler::State;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Apps,
    Processes,
    Cpu,
    Memory,
    Gpu,
    Drives,
    Network,
    Battery,
}

impl Page {
    pub fn label(self) -> &'static str {
        match self {
            Page::Apps => "Apps",
            Page::Processes => "Processes",
            Page::Cpu => "Processor",
            Page::Memory => "Memory",
            Page::Gpu => "GPU",
            Page::Drives => "Drives",
            Page::Network => "Network",
            Page::Battery => "Battery",
        }
    }
}

pub struct App {
    state: Arc<Mutex<State>>,
    page: Page,
}

impl App {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self {
            state,
            page: Page::Apps,
        }
    }

    /// Pages that have data on this Mac, in sidebar order.
    fn pages(_s: &State) -> Vec<Page> {
        vec![
            Page::Apps,
            Page::Processes,
            Page::Cpu,
            Page::Memory,
            Page::Drives,
            Page::Network,
        ]
    }

    /// Draws one frame. Public so UI tests can drive it without a window.
    pub fn show(&mut self, ui: &mut Ui) {
        let state = Arc::clone(&self.state);
        let s = state.lock().unwrap_or_else(|e| e.into_inner());

        egui::Panel::left("sidebar")
            .exact_size(190.0)
            .resizable(false)
            .show(ui, |ui| {
                ui.add_space(8.0);
                for page in Self::pages(&s) {
                    let selected = self.page == page;
                    if ui
                        .add_sized(
                            [ui.available_width(), 28.0],
                            egui::Button::selectable(selected, page.label()),
                        )
                        .clicked()
                    {
                        self.page = page;
                    }
                }
            });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading(self.page.label());
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| match self.page {
                    Page::Cpu => pages::cpu::show(ui, &s),
                    Page::Memory => pages::memory::show(ui, &s),
                    _ => {
                        ui.weak("Collecting…");
                    }
                });
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}
