use disk_sweep::APP_NAME;

struct Shell;

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        resources_mac::widgets::header_bar(ui, APP_NAME, None);
    }
}

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
            Ok(Box::new(Shell))
        }),
    )
}
