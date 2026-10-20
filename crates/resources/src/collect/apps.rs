use egui::ColorImage;
use objc2::AllocAnyThread;
use objc2_app_kit::{
    NSApplicationActivationPolicy, NSBitmapImageRep, NSCompositingOperation, NSDeviceRGBColorSpace,
    NSGraphicsContext, NSImage, NSRunningApplication, NSWorkspace,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use crate::sample::AppInfo;

/// Apps that appear in the Dock (regular activation policy), like Activity Monitor's app list.
pub fn running_apps() -> Vec<AppInfo> {
    // Called from the sampler thread, which has no autorelease pool of its own.
    objc2::rc::autoreleasepool(|_| collect())
}

fn collect() -> Vec<AppInfo> {
    NSWorkspace::sharedWorkspace()
        .runningApplications()
        .iter()
        .filter(|app| app.activationPolicy() == NSApplicationActivationPolicy::Regular)
        .filter_map(|app| {
            Some(AppInfo {
                pid: u32::try_from(app.processIdentifier()).ok()?,
                name: app
                    .localizedName()
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                bundle_id: app.bundleIdentifier().map(|s| s.to_string()),
                icon: None,
            })
        })
        .collect()
}

/// The Dock icon of a running app at 64 × 64 px, or `None` if the app is gone or has none.
pub fn app_icon(pid: u32) -> Option<ColorImage> {
    let pid = libc::pid_t::try_from(pid).ok()?;
    objc2::rc::autoreleasepool(|_| {
        let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)?;
        render_icon(&*app.icon()?, 64)
    })
}

/// Draws `img` into a `px` × `px` RGBA bitmap. Safe off the main thread: the bitmap context
/// is private to this call and the current context is thread-local.
pub fn render_icon(img: &NSImage, px: usize) -> Option<ColorImage> {
    let n = isize::try_from(px).ok()?;
    // SAFETY: null planes ask AppKit to allocate the buffer; the colour space is a valid constant.
    let rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            n,
            n,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            0,
            32,
        )
    }?;
    let ctx = NSGraphicsContext::graphicsContextWithBitmapImageRep(&rep)?;
    NSGraphicsContext::saveGraphicsState_class();
    NSGraphicsContext::setCurrentContext(Some(&ctx));
    let side = px as f64;
    img.drawInRect_fromRect_operation_fraction(
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(side, side)),
        NSRect::ZERO,
        NSCompositingOperation::SourceOver,
        1.0,
    );
    ctx.flushGraphics();
    NSGraphicsContext::restoreGraphicsState_class();

    let data = rep.bitmapData();
    let stride = usize::try_from(rep.bytesPerRow()).ok()?;
    if data.is_null() || stride < px * 4 {
        return None;
    }
    // SAFETY: AppKit owns a `stride * px` byte buffer that lives as long as `rep`.
    let bytes = unsafe { std::slice::from_raw_parts(data, stride * px) };
    let rgba: Vec<u8> = bytes
        .chunks_exact(stride)
        .flat_map(|row| &row[..px * 4])
        .copied()
        .collect();
    // NSBitmapImageRep stores premultiplied alpha unless asked otherwise.
    Some(ColorImage::from_rgba_premultiplied([px, px], &rgba))
}
