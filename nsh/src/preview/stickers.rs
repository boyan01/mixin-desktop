//! Production sticker panel chrome and tile geometry, with account-free fixtures.
use super::{Actions, fixtures};
use crate::{
    composer::stickers::{self, Tab},
    ui,
};
use nsh_core::prelude::*;

pub(super) const ACTIONS: &[&str] = &["Personal", "Albums", "Empty", "Error", "Reset"];

pub(super) fn scene(actions: Actions) -> View {
    let tab = signal(Tab::Recent);
    let albums = signal_with(Vec::new(), SignalEquals::Never);
    let empty = signal(false);
    let failed = signal(false);
    for name in ACTIONS {
        actions.on(
            name,
            move || {
                failed.set(false);
                tab.set(Tab::Recent);
                albums.set(Vec::new());
                empty.set(false);
            },
            move || match *name {
                "Personal" => tab.set(Tab::Personal),
                "Albums" => {
                    let fixtures = fixtures::sticker_albums();
                    tab.set(Tab::Album(fixtures[0].album_id.clone()));
                    albums.set(fixtures);
                }
                "Empty" => empty.set(true),
                "Error" => failed.set(true),
                _ => {}
            },
        );
    }
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .bg(ui::surface_chat)
        .child(
            view()
                .rounded(ui::radius_lg())
                .overflow_hidden()
                .bg(ui::surface_popover)
                .border_1()
                .border_color(ui::border)
                .shadow_lg()
                .child(stickers::panel_surface(
                    None,
                    tab,
                    albums,
                    view()
                        .flex_1()
                        .min_h_0()
                        .child(when(failed, || stickers::load_error(|| {})))
                        .child(when(empty, move || {
                            stickers::empty_stickers(move || tab.get())
                        }))
                        .child(
                            virtual_list(
                                move || 16_usize + usize::from(tab.get() == Tab::Personal),
                                move |at| {
                                    view()
                                        .items_center()
                                        .child(when(
                                            move || tab.get() == Tab::Personal && at.index() == 0,
                                            move || stickers::import_tile(signal(false), || {}),
                                        ))
                                        .child(when(
                                            move || tab.get() != Tab::Personal || at.index() > 0,
                                            move || {
                                                stickers::sticker_tile().child(
                                                    crate::sticker::framed_asset(
                                                        None,
                                                        move || {
                                                            (
                                                                None,
                                                                fixtures::sticker_image(
                                                                    at.index().saturating_sub(
                                                                        usize::from(
                                                                            tab.get()
                                                                                == Tab::Personal,
                                                                        ),
                                                                    ),
                                                                ),
                                                                "png".into(),
                                                            )
                                                        },
                                                        (
                                                            ui::sticker_art_size().0,
                                                            ui::sticker_art_size().0,
                                                        ),
                                                    ),
                                                )
                                            },
                                        ))
                                },
                            )
                            .columns(move || stickers::columns(stickers::cell_size().0))
                            .estimated_item_size(stickers::cell_size())
                            .content_padding_x(ui::space_md())
                            .content_padding_y(ui::space_sm())
                            .flex_1()
                            .min_h_0()
                            .style_when(empty, |s| s.hidden().invisible()),
                        ),
                    || {},
                )),
        )
}
