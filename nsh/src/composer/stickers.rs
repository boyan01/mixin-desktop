use mixin_desktop_api::{
    AccountClient, ClientResult, MessageListView, StickerAlbumItem, StickerItem,
};
use nsh_base::{ButtonElement, PopoverEvent};
use nsh_core::{element::desktop::MenuItem, prelude::*};
use std::{future::Future, rc::Rc, sync::Arc};

use super::{emoji, gifs, sticker_store, tool_button, tr};
use crate::{
    i18n::{self, Key},
    runtime,
    ui::{self, Typography as _},
};

/// A sticker bitmap and its margin, which the grid's column count depends on.
const STICKER_CELL: f32 = 80.0;

/// How many `cell`-sized columns fit the panel at the current window width.
pub(super) fn columns(cell: f32) -> u32 {
    (((window().width.0 * 0.95).min(ui::sticker_panel_width().0) - ui::space_md().0 * 2.0) / cell)
        .floor()
        .max(1.0) as u32
}

/// A compact tab of the panel, with the shared selected treatment.
pub(super) fn panel_tab() -> ButtonElement {
    ui::button(ui::ButtonTone::Ghost, ui::ButtonSize::Small)
        .min_w(ui::button_height_small())
        .h(ui::button_height_small())
        .px_1()
}

/// What a panel tab shows: a glyph that has an icon uses the icon.
#[derive(Clone, Copy)]
pub(super) enum TabLabel {
    Icon(ui::Icon),
    Text(&'static str),
}

impl TabLabel {
    pub(super) fn element(self) -> AnyElement {
        match self {
            Self::Icon(icon) => ui::icon(icon).into_element(),
            Self::Text(label) => text(label).text_role(ui::TextRole::Body).into_element(),
        }
    }
}

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
    // Commands outlive the tab or cell that started them, so `busy` always settles.
    owner: Owner,
    pub open: Signal<bool>,
    pub tab: Signal<Tab>,
    pub revision: Signal<u64>,
    pub busy: Signal<bool>,
    pub failed: Signal<bool>,
    pub albums: Signal<Vec<StickerAlbumItem>>,
    navigation: crate::navigation::ChatNavigation,
}
impl State {
    pub(super) fn new(
        account: Arc<AccountClient>,
        conversation: String,
        open: bool,
        failed: Signal<bool>,
    ) -> Self {
        Self {
            account: stored(account),
            conversation: stored(conversation),
            owner: Owner::current(),
            open: signal(open),
            tab: signal(Tab::Recent),
            revision: signal(0),
            busy: signal(false),
            failed,
            albums: signal_with(Vec::new(), SignalEquals::Never),
            navigation: use_context::<crate::navigation::ChatNavigation>()
                .expect("Home provides chat navigation"),
        }
    }

    pub(super) fn client(self) -> Arc<AccountClient> {
        self.account.with(|account| account.clone())
    }

    pub(super) fn reload(self) {
        self.revision
            .update(|revision| *revision = revision.wrapping_add(1));
    }

    pub(super) fn observe_updates(self) {
        use futures::StreamExt as _;
        // Subscribe before the initial queries. A metadata update completing
        // during the first read must still invalidate the visible projections.
        let changes = runtime::observe(self.client().conversation_changes());
        spawn(async move {
            futures::pin_mut!(changes);
            while let Some(change) = changes.next().await {
                if change.reload_all {
                    self.reload();
                }
            }
        });
    }

    /// Runs one durable command at a time, then `done` with its success.
    fn run<T: Send + 'static>(
        self,
        work: impl Future<Output = ClientResult<T>> + Send + 'static,
        done: impl FnOnce(bool) + 'static,
    ) {
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        self.failed.set(false);
        self.owner.run(|| {
            spawn(async move {
                let succeeded = runtime::commit(work).await.is_ok();
                self.failed.set(!succeeded);
                self.busy.set(false);
                done(succeeded);
            });
        });
    }

    /// Changes sticker data and refreshes what the panel shows.
    pub(super) fn update<T: Send + 'static>(
        self,
        work: impl Future<Output = ClientResult<T>> + Send + 'static,
    ) {
        self.run(work, move |_| self.reload());
    }

    /// Sends a message from the panel, which closes once it is accepted.
    pub(super) fn send<T: Send + 'static>(
        self,
        work: impl Future<Output = ClientResult<T>> + Send + 'static,
    ) {
        self.run(work, move |succeeded| {
            if succeeded {
                self.open.set(false);
                self.navigation.show_latest();
            }
        });
    }

    fn send_sticker(self, sticker: String) {
        let client = self.client();
        let id = self.conversation.with(|id| id.clone());
        self.send(async move { client.message().send_sticker(id, sticker).await });
    }
}

pub(crate) fn stickers(
    account: Arc<AccountClient>,
    id: String,
    failed: Signal<bool>,
    insert: Rc<dyn Fn(String)>,
    // Stickers and GIFs carry no quote, so a pending reply stays in the composer.
    _quote: Signal<Option<MessageListView>>,
) -> impl IntoElement {
    let i18n = i18n::current();
    let state = State::new(account, id, false, failed);
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
        .offset(ui::space_sm())
        .trigger(
            tool_button()
                .accessibility_label(move || Some(i18n.text(Key::Emoji).into()))
                .child(ui::icon(ui::Icon::Sticker)),
        )
        .child(when(state.open, move || {
            state.observe_updates();
            // Panel errors belong to the panel, not to the composer behind it.
            cleanup(move || {
                let _ = state.failed.try_set(false);
            });
            effect(move || {
                state.revision.get();
                let client = state.client();
                spawn(async move {
                    match runtime::run(async move { client.sticker().sticker_albums().await }).await
                    {
                        Ok(values) => state.albums.set(values),
                        Err(_) => state.failed.set(true),
                    }
                });
            });
            view()
                .w(ui::sticker_panel_width())
                .max_w(relative(0.95))
                .h(ui::sticker_panel_height())
                .min_h_0()
                .p_3()
                .gap_2()
                .child(when(state.failed, move || {
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(ui::error_text(move || i18n.text(Key::CommandFailed)).flex_1())
                        .child(ui::action(
                            move || i18n.text(Key::Retry),
                            move || {
                                state.failed.set(false);
                                state.reload();
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
                                    (
                                        Tab::Store,
                                        TabLabel::Icon(ui::Icon::Add),
                                        "Sticker store",
                                        "贴纸商店",
                                    ),
                                    (Tab::Emoji, TabLabel::Text("😀"), "Emoji", "表情"),
                                    (
                                        Tab::Recent,
                                        TabLabel::Icon(ui::Icon::Clock),
                                        "Recent stickers",
                                        "最近使用的贴纸",
                                    ),
                                    (
                                        Tab::Personal,
                                        TabLabel::Icon(ui::Icon::Star),
                                        "My stickers",
                                        "我的贴纸",
                                    ),
                                    (Tab::Gif, TabLabel::Text("GIF"), "GIFs", "GIF"),
                                ]
                                .into_iter()
                                .map(
                                    move |(tab, title, en, zh)| {
                                        let tab = stored(tab);
                                        panel_tab()
                                            .accessibility_label(move || {
                                                Some(tr(i18n, en, zh).into())
                                            })
                                            .selected(move || {
                                                state.tab.get() == tab.with(|t| t.clone())
                                            })
                                            .on_press(move || {
                                                state.tab.set(tab.with(|t| t.clone()))
                                            })
                                            .child(title.element())
                                    },
                                ),
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
                                    panel_tab()
                                        .accessibility_label(move || {
                                            Some(album.with(|a| a.1.clone()).into())
                                        })
                                        .on_press(move || {
                                            state.tab.set(Tab::Album(album.with(|a| a.0.clone())))
                                        })
                                        .selected(move || {
                                            state.tab.get()
                                                == Tab::Album(album.with(|a| a.0.clone()))
                                        })
                                        .child(crate::sticker::album_icon(
                                            state.client(),
                                            move || album.with(|a| a.2.clone()),
                                            ui::icon_lg().0,
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
        let client = state.client();
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
        state.send_sticker(sticker.sticker_id)
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
                .child(
                    text(move || tr(i18n, "Loading stickers…", "正在加载贴纸…"))
                        .text_role(ui::TextRole::Body),
                )
        }))
        .child(when(
            move || !loading.get() && !state.failed.get() && items.with(Vec::is_empty),
            move || ui::placeholder(move || i18n.text(Key::NoStickers)),
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
                    ui::native_context_menu(
                        ui::command()
                            .size_20()
                            .px_1()
                            .py_1()
                            .disabled(state.busy)
                            .selected(move || {
                                selected.is_some_and(|selected| {
                                    items.with(|rows| {
                                        rows.get(at.index()).is_some_and(|sticker| {
                                            selected.with(|id| *id == sticker.sticker_id)
                                        })
                                    })
                                })
                            })
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
                                state.client(),
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
                        move || {
                            let Some(sticker) = items.with(|rows| rows.get(at.index()).cloned())
                            else {
                                return Vec::new();
                            };
                            let personal = tab.with(|t| *t == Tab::Personal);
                            let title = if personal {
                                tr(i18n, "Delete", "删除")
                            } else {
                                tr(i18n, "Add Sticker", "添加贴纸")
                            };
                            vec![MenuItem::new(title).on_select(move || {
                                let client = state.client();
                                let id = sticker.sticker_id.clone();
                                state.update(async move {
                                    if personal {
                                        client.sticker().remove_sticker(id).await
                                    } else {
                                        client.sticker().add_sticker(id).await
                                    }
                                });
                            })]
                        },
                    )
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
            .columns(move || columns(STICKER_CELL))
            .estimated_item_size(px(STICKER_CELL))
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
                        let client = state.client();
                        state.owner.run(|| {
                            spawn(async move {
                                match pick_paths(nsh_core::file_dialog::OpenDialog::files()).await {
                                    Ok(Some(paths)) if !paths.is_empty() => {
                                        let path = paths[0].to_string_lossy().into_owned();
                                        state.update(async move {
                                            client.sticker().add_sticker_from_path(path).await
                                        });
                                    }
                                    Ok(_) => {}
                                    Err(_) => state.failed.set(true),
                                }
                            });
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
