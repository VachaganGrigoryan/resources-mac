//! The sampler thread calls AppKit every second for the app's whole life; it must not grow.

use resources_mac::collect::apps::running_apps;

fn peak_rss_bytes() -> i64 {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: getrusage only writes into the provided struct.
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    usage.ru_maxrss as i64 // bytes on macOS
}

#[test]
fn running_apps_does_not_leak_on_background_thread() {
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
