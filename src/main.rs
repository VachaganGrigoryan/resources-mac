use std::sync::{Arc, Mutex};

use resources_mac::app::App;
use resources_mac::pages::settings::Settings;
use resources_mac::sampler::{self, State};
use resources_mac::theme;

fn main() -> eframe::Result {
    let options = resources_ui::native_options(
        resources_mac::menubar::MAIN_WINDOW,
        resources_mac::APP_ICON_PNG,
    );
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
