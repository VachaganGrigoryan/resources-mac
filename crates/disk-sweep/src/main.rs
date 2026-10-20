use std::sync::{Arc, Mutex};

use disk_sweep::APP_NAME;
use disk_sweep::app::App;
use disk_sweep::pages::settings::Settings;
use disk_sweep::worker::{self, State};

fn main() -> eframe::Result {
    let options = resources_ui::native_options(APP_NAME, disk_sweep::APP_ICON_PNG);
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| {
            resources_ui::theme::apply(&cc.egui_ctx);
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
