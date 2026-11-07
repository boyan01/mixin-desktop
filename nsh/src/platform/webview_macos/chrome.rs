//! Native browser chrome follows semantic system colors or the page's theme-color.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use nsh_platform_macos::objc2_app_kit::{
    NSAccessibility, NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua,
    NSAppearanceNameDarkAqua, NSButton, NSColor, NSImage, NSImageScaling, NSImageView, NSTextField,
    NSView, NSWindow,
};
use objc2::{
    AnyThread,
    rc::{Retained, Weak},
};
use objc2_foundation::{NSData, NSString};
use serde_json::Value;
use std::cell::{Cell, RefCell};

#[derive(Default)]
pub(super) struct Chrome {
    window: RefCell<Option<Weak<NSWindow>>>,
    title: RefCell<Option<Weak<NSTextField>>>,
    icon: RefCell<Option<Weak<NSImageView>>>,
    buttons: RefCell<Vec<Weak<NSButton>>>,
    back: RefCell<Option<Weak<NSButton>>>,
    can_back: Cell<bool>,
    capsule: RefCell<Option<(Weak<NSView>, Weak<NSView>)>>,
    custom: Cell<Option<[u8; 3]>>,
    dark: Cell<bool>,
    cache: RefCell<Option<(String, String)>>,
}

impl Chrome {
    pub(super) fn attach(
        &self,
        window: &NSWindow,
        title: Option<&NSTextField>,
        icon: Option<&NSImageView>,
        buttons: &[Retained<NSButton>],
        session: &str,
        app: &str,
    ) {
        *self.window.borrow_mut() = Some(Weak::new(window));
        *self.title.borrow_mut() = title.map(Weak::new);
        *self.icon.borrow_mut() = icon.map(Weak::new);
        *self.buttons.borrow_mut() = buttons.iter().map(|button| Weak::new(&**button)).collect();
        *self.back.borrow_mut() = title
            .and_then(|_| buttons.first())
            .map(|button| Weak::new(&**button));
        *self.cache.borrow_mut() = Some((session.to_owned(), app.to_owned()));
        self.custom.set(super::session::theme(session, app));
        self.apply();
    }

    pub(super) fn title(&self, title: &str) {
        if let Some(label) = self.title.borrow().as_ref().and_then(Weak::load) {
            label.setStringValue(&NSString::from_str(title));
        }
        if let Some(window) = self.window.borrow().as_ref().and_then(Weak::load) {
            window.setTitle(&NSString::from_str(title));
        }
    }

    pub(super) fn labels(&self, labels: &Value) {
        let buttons = self.buttons.borrow();
        let keys = ["Back", "More", "Close"];
        let keys = if buttons.len() == 3 {
            &keys[..]
        } else {
            &keys[1..]
        };
        for (button, key) in buttons.iter().zip(keys) {
            if let Some(button) = button.load() {
                button.setAccessibilityLabel(Some(&NSString::from_str(super::label(labels, key))));
            }
        }
    }

    pub(super) fn system_dark(&self, dark: bool) {
        self.dark.set(dark);
        self.apply();
    }

    pub(super) fn theme(&self, value: &Value) {
        let color = value
            .as_array()
            .filter(|channels| channels.len() == 3)
            .and_then(|channels| {
                Some([
                    u8::try_from(channels[0].as_u64()?).ok()?,
                    u8::try_from(channels[1].as_u64()?).ok()?,
                    u8::try_from(channels[2].as_u64()?).ok()?,
                ])
            });
        self.custom.set(color);
        if let Some((session, app)) = self.cache.borrow().as_ref() {
            super::session::set_theme(session, app, color);
        }
        self.apply();
    }

    pub(super) fn icon(&self, value: Option<&str>) {
        let Some(icon) = self.icon.borrow().as_ref().and_then(Weak::load) else {
            return;
        };
        let image = value
            .filter(|value| value.len() <= 128 * 1024)
            .and_then(|value| STANDARD.decode(value).ok())
            .and_then(|bytes| NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(&bytes)));
        icon.setImage(image.as_deref());
        icon.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
        icon.setHidden(self.can_back.get());
        self.apply();
    }

    pub(super) fn navigation(&self, can_back: bool) {
        self.can_back.set(can_back);
        if let Some(back) = self.back.borrow().as_ref().and_then(Weak::load) {
            back.setHidden(!can_back);
            back.setEnabled(can_back);
        }
        if let Some(icon) = self.icon.borrow().as_ref().and_then(Weak::load) {
            icon.setHidden(can_back);
        } else if let Some(title) = self.title.borrow().as_ref().and_then(Weak::load) {
            let mut frame = title.frame();
            let x = if can_back { 44.0 } else { 12.0 };
            frame.size.width += frame.origin.x - x;
            frame.origin.x = x;
            title.setFrame(frame);
        }
    }

    pub(super) fn capsule(&self, capsule: &NSView, divider: &NSView) {
        *self.capsule.borrow_mut() = Some((Weak::new(capsule), Weak::new(divider)));
    }

    fn apply(&self) {
        let Some(window) = self.window.borrow().as_ref().and_then(Weak::load) else {
            return;
        };
        let custom = self.custom.get();
        let dark = custom.map_or(self.dark.get(), |rgb| {
            let linear = |channel: u8| {
                let value = f64::from(channel) / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(rgb[0]) + 0.7152 * linear(rgb[1]) + 0.0722 * linear(rgb[2]) < 0.179
        });
        window.setAppearance(
            NSAppearance::appearanceNamed(unsafe {
                if dark {
                    NSAppearanceNameDarkAqua
                } else {
                    NSAppearanceNameAqua
                }
            })
            .as_deref(),
        );
        let background = custom
            .map(|rgb| {
                NSColor::colorWithSRGBRed_green_blue_alpha(
                    f64::from(rgb[0]) / 255.0,
                    f64::from(rgb[1]) / 255.0,
                    f64::from(rgb[2]) / 255.0,
                    1.0,
                )
            })
            .unwrap_or_else(NSColor::windowBackgroundColor);
        window.setBackgroundColor(Some(&background));
        if let Some((capsule, divider)) = self.capsule.borrow().as_ref() {
            let capsule = capsule.load();
            let divider = divider.load();
            let draw = block2::RcBlock::new(move || {
                if let Some(layer) = capsule.as_ref().and_then(|capsule| capsule.layer()) {
                    layer.setBackgroundColor(Some(&NSColor::controlBackgroundColor().CGColor()));
                    layer.setBorderColor(Some(&NSColor::separatorColor().CGColor()));
                }
                if let Some(layer) = divider.as_ref().and_then(|divider| divider.layer()) {
                    layer.setBackgroundColor(Some(&NSColor::separatorColor().CGColor()));
                }
            });
            window
                .effectiveAppearance()
                .performAsCurrentDrawingAppearance(&draw);
        }

        if let Some(icon) = self.icon.borrow().as_ref().and_then(Weak::load)
            && let Some(layer) = icon.layer()
        {
            let background = if icon.image().is_some() {
                NSColor::clearColor()
            } else {
                NSColor::quaternaryLabelColor()
            };
            let draw =
                block2::RcBlock::new(move || layer.setBackgroundColor(Some(&background.CGColor())));
            window
                .effectiveAppearance()
                .performAsCurrentDrawingAppearance(&draw);
        }
        if let Some(title) = self.title.borrow().as_ref().and_then(Weak::load) {
            title.setTextColor(Some(&NSColor::labelColor()));
        }
        for button in self.buttons.borrow().iter().filter_map(Weak::load) {
            button.setContentTintColor(Some(&NSColor::labelColor()));
        }
    }
}
