use mixin_desktop_api::{
    AccountClient, ClientResult, MessageListView, StickerAlbumItem, StickerItem,
};
use nsh_base::{ButtonElement, PopoverEvent};
use nsh_core::{element::desktop::MenuItem, prelude::*};
use std::{future::Future, rc::Rc, sync::Arc};

use super::{emoji, sticker_store, tool_button, tr};
use crate::{
    i18n::{self, Key},
    runtime,
    ui::{self, Typography as _},
};

/// A sticker bitmap and its margin, which the grid's column count depends on.
pub(crate) fn cell_size() -> Points {
    ui::sticker_cell_size()
}

/// How many `cell`-sized columns fit the panel at the current window width.
pub(crate) fn columns(cell: f32) -> u32 {
    (((window().width.0 * 0.95).min(ui::sticker_panel_width().0) - ui::space_md().0 * 2.0) / cell)
        .floor()
        .max(1.0) as u32
}

/// A compact tab of the panel, with the shared selected treatment.
pub(super) fn panel_tab() -> ButtonElement {
    ui::button(ui::ButtonTone::Ghost, ui::ButtonSize::Small)
        .min_w(ui::sticker_tab_size())
        .h(ui::sticker_tab_size())
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
pub(crate) enum Tab {
    Emoji,
    Recent,
    Personal,
    Store,
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

    fn import_sticker(self) {
        if self.busy.get_untracked() {
            return;
        }
        let client = self.client();
        self.owner.run(|| {
            spawn(async move {
                match pick_paths(nsh_core::file_dialog::OpenDialog::files()).await {
                    Ok(Some(paths)) if !paths.is_empty() => {
                        let path = paths[0].to_string_lossy().into_owned();
                        self.update(
                            async move { client.sticker().add_sticker_from_path(path).await },
                        );
                    }
                    Ok(_) => {}
                    Err(_) => self.failed.set(true),
                }
            });
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
    // Stickers carry no quote, so a pending reply stays in the composer.
    _quote: Signal<Option<MessageListView>>,
) -> impl IntoElement {
    let i18n = i18n::current();
    let state = State::new(account, id, false, failed);
    let insert = stored(insert);
    // The panel opens while the pointer rests on the button or the panel, and closes a
    // moment after it leaves both, so crossing the gap between them keeps it open.
    let over_button = signal(false);
    let over_panel = signal(false);
    let hover_owner = Owner::current();
    let hover = move |entered: bool, flag: Signal<bool>| {
        flag.set(entered);
        let delay = if entered { 120 } else { 250 };
        hover_owner.run(|| {
            set_timeout(std::time::Duration::from_millis(delay), move || {
                let hovered = over_button.get_untracked() || over_panel.get_untracked();
                if hovered != state.open.get_untracked() && !state.busy.get_untracked() {
                    state.open.set(hovered);
                }
            });
        });
    };
    ui::popover_flush()
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
            view()
                .tooltip(ui::hint(move || {
                    tr(i18n, "Emoji and stickers", "表情与贴纸").to_owned()
                }))
                .on_mouse_enter(move |_: &MouseEvent| hover(true, over_button))
                .on_mouse_leave(move |_: &MouseEvent| hover(false, over_button))
                .child(
                    tool_button()
                        .accessibility_label(move || Some(i18n.text(Key::Emoji).into()))
                        .style_when(state.open, |s| {
                            s.bg(ui::state_selected).text_color(ui::fg_primary)
                        })
                        .child(ui::icon(ui::Icon::Sticker)),
                ),
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
            panel_surface(
                Some(state.client()),
                state.tab,
                state.albums,
                view()
                    .flex_1()
                    .min_h_0()
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
                            tab => grid(state, tab).into_element(),
                        })
                    })),
            )
            .on_mouse_enter(move |_: &MouseEvent| hover(true, over_panel))
            .on_mouse_leave(move |_: &MouseEvent| hover(false, over_panel))
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
    let personal = tab.with(|t| *t == Tab::Personal);
    view()
        .flex_1()
        .min_h_0()
        .gap_2()
        .child(when(loading, move || {
            view()
                .flex_1()
                .justify_center()
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
            move || !loading.get() && !state.failed.get() && !personal && items.with(Vec::is_empty),
            move || empty_stickers(move || tab.with(|t| t.clone())),
        ))
        .child(
            virtual_list(
                move || items.with(Vec::len) + usize::from(personal),
                move |at| {
                    let select = select.clone();
                    let index = move || at.index().checked_sub(usize::from(personal));
                    crate::sticker::visibility(move || {
                        viewable
                            .get()
                            .is_some_and(|(first, last)| (first..=last).contains(&at.index()))
                    });
                    view()
                        .items_center()
                        .child(when(
                            move || personal && at.index() == 0,
                            move || import_tile(state.busy, move || state.import_sticker()),
                        ))
                        .child(when(
                            move || index().is_some(),
                            move || {
                                let select = select.clone();
                                ui::native_context_menu(
                                    sticker_tile()
                                        .disabled(state.busy)
                                        .selected(move || {
                                            selected.is_some_and(|selected| {
                                                items.with(|rows| {
                                                    index()
                                                        .and_then(|index| rows.get(index))
                                                        .is_some_and(|sticker| {
                                                            selected.with(|id| {
                                                                *id == sticker.sticker_id
                                                            })
                                                        })
                                                })
                                            })
                                        })
                                        .on_press(move || {
                                            if let Some(sticker) = items.with(|rows| {
                                                index().and_then(|index| rows.get(index)).cloned()
                                            }) {
                                                select(sticker);
                                            }
                                        })
                                        .accessibility_label(move || {
                                            Some(
                                                items
                                                    .with(|rows| {
                                                        index()
                                                            .and_then(|index| rows.get(index))
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
                                                    index()
                                                        .and_then(|index| rows.get(index))
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
                                            ui::sticker_art_size().0,
                                        )),
                                    move || {
                                        let Some(sticker) = items.with(|rows| {
                                            index().and_then(|index| rows.get(index)).cloned()
                                        }) else {
                                            return Vec::new();
                                        };
                                        let personal = tab.with(|t| *t == Tab::Personal);
                                        let title = if personal {
                                            tr(i18n, "Delete", "删除")
                                        } else {
                                            tr(i18n, "Add Sticker", "添加贴纸")
                                        };
                                        let icon = if personal {
                                            ui::MenuIcon::Delete
                                        } else {
                                            ui::MenuIcon::Sticker
                                        };
                                        vec![MenuItem::new(title).icon(icon.src()).on_select(
                                            move || {
                                                let client = state.client();
                                                let id = sticker.sticker_id.clone();
                                                state.update(async move {
                                                    if personal {
                                                        client.sticker().remove_sticker(id).await
                                                    } else {
                                                        client.sticker().add_sticker(id).await
                                                    }
                                                });
                                            },
                                        )]
                                    },
                                )
                            },
                        ))
                },
            )
            .content_padding_x(ui::space_md())
            .content_padding_y(ui::space_sm())
            .keys(move || {
                let mut keys = Vec::new();
                if personal {
                    keys.push("import-sticker".to_owned());
                }
                items.with(|rows| keys.extend(rows.iter().map(|s| s.sticker_id.clone())));
                keys
            })
            .on_viewable_items_change(move |event: &ViewableItems| viewable.set(event.viewable))
            .columns(move || columns(cell_size().0))
            .estimated_item_size(cell_size())
            .flex_1()
            .min_h_0()
            .style_when(
                move || !personal && items.with(Vec::is_empty),
                |style| style.hidden().invisible(),
            ),
        )
}

/// The same panel shell is used by the account-backed composer and local previews.
pub(crate) fn panel_surface(
    account: Option<Arc<AccountClient>>,
    tab: Signal<Tab>,
    albums: Signal<Vec<StickerAlbumItem>>,
    content: impl IntoElement,
) -> View {
    let i18n = i18n::current();
    let account = stored(account);
    view()
        .style_with(|style| {
            style.w(px(
                (window().width.0 * 0.95).min(ui::sticker_panel_width().0)
            ))
        })
        .h(ui::sticker_panel_height())
        .min_h_0()
        .child(view().flex_1().min_h_0().child(content))
        .child(ui::divider().mx_0().my_0().flex_shrink_0())
        .child(
            // Keep the native horizontal indicator outside the clipped footer.
            view()
                .h(ui::sticker_tab_bar_height())
                .flex_shrink_0()
                .overflow_hidden()
                .bg(ui::surface_card)
                .child(
                    scroll()
                        .horizontal()
                        .scrollbar(false)
                        .w_full()
                        .h(ui::sticker_tab_bar_height() + ui::space_xl())
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .px_2()
                                .h(ui::sticker_tab_bar_height())
                                .children(
                                    [
                                        (
                                            Tab::Store,
                                            ui::Icon::StickerStore,
                                            "Sticker store",
                                            "贴纸商店",
                                        ),
                                        (Tab::Emoji, ui::Icon::Sticker, "Emoji", "表情"),
                                        (Tab::Recent, ui::Icon::Clock, "Recently used", "最近使用"),
                                        (
                                            Tab::Personal,
                                            ui::Icon::StickerFavorite,
                                            "My stickers",
                                            "我的贴纸",
                                        ),
                                    ]
                                    .into_iter()
                                    .map(
                                        move |(destination, icon, en, zh)| {
                                            let target = stored(destination);
                                            panel_tab()
                                                .flex_shrink_0()
                                                .accessibility_label(move || {
                                                    Some(tr(i18n, en, zh).into())
                                                })
                                                .selected(move || {
                                                    tab.get() == target.with(|v| v.clone())
                                                })
                                                .on_press(move || {
                                                    tab.set(target.with(|v| v.clone()))
                                                })
                                                .child(
                                                    view()
                                                        .tooltip(ui::hint(move || {
                                                            tr(i18n, en, zh).into()
                                                        }))
                                                        .child(ui::icon(icon).size(ui::icon_lg())),
                                                )
                                        },
                                    ),
                                )
                                .child(each(
                                    move || {
                                        albums.with(|rows| {
                                            rows.iter()
                                                .map(|a| {
                                                    (
                                                        a.album_id.clone(),
                                                        a.name.clone(),
                                                        a.icon_url.clone(),
                                                    )
                                                })
                                                .collect::<Vec<_>>()
                                        })
                                    },
                                    |album| album.0.clone(),
                                    move |album, _| {
                                        panel_tab()
                                            .flex_shrink_0()
                                            .accessibility_label(move || {
                                                Some(album.with(|a| a.1.clone()).into())
                                            })
                                            .selected(move || {
                                                tab.get() == Tab::Album(album.with(|a| a.0.clone()))
                                            })
                                            .on_press(move || {
                                                tab.set(Tab::Album(album.with(|a| a.0.clone())))
                                            })
                                            .child(
                                                view()
                                                    .tooltip(ui::hint(move || {
                                                        album.with(|a| a.1.clone())
                                                    }))
                                                    .child(crate::sticker::framed_album_icon(
                                                        account.with(|a| a.clone()),
                                                        move || album.with(|a| a.2.clone()),
                                                        ui::icon_lg().0,
                                                    )),
                                            )
                                    },
                                )),
                        ),
                ),
        )
}

pub(crate) fn import_tile(busy: Signal<bool>, import: impl Fn() + 'static) -> ButtonElement {
    let i18n = i18n::current();
    sticker_tile()
        .disabled(busy)
        .accessibility_label(move || Some(tr(i18n, "Import sticker…", "导入贴纸…").into()))
        .on_press(import)
        .child(
            view()
                .w(ui::sticker_art_size())
                .h(ui::sticker_art_size())
                .rounded(ui::radius_md())
                .border_2()
                .border_dashed()
                .border_color(ui::fg_tertiary)
                .items_center()
                .justify_center()
                .child(
                    ui::icon(ui::Icon::Add)
                        .size(ui::icon_lg())
                        .text_color(ui::fg_secondary),
                ),
        )
}

pub(crate) fn sticker_tile() -> ButtonElement {
    ui::command().size(cell_size()).px_2().py_2()
}

pub(crate) fn empty_stickers(tab: impl Fn() -> Tab + 'static) -> View {
    let i18n = i18n::current();
    let tab = computed(tab);
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_2()
        .px_6()
        .child(
            ui::icon(ui::Icon::Sticker)
                .size(ui::icon_lg())
                .text_color(ui::fg_tertiary),
        )
        .child(
            text(move || i18n.text(Key::NoStickers))
                .text_role(ui::TextRole::Body)
                .text_color(ui::fg_secondary),
        )
        .child(
            text(move || {
                if tab.get() == Tab::Recent {
                    tr(
                        i18n,
                        "Stickers you send appear here",
                        "发送过的贴纸会显示在这里",
                    )
                } else {
                    ""
                }
            })
            .text_role(ui::TextRole::Caption)
            .text_color(ui::fg_secondary),
        )
}
