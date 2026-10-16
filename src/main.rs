use std::sync::{Arc, Mutex};

use resources_mac::app::App;
use resources_mac::sampler::{self, State};
use resources_mac::theme;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Resources")
            .with_inner_size([1000.0, 680.0])
            .with_min_inner_size([720.0, 480.0]),
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
