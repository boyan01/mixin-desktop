//! The native desktop header follows the shared Back / More / Close layout.
use std::rc::Rc;

use gtk::{gio, prelude::*};
use nsh_platform_linux::gtk4 as gtk;
use serde_json::json;
use webkit6::prelude::WebViewExt;

use super::window::{State, is_web};

pub(super) fn install(state: &Rc<State>) {
    let header = &state.header;
    let immersive = state
        .page
        .borrow()
        .get("immersive")
        .and_then(|v| v.as_bool())
        .unwrap_or_default();
    header.set_height_request(if immersive { 28 } else { 48 });
    header.set_margin_start(8);
    header.set_margin_end(8);
    state.back.set_tooltip_text(Some(&state.label("Back")));
    state.back.set_visible(false);
    let weak = Rc::downgrade(state);
    state.back.connect_clicked(move |_| {
        if let Some(state) = weak.upgrade().filter(|s| !s.closed.get())
            && state.web.can_go_back()
        {
            state.web.go_back();
        }
    });
    state.title.set_hexpand(true);
    state.title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    let actions = gio::SimpleActionGroup::new();
    for name in ["refresh", "copy", "browser", "share"] {
        let action = gio::SimpleAction::new(name, None);
        let weak = Rc::downgrade(state);
        action.connect_activate(move |_, _| {
            let Some(state) = weak.upgrade().filter(|s| !s.closed.get()) else { return; };
            let Some(uri) = state.web.uri().filter(|uri| is_web(uri)) else { return; };
            match name {
                "refresh" => state.web.reload(),
                "copy" => state.window.clipboard().set_text(&uri),
                "browser" => state.emit("browser", json!(uri.as_str())),
                "share" => state.emit("share", json!({"url":uri.as_str(),"title":state.web.title().as_deref().unwrap_or_default()})),
                _ => {}
            }
        });
        actions.add_action(&action);
    }
    state.window.insert_action_group("bot", Some(&actions));
    let more = &state.more;
    more.set_menu_model(Some(&state.menu));
    let close = &state.close_button;
    let weak = Rc::downgrade(state);
    close.connect_clicked(move |_| {
        if let Some(state) = weak.upgrade() {
            state.close();
        }
    });
    if !immersive {
        header.append(&state.back);
        state.icon.set_pixel_size(24);
        header.append(&state.icon);
        header.append(&state.title);
        header.append(more);
    } else {
        let space = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        space.set_hexpand(true);
        header.append(&space);
    }
    header.append(close);
    let handle = gtk::WindowHandle::new();
    handle.set_child(Some(header));
    state.window.set_titlebar(Some(&handle));
    state.web.set_hexpand(true);
    state.web.set_vexpand(true);
    state.window.set_child(Some(&state.web));
    state.header.add_css_class(&state.style_name);
    gtk::style_context_add_provider_for_display(
        &gtk::prelude::WidgetExt::display(&state.window),
        &state.colors,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let weak = Rc::downgrade(state);
    state.window.connect_close_request(move |_| {
        if let Some(state) = weak.upgrade() {
            state.close();
        }
        gtk::glib::Propagation::Stop
    });
}

pub(super) fn labels(state: &State) {
    state.menu.remove_all();
    for (label, action) in [
        ("Refresh", "refresh"),
        ("Copy link", "copy"),
        ("Open in browser", "browser"),
    ] {
        state
            .menu
            .append(Some(&state.label(label)), Some(&format!("bot.{action}")));
    }
    if state
        .page
        .borrow()
        .get("shareable")
        .and_then(|v| v.as_bool())
        == Some(true)
    {
        state
            .menu
            .append(Some(&state.label("Forward…")), Some("bot.share"));
    }
    state.back.set_tooltip_text(Some(&state.label("Back")));
    state.more.set_tooltip_text(Some(&state.label("More")));
    state
        .close_button
        .set_tooltip_text(Some(&state.label("Close")));
    state
        .back
        .update_property(&[gtk::accessible::Property::Label(&state.label("Back"))]);
    state
        .more
        .update_property(&[gtk::accessible::Property::Label(&state.label("More"))]);
    state
        .close_button
        .update_property(&[gtk::accessible::Property::Label(&state.label("Close"))]);
}

pub(super) fn theme(state: &State, color: Option<&serde_json::Value>) {
    let color = color
        .and_then(|v| v.as_array())
        .filter(|a| a.len() == 3)
        .and_then(|a| {
            let rgb = a
                .iter()
                .map(|v| v.as_u64().filter(|n| *n <= 255))
                .collect::<Option<Vec<_>>>()?;
            Some((rgb[0], rgb[1], rgb[2]))
        });
    let css = color
        .map(|(r, g, b)| {
            let fg = if r * 299 + g * 587 + b * 114 > 128000 {
                "black"
            } else {
                "white"
            };
            format!(
                ".{} {{ background-color: rgb({r},{g},{b}); color: {fg}; }}",
                state.style_name
            )
        })
        .unwrap_or_default();
    state.colors.load_from_string(&css);
}

pub(super) fn update(state: &State) {
    let back = state.web.can_go_back();
    state.back.set_visible(back);
    state
        .icon
        .set_visible(!back && state.icon.paintable().is_some());
    if let Some(title) = state.web.title().filter(|title| !title.is_empty()) {
        state.title.set_text(&title);
        state.window.set_title(Some(&title));
    }
}
