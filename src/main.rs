struct Placeholder;

impl eframe::App for Placeholder {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |_| {});
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Resources")
            .with_inner_size([1000.0, 680.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Resources",
        options,
        Box::new(|_| Ok(Box::new(Placeholder))),
    )
}
