//! Live stats in the macOS menu bar (`NSStatusItem`), and hiding the app to it.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSVariableStatusItemLength,
};
use objc2_foundation::NSString;

/// Title of the main window, set in `main.rs`.
pub const MAIN_WINDOW: &str = "Resources";

define_class!(
    // Menu items hold their target weakly; `MenuBar` keeps this alive.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "ResourcesMenuTarget"]
    struct Target;

    impl Target {
        #[unsafe(method(open:))]
        fn open(&self, _sender: Option<&AnyObject>) {
            show_app(self.mtm());
        }
    }
);

impl Target {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

fn item(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<objc2::runtime::Sel>,
    key: &str,
) -> Retained<NSMenuItem> {
    // SAFETY: the actions are `open:` on our `Target` or NSApplication's `terminate:`.
    unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(title),
            action,
            &NSString::from_str(key),
        )
    }
}

/// A status item showing `title`, with one disabled menu row per line of detail, then
/// "Open Resources" and "Quit Resources". Removed from the menu bar on drop.
pub struct MenuBar {
    mtm: MainThreadMarker,
    status: Retained<NSStatusItem>,
    menu: Retained<NSMenu>,
    lines: Vec<Retained<NSMenuItem>>,
    title: String,
    _target: Retained<Target>,
}

impl MenuBar {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let status =
            NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        let menu = NSMenu::new(mtm);
        menu.setAutoenablesItems(false);
        let target = Target::new(mtm);
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        let open = item(mtm, "Open Resources", Some(sel!(open:)), "");
        // SAFETY: `target` outlives the item: both live as long as `self`.
        unsafe { open.setTarget(Some(&target)) };
        menu.addItem(&open);
        // No target: goes up the responder chain to NSApplication.
        menu.addItem(&item(mtm, "Quit Resources", Some(sel!(terminate:)), "q"));
        status.setMenu(Some(&menu));
        Self {
            mtm,
            status,
            menu,
            lines: Vec::new(),
            title: String::new(),
            _target: target,
        }
    }

    pub fn update(&mut self, title: &str, lines: &[String]) {
        if self.title != title {
            if let Some(button) = self.status.button(self.mtm) {
                button.setTitle(&NSString::from_str(title));
            }
            self.title = title.to_owned();
        }
        if self.lines.len() != lines.len() {
            for line in self.lines.drain(..) {
                self.menu.removeItem(&line);
            }
            for (i, text) in lines.iter().enumerate() {
                let row = item(self.mtm, text, None, "");
                row.setEnabled(false);
                self.menu.insertItem_atIndex(&row, i as isize);
                self.lines.push(row);
            }
        } else {
            for (row, text) in self.lines.iter().zip(lines) {
                if row.title().to_string() != *text {
                    row.setTitle(&NSString::from_str(text));
                }
            }
        }
    }
}

impl Drop for MenuBar {
    fn drop(&mut self) {
        NSStatusBar::systemStatusBar().removeStatusItem(&self.status);
    }
}

/// Hides every window of the app (like ⌘H); the Dock icon or "Open Resources" brings it back.
pub fn hide_app(mtm: MainThreadMarker) {
    NSApplication::sharedApplication(mtm).hide(None);
}

fn show_app(mtm: MainThreadMarker) {
    let app = NSApplication::sharedApplication(mtm);
    app.unhide(None);
    // `activate` needs macOS 14; the app supports 13.
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
    for w in app.windows() {
        if w.title().to_string() == MAIN_WINDOW {
            w.makeKeyAndOrderFront(None);
        }
    }
}

/// Keeps the window titled `title` on screen when the app hides (the desktop widget).
pub fn keep_visible_when_hidden(mtm: MainThreadMarker, title: &str) {
    for w in NSApplication::sharedApplication(mtm).windows() {
        if w.title().to_string() == title {
            w.setCanHide(false);
        }
    }
}
