use mixin_desktop_api::{AccountClient, MessageListView, StickerAlbumItem, StickerItem};
use nsh_base::{PopoverEvent, menu_item, theme};
use nsh_core::prelude::*;
use std::{rc::Rc, sync::Arc};

use super::{emoji, gifs, sticker_store, tr};
use crate::{
    i18n::{self, Key},
    runtime, ui,
};

#[derive(Clone, PartialEq)]
pub(super) enum Tab {
    Emoji,
    Recent,
    Personal,
    Store,
    Gif,
    Album(String),
}
#[derive(Clone, Copy)]
pub(super) struct State {
    pub account: Stored<Arc<AccountClient>>,
    pub conversation: Stored<String>,
    pub open: Signal<bool>,
    pub tab: Signal<Tab>,
    pub revision: Signal<u64>,
    pub busy: Signal<bool>,
    pub failed: Signal<bool>,
    pub albums: Signal<Vec<StickerAlbumItem>>,
    pub quote: Signal<Option<MessageListView>>,
}
impl State {
    pub(super) fn observe_updates(self) {
        use futures::StreamExt as _;
        // Subscribe before the initial queries. A metadata update completing
        // during the first read must still invalidate the visible projections.
        let account = self.account.with(|account| account.clone());
        let changes = runtime::observe(account.conversation_changes());
        spawn(async move {
            futures::pin_mut!(changes);
            while let Some(change) = changes.next().await {
                if change.reload_all {
                    self.revision
                        .update(|revision| *revision = revision.wrapping_add(1));
                }
            }
        });
    }

    pub fn clear_quote(self, sent: Option<String>) {
        if self
            .quote
            .with(|q| q.as_ref().map(|m| m.message_id.clone()))
            == sent
        {
            self.quote.set(None);
        }
    }
    pub fn send(self, sticker: String) {
        if self.busy.get_untracked() {
            return;
        }
        let client = self.account.with(|a| a.clone());
        let id = self.conversation.with(|id| id.clone());
        let sent_quote = self
            .quote
            .with(|q| q.as_ref().map(|m| m.message_id.clone()));
        self.busy.set(true);
        self.failed.set(false);
        spawn(async move {
            match runtime::commit(async move { client.message().send_sticker(id, sticker).await })
                .await
            {
                Ok(_) => {
                    self.clear_quote(sent_quote);
                    self.revision.update(|v| *v += 1);
                    self.open.set(false);
                }
                Err(_) => self.failed.set(true),
            }
            self.busy.set(false);
        });
    }
}

pub(crate) fn stickers(
    account: Arc<AccountClient>,
    id: String,
    failed: Signal<bool>,
    insert: Rc<dyn Fn(String)>,
    quote: Signal<Option<MessageListView>>,
) -> impl IntoElement {
    let i18n = i18n::current();
    let state = State {
        account: stored(account),
        conversation: stored(id),
        open: signal(false),
        tab: signal(Tab::Recent),
        revision: signal(0),
        busy: signal(false),
        failed,
        albums: signal_with(Vec::new(), SignalEquals::Never),
        quote,
    };
    let insert = stored(insert);
    ui::popover()
        .open(move || Some(state.open.get()))
        .on_open_change(move |event: &PopoverEvent| {
            if let PopoverEvent::OpenChange(value, _) = event
                && !state.busy.get_untracked()
            {
                state.open.set(*value);
            }
        })
        .offset(px(8.0))
        .trigger(
            ui::icon_button()
                .size_10()
                .px_1()
                .py_1()
                .accessibility_label(move || Some(i18n.text(Key::Emoji).into()))
                .child(ui::icon(ui::Icon::Sticker).size_6()),
        )
        .child(when(state.open, move || {
            state.observe_updates();
            effect(move || {
                state.revision.get();
                let client = state.account.with(|a| a.clone());
                spawn(async move {
                    match runtime::run(async move { client.sticker().sticker_albums().await }).await
                    {
                        Ok(values) => state.albums.set(values),
                        Err(_) => state.failed.set(true),
                    }
                });
            });
            view()
                .w(px(464.0))
                .max_w(relative(0.95))
                .h(px(407.0))
                .min_h_0()
                .p_3()
                .gap_2()
                .child(when(state.failed, move || {
                    view()
                        .flex_row()
                        .gap_2()
                        .child(
                            text(move || i18n.text(Key::CommandFailed))
                                .text_xs()
                                .text_color(move || theme().destructive),
                        )
                        .child(ui::action(
                            move || i18n.text(Key::Retry),
                            move || {
                                state.failed.set(false);
                                state.revision.update(|v| *v += 1);
                            },
                        ))
                }))
                .child(dynamic(move || {
                    let tab = state.tab.get();
                    untracked(|| match tab {
                        Tab::Emoji => emoji::panel(
                            insert.with(|i| i.clone()),
                            state.account.with(|a| a.account_id()),
                        )
                        .into_element(),
                        Tab::Store => sticker_store::panel(state).into_element(),
                        Tab::Gif => gifs::panel(state).into_element(),
                        tab => grid(state, tab).into_element(),
                    })
                }))
                .child(
                    scroll().horizontal().child(
                        view()
                            .flex_row()
                            .gap_1()
                            .children(
                                [
                                    (Tab::Store, "+"),
                                    (Tab::Emoji, "😀"),
                                    (Tab::Recent, "◷"),
                                    (Tab::Personal, "☆"),
                                    (Tab::Gif, "GIF"),
                                ]
                                .into_iter()
                                .map(move |(tab, title)| {
                                    let tab = stored(tab);
                                    ui::command()
                                        .min_w_8()
                                        .px_2()
                                        .py_1()
                                        .style_when(
                                            move || state.tab.get() == tab.with(|t| t.clone()),
                                            |s| s.bg(move || theme().accent),
                                        )
                                        .on_press(move || state.tab.set(tab.with(|t| t.clone())))
                                        .child(text(title).text_sm())
                                }),
                            )
                            .child(each(
                                move || {
                                    state.albums.with(|rows| {
                                        rows.iter()
                                            .map(|album| {
                                                (
                                                    album.album_id.clone(),
                                                    album.name.clone(),
                                                    album.icon_url.clone(),
                                                )
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                },
                                |album| album.0.clone(),
                                move |album, _| {
                                    ui::command()
                                        .size_8()
                                        .px_1()
                                        .py_1()
                                        .accessibility_label(move || {
                                            Some(album.with(|a| a.1.clone()).into())
                                        })
                                        .on_press(move || {
                                            state.tab.set(Tab::Album(album.with(|a| a.0.clone())))
                                        })
                                        .style_when(
                                            move || {
                                                state.tab.get()
                                                    == Tab::Album(album.with(|a| a.0.clone()))
                                            },
                                            |s| s.bg(move || theme().accent),
                                        )
                                        .child(crate::sticker::album_icon(
                                            state.account.with(|value| value.clone()),
                                            move || album.with(|a| a.2.clone()),
                                            24.0,
                                        ))
                                },
                            )),
                    ),
                )
        }))
}

pub(super) fn grid(state: State, tab: Tab) -> impl IntoElement {
    let items = signal_with(Vec::<StickerItem>::new(), SignalEquals::Never);
    let loading = signal(true);
    let tab = stored(tab);
    effect(move || {
        state.revision.get();
        let client = state.account.with(|a| a.clone());
        let tab = tab.with(|t| t.clone());
        loading.set(true);
        spawn(async move {
            match runtime::run(async move {
                match tab {
                    Tab::Recent => client.sticker().recent_stickers().await,
                    Tab::Personal => client.sticker().personal_stickers().await,
                    Tab::Album(id) => client.sticker().album_stickers(id).await,
                    _ => Ok(Vec::new()),
                }
            })
            .await
            {
                Ok(values) => items.set(values),
                Err(_) => state.failed.set(true),
            }
            loading.set(false);
        });
    });
    grid_items(state, tab, items, loading, None, move |sticker| {
        state.send(sticker.sticker_id)
    })
}

pub(super) fn grid_items(
    state: State,
    tab: Stored<Tab>,
    items: Signal<Vec<StickerItem>>,
    loading: Signal<bool>,
    selected: Option<Signal<String>>,
    select: impl Fn(StickerItem) + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    let viewable = signal(None::<(usize, usize)>);
    let select = Rc::new(select);
    view()
        .flex_1()
        .min_h_0()
        .gap_2()
        .child(when(loading, move || {
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(ui::busy_indicator(
                    move || loading.get(),
                    move || tr(i18n, "Loading stickers…", "正在加载贴纸…").into(),
                ))
                .child(text(move || tr(i18n, "Loading stickers…", "正在加载贴纸…")).text_sm())
        }))
        .child(when(
            move || !loading.get() && items.with(Vec::is_empty),
            move || {
                view()
                    .flex_1()
                    .min_h_0()
                    .items_center()
                    .justify_center()
                    .child(
                        text(move || i18n.text(Key::NoStickers))
                            .text_sm()
                            .text_color(move || theme().muted_foreground),
                    )
            },
        ))
        .child(
            virtual_list(
                move || items.with(Vec::len),
                move |at| {
                    let select = select.clone();
                    crate::sticker::visibility(move || {
                        viewable
                            .get()
                            .is_some_and(|(first, last)| (first..=last).contains(&at.index()))
                    });
                    ui::context_menu()
                        .trigger(
                            ui::command()
                                .size_20()
                                .px_1()
                                .py_1()
                                .disabled(state.busy)
                                .style_when(
                                    move || {
                                        selected.is_some_and(|selected| {
                                            items.with(|rows| {
                                                rows.get(at.index()).is_some_and(|sticker| {
                                                    selected.with(|id| *id == sticker.sticker_id)
                                                })
                                            })
                                        })
                                    },
                                    |style| style.bg(move || theme().accent),
                                )
                                .on_press(move || {
                                    if let Some(sticker) =
                                        items.with(|rows| rows.get(at.index()).cloned())
                                    {
                                        select(sticker);
                                    }
                                })
                                .accessibility_label(move || {
                                    Some(
                                        items
                                            .with(|rows| {
                                                rows.get(at.index())
                                                    .map(|s| s.name.clone())
                                                    .unwrap_or_default()
                                            })
                                            .into(),
                                    )
                                })
                                .child(crate::sticker::asset(
                                    state.account.with(|value| value.clone()),
                                    move || {
                                        items.with(|rows| {
                                            rows.get(at.index())
                                                .map(|s| {
                                                    (
                                                        Some(s.sticker_id.clone()),
                                                        s.asset_url.clone(),
                                                        s.asset_type.clone(),
                                                    )
                                                })
                                                .unwrap_or_default()
                                        })
                                    },
                                    64.0,
                                )),
                        )
                        .child(dynamic(move || {
                            let sticker =
                                untracked(|| items.with(|rows| rows.get(at.index()).cloned()));
                            let Some(sticker) = sticker else {
                                return view().hidden().into_element();
                            };
                            let personal = tab.with(|t| *t == Tab::Personal);
                            let title = if personal {
                                tr(i18n, "Remove sticker", "移除贴纸")
                            } else {
                                tr(i18n, "Add sticker", "添加贴纸")
                            };
                            menu_item()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .hover(|s| s.bg(move || theme().accent))
                                .child(text(title).text_sm())
                                .on_select(move || {
                                    if state.busy.get_untracked() {
                                        return;
                                    }
                                    let client = state.account.with(|a| a.clone());
                                    let id = sticker.sticker_id.clone();
                                    state.busy.set(true);
                                    spawn(async move {
                                        state.failed.set(
                                            runtime::commit(async move {
                                                if personal {
                                                    client.sticker().remove_sticker(id).await
                                                } else {
                                                    client.sticker().add_sticker(id).await
                                                }
                                            })
                                            .await
                                            .is_err(),
                                        );
                                        state.busy.set(false);
                                        state.revision.update(|v| *v += 1);
                                    });
                                })
                                .into_element()
                        }))
                },
            )
            .keys(move || {
                items.with(|rows| {
                    rows.iter()
                        .map(|s| s.sticker_id.clone())
                        .collect::<Vec<_>>()
                })
            })
            .on_viewable_items_change(move |event: &ViewableItems| viewable.set(event.viewable))
            .columns(move || {
                (((window().width.0 * 0.95).min(464.0) - 24.0) / 80.0)
                    .floor()
                    .max(1.0) as u32
            })
            .estimated_item_size(px(80.0))
            .flex_1()
            .min_h_0()
            .style_when(
                move || items.with(Vec::is_empty),
                |style| style.hidden().invisible(),
            ),
        )
        .child(when(
            move || tab.with(|t| *t == Tab::Personal),
            move || {
                ui::action(
                    move || tr(i18n, "Import sticker…", "导入贴纸…"),
                    move || {
                        if state.busy.get_untracked() {
                            return;
                        }
                        let client = state.account.with(|a| a.clone());
                        spawn(async move {
                            match pick_paths(nsh_core::file_dialog::OpenDialog::files()).await {
                                Ok(Some(paths)) if !paths.is_empty() => {
                                    let path = paths[0].to_string_lossy().into_owned();
                                    state.busy.set(true);
                                    state.failed.set(
                                        runtime::commit(async move {
                                            client.sticker().add_sticker_from_path(path).await
                                        })
                                        .await
                                        .is_err(),
                                    );
                                    state.busy.set(false);
                                    state.revision.update(|v| *v += 1);
                                }
                                Ok(_) => {}
                                Err(_) => state.failed.set(true),
                            }
                        });
                    },
                )
                .disabled(state.busy)
            },
        ))
        .child(when(
            move || matches!(tab.with(|t| t.clone()), Tab::Album(_)),
            move || {
                let album = tab.with(|t| {
                    if let Tab::Album(id) = t {
                        id.clone()
                    } else {
                        String::new()
                    }
                });
                sticker_store::album_toggle(state, album)
            },
        ))
}
