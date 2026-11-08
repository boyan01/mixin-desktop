mod album_order;

use mixin_desktop_api::{AccountClient, StickerAlbumItem, StickerDetailItem, StickerItem};
use nsh_core::prelude::*;
use std::{rc::Rc, sync::Arc};

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

/// The store lives outside the hover popover, so pointer exit cannot dismiss it.
pub(super) fn dialog(picker: State, open: Signal<bool>) -> impl IntoElement {
    let presence = crate::motion::Presence::modal(move || open.get());
    when(
        move || presence.mounted(),
        move || {
            let state = State::new(picker.client(), String::new(), true, signal(false));
            state.observe_updates();
            let store = signal_with(Vec::<StickerAlbumItem>::new(), SignalEquals::Never);
            let loading = signal(true);
            let managing = signal(false);
            effect(move || {
                state.revision.get();
                let client = state.client();
                loading.set(true);
                spawn(async move {
                    match runtime::run(async move {
                        let store = client.sticker().sticker_store_albums().await?;
                        let albums = client.sticker().sticker_albums().await?;
                        Ok((store, albums))
                    })
                    .await
                    {
                        Ok((rows, albums)) => batch(|| {
                            store.set(rows);
                            state.albums.set(albums);
                        }),
                        Err(_) => state.failed.set(true),
                    }
                    loading.set(false);
                });
            });
            let close = move || {
                if !state.busy.get_untracked() {
                    open.set(false);
                }
            };
            ui::dialog(
                presence,
                close,
                StoreView {
                    account: Some(state.client()),
                    store,
                    albums: state.albums,
                    managing,
                    busy: state.busy,
                }
                .render(
                    move |id, added| {
                        let client = state.client();
                        state.update(async move {
                            client.sticker().set_sticker_album_added(id, added).await
                        });
                    },
                    close,
                    feedback(store, loading, state.failed, managing, move || {
                        state.failed.set(false);
                        state.reload();
                    }),
                    album_order::list(
                        state.albums,
                        state.busy,
                        move |ids| {
                            let client = state.client();
                            state.update(async move {
                                client.sticker().set_sticker_album_order(ids).await
                            });
                        },
                        move |id| {
                            let client = state.client();
                            state.update(async move {
                                client.sticker().set_sticker_album_added(id, false).await
                            });
                        },
                    ),
                ),
            )
        },
    )
}

/// Presentation bindings shared by the account dialog and offline previews.
struct StoreView {
    account: Option<Arc<AccountClient>>,
    store: Signal<Vec<StickerAlbumItem>>,
    albums: Signal<Vec<StickerAlbumItem>>,
    managing: Signal<bool>,
    busy: Signal<bool>,
}

impl StoreView {
    fn render(
        self,
        toggle: impl Fn(String, bool) + 'static,
        close: impl Fn() + 'static,
        status: impl IntoElement,
        order: impl IntoElement,
    ) -> View {
        let Self {
            account,
            store,
            albums,
            managing,
            busy,
        } = self;
        let i18n = i18n::current();
        let account = stored(account);
        let toggle = stored(Rc::new(toggle) as Rc<dyn Fn(String, bool)>);
        let viewable = signal(None::<(usize, usize)>);
        ui::dialog_card(ui::DialogWidth::Wide)
            .h(ui::dialog_height_tall())
            .p_0()
            .gap_0()
            .overflow_hidden()
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .flex_shrink_0()
                    .child(when(managing, move || {
                        ui::icon_button()
                            .size_8()
                            .px_0()
                            .py_0()
                            .accessibility_label(move || Some(tr(i18n, "Back", "返回").into()))
                            .on_press(move || managing.set(false))
                            .child(ui::icon(ui::Icon::ArrowLeft))
                    }))
                    .child(
                        ui::dialog_title(move || {
                            if managing.get() {
                                tr(i18n, "Subscribed stickers", "已订阅的表情")
                            } else {
                                tr(i18n, "Sticker store", "贴纸商店")
                            }
                        })
                        .flex_1()
                        .min_w_0()
                        .number_of_lines(1),
                    )
                    .child(when(
                        move || !managing.get(),
                        move || {
                            ui::button(ui::ButtonTone::Ghost, ui::ButtonSize::Small)
                                .on_press(move || managing.set(true))
                                .child(text(move || tr(i18n, "Manage subscriptions", "管理已订阅")))
                        },
                    ))
                    .child(ui::dialog_close(close).mt_0().mr_0().disabled(busy)),
            )
            .child(ui::divider().mx_0().my_0().flex_shrink_0())
            .child(status)
            .child(
                virtual_list(
                    move || store.with(Vec::len),
                    move |at| {
                        crate::sticker::visibility(move || {
                            !managing.get()
                                && viewable.get().is_some_and(|(first, last)| {
                                    (first..=last).contains(&at.index())
                                })
                        });
                        view()
                            .flex_row()
                            .items_center()
                            .min_h(ui::list_row_height())
                            .gap_3()
                            .px_4()
                            .py_3()
                            .child(crate::sticker::framed_album_icon(
                                account.with(|a| a.clone()),
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
                                    .gap_1()
                                    .child(
                                        text(move || {
                                            store.with(|rows| {
                                                rows.get(at.index())
                                                    .map(|a| a.name.clone())
                                                    .unwrap_or_default()
                                            })
                                        })
                                        .text_role(ui::TextRole::Body)
                                        .text_emphasis()
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
                                        .text_color(ui::fg_secondary)
                                        .number_of_lines(2),
                                    ),
                            )
                            .child(
                                ui::button(ui::ButtonTone::Ghost, ui::ButtonSize::Small)
                                    .flex_shrink_0()
                                    .disabled(busy)
                                    .text_color(move || {
                                        if store.with(|rows| {
                                            rows.get(at.index()).is_some_and(|a| a.added)
                                        }) {
                                            ui::fg_primary()
                                        } else {
                                            ui::accent()
                                        }
                                    })
                                    .on_press(move || {
                                        if let Some(album) =
                                            store.with(|rows| rows.get(at.index()).cloned())
                                        {
                                            toggle.with(|toggle| {
                                                toggle(album.album_id, !album.added)
                                            });
                                        }
                                    })
                                    .child(text(move || {
                                        if store.with(|rows| {
                                            rows.get(at.index()).is_some_and(|a| a.added)
                                        }) {
                                            tr(i18n, "Remove", "移除")
                                        } else {
                                            tr(i18n, "Add", "添加")
                                        }
                                    })),
                            )
                    },
                )
                .keys(move || {
                    store.with(|rows| rows.iter().map(|a| a.album_id.clone()).collect::<Vec<_>>())
                })
                .on_viewable_items_change(move |event: &ViewableItems| viewable.set(event.viewable))
                .estimated_item_size(ui::list_row_height())
                .content_padding_y(ui::space_sm())
                .flex_1()
                .min_h_0()
                .style_when(
                    move || managing.get() || store.with(Vec::is_empty),
                    |s| s.hidden().invisible(),
                ),
            )
            .child(view().flex_1().min_h_0().child(order).style_when(
                move || !managing.get() || albums.with(Vec::is_empty),
                |s| s.hidden().invisible(),
            ))
            .child(when(
                move || managing.get() && albums.with(Vec::is_empty),
                move || {
                    ui::placeholder(move || {
                        tr(i18n, "No subscribed sticker albums", "还没有订阅表情专辑")
                    })
                },
            ))
    }
}

fn feedback(
    store: Signal<Vec<StickerAlbumItem>>,
    loading: Signal<bool>,
    failed: Signal<bool>,
    managing: Signal<bool>,
    retry: impl Fn() + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    let retry = stored(retry);
    view()
        .gap_2()
        .style_when(
            move || !managing.get() && !loading.get() && !failed.get() && store.with(Vec::is_empty),
            |s| s.flex_1().min_h_0(),
        )
        .child(when(loading, move || {
            view().px_4().py_2().child(ui::busy_indicator(
                move || loading.get(),
                move || tr(i18n, "Loading stickers…", "正在加载贴纸…").into(),
            ))
        }))
        .child(when(failed, move || {
            view()
                .flex_row()
                .items_center()
                .px_4()
                .gap_2()
                .child(ui::error_text(move || i18n.text(Key::CommandFailed)).flex_1())
                .child(
                    ui::button(ui::ButtonTone::Ghost, ui::ButtonSize::Small)
                        .on_press(move || retry.with(|retry| retry()))
                        .child(text(move || i18n.text(Key::Retry))),
                )
        }))
        .child(when(
            move || !managing.get() && !loading.get() && !failed.get() && store.with(Vec::is_empty),
            move || {
                ui::placeholder(move || tr(i18n, "No sticker albums available", "暂无贴纸专辑"))
            },
        ))
        .style_when(
            move || {
                !loading.get() && !failed.get() && (managing.get() || !store.with(Vec::is_empty))
            },
            |s| s.hidden().invisible(),
        )
}

#[cfg(debug_assertions)]
pub(crate) fn detached(
    store: Signal<Vec<StickerAlbumItem>>,
    albums: Signal<Vec<StickerAlbumItem>>,
    managing: Signal<bool>,
) -> View {
    let busy = signal(false);
    let toggle = move |id: String, added: bool| {
        store.update(|rows| {
            if let Some(album) = rows.iter_mut().find(|a| a.album_id == id) {
                album.added = added;
            }
        });
        if added {
            if let Some(album) = store.with(|rows| rows.iter().find(|a| a.album_id == id).cloned())
            {
                albums.update(|rows| rows.push(album));
            }
        } else {
            albums.update(|rows| rows.retain(|a| a.album_id != id));
        }
    };
    StoreView {
        account: None,
        store,
        albums,
        managing,
        busy,
    }
    .render(
        toggle,
        || {},
        feedback(store, signal(false), signal(false), managing, || {}),
        album_order::list(
            albums,
            busy,
            move |ids| {
                albums.update(|rows| {
                    rows.sort_by_key(|row| ids.iter().position(|id| *id == row.album_id))
                });
            },
            move |id| toggle(id, false),
        ),
    )
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
