//! Shared look and app shell for Resources for Mac and Disk Sweep: the GNOME Resources
//! theme, Phosphor icons, widgets, the sidebar + header window layout and the confirm dialog.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

pub mod confirm;
pub mod history;
pub mod icons;
pub mod settings;
pub mod shell;
#[cfg(feature = "test-support")]
pub mod test_support;
pub mod theme;
pub mod widgets;

pub use history::{HISTORY_LEN, History};

/// The value behind `m`, even if a worker thread panicked while holding it.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Last path component (`~/WS` → `WS`), or the whole path for `/`.
pub fn display_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// Main window of both apps: 1000×680, sidebar and header bar under a transparent title bar.
pub fn native_options(title: &str, icon_png: &[u8]) -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_icon(
                eframe::icon_data::from_png_bytes(icon_png).expect("embedded icon is a valid PNG"),
            )
            .with_inner_size([1000.0, 680.0])
            .with_min_inner_size([720.0, 480.0])
            // GNOME-style: the sidebar and header bar run under a transparent title bar.
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false),
        // These apps stay open for hours: never wake the discrete GPU on dual-GPU Intel MacBooks.
        wgpu_options: eframe::WgpuConfiguration {
            wgpu_setup: eframe::egui_wgpu::WgpuSetupCreateNew {
                power_preference: eframe::wgpu::PowerPreference::LowPower,
                ..eframe::egui_wgpu::WgpuSetupCreateNew::without_display_handle()
            }
            .into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_is_last_component() {
        assert_eq!(display_name(Path::new("/Users/me/WS")), "WS");
        assert_eq!(display_name(Path::new("/")), "/");
    }

    #[test]
    fn lock_survives_poison() {
        let m = std::sync::Arc::new(Mutex::new(1));
        let m2 = m.clone();
        let _ = std::thread::spawn(move || {
            let _g = m2.lock().unwrap();
            panic!("poison");
        })
        .join();
        assert_eq!(*lock(&m), 1);
    }
}
