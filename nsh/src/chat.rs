use std::collections::{HashMap, HashSet};
use std::sync::Arc;

mod announcement;
mod audio;
mod compose;
mod gallery;
mod header;
mod input_edit;
mod media_preview;
mod media_viewer;
mod mentions;
mod video;
pub(crate) use media_preview::MediaPreview;
mod message_actions;
mod message_appearance;
mod message_content;
mod message_detail;
mod message_forward;
mod message_links;
mod message_location;
pub(crate) mod message_media;
mod message_post;
mod message_qr;
mod message_quote;
mod message_row;
#[cfg(debug_assertions)]
pub(crate) use message_row::detached as detached_messages;
#[cfg(debug_assertions)]
pub(crate) use message_row::{
    Detached as DetachedMessages, detached_list as detached_message_list,
};
#[cfg(debug_assertions)]
pub(crate) use timeline::{FOCUS, Motion, blink, place as place_jump};
mod message_selection;
mod message_snapshot;
mod message_system;
mod message_text;
mod message_transcript;
mod panes;
pub(crate) use panes::{Ready, panes};
mod pinned;
pub(crate) mod reedit;
mod timeline;
mod unread;

use mixin_desktop_api::{AccountClient, ConversationListData, MessageListView};
use nsh_core::{platform::Lifecycle, prelude::*};

use crate::{
    composer,
    i18n::{self, Key},
    inspector, runtime,
    ui::{self, Typography as _},
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn screen(
    account: Arc<AccountClient>,
    conversation: ConversationListData,
    target: Signal<Option<String>>,
    drafts: Signal<HashMap<String, String>>,
    pending: Signal<HashSet<String>>,
    session_owner: Owner,
    back: impl Fn() + 'static,
    ready: Ready,
) -> impl IntoElement {
    let i18n = i18n::current();
    let navigation =
        use_context::<crate::navigation::Navigation>().expect("Home provides navigation");
    let narrow = move || window().width.0 - navigation.width().0 < 620.0;
    let narrow_detail = move || {
        let content = window().width.0 - navigation.width().0;
        (if narrow() { content } else { content - 300.0 }) < 620.0
    };
    let chat_navigation =
        use_context::<crate::navigation::ChatNavigation>().expect("Home provides chat navigation");
    let side = chat_navigation.panel;
    // The panel's slot widens on the frame clock and the conversation lays out
    // at each width, so its rows reflow in place instead of sliding between
    // two layouts; the panel, pinned to the slot's trailing edge, slides in.
    let panel_width = crate::motion::Tween::new(if side.get_untracked().is_some() {
        1.0
    } else {
        0.0
    });
    effect(move || {
        let open = side.get().is_some();
        untracked(|| {
            panel_width.to(
                if open { 1.0 } else { 0.0 },
                crate::motion::PANEL_WIDTH.duration_ms,
            )
        });
    });
    let panel_mounted = computed(move || side.get().is_some() || panel_width.get() > 0.0);
    let shown_panel = signal(side.get_untracked());
    effect(move || {
        if let Some(mode) = side.get() {
            shown_panel.set(Some(mode));
        }
    });
    let covered = computed(move || panel_mounted.get() && narrow_detail());
    let group = conversation.category == "GROUP";
    let header = signal(conversation.clone());
    let detail_account = stored(account.clone());
    let detail_id = stored(conversation.conversation_id.clone());
    let messages = signal(Vec::<MessageListView>::new());
    let viewable = signal(None::<(usize, usize)>);
    let history = element_ref();
    let media_viewport = message_media::viewport();
    let encrypted = signal(false);
    let client = account.clone();
    let id = conversation.conversation_id.clone();
    spawn(async move {
        if let Ok(value) =
            runtime::run(async move { client.message().conversation_is_encrypted(id).await }).await
        {
            encrypted.set(value);
        }
    });
    let eligibility = compose::Eligibility::mount(
        account.clone(),
        conversation.conversation_id.clone(),
        header,
    );
    let can_send = eligibility.can_send;
    let operation_failed = signal(false);
    let quote = signal_with(None::<MessageListView>, SignalEquals::Never);
    mentions::MentionNames::mount(account.clone(), messages, Some(quote));
    let media = crate::media_client().as_ref().clone();
    let playback = use_context::<crate::playback::Playback>()
        .expect("Home provides playback")
        .snapshot_signal();
    let actions = message_actions::Actions {
        owner: Owner::current(),
        text_selection: message_text::Selection::new(),
        account: message_actions::Account::new(account.clone()),
        i18n,
        quote,
        can_send,
        selected: signal(Vec::new()),
        forwarding: signal(Vec::new()),
        combine: signal(false),
        policies: signal_with(HashMap::new(), SignalEquals::Never),
        failed: operation_failed,
        busy: signal(false),
        pinned_page: false,
        media: stored(media),
        playback,
        preview: signal_with(None, SignalEquals::Never),
        preview_history: signal_with(Vec::new(), SignalEquals::Never),
        toasts: use_context::<ui::Toasts>().expect("Home provides toasts"),
    };
    let unobstructed = computed(move || {
        actions.preview.with(Option::is_none)
            && actions.forwarding.with(Vec::is_empty)
            && !covered.get()
            && !navigation.settings.get()
    });
    let security = use_context::<crate::security::Handle>().expect("Home provides security");
    let attentive = computed(move || {
        unobstructed.get() && lifecycle() == Lifecycle::Active && !security.blocked()
    });
    let unread = unread::Unread::mount(
        account.clone(),
        &conversation,
        target,
        operation_failed,
        messages,
        attentive,
    );
    let timeline =
        timeline::Timeline::mount(conversation.clone(), target, history, messages, actions);
    let loading = timeline.loading;
    let failed = timeline.failed;
    // Once: the pane holding the previous chat gives way to this one.
    let announced = stored(false);
    effect(move || {
        if timeline.settled() && !announced.replace(true) {
            untracked(|| ready());
        }
    });
    unread.mark_read_while_open(messages, move || !loading.get() && !failed.get());
    let rows = message_row::Rows {
        actions,
        messages,
        target,
        highlight: timeline.highlight,
        highlight_opacity: timeline.highlight_opacity,
        unread: Some(unread),
        history,
        own_id: stored(account.account_id()),
        sender_names: computed(move || group || header.with(|row| row.is_bot_group)),
        viewable,
        unobstructed,
        transcript: None,
    };
    // Escape steps back one layer: the reply, the selection, then the side panel.
    // Inputs keep theirs.
    on_key_with(
        "escape",
        KeyOptions {
            when: nsh_core::keymap::When::NotEditing,
            ..KeyOptions::default()
        },
        move |_| {
            // A reply is the nearest layer: Escape cancels it first. The
            // composer's own binding does the same while the input has focus.
            if !navigation.settings.get_untracked() && actions.quote.with_untracked(Option::is_some)
            {
                actions.quote.set(None);
                return;
            }
            if navigation.settings.get_untracked()
                || actions.preview.with_untracked(Option::is_some)
                || actions.forwarding.with_untracked(|ids| !ids.is_empty())
            {
                return;
            }
            if actions.selected.with_untracked(|ids| !ids.is_empty()) {
                if !actions.busy.get_untracked() {
                    actions.selected.set(Vec::new());
                }
            } else if side.get_untracked().is_some() {
                chat_navigation.back_panel();
            }
        },
    );
    let focus_layout = computed(move || (narrow(), covered.get(), side.get().is_none()));
    let dragging = signal(false);
    let incoming_files = signal(Vec::<composer::AttachmentSource>::new());
    let no_rows = move || !loading.get() && !failed.get() && messages.with(Vec::is_empty);
    let show_encryption = move || encrypted.get() && !header.with(|row| row.is_bot);
    let pinned = pinned::Pinned::mount(
        account.clone(),
        conversation.conversation_id.clone(),
        target,
        side,
    );
    // The chrome floats over the timeline: the rows scroll under it, and what
    // must stay readable starts below its measured height.
    let chrome_height = signal(0.0_f32);
    let chrome_inset = move || px(chrome_height.get());
    // The composer floats over the bottom the same way; the newest rows scroll
    // up from under it, past a scroll padding of its height.
    let composer_height = signal(0.0_f32);
    let composer_inset = move || px(composer_height.get());
    let chat_width = signal(0.0_f32);
    let inline_pin = computed(move || chat_width.get() >= ui::glass_inline_pin_width().0);
    view()
        .on_file_drag_enter(move |_: &()| dragging.set(true))
        .on_file_drag_leave(move |_: &()| dragging.set(false))
        .on_file_drop(move |event: &FileDropEvent| {
            dragging.set(false);
            if can_send.get_untracked() {
                incoming_files.set(composer::local_sources(&event.uris));
            }
        })
        .flex_1()
        .flex_row()
        .min_w_0()
        .relative()
        .child(
            view()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .relative()
                .overflow_hidden()
                .style_when(move || covered.get(), |style| style.hidden())
                // Sized by the window and pinned to the leading edge, so a
                // panel beside the chat clips the pattern instead of rescaling it.
                .child(
                    ui::wallpaper()
                        .absolute()
                        .top_0()
                        .left_0()
                        .w(move || window().width)
                        .h(move || window().height)
                        .object_fit(ResizeMode::Cover)
                        .opacity(ui::chat_pattern_opacity()),
                )
                .bg(ui::surface_chat)
                .on_layout(move |bounds: &Bounds<Points>| {
                    let width = bounds.size.width.0;
                    if chat_width.get_untracked() != width {
                        chat_width.set(width);
                    }
                })
                .child(when(no_rows, move || {
                    view()
                        .flex_1()
                        .pt(chrome_inset)
                        .pb(composer_inset)
                        .child(ui::empty(
                            move || i18n.text(Key::NoMessages),
                            move || i18n.text(Key::NoMessagesBody),
                        ))
                }))
                .child(when(
                    move || no_rows() && show_encryption(),
                    move || message_system::encryption_notice(actions),
                ))
                .child(
                    view()
                        .flex_1()
                        .min_h_0()
                        .relative()
                        .on_layout(move |bounds: &Bounds<Points>| {
                            media_viewport(bounds);
                            timeline.reveal.measure(bounds.size.height.0);
                        })
                        .child(
                            timeline.reveal.surface(view().flex_1().min_h_0()).child(
                                virtual_list(
                                    move || messages.with(Vec::len),
                                    move |at| rows.row(at),
                                )
                                // The newest end of the inverted list: room under the composer.
                                .header(view().h(composer_inset))
                                // The oldest end of the inverted list: room under the chrome.
                                .footer(view().child(view().h(chrome_inset)).child(when(
                                    move || {
                                        !loading.get()
                                            && !failed.get()
                                            && !timeline.has_older.get()
                                            && show_encryption()
                                    },
                                    move || message_system::encryption_notice(actions),
                                )))
                                .keys(move || {
                                    messages.with(|rows| {
                                        rows.iter()
                                            .map(|row| row.message_id.clone())
                                            .collect::<Vec<_>>()
                                    })
                                })
                                .element_ref(&history)
                                .on_viewable_items_change(move |event: &ViewableItems| {
                                    viewable.set(event.viewable);
                                    timeline.viewable(event);
                                    unread.viewable(event, messages);
                                })
                                .event_throttle(32.0)
                                .on_scroll(move |event: &ScrollEvent| timeline.scrolled(event))
                                .on_load_more(move || timeline.older())
                                .on_load_more_at_start(move || timeline.newer())
                                .inverted()
                                .estimated_item_size(ui::list_row_height())
                                .flex_1()
                                .min_h_0(),
                            ),
                        )
                        .child(
                            view()
                                .absolute()
                                .right_4()
                                .bottom(move || px(composer_height.get() + ui::space_lg().0))
                                .gap_2()
                                .items_end()
                                .child(unread.jump_button(i18n))
                                .child(timeline.jump_button(i18n)),
                        )
                        .style_when(
                            move || messages.with(Vec::is_empty) || loading.get(),
                            |style| style.hidden(),
                        ),
                )
                .child(
                    view()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .pointer_events(PointerEvents::BoxNone)
                        .on_layout(move |bounds: &Bounds<Points>| {
                            let value = bounds.size.height.0;
                            if composer_height.get_untracked() != value {
                                composer_height.set(value);
                                timeline.reveal.measure_bottom(value);
                            }
                        })
                        .child(ui::edge_fade(false, ui::surface_chat))
                        .child(compose::bar(compose::Props {
                            account: account.clone(),
                            conversation_id: conversation.conversation_id.clone(),
                            header,
                            group,
                            encrypted,
                            actions,
                            eligibility,
                            target,
                            drafts,
                            pending,
                            session_owner,
                            incoming_files,
                            focus_layout,
                        })),
                )
                .child(chrome(
                    chrome_height,
                    pinned,
                    inline_pin,
                    header::bar(header, group, narrow, back, pinned, inline_pin),
                    view()
                        .pointer_events(PointerEvents::BoxNone)
                        .child(timeline.controls(i18n))
                        .child(unread.controls(i18n))
                        .child(message_selection::selection_bar(actions, detail_id))
                        .child(announcement::banner(
                            account.clone(),
                            conversation.conversation_id.clone(),
                        )),
                )),
        )
        .child(when(
            move || dragging.get() && can_send.get(),
            move || {
                // Files held over the chat: say where they will go.
                view()
                    .absolute()
                    .inset_0()
                    .m_2()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .rounded(ui::radius_lg())
                    .border_2()
                    .border_color(ui::accent)
                    .bg(|| ui::accent().opacity(0.12))
                    .pointer_events(PointerEvents::None)
                    .child(
                        ui::icon(ui::Icon::Upload)
                            .size(ui::icon_lg())
                            .text_color(ui::accent),
                    )
                    .child(
                        text(move || i18n.tr("Drop files to send", "拖拽文件到此处"))
                            .text_role(ui::TextRole::Title)
                            .text_color(ui::accent),
                    )
            },
        ))
        .child(message_forward::forwarding(actions))
        .child(message_detail::preview(actions))
        .child(
            view()
                .flex_row()
                .min_h_0()
                .overflow_hidden()
                .justify_end()
                .style_with(move |style| {
                    if !panel_mounted.get() {
                        style.w(px(0.0)).flex_shrink_0()
                    } else if narrow_detail() {
                        style.flex_1().min_w_0()
                    } else {
                        style
                            .w(px(ui::sidebar_width().0 * panel_width.get()))
                            .flex_shrink_0()
                    }
                })
                .child(when(
                    move || panel_mounted.get(),
                    move || {
                        let open = move || side.get().is_some();
                        view()
                            .flex_row()
                            .min_h_0()
                            .flex_shrink_0()
                            .accessibility_elements_hidden(move || !open())
                            .pointer_events(move || {
                                if open() {
                                    PointerEvents::Auto
                                } else {
                                    PointerEvents::None
                                }
                            })
                            .style_with(move |style| {
                                if narrow_detail() {
                                    style.flex_1().min_w_0()
                                } else {
                                    style.w(ui::sidebar_width)
                                }
                            })
                            .child(dynamic(move || {
                                let mode = shown_panel.get();
                                untracked(|| match mode {
                                    Some(mode) => inspector::screen(
                                        detail_account.with(|account| account.clone()),
                                        detail_id.with(|id| id.clone()),
                                        mode,
                                        move || chat_navigation.back_panel(),
                                        target,
                                        narrow_detail,
                                    )
                                    .into_element(),
                                    None => view().hidden().into_element(),
                                })
                            }))
                    },
                )),
        )
}

/// The chrome floating over the top of the timeline: the title bar, the pinned
/// message on a row of its own when the title bar has no room for it, then the
/// chat's notices. Empty space passes presses to the rows under it.
fn chrome(
    height: Signal<f32>,
    pinned: pinned::Pinned,
    inline_pin: Memo<bool>,
    title: impl IntoElement,
    notices: impl IntoElement,
) -> View {
    view()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .pointer_events(PointerEvents::BoxNone)
        .on_layout(move |bounds: &Bounds<Points>| {
            let value = bounds.size.height.0;
            if height.get_untracked() != value {
                height.set(value);
            }
        })
        .child(ui::edge_fade(true, ui::surface_chat))
        .child(title)
        .child(
            view()
                .px_3()
                .pb_2()
                .flex_row()
                .pointer_events(PointerEvents::BoxNone)
                .style_when(
                    move || inline_pin.get() || !pinned.visible(),
                    |style| style.hidden(),
                )
                .child(when(move || !inline_pin.get(), move || pinned.view(2))),
        )
        .child(notices)
}

/// The floating chrome over fixed rows, for the debug preview: no account.
#[cfg(debug_assertions)]
pub(crate) fn preview_chrome<E: IntoElement>(
    row: ConversationListData,
    pin: Signal<Option<String>>,
    list: impl FnOnce(Signal<f32>, Signal<f32>) -> E,
    composer: impl IntoElement,
) -> impl IntoElement {
    let header = signal(row);
    let group = header.with_untracked(|row| row.category == "GROUP");
    let side = use_context::<crate::navigation::ChatNavigation>()
        .expect("the preview provides chat navigation")
        .panel;
    let pinned = pinned::Pinned::detached(pin, side);
    let height = signal(0.0_f32);
    let bottom = signal(0.0_f32);
    let width = signal(0.0_f32);
    let inline_pin = computed(move || width.get() >= ui::glass_inline_pin_width().0);
    view()
        .flex_1()
        .min_h_0()
        .relative()
        .on_layout(move |bounds: &Bounds<Points>| width.set(bounds.size.width.0))
        .child(view().flex_1().min_h_0().child(list(height, bottom)))
        .child(
            view()
                .absolute()
                .left_0()
                .right_0()
                .bottom_0()
                .pointer_events(PointerEvents::BoxNone)
                .on_layout(move |bounds: &Bounds<Points>| bottom.set(bounds.size.height.0))
                .child(ui::edge_fade(false, ui::surface_chat))
                .child(composer),
        )
        .child(chrome(
            height,
            pinned,
            inline_pin,
            header::bar(header, group, || false, || {}, pinned, inline_pin),
            view(),
        ))
}

#[cfg(debug_assertions)]
pub(crate) fn preview_buttons(
    kind: &str,
    account_id: String,
    rows: Vec<MessageListView>,
    failed: Signal<bool>,
) -> impl IntoElement {
    let actions = message_actions::Actions::detached(account_id);
    let conversation = stored(String::new());
    match kind {
        "transcript" => message_transcript::detached(actions, rows, failed).into_element(),
        "summary" => message_transcript::preview_summary(actions, rows, failed).into_element(),
        "selection" => {
            actions
                .selected
                .set(rows.iter().take(2).map(|r| r.message_id.clone()).collect());
            for row in &rows {
                actions.policies.update(|p| {
                    p.insert(row.message_id.clone(), {
                        let mut policy = mixin_desktop_api::MessageActions::default();
                        policy.can_forward = true;
                        policy.can_combine_forward = true;
                        policy
                    });
                });
            }
            view()
                .flex_1()
                .justify_end()
                .child(message_selection::selection_bar(actions, conversation))
                .into_element()
        }
        "markdown-error" => {
            message_post::attachment_error(stored(rows[0].clone()), actions, signal(0))
                .into_element()
        }
        "add-sticker" => {
            actions
                .preview
                .set(Some(message_actions::Detail::AddSticker(Box::new(
                    rows[0].clone(),
                ))));
            message_detail::detached(actions).into_element()
        }
        "qr" => {
            actions.preview.set(Some(message_actions::Detail::Qr(
                "https://mixin.one".into(),
            )));
            message_detail::detached(actions).into_element()
        }
        _ => view().hidden().into_element(),
    }
}

#[cfg(debug_assertions)]
pub(crate) use {
    compose::draft_error, pinned::load_error as pinned_error,
    timeline::load_error as timeline_error, unread::load_error as unread_error,
};

#[cfg(debug_assertions)]
pub(crate) use message_forward::detached as preview_forward;

#[cfg(debug_assertions)]
pub(crate) use gallery::detached as preview_photos;

#[cfg(debug_assertions)]
pub(crate) use compose::preview_unavailable;

#[cfg(debug_assertions)]
pub(crate) use message_post::detached as preview_markdown;

#[cfg(target_os = "macos")]
pub(crate) use gallery::input::register as register_image_input;

#[cfg(debug_assertions)]
pub(crate) use message_detail::preview_video_detail;

#[cfg(debug_assertions)]
pub(crate) fn preview_reply(
    account_id: String,
    quote: Signal<Option<MessageListView>>,
    input: composer::Input,
) -> impl IntoElement {
    let mut actions = message_actions::Actions::detached(account_id);
    actions.quote = quote;
    message_quote::reply_bar(actions, move || input.focus())
}
