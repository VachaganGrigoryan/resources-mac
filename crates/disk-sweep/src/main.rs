use std::sync::{Arc, Mutex};

use disk_sweep::APP_NAME;
use disk_sweep::app::App;
use disk_sweep::pages::settings::Settings;
use disk_sweep::worker::{self, State};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_icon(
                eframe::icon_data::from_png_bytes(resources_mac::APP_ICON_PNG)
                    .expect("embedded icon is a valid PNG"),
            )
            .with_inner_size([1000.0, 680.0])
            .with_min_inner_size([720.0, 480.0])
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false),
        // Same as Resources: never wake the discrete GPU on dual-GPU Intel MacBooks.
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
        APP_NAME,
        options,
        Box::new(|cc| {
            resources_mac::theme::apply(&cc.egui_ctx);
            let settings: Settings = cc
                .storage
                .and_then(|st| eframe::get_value(st, Settings::KEY))
                .unwrap_or_default();
            cc.egui_ctx.set_theme(settings.theme);
            let shared = Arc::new(Mutex::new(State::default()));
            for root in &settings.roots {
                worker::start_scan(&shared, root.clone(), cc.egui_ctx.clone());
            }
            worker::refresh_docker(&shared, cc.egui_ctx.clone());
            Ok(Box::new(App::new(shared).with_settings(settings)))
        }),
    )
}
