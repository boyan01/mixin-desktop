mod album_order;

use mixin_desktop_api::{AccountClient, StickerAlbumItem, StickerDetailItem, StickerItem};
use nsh_core::prelude::*;
use std::sync::Arc;

use super::{
    stickers::{State, Tab},
    tr,
};
use crate::{
    i18n::{self, Key},
    runtime,
    ui::{self, Typography as _},
};

/// The album's bitmap in the store list.
const ALBUM_ICON: f32 = 48.0;

pub(super) fn panel(state: State) -> impl IntoElement {
    let i18n = i18n::current();
    let store = signal_with(Vec::<StickerAlbumItem>::new(), SignalEquals::Never);
    let loading = signal(true);
    let viewable = signal(None::<(usize, usize)>);
    effect(move || {
        state.revision.get();
        let client = state.client();
        loading.set(true);
        spawn(async move {
            match runtime::run(async move { client.sticker().sticker_store_albums().await }).await {
                Ok(values) => store.set(values),
                Err(_) => state.failed.set(true),
            }
            loading.set(false);
        });
    });
    view()
        .flex_1()
        .min_h_0()
        .gap_2()
        .p_3()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(move || tr(i18n, "Sticker store", "贴纸商店"))
                        .text_role(ui::TextRole::Body)
                        .text_emphasis()
                        .flex_1(),
                )
                .child(
                    ui::action(
                        move || tr(i18n, "Refresh", "刷新"),
                        move || {
                            let client = state.client();
                            state.update(async move { client.sticker().refresh_stickers().await });
                        },
                    )
                    .disabled(state.busy),
                ),
        )
        .child(when(loading, move || {
            text(move || tr(i18n, "Loading stickers…", "正在加载贴纸…"))
                .text_role(ui::TextRole::Body)
                .text_color(ui::fg_secondary)
        }))
        .child(when(
            move || !loading.get() && !state.failed.get() && store.with(Vec::is_empty),
            move || {
                ui::placeholder(move || tr(i18n, "No sticker albums available", "暂无贴纸专辑"))
            },
        ))
        .child(
            virtual_list(
                move || store.with(Vec::len),
                move |at| {
                    crate::sticker::visibility(move || {
                        viewable
                            .get()
                            .is_some_and(|(first, last)| (first..=last).contains(&at.index()))
                    });
                    ui::command()
                        .justify_start()
                        .gap_3()
                        .disabled(state.busy)
                        .on_press(move || {
                            if let Some(id) =
                                store.with(|rows| rows.get(at.index()).map(|a| a.album_id.clone()))
                            {
                                state.tab.set(Tab::Album(id));
                            }
                        })
                        .child(crate::sticker::album_icon(
                            state.client(),
                            move || {
                                store.with(|rows| {
                                    rows.get(at.index())
                                        .map(|a| a.icon_url.clone())
                                        .unwrap_or_default()
                                })
                            },
                            ALBUM_ICON,
                        ))
                        .child(
                            view()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    text(move || {
                                        store.with(|rows| {
                                            rows.get(at.index())
                                                .map(|a| a.name.clone())
                                                .unwrap_or_default()
                                        })
                                    })
                                    .text_role(ui::TextRole::Body)
                                    .number_of_lines(1),
                                )
                                .child(
                                    text(move || {
                                        store.with(|rows| {
                                            rows.get(at.index())
                                                .map(|a| a.description.clone())
                                                .unwrap_or_default()
                                        })
                                    })
                                    .text_role(ui::TextRole::Caption)
                                    .number_of_lines(2)
                                    .text_color(ui::fg_secondary),
                                ),
                        )
                },
            )
            .keys(move || {
                store.with(|rows| rows.iter().map(|a| a.album_id.clone()).collect::<Vec<_>>())
            })
            .on_viewable_items_change(move |event: &ViewableItems| viewable.set(event.viewable))
            .estimated_item_size(px(ALBUM_ICON) + ui::space_lg() + ui::space_xs())
            .flex_1()
            .min_h_0()
            .style_when(
                move || store.with(Vec::is_empty),
                |style| style.hidden().invisible(),
            ),
        )
        .child(when(
            move || !state.albums.with(Vec::is_empty),
            move || {
                view()
                    .flex_1()
                    .min_h_0()
                    .gap_2()
                    .child(
                        text(move || tr(i18n, "Your albums", "我的贴纸专辑"))
                            .text_role(ui::TextRole::Caption)
                            .text_color(ui::fg_secondary),
                    )
                    .child(album_order::list(state))
            },
        ))
}

pub(crate) fn detail(account: Arc<AccountClient>, sticker_id: String) -> impl IntoElement {
    let i18n = i18n::current();
    // The detail grid selects a preview. Its callbacks never send a message.
    let state = State::new(account, String::new(), true, signal(false));
    state.observe_updates();
    let selected = signal(sticker_id);
    let details = signal_with(None::<StickerDetailItem>, SignalEquals::Never);
    let items = signal_with(Vec::<StickerItem>::new(), SignalEquals::Never);
    let loading = signal(true);
    effect(move || {
        state.revision.get();
        let id = selected.get();
        let client = state.client();
        loading.set(true);
        spawn(async move {
            match runtime::run(async move {
                let detail = client.sticker().sticker_detail(id).await?;
                let albums = client.sticker().sticker_albums().await?;
                Ok((detail, albums))
            })
            .await
            {
                Ok((detail, albums)) => {
                    state.albums.set(albums);
                    items.set(if detail.album.is_some() {
                        detail.album_stickers.clone()
                    } else {
                        vec![detail.sticker.clone()]
                    });
                    details.set(Some(detail));
                }
                Err(_) => state.failed.set(true),
            }
            loading.set(false);
        });
    });
    view()
        .flex_1()
        .min_h_0()
        .w_full()
        .gap_3()
        .child(
            text(move || {
                details.with(|detail| {
                    detail
                        .as_ref()
                        .map(|detail| {
                            detail.album.as_ref().map_or_else(
                                || detail.sticker.name.clone(),
                                |album| album.name.clone(),
                            )
                        })
                        .unwrap_or_default()
                })
            })
            .text_role(ui::TextRole::Body)
            .text_emphasis(),
        )
        .child(view().items_center().child(crate::sticker::asset(
            state.client(),
            move || {
                details.with(|detail| {
                    detail
                        .as_ref()
                        .map(|detail| {
                            (
                                Some(detail.sticker.sticker_id.clone()),
                                detail.sticker.asset_url.clone(),
                                detail.sticker.asset_type.clone(),
                            )
                        })
                        .unwrap_or_default()
                })
            },
            256.0,
        )))
        .child(dynamic(move || {
            let detail = details.get();
            untracked(|| match detail {
                Some(StickerDetailItem {
                    album: Some(album), ..
                }) => super::stickers::grid_items(
                    state,
                    stored(Tab::Album(album.album_id)),
                    items,
                    loading,
                    Some(selected),
                    move |sticker| selected.set(sticker.sticker_id),
                )
                .into_element(),
                Some(_) => view().hidden().into_element(),
                None => text(move || {
                    if state.failed.get() {
                        i18n.text(Key::CommandFailed)
                    } else {
                        tr(i18n, "Loading stickers…", "正在加载贴纸…")
                    }
                })
                .text_role(ui::TextRole::Body)
                .into_element(),
            })
        }))
        .child(when(state.failed, move || {
            ui::action(
                move || i18n.text(Key::Retry),
                move || {
                    state.failed.set(false);
                    state.reload();
                },
            )
        }))
}
