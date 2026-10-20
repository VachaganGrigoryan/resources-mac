//! The window icon is embedded at build time; a bad asset must fail CI, not the first launch.

#[test]
fn icon_png_decodes_at_256() {
    let icon = eframe::icon_data::from_png_bytes(disk_sweep::APP_ICON_PNG).expect("valid PNG");
    assert_eq!((icon.width, icon.height), (256, 256));
    assert_eq!(icon.rgba.len(), 256 * 256 * 4);
}
