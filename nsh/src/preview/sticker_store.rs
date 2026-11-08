//! The production store dialog and subscription manager over local fixtures.
use super::{Actions, fixtures};
use crate::{composer::sticker_store, ui};
use nsh_core::prelude::*;

pub(super) const ACTIONS: &[&str] = &["Manage", "Empty", "NoAlbums"];

pub(super) fn scene(actions: Actions) -> View {
    let store = signal_with(fixtures::sticker_store_albums(), SignalEquals::Never);
    let albums = signal_with(
        fixtures::sticker_store_albums()
            .into_iter()
            .filter(|a| a.added)
            .collect(),
        SignalEquals::Never,
    );
    let managing = signal(false);
    for name in ACTIONS {
        actions.on(
            name,
            move || {
                managing.set(false);
                store.set(fixtures::sticker_store_albums());
                albums.set(
                    fixtures::sticker_store_albums()
                        .into_iter()
                        .filter(|a| a.added)
                        .collect(),
                );
            },
            move || match *name {
                "Manage" => managing.set(true),
                "Empty" => {
                    store.set(Vec::new());
                    albums.set(Vec::new());
                }
                "NoAlbums" => {
                    albums.set(Vec::new());
                    managing.set(true);
                }
                _ => {}
            },
        );
    }
    view()
        .flex_1()
        .p_6()
        .items_center()
        .justify_center()
        .bg(ui::surface_chat)
        .relative()
        .child(ui::scrim())
        .child(sticker_store::detached(store, albums, managing))
}
