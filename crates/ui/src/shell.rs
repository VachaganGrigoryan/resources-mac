//! Window layout shared by both apps: a sidebar of pages on the left, the selected page on the
//! right under a GNOME-style header bar. Call [`sidebar`] first, then [`content`].

use egui::{Frame, Pos2, RichText, Ui};

use crate::icons;
use crate::theme::palette;
use crate::widgets::{header_bar, icon_button, menu_item, page_column};

/// Sidebar width, as in the GNOME screenshots.
pub const SIDEBAR_W: f32 = 240.0;

/// One row of the sidebar's Main Menu.
pub enum MenuEntry<'a> {
    /// Icon and label; the label is what [`sidebar`] returns when it is clicked.
    Item(&'a str, &'a str),
    Separator,
}

/// Left panel: app name, an optional Main Menu button beside it, then `rows` in a scroll
/// area. Returns the label of the menu entry clicked this frame.
pub fn sidebar<'a>(
    ui: &mut Ui,
    app_name: &str,
    menu: &[MenuEntry<'a>],
    rows: impl FnOnce(&mut Ui),
) -> Option<&'a str> {
    let mut clicked = None;
    let p = palette(ui.visuals());
    egui::Panel::left("sidebar")
        .exact_size(SIDEBAR_W)
        .resizable(false)
        .frame(Frame::new().fill(p.sidebar).inner_margin(8))
        .show(ui, |ui| {
            // Same baseline as the page header; the traffic lights sit to the left.
            ui.add_space(6.0);
            let title = ui
                .vertical_centered(|ui| ui.label(RichText::new(app_name).heading().strong()))
                .inner
                .rect;
            if !menu.is_empty() {
                let at = egui::Rect::from_center_size(
                    egui::pos2(ui.max_rect().right() - 18.0, title.center().y),
                    egui::vec2(32.0, 32.0),
                );
                let button = ui.put(at, |ui: &mut Ui| icon_button(ui, icons::LIST, "Main Menu"));
                egui::Popup::menu(&button).show(|ui| {
                    ui.set_width(200.0);
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for entry in menu {
                        match entry {
                            MenuEntry::Item(icon, label) => {
                                if menu_item(ui, icon, label).clicked() {
                                    clicked = Some(*label);
                                }
                            }
                            MenuEntry::Separator => {
                                ui.separator();
                            }
                        }
                    }
                });
            }
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    rows(ui);
                });
        });
    clicked
}

/// The rest of the window, in the content colour.
pub fn content<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let p = palette(ui.visuals());
    egui::CentralPanel::default()
        .frame(Frame::new().fill(p.content))
        .show(ui, add)
        .inner
}

/// Page header bar. `extra` can place buttons over it; it gets the header's top-left corner.
pub fn header(ui: &mut Ui, title: &str, subtitle: Option<&str>, extra: impl FnOnce(&mut Ui, Pos2)) {
    ui.add_space(14.0);
    let top = ui.cursor().min;
    header_bar(ui, title, subtitle);
    extra(ui, top);
    ui.add_space(6.0);
}

/// A vertically scrolling page body in the centred page column.
pub fn scrolled_column<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            let r = page_column(ui, add);
            ui.add_space(24.0);
            r
        })
        .inner
}
