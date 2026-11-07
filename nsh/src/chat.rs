use std::collections::{HashMap, HashSet};
use std::sync::Arc;

mod audio;
mod compose;
mod gallery;
mod header;
mod input_edit;
mod media_preview;
mod mentions;
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
mod message_selection;
mod message_snapshot;
mod message_system;
mod message_text;
mod message_transcript;
mod pinned;
pub(crate) mod reedit;
mod timeline;
mod unread;

use mixin_desktop_api::{AccountClient, ConversationListData, MediaClient, MessageListView};
use nsh_core::{platform::Lifecycle, prelude::*};

use crate::{
    composer,
    i18n::{self, Key},
    inspector, runtime, ui,
};

pub(crate) fn screen(
    account: Arc<AccountClient>,
    conversation: ConversationListData,
    target: Signal<Option<String>>,
    drafts: Signal<HashMap<String, String>>,
    pending: Signal<HashSet<String>>,
    session_owner: Owner,
    back: impl Fn() + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    let chat_presence = crate::motion::Presence::fade(|| true);
    let navigation = use_context::<crate::sidebar::Navigation>().expect("Home provides navigation");
    let narrow = move || window().width.0 - navigation.width().0 < 620.0;
    let narrow_detail = move || {
        let content = window().width.0 - navigation.width().0;
        (if narrow() { content } else { content - 300.0 }) < 620.0
    };
    let chat_navigation =
        use_context::<crate::navigation::ChatNavigation>().expect("Home provides chat navigation");
    let side = chat_navigation.panel;
    let panel_presence = crate::motion::Presence::panel(move || side.get().is_some());
    let shown_panel = signal(side.get_untracked());
    effect(move || {
        if let Some(mode) = side.get() {
            shown_panel.set(Some(mode));
        }
    });
    let covered = computed(move || panel_presence.mounted() && narrow_detail());
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
    let media = use_context::<Stored<Option<Arc<MediaClient>>>>()
        .expect("App provides media")
        .with(|media| {
            media
                .as_ref()
                .expect("Desktop media is initialized")
                .as_ref()
                .clone()
        });
    let playback = use_context::<crate::playback::Playback>()
        .expect("Home provides playback")
        .snapshot_signal();
    let actions = message_actions::Actions {
        owner: Owner::current(),
        text_selection: message_text::Selection::new(),
        account: stored(account.clone()),
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
    };
    let unobstructed = computed(move || {
        actions.preview.with(Option::is_none)
            && actions.forwarding.with(Vec::is_empty)
            && !covered.get()
    });
    let security = use_context::<crate::security::Handle>().expect("Home provides security");
    let attentive = computed(move || {
        unobstructed.get() && lifecycle() == Lifecycle::Active && !security.blocked()
    });
    let unread = unread::Unread::mount(
        account.clone(),
        conversation.conversation_id.clone(),
        conversation.unseen_count,
        target,
        operation_failed,
        messages,
        attentive,
    );
    let timeline =
        timeline::Timeline::mount(conversation.clone(), target, history, messages, actions);
    let loading = timeline.loading;
    let failed = timeline.failed;
    unread.mark_read_while_visible(messages, move || {
        !loading.get() && !failed.get() && !timeline.has_newer.get()
    });
    let rows = message_row::Rows {
        actions,
        messages,
        target,
        timeline,
        unread,
        history,
        own_id: stored(account.account_id()),
        sender_names: computed(move || group || header.with(|row| row.is_bot_group)),
        encrypted,
        viewable,
        unobstructed,
    };
    let focus_layout = computed(move || (narrow(), covered.get(), side.get().is_none()));
    let incoming_files = signal(Vec::<composer::AttachmentSource>::new());
    let no_rows = move || !loading.get() && !failed.get() && messages.with(Vec::is_empty);
    let show_encryption = move || encrypted.get() && !header.with(|row| row.is_bot);
    chat_presence.surface(
        view()
            .on_file_drop(move |event: &FileDropEvent| {
                if can_send.get_untracked() {
                    incoming_files.set(local_paths(&event.uris));
                }
            })
            .flex_1()
            .flex_row()
            .min_w_0()
            .child(
                view()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .relative()
                    .style_when(move || covered.get(), |style| style.hidden())
                    .child(
                        ui::wallpaper()
                            .absolute()
                            .inset_0()
                            .size_full()
                            .object_fit(ResizeMode::Cover)
                            .opacity(ui::chat_pattern_opacity()),
                    )
                    .bg(ui::surface_chat)
                    .child(header::bar(header, group, narrow, back))
                    .child(timeline.controls(i18n))
                    .child(unread.controls(i18n))
                    .child(message_selection::selection_bar(actions, detail_id))
                    .child(pinned::banner(
                        account.clone(),
                        conversation.conversation_id.clone(),
                        target,
                        side,
                    ))
                    .child(when(loading, move || {
                        view()
                            .flex_1()
                            .items_center()
                            .justify_center()
                            .child(ui::busy_indicator(
                                || true,
                                move || i18n.text(Key::LoadingMessages).into(),
                            ))
                    }))
                    .child(when(no_rows, move || {
                        ui::empty(
                            move || i18n.text(Key::NoMessages),
                            move || i18n.text(Key::NoMessagesBody),
                        )
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
                            .on_layout(media_viewport)
                            .child(
                                virtual_list(
                                    move || messages.with(Vec::len),
                                    move |at| rows.row(at),
                                )
                                .footer(when(
                                    move || {
                                        !loading.get()
                                            && !failed.get()
                                            && !timeline.has_older.get()
                                            && show_encryption()
                                    },
                                    move || message_system::encryption_notice(actions),
                                ))
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
                            )
                            .child(timeline.paging_indicator(i18n))
                            .child(
                                view()
                                    .absolute()
                                    .right_4()
                                    .bottom_4()
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
            .child(message_forward::forwarding(actions))
            .child(message_detail::preview(actions))
            .child(when(
                move || panel_presence.mounted(),
                move || {
                    panel_presence.surface(
                        view()
                            .flex_row()
                            .min_h_0()
                            .style_when(narrow_detail, |style| style.flex_1().min_w_0())
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
                            })),
                    )
                },
            )),
    )
}

fn local_paths(uris: &[String]) -> Vec<composer::AttachmentSource> {
    uris.iter()
        .filter_map(|uri| url::Url::parse(uri).ok()?.to_file_path().ok())
        .filter(|path| path.is_absolute())
        .map(|path| composer::AttachmentSource::Path(path.to_string_lossy().into_owned()))
        .collect()
}
