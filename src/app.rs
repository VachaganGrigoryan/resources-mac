//! Window layout: sidebar of pages with live mini graphs on the left, the selected page on
//! the right under a GNOME-style header bar.

use std::sync::{Arc, Mutex};

use egui::{Frame, RichText, Ui};

use crate::devices::{self, battery_display_name, drive_display_name, net_display_name};
use crate::icons;
use crate::pages;
use crate::sampler::{NetStat, State};
use crate::theme::{color, palette};
use crate::widgets::{Series, header_bar, icon_button, page_column, sidebar_item};

/// Sidebar width, as in the GNOME screenshots.
const SIDEBAR_W: f32 = 240.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Apps,
    Processes,
    Cpu,
    Memory,
    Gpu,
    /// Keyed by mount point.
    Drive(String),
    /// Keyed by interface name (`en0`).
    Network(String),
    /// All system tunnels merged into one entry.
    Vpn,
    Battery,
}

/// One sidebar row: where it leads, how it looks and what its mini graph plots.
struct Entry<'a> {
    page: Page,
    icon: &'static str,
    label: String,
    graph: Option<(Vec<Series<'a>>, Option<f64>)>,
}

fn net_label(n: &NetStat) -> (String, &'static str) {
    net_display_name(&n.name, devices::port(n))
}

/// Sidebar rows for everything this Mac reports, in upstream's order.
fn entries(s: &State) -> Vec<Entry<'_>> {
    let mut out = vec![
        Entry {
            page: Page::Apps,
            icon: icons::APP_WINDOW,
            label: "Apps".into(),
            graph: None,
        },
        Entry {
            page: Page::Processes,
            icon: icons::LIST_BULLETS,
            label: "Processes".into(),
            graph: None,
        },
        Entry {
            page: Page::Cpu,
            icon: icons::CPU,
            label: "Processor".into(),
            graph: Some((vec![(&s.cpu_total, color::CPU)], Some(100.0))),
        },
        Entry {
            page: Page::Memory,
            icon: icons::MEMORY,
            label: "Memory".into(),
            graph: Some((vec![(&s.mem_hist, color::MEMORY)], Some(100.0))),
        },
    ];
    if let Some(g) = &s.gpu {
        out.push(Entry {
            page: Page::Gpu,
            icon: icons::GRAPHICS_CARD,
            label: "GPU".into(),
            graph: Some((vec![(&g.util_hist, color::GPU)], Some(100.0))),
        });
    }
    for d in &s.disks {
        let (label, icon) = drive_display_name(d.total, d.removable);
        out.push(Entry {
            page: Page::Drive(d.mount.clone()),
            icon,
            label,
            graph: Some((
                vec![(&d.read_hist, color::DRIVE), (&d.write_hist, color::WRITE)],
                None,
            )),
        });
    }
    let separate = s
        .nets
        .iter()
        .filter(|n| devices::visible(n) && !devices::is_tunnel(&n.name, devices::port(n)));
    for n in separate {
        let (label, icon) = net_label(n);
        out.push(Entry {
            page: Page::Network(n.name.clone()),
            icon,
            label,
            graph: Some((
                vec![(&n.rx_hist, color::NETWORK), (&n.tx_hist, color::TX)],
                None,
            )),
        });
    }
    if let Some(v) = &s.vpn {
        out.push(Entry {
            page: Page::Vpn,
            icon: icons::SHIELD_CHECK,
            label: "VPN Connection".into(),
            graph: Some((
                vec![(&v.rx_hist, color::NETWORK), (&v.tx_hist, color::TX)],
                None,
            )),
        });
    }
    // Two drives of the same size read alike; tell same-named devices apart.
    let labels: Vec<String> = out.iter().map(|e| e.label.clone()).collect();
    for e in &mut out {
        if labels.iter().filter(|l| **l == e.label).count() > 1 {
            match &e.page {
                Page::Drive(key) | Page::Network(key) => e.label = format!("{} ({key})", e.label),
                _ => {}
            }
        }
    }
    if let Some(b) = &s.battery {
        out.push(Entry {
            page: Page::Battery,
            icon: icons::BATTERY_HIGH,
            label: battery_display_name(b.design_wh),
            graph: Some((vec![(&b.charge_hist, color::BATTERY)], Some(100.0))),
        });
    }
    out
}

pub struct App {
    state: Arc<Mutex<State>>,
    page: Page,
    apps: pages::apps::AppsView,
    procs: pages::processes::ProcView,
    about: bool,
}

impl App {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self {
            state,
            page: Page::Apps,
            apps: Default::default(),
            procs: Default::default(),
            about: false,
        }
    }

    /// Header title and subtitle for the current page.
    fn title(&self, s: &State) -> (String, Option<String>) {
        match &self.page {
            Page::Apps => ("Apps".into(), None),
            Page::Processes => ("Processes".into(), None),
            Page::Cpu if !s.cpu_model.is_empty() => (s.cpu_model.clone(), Some("Processor".into())),
            Page::Cpu => ("Processor".into(), None),
            Page::Memory => ("Memory".into(), None),
            Page::Gpu => (
                s.gpu.as_ref().map(|g| g.name.clone()).unwrap_or_default(),
                Some("GPU".into()),
            ),
            Page::Drive(mount) => s
                .disks
                .iter()
                .find(|d| &d.mount == mount)
                .map(|d| {
                    (
                        d.name.clone(),
                        Some(drive_display_name(d.total, d.removable).0),
                    )
                })
                .unwrap_or_default(),
            Page::Network(name) => s
                .nets
                .iter()
                .find(|n| &n.name == name)
                .map(|n| (n.name.clone(), Some(net_label(n).0)))
                .unwrap_or_default(),
            Page::Vpn => (s.vpn_members.join(", "), Some("VPN Connection".into())),
            Page::Battery => s
                .battery
                .as_ref()
                .map(|b| {
                    let name = battery_display_name(b.design_wh);
                    (b.model.clone().unwrap_or_else(|| name.clone()), Some(name))
                })
                .unwrap_or_default(),
        }
    }

    /// Draws one frame. Public so UI tests can drive it without a window.
    pub fn show(&mut self, ui: &mut Ui) {
        let state = Arc::clone(&self.state);
        let s = state.lock().unwrap_or_else(|e| e.into_inner());
        let entries = entries(&s);
        // A drive was ejected or an interface went away while its page was open.
        if !entries.iter().any(|e| e.page == self.page) {
            self.page = Page::Apps;
        }
        let p = palette(ui.visuals());

        egui::Panel::left("sidebar")
            .exact_size(SIDEBAR_W)
            .resizable(false)
            .frame(Frame::new().fill(p.sidebar).inner_margin(8))
            .show(ui, |ui| {
                // Same baseline as the page header; the traffic lights sit to the left.
                ui.add_space(6.0);
                let title = ui
                    .vertical_centered(|ui| ui.label(RichText::new("Resources").strong()))
                    .inner
                    .rect;
                let at = egui::Rect::from_center_size(
                    egui::pos2(ui.max_rect().right() - 18.0, title.center().y),
                    egui::vec2(32.0, 32.0),
                );
                let menu = ui.put(at, |ui: &mut Ui| icon_button(ui, icons::LIST, "Main Menu"));
                egui::Popup::menu(&menu).show(|ui| {
                    if ui.button("About Resources").clicked() {
                        self.about = true;
                    }
                });
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for e in &entries {
                            let graph = e.graph.as_ref().map(|(g, max)| (g.as_slice(), *max));
                            let selected = self.page == e.page;
                            if sidebar_item(ui, e.icon, &e.label, selected, graph).clicked() {
                                self.page = e.page.clone();
                            }
                        }
                    });
            });

        egui::CentralPanel::default()
            .frame(Frame::new().fill(p.content))
            .show(ui, |ui| {
                let (title, subtitle) = self.title(&s);
                ui.add_space(14.0);
                let top = ui.cursor().min;
                header_bar(ui, &title, subtitle.as_deref());
                let search = match self.page {
                    Page::Apps => Some(&mut self.apps.search),
                    Page::Processes => Some(&mut self.procs.search),
                    _ => None,
                };
                if let Some(search) = search {
                    let at = egui::Rect::from_min_size(
                        top + egui::vec2(12.0, -4.0),
                        egui::vec2(32.0, 32.0),
                    );
                    let button = |ui: &mut Ui| icon_button(ui, icons::MAGNIFYING_GLASS, "Search");
                    if ui.put(at, button).clicked() {
                        search.toggle();
                    }
                }
                ui.add_space(6.0);
                if self.about {
                    self.about = about(ui);
                }
                match &self.page {
                    Page::Apps => self.apps.show(ui, &s),
                    Page::Processes => self.procs.show(ui, &s),
                    page => {
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                page_column(ui, |ui| detail(ui, page, &s));
                                ui.add_space(24.0);
                            });
                    }
                }
            });
    }
}

/// About dialog; returns whether it stays open.
fn about(ui: &Ui) -> bool {
    let mut open = true;
    let resp = egui::Modal::new(egui::Id::unique("about")).show(ui.ctx(), |ui| {
        ui.set_width(320.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(icons::CPU).size(40.0).color(color::CPU));
            ui.label(RichText::new("Resources").size(20.0).strong());
            ui.label(RichText::new(env!("CARGO_PKG_VERSION")).weak());
            ui.add_space(8.0);
            ui.label("A system monitor for macOS, inspired by GNOME Resources.");
            ui.add_space(8.0);
            ui.label(RichText::new("github.com/VachaganGrigoryan/resources-mac").small());
            ui.label(
                RichText::new("GPL-3.0-or-later · Icons: Phosphor (MIT)")
                    .small()
                    .weak(),
            );
            ui.add_space(8.0);
            if ui.button("Close").clicked() {
                open = false;
            }
        });
    });
    open && !resp.should_close()
}

/// Body of a resource page; the device pages look their device up by key.
fn detail(ui: &mut Ui, page: &Page, s: &State) {
    match page {
        Page::Cpu => pages::cpu::show(ui, s),
        Page::Memory => pages::memory::show(ui, s),
        Page::Gpu => {
            if let Some(g) = &s.gpu {
                pages::gpu::show(ui, g);
            }
        }
        Page::Drive(mount) => {
            if let Some(d) = s.disks.iter().find(|d| &d.mount == mount) {
                pages::drives::show(ui, d);
            }
        }
        Page::Network(name) => {
            if let Some(n) = s.nets.iter().find(|n| &n.name == name) {
                pages::network::show(ui, n, &[]);
            }
        }
        Page::Vpn => {
            if let Some(v) = &s.vpn {
                let members: Vec<&NetStat> = s
                    .nets
                    .iter()
                    .filter(|n| s.vpn_members.contains(&n.name))
                    .collect();
                pages::network::show(ui, v, &members);
            }
        }
        Page::Battery => {
            if let Some(b) = &s.battery {
                pages::battery::show(ui, b);
            }
        }
        Page::Apps | Page::Processes => {}
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}
