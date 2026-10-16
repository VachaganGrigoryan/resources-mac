//! Live stats in the macOS menu bar (`NSStatusItem`), and hiding the app to it.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSCellImagePosition, NSFont, NSFontWeightRegular, NSImage, NSMenu, NSMenuItem,
    NSStatusBar, NSStatusItem, NSVariableStatusItemLength,
};
use objc2_foundation::NSString;

use crate::sampler::State;
use crate::stats::{self, Stat};

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

/// One status item: a stat's SF Symbol and value (or, with no stats chosen, a gauge), with
/// a menu of every stat's value plus "Open Resources" and "Quit Resources".
/// Removed from the menu bar on drop.
struct Item {
    stat: Option<Stat>,
    status: Retained<NSStatusItem>,
    menu: Retained<NSMenu>,
    lines: Vec<Retained<NSMenuItem>>,
    symbol: &'static str,
    /// False when this macOS lacks the symbol; the title then carries a text label.
    has_image: bool,
    title: String,
}

impl Item {
    fn new(mtm: MainThreadMarker, stat: Option<Stat>, target: &Target) -> Self {
        let status =
            NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        if let Some(button) = status.button(mtm) {
            // Fixed-width digits, so the item doesn't jitter as values change.
            // SAFETY: reading an AppKit constant.
            let weight = unsafe { NSFontWeightRegular };
            let font =
                NSFont::monospacedDigitSystemFontOfSize_weight(NSFont::systemFontSize(), weight);
            button.setFont(Some(&font));
            button.setImagePosition(NSCellImagePosition::ImageLeft);
        }
        let menu = NSMenu::new(mtm);
        menu.setAutoenablesItems(false);
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        let open = item(mtm, "Open Resources", Some(sel!(open:)), "");
        // SAFETY: `MenuBar` keeps `target` alive as long as its items.
        unsafe { open.setTarget(Some(target)) };
        menu.addItem(&open);
        // No target: goes up the responder chain to NSApplication.
        menu.addItem(&item(mtm, "Quit Resources", Some(sel!(terminate:)), "q"));
        status.setMenu(Some(&menu));
        Self {
            stat,
            status,
            menu,
            lines: Vec::new(),
            symbol: "",
            has_image: false,
            title: String::new(),
        }
    }

    fn update(&mut self, mtm: MainThreadMarker, s: &State, lines: &[String]) {
        let (symbol, text) = match self.stat {
            Some(st) => (stats::symbol(st, s), stats::value(st, s)),
            None => ("gauge", Some(String::new())),
        };
        // A stat this Mac stops reporting (an unplugged drive) hides its item.
        self.status.setVisible(text.is_some());
        if let (Some(button), Some(value)) = (self.status.button(mtm), text) {
            if self.symbol != symbol {
                let label = self.stat.map_or("Resources", Stat::label);
                let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
                    &NSString::from_str(symbol),
                    Some(&NSString::from_str(label)),
                );
                if let Some(image) = &image {
                    // Template images follow the menu bar's light/dark tint.
                    image.setTemplate(true);
                }
                button.setImage(image.as_deref());
                self.has_image = image.is_some();
                self.symbol = symbol;
            }
            let title = match (self.has_image, self.stat) {
                (true, _) => value,
                (false, Some(st)) => stats::short(st, s).unwrap_or(value),
                (false, None) => "Resources".into(),
            };
            if self.title != title {
                button.setTitle(&NSString::from_str(&title));
                self.title = title;
            }
        }
        if self.lines.len() != lines.len() {
            for line in self.lines.drain(..) {
                self.menu.removeItem(&line);
            }
            for (i, text) in lines.iter().enumerate() {
                let row = item(mtm, text, None, "");
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

impl Drop for Item {
    fn drop(&mut self) {
        NSStatusBar::systemStatusBar().removeStatusItem(&self.status);
    }
}

/// The app's menu bar items: one per chosen stat, in the chosen order.
pub struct MenuBar {
    mtm: MainThreadMarker,
    items: Vec<Item>,
    target: Retained<Target>,
}

impl MenuBar {
    pub fn new(mtm: MainThreadMarker) -> Self {
        Self {
            mtm,
            items: Vec::new(),
            target: Target::new(mtm),
        }
    }

    /// Shows one item per stat in `stats` (a gauge when empty), each with the menu `lines`.
    pub fn update(&mut self, stats: &[Stat], s: &State, lines: &[String]) {
        let wanted: Vec<Option<Stat>> = if stats.is_empty() {
            vec![None]
        } else {
            stats.iter().copied().map(Some).collect()
        };
        if !self.items.iter().map(|i| i.stat).eq(wanted.iter().copied()) {
            self.items.clear();
            // Each new status item lands left of the previous ones: add the last stat first.
            for stat in wanted.into_iter().rev() {
                self.items.push(Item::new(self.mtm, stat, &self.target));
            }
            self.items.reverse();
        }
        for item in &mut self.items {
            item.update(self.mtm, s, lines);
        }
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
