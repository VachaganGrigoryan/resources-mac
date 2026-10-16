use objc2_app_kit::NSWorkspace;
use objc2_foundation::NSString;
use resources_mac::collect::apps::render_icon;

#[test]
fn calculator_icon_renders_64px_with_opaque_pixels() {
    objc2::rc::autoreleasepool(|_| {
        let path = NSString::from_str("/System/Applications/Calculator.app");
        let img = NSWorkspace::sharedWorkspace().iconForFile(&path);
        let icon = render_icon(&img, 64).expect("icon rendered");
        assert_eq!(icon.size, [64, 64]);
        let opaque = icon.pixels.iter().filter(|p| p.a() == 255).count();
        assert!(opaque > 64 * 64 / 4, "only {opaque} opaque pixels");
    });
}
