use objc2_app_kit::{NSApplicationActivationPolicy, NSWorkspace};

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
            })
        })
        .collect()
}
