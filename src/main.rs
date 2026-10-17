use std::sync::{Arc, Mutex};

use resources_mac::app::App;
use resources_mac::pages::settings::Settings;
use resources_mac::sampler::{self, State};
use resources_mac::theme;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(resources_mac::menubar::MAIN_WINDOW)
            .with_icon(
                eframe::icon_data::from_png_bytes(resources_mac::APP_ICON_PNG)
                    .expect("embedded icon is a valid PNG"),
            )
            .with_inner_size([1000.0, 680.0])
            .with_min_inner_size([720.0, 480.0])
            // GNOME-style: the sidebar and header bar run under a transparent title bar.
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false),
        // A monitor stays open for hours: never wake the discrete GPU on dual-GPU Intel MacBooks.
        wgpu_options: eframe::WgpuConfiguration {
            wgpu_setup: eframe::egui_wgpu::WgpuSetupCreateNew {
                power_preference: eframe::wgpu::PowerPreference::LowPower,
                ..eframe::egui_wgpu::WgpuSetupCreateNew::without_display_handle()
            }
            .into(),
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "Resources",
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            let state = Arc::new(Mutex::new(State::default()));
            sampler::spawn(Arc::clone(&state), cc.egui_ctx.clone());
            let settings: Settings = cc
                .storage
                .and_then(|st| eframe::get_value(st, Settings::KEY))
                .unwrap_or_default();
            cc.egui_ctx.set_theme(settings.theme);
            Ok(Box::new(App::new(state).with_settings(settings)))
        }),
    )
}
