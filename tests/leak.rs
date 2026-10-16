//! The sampler thread calls AppKit every second for the app's whole life; it must not grow.

use resources_mac::collect::apps::running_apps;

/// Peak RSS is process-wide, so the tests must not allocate concurrently.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn peak_rss_bytes() -> i64 {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: getrusage only writes into the provided struct.
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    usage.ru_maxrss as i64 // bytes on macOS
}

#[test]
fn running_apps_does_not_leak_on_background_thread() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        for _ in 0..2_000 {
            running_apps();
        }
        let before = peak_rss_bytes();
        for _ in 0..20_000 {
            running_apps();
        }
        let grown = peak_rss_bytes() - before;
        assert!(grown < 2 << 20, "peak RSS grew by {} KiB", grown >> 10);
    })
    .join()
    .unwrap();
}

#[test]
fn icon_rendering_does_not_leak_on_background_thread() {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::NSString;
    use resources_mac::collect::apps::render_icon;

    // Same work as `app_icon` (fetch + draw into a fresh bitmap), repeated.
    fn once() {
        objc2::rc::autoreleasepool(|_| {
            let path = NSString::from_str("/System/Applications/Calculator.app");
            let img = NSWorkspace::sharedWorkspace().iconForFile(&path);
            render_icon(&img, 64).expect("icon rendered");
        });
    }
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        for _ in 0..300 {
            once();
        }
        let before = peak_rss_bytes();
        for _ in 0..3_000 {
            once();
        }
        let grown = peak_rss_bytes() - before;
        assert!(grown < 2 << 20, "peak RSS grew by {} KiB", grown >> 10);
    })
    .join()
    .unwrap();
}
