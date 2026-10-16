use std::sync::{Arc, Mutex};

use resources_mac::app::App;
use resources_mac::sampler::{self, State};
use resources_mac::theme;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Resources")
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
            Ok(Box::new(App::new(state)))
        }),
    )
}
