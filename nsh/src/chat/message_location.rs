use mixin_desktop_api::MessageListView;
use nsh_core::{PointerEvents, element::ImageSrc, prelude::*};
use serde_json::Value;

use super::message_actions::Actions;
use crate::{
    i18n::Key,
    ui::{self, Typography},
};

/// The map's height, measured from Flutter. Tiles are placed against it.
const HEIGHT: f32 = 180.0;
const TILE_SIZE: f64 = 256.0;
const ZOOM: u32 = 14;
const MERCATOR_LATITUDE: f64 = 85.051_128_779_806_6;

#[derive(Clone, PartialEq)]
struct Tile {
    x: i32,
    y: i32,
    left: f32,
    top: f32,
}

fn tiles(latitude: f64, longitude: f64, width: f32) -> Vec<Tile> {
    let count = (1_u32 << ZOOM) as f64;
    let latitude = latitude.clamp(-MERCATOR_LATITUDE, MERCATOR_LATITUDE);
    let center_x = (longitude + 180.0) / 360.0 * count * TILE_SIZE;
    let center_y = (1.0 - latitude.to_radians().tan().asinh() / std::f64::consts::PI) / 2.0
        * count
        * TILE_SIZE;
    let left = (center_x - f64::from(width) / 2.0).floor();
    let top = (center_y - f64::from(HEIGHT) / 2.0).floor();
    let first_x = (left / TILE_SIZE).floor() as i32;
    let last_x = ((left + f64::from(width) - 0.001) / TILE_SIZE).floor() as i32;
    let first_y = (top / TILE_SIZE).floor().max(0.0) as i32;
    let last_y = ((top + f64::from(HEIGHT) - 0.001) / TILE_SIZE)
        .floor()
        .min(count - 1.0) as i32;
    let mut tiles = Vec::with_capacity(6);
    for x in first_x..=last_x {
        for y in first_y..=last_y {
            tiles.push(Tile {
                x: x.rem_euclid(count as i32),
                y,
                left: (f64::from(x) * TILE_SIZE - left).floor() as f32,
                top: (f64::from(y) * TILE_SIZE - top).floor() as f32,
            });
        }
    }
    tiles
}

fn tile_url(x: i32, y: i32) -> String {
    // Preserve the Flutter client's existing provider and tile request format.
    format!(
        "https://www.google.com/maps/vt/pb=!1m4!1m3!1i{ZOOM}!2i{x}!3i{y}!2m3!1e0!2sm!3i420120488!3m7!2sen!5e1105!12m4!1e68!2m2!1sset!2sRoadmap!4e0!5m1!1e0!23i4111425"
    )
}

pub(super) fn location(row: &MessageListView, actions: Actions) -> AnyElement {
    let i18n = actions.i18n;
    let Some(value) = serde_json::from_str::<Value>(&row.content).ok() else {
        return invalid(actions);
    };
    let coordinates = value
        .get("latitude")
        .and_then(Value::as_f64)
        .zip(value.get("longitude").and_then(Value::as_f64))
        .filter(|(latitude, longitude)| {
            latitude.is_finite()
                && longitude.is_finite()
                && (-90.0..=90.0).contains(latitude)
                && (-180.0..=180.0).contains(longitude)
        });
    let Some((latitude, longitude)) = coordinates else {
        return invalid(actions);
    };
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let address = value
        .get("address")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let description = format!(
        "{} {name} {address} {latitude:.5}, {longitude:.5}",
        i18n.text(Key::Location)
    );
    let mut card = view()
        .w(ui::card_width())
        .max_w(relative(1.0))
        .gap_1()
        .child(preview(latitude, longitude, description, actions));
    if !name.trim().is_empty() {
        card = card.child(
            text(name.to_owned())
                .text_role(ui::TextRole::Body)
                .text_emphasis()
                .number_of_lines(2),
        );
    }
    if !address.trim().is_empty() {
        card = card.child(
            text(address.to_owned())
                .text_role(ui::TextRole::Caption)
                .number_of_lines(2),
        );
    }
    card.child(
        text(format!("{latitude:.5}, {longitude:.5}"))
            .text_role(ui::TextRole::Caption)
            .text_color(ui::fg_secondary),
    )
    .into_element()
}

fn invalid(actions: Actions) -> AnyElement {
    text(move || actions.i18n.tr("Invalid location", "无法读取位置"))
        .text_role(ui::TextRole::Body)
        .into_element()
}

fn preview(
    latitude: f64,
    longitude: f64,
    description: String,
    actions: Actions,
) -> impl IntoElement {
    let width = signal(ui::card_width().0);
    let target = stored(format!(
        "https://www.google.com/maps/search/?api=1&query={latitude},{longitude}"
    ));
    let open = move || {
        let target = target.with(|value| value.clone());
        actions.task(async move {
            actions.failed.set(!open_url(target).await);
        });
    };
    view()
        .w_full()
        .h(px(HEIGHT))
        .relative()
        .overflow_hidden()
        .rounded(ui::radius_md())
        .bg(ui::surface_card)
        .on_layout(move |bounds: &Bounds<Points>| {
            width.set(bounds.size.width.0.clamp(1.0, ui::card_width().0));
        })
        .child(
            view()
                .absolute()
                .inset_0()
                .items_center()
                .justify_center()
                .gap_1()
                .child(text(move || actions.i18n.text(Key::Location)).text_role(ui::TextRole::Body))
                .child(text(format!("{latitude:.5}, {longitude:.5}")).text_role(ui::TextRole::Caption))
                .text_color(ui::fg_secondary),
        )
        .child(each(
            move || tiles(latitude, longitude, width.get()),
            |tile| (tile.x, tile.y),
            move |tile, _| {
                let failed = signal(false);
                // A bitmap of the map service, placed by the projection above.
                image(move || tile.with(|tile| tile_url(tile.x, tile.y)))
                    .absolute()
                    .left(move || px(tile.with(|tile| tile.left)))
                    .top(move || px(tile.with(|tile| tile.top)))
                    .size(px(TILE_SIZE as f32))
                    .opacity(move || if failed.get() { 0.0_f32 } else { 1.0_f32 })
                    .on_error(move |_: &str| failed.set(true))
                    .accessibility_elements_hidden(true)
            },
        ))
        // The upper half of the map, so the pin's tip lands on its center.
        .child(
            view()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .h(relative(0.5))
                .items_center()
                .justify_end()
                .pointer_events(PointerEvents::None)
                .child(
                    svg(ImageSrc::svg_markup(
                        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 22 33' fill='none' stroke='currentColor' stroke-width='2'><path d='M11 32V21'/><circle cx='11' cy='11' r='10'/><circle cx='11' cy='11' r='3'/></svg>",
                    ))
                    .w_6()
                    .h_9()
                    .text_color(ui::accent)
                    .accessibility_elements_hidden(true),
                ),
        )
        .child(
            ui::command()
                .absolute()
                .inset_0()
                .p_0()
                .accessibility_label(move || Some(description.clone().into()))
                .on_press(open),
        )
        // A state fill replaces a button's own, so the plate sits on a parent.
        .child(
            view()
                .absolute()
                .bottom_1()
                .right_1()
                .rounded(ui::radius_md())
                .bg(ui::surface_pane)
                .child(
                    ui::command()
                        .min_h_0()
                        .px_1()
                        .py_1()
                        .on_press(open)
                        .child(text("Google Maps").text_role(ui::TextRole::Caption)),
                ),
        )
}
