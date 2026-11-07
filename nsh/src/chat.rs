use std::collections::{HashMap, HashSet};
use std::sync::Arc;

mod audio;
mod gallery;
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
mod message_selection;
mod message_snapshot;
mod message_system;
mod message_text;
mod message_transcript;
mod pinned;
pub(crate) mod reedit;
mod timeline;
mod unread;

use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, ConversationListData, MediaClient, MessageListView};
use nsh_base::theme;
use nsh_core::{platform::Lifecycle, prelude::*};

use crate::{
    composer,
    i18n::{self, Key},
    inspector, presentation, runtime, ui,
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
    let appearance =
        use_context::<crate::appearance::Appearance>().expect("Home provides chat appearance");
    let links =
        use_context::<crate::links::LinkNavigation>().expect("Home provides link navigation");
    let header = signal_with(conversation.clone(), SignalEquals::Never);
    let draft_writer = use_context::<crate::drafts::Drafts>().expect("Home provides drafts");
    let navigation = use_context::<crate::sidebar::Navigation>().expect("Home provides navigation");
    let narrow = move || window().width.0 - navigation.width().0 < 620.0;
    let narrow_detail = move || {
        let content = window().width.0 - navigation.width().0;
        (if narrow() { content } else { content - 300.0 }) < 620.0
    };
    let chat_navigation =
        use_context::<crate::navigation::ChatNavigation>().expect("Home provides chat navigation");
    let side = chat_navigation.panel;
    let toggle_panel = move |mode| chat_navigation.toggle_panel(mode);
    let panel_presence = crate::motion::Presence::panel(move || side.get().is_some());
    let shown_panel = signal(side.get_untracked());
    effect(move || {
        if let Some(mode) = side.get() {
            shown_panel.set(Some(mode));
        }
    });
    let pinned = signal(false);
    let detail_account = stored(account.clone());
    let detail_id = stored(conversation.conversation_id.clone());
    let account_id = stored(account.account_id());
    let messages = signal_with(Vec::<MessageListView>::new(), SignalEquals::Never);
    let sticker_viewable = signal(None::<(usize, usize)>);
    let retry = signal(0_u64);
    let history = element_ref();
    let media_viewport = message_media::viewport();
    let draft_id = stored(conversation.conversation_id.clone());
    drafts.update(|rows| {
        rows.entry(conversation.conversation_id.clone())
            .or_insert_with(|| conversation.draft.clone());
    });
    let draft = computed(move || {
        drafts.with(|rows| {
            rows.get(&draft_id.with(|id| id.clone()))
                .cloned()
                .unwrap_or_default()
        })
    });
    let owner = Owner::current();
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
    let can_send = signal(false);
    let eligibility_loaded = signal(false);
    let eligibility_failed = signal(false);
    let eligibility_retry = signal(0_u64);
    let eligibility_account = account.clone();
    let eligibility_id = conversation.conversation_id.clone();
    effect(move || {
        eligibility_retry.get();
        let eligibility_account = eligibility_account.clone();
        let eligibility_id = eligibility_id.clone();
        spawn(async move {
            let mut changes =
                Box::pin(runtime::observe(eligibility_account.conversation_changes()));
            loop {
                can_send.set(false);
                eligibility_loaded.set(false);
                eligibility_failed.set(false);
                let client = eligibility_account.clone();
                let id = eligibility_id.clone();
                match runtime::run(async move { client.conversation().can_send(id).await }).await {
                    Ok(allowed) => {
                        can_send.set(allowed);
                        eligibility_loaded.set(true);
                    }
                    Err(error) => {
                        eligibility_failed.set(true);
                        log::warn!("Could not load conversation send permission: {error}");
                    }
                }
                loop {
                    let Some(event) = changes.next().await else {
                        return;
                    };
                    if event.reload_all || event.conversation_ids.contains(&eligibility_id) {
                        break;
                    }
                }
            }
        });
    });
    let input_focused = signal(false);
    let sending =
        computed(move || pending.with(|ids| ids.contains(&draft_id.with(|id| id.clone()))));
    let send_failed = signal(false);
    let operation_failed = signal(false);
    let quote = signal_with(None::<MessageListView>, SignalEquals::Never);
    let voice = composer::Voice::mount(
        account.clone(),
        conversation.conversation_id.clone(),
        operation_failed,
        quote,
    );
    mentions::MentionNames::mount(account.clone(), messages, Some(quote));
    let selected = signal(Vec::<String>::new());
    let policies = signal_with(HashMap::new(), SignalEquals::Never);
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
        text_selection: message_text::Selection::new(),
        account: stored(account.clone()),
        i18n,
        quote,
        can_send,
        selected,
        forwarding: signal(Vec::new()),
        combine: signal(false),
        policies,
        failed: operation_failed,
        busy: signal(false),
        pinned_page: false,
        media: stored(media),
        playback,
        preview: signal_with(None, SignalEquals::Never),
        preview_history: signal_with(Vec::new(), SignalEquals::Never),
    };
    let unread = unread::Unread::mount(
        account.clone(),
        conversation.conversation_id.clone(),
        conversation.unseen_count,
        target,
        retry,
        operation_failed,
        messages,
    );
    let timeline = timeline::Timeline::mount(
        conversation.clone(),
        target,
        retry,
        history,
        messages,
        header,
        actions,
    );
    let loading = timeline.loading;
    let failed = timeline.failed;
    let read_account = account.clone();
    let read_id = conversation.conversation_id.clone();
    effect(move || {
        if lifecycle() != Lifecycle::Active
            || loading.get()
            || timeline.busy.get()
            || failed.get()
            || !unread.ready.get()
            || !unread.latest_visible.get()
            || timeline.has_newer.get()
            || actions.preview.with(Option::is_some)
            || actions.forwarding.with(|ids| !ids.is_empty())
            || (panel_presence.mounted() && narrow_detail())
        {
            return;
        }
        messages.with(|_| ());
        let client = read_account.clone();
        let id = read_id.clone();
        spawn(async move {
            if let Err(error) =
                runtime::run(async move { client.message().mark_conversation_read(id).await }).await
            {
                log::warn!("Could not mark the visible conversation read: {error}");
            }
        });
    });
    let initial_caret = draft.get_untracked().encode_utf16().count() as u32;
    let selection = signal((initial_caret, initial_caret));
    let input = composer::Input::mount(
        account.clone(),
        draft,
        selection,
        std::rc::Rc::new(move |value| {
            draft_writer.save(draft_id.with(|id| id.clone()), value.clone());
            drafts.update(|rows| {
                rows.insert(draft_id.with(|id| id.clone()), value);
            });
        }),
    );
    let focus_layout = computed(move || {
        (
            narrow(),
            panel_presence.mounted() && narrow_detail(),
            side.get().is_none(),
        )
    });
    let focus_pending = signal(false);
    effect(move || {
        let (_, covered, panel_closed) = focus_layout.get();
        if covered {
            input.blur();
        }
        focus_pending.set(!covered && panel_closed);
    });
    effect(move || {
        if focus_pending.get() && can_send.get() {
            input.focus();
            focus_pending.set(false);
        }
    });
    effect(move || {
        if quote.with(Option::is_some) {
            input.focus();
        }
    });
    let id = conversation.conversation_id.clone();
    let composer_account = stored(account.clone());
    let send_account = account.clone();
    let send = std::rc::Rc::new(move || {
        if !can_send.get_untracked() || sending.get_untracked() || input.composing() {
            return;
        }
        let body = draft.get_untracked();
        if body.trim().is_empty() {
            return;
        }
        let client = send_account.clone();
        let quote_id = quote.with(|q| q.as_ref().map(|m| m.message_id.clone()));
        let sent_quote_id = quote_id.clone();
        let id = id.clone();
        let draft_key = id.clone();
        pending.update(|ids| {
            ids.insert(draft_key.clone());
        });
        send_failed.set(false);
        session_owner.run(|| {
            spawn(async move {
                let text = body.clone();
                match runtime::commit(async move {
                    client.message().send_text(id, text, quote_id, false).await
                })
                .await
                {
                    Ok(_) => {
                        drafts.update(|rows| {
                            if rows.get(&draft_key) == Some(&body) {
                                rows.insert(draft_key.clone(), String::new());
                                draft_writer.save(draft_key.clone(), String::new());
                            }
                        });
                        pending.update(|ids| {
                            ids.remove(&draft_key);
                        });
                        if !owner.is_live() {
                            return;
                        }
                        if quote
                            .with(|q| q.as_ref().map(|m| &m.message_id) == sent_quote_id.as_ref())
                        {
                            quote.set(None);
                        }
                        batch(|| {
                            target.set(None);
                        });
                        retry.update(|value| *value += 1);
                    }
                    Err(error) => {
                        if matches!(error, mixin_desktop_api::ClientError::PermissionDenied)
                            && owner.is_live()
                        {
                            can_send.set(false);
                            eligibility_loaded.set(true);
                        }
                        pending.update(|ids| {
                            ids.remove(&draft_key);
                        });
                        if !owner.is_live() {
                            return;
                        }
                        send_failed.set(true);
                    }
                }
            })
        });
    });
    let incoming_files = signal(Vec::<composer::AttachmentSource>::new());
    let apply: std::rc::Rc<dyn Fn(String, (u32, u32))> = std::rc::Rc::new(move |text, caret| {
        if sending.get_untracked() {
            return;
        }
        draft_writer.save(draft_id.with(|id| id.clone()), text.clone());
        batch(|| {
            drafts.update(|rows| {
                rows.insert(draft_id.with(|id| id.clone()), text);
            });
            selection.set(caret);
        });
        input.focus();
    });
    let reedit = use_context::<reedit::Reedit>().expect("Home provides recall editing");
    let apply_reedit = apply.clone();
    effect(move || {
        if sending.get() {
            return;
        }
        if let Some(content) = reedit.take_request(&draft_id.with(|id| id.clone())) {
            let apply = apply_reedit.clone();
            input.after_composition(std::rc::Rc::new(move || {
                let mut text = draft.get_untracked();
                text.push_str(&content);
                let caret = text.encode_utf16().count() as u32;
                apply(text, (caret, caret));
            }));
        }
    });
    let apply_insertion = apply.clone();
    let insert: std::rc::Rc<dyn Fn(String)> = std::rc::Rc::new(move |text| {
        let apply = apply_insertion.clone();
        input.after_composition(std::rc::Rc::new(move || {
            let (value, caret) =
                input_edit::insert(&draft.get_untracked(), selection.get_untracked(), &text);
            apply(value, caret);
        }));
    });
    let name = computed(move || header.with(|row| SharedString::from(row.name.clone())));
    let group = conversation.category == "GROUP";
    chat_presence.surface(view()
        .on_file_drop(move |event: &FileDropEvent| { if can_send.get_untracked() { incoming_files.set(local_paths(&event.uris)); } })
        .flex_1()
        .flex_row()
        .min_w_0()
        .child(
            view()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .relative()
                .style_when(move || panel_presence.mounted() && narrow_detail(), |s| s.hidden())
                .child(ui::wallpaper().absolute().inset_0().size_full().object_fit(ResizeMode::Cover).opacity(move || theme().number("chat-pattern-opacity").unwrap_or(0.06)))
                .bg(move || {
                    theme()
                        .color("chat-background")
                        .unwrap_or(theme().background)
                })
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .px_6()
                        .py_4()
                        .bg(move || theme().background)
                        .border_b_1()
                        .border_color(move || theme().border)
                        .child(
                            ui::command()
                                .style_when(move || !narrow(), |s| s.hidden())
                                .on_press(back)
                                .accessibility_label(move || Some(i18n.text(Key::Back).into()))
                                .child(ui::icon(ui::Icon::ArrowLeft)),
                        )
                        .child(
                            ui::command()
                                .flex_1()
                                .min_w_0()
                                .px_0()
                                .py_0()
                                .gap_3()
                                .justify_start()
                                .accessibility_label(move || Some(i18n.text(Key::ChatInfo).into()))
                                .on_press(move || toggle_panel(inspector::Mode::Info))
                                .child(ui::conversation_avatar(move || header.with(|row| Some(row.clone())), px(40.0)))
                                .child(
                                    view()
                                        .flex_1()
                                        .min_w_0()
                                        .gap_1()
                                        .child(view().flex_row().min_w_0().items_center().gap_1()
                                            .child(text(move || name.with(|value| value.clone()))
                                                .min_w_0()
                                                .flex_shrink(1.0)
                                                .number_of_lines(1)
                                                .font_weight(FontWeight::SEMIBOLD))
                                            .child(view().flex_shrink_0().child(dynamic(move || {
                                                let row = header.get();
                                                untracked(|| crate::appearance::identity_badge(group, row.is_verified, row.is_bot, row.membership.as_deref()))
                                            }))))
                                        .child(text(move || {
                                            if group {
                                                i18n.members(header.with(|row| row.participant_count))
                                            } else {
                                                header.with(|row| row.identity_number.clone())
                                            }
                                        })
                                            .text_xs()
                                            .text_color(move || theme().muted_foreground)),
                                ),
                        )
                        .child(when(
                            move || !group && header.with(|row| row.is_bot && !row.owner_id.is_empty()),
                            move || view()
                                .flex_none()
                                .tooltip(nsh_base::tooltip()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .bg(move || theme().popover)
                                    .text_color(move || theme().popover_foreground)
                                    .border_1()
                                    .border_color(move || theme().border)
                                    .child(text(move || i18n.tr("Open app", "打开应用")).text_sm()))
                                .child(ui::icon_button()
                                    .size_10()
                                    .px_1()
                                    .accessibility_label(move || Some(i18n.tr("Open app", "打开应用").into()))
                                    .on_press(move || links.open(
                                        format!("mixin://apps/{}?action=open", header.with_untracked(|row| row.owner_id.clone())),
                                        Some(detail_id.with(|id| id.clone())),
                                    ))
                                    .child(ui::icon(ui::Icon::Bot).size_6())),
                        ))
                        .child(
                            ui::command()
                                .size_10()
                                .px_1()
                                .accessibility_label(move || {
                                    Some(i18n.text(Key::SearchMessages).into())
                                })
                                .selected(move || side.get() == Some(inspector::Mode::Search))
                                .selected_style(|style| style.bg(move || theme().accent))
                                .on_press(move || toggle_panel(inspector::Mode::Search))
                                .child(ui::icon(ui::Icon::Search).size_6()),
                        )
                        .child(
                            ui::command()
                                .size_10()
                                .px_1()
                                .accessibility_label(move || Some(i18n.text(Key::ChatInfo).into()))
                                .selected(move || side.get() == Some(inspector::Mode::Info))
                                .selected_style(|style| style.bg(move || theme().accent))
                                .on_press(move || toggle_panel(inspector::Mode::Info))
                                .child(ui::icon(ui::Icon::Info).size_6()),
                        ),
                )
                .child(timeline.controls(i18n))
                .child(unread.controls(i18n))
                .child(message_selection::selection_bar(actions, detail_id))
                .child(pinned::banner(account.clone(), conversation.conversation_id.clone(), pinned, target, side))
                .child(when(loading, move || {
                    ui::centered(move || i18n.text(Key::LoadingMessages))
                }))
                .child(when(
                    move || !loading.get() && !failed.get() && messages.with(Vec::is_empty),
                    move || ui::empty(i18n, Key::NoMessages, Key::NoMessagesBody),
                ))
                .child(when(move || !loading.get() && !failed.get() && messages.with(Vec::is_empty) && encrypted.get() && !header.with(|header| header.is_bot), move || message_system::encryption_notice(actions)))
                .child(
                    view().flex_1().min_h_0().relative().on_layout(media_viewport).child(virtual_list(
                        move || messages.with(Vec::len),
                        move |at| {
                            crate::sticker::visibility(move || {
                                sticker_viewable.get().is_some_and(|(first, last)| (first..=last).contains(&at.index()))
                                    && !actions.preview.with(Option::is_some)
                                    && actions.forwarding.with(Vec::is_empty)
                                    && !(panel_presence.mounted() && narrow_detail())
                            });
                            let row_ref = element_ref();
                            unread.observe_row(
                                move || messages.with(|rows| rows.get(at.index()).map(|row| row.message_id.clone())),
                                move || at.index() == 0,
                                row_ref,
                                history,
                                move || !timeline.busy.get() && !failed.get()
                                    && !actions.preview.with(Option::is_some)
                                    && actions.forwarding.with(Vec::is_empty)
                                    && !(panel_presence.mounted() && narrow_detail()),
                            );
                            let mine = move || messages.with(|rows| rows.get(at.index()).is_some_and(|m| account_id.with(|id| m.sender_id == *id)));
                            let new_day = move || messages.with(|rows| rows.get(at.index()).is_some_and(|m| rows.get(at.index() + 1).is_none_or(|older| presentation::day(i18n, m.created_at_micros) != presentation::day(i18n, older.created_at_micros))));
                            let sender_group = move || (group || header.with(|row| row.is_bot_group)) && !mine();
                            let new_sender = move || messages.with(|rows| {
                                rows.get(at.index()).is_some_and(|row| rows.get(at.index() + 1).is_none_or(|older| {
                                    older.sender_id != row.sender_id || older.category.starts_with("SYSTEM_") || older.category == "MESSAGE_PIN"
                                        || presentation::day(i18n, row.created_at_micros) != presentation::day(i18n, older.created_at_micros)
                                }))
                            });
                            view().element_ref(&row_ref).px_2().py_1().gap_2()
                                .child(unread.divider(i18n, messages, move || at.index()))
                                .child(when(new_day, move || view().items_center().py_2()
                                    .child(view().px_6().py_1().rounded_full().bg(move || theme().color("day-background").unwrap_or(theme().accent))
                                        .child(text(move || messages.with(|rows| rows.get(at.index()).map(|m| presentation::day(i18n, m.created_at_micros)).unwrap_or_default())).text_xs().text_color(move || theme().color("day-foreground").unwrap_or(theme().foreground))))))
                                .child(when(move || messages.with(|rows| rows.get(at.index()).is_some_and(|row| message_system::centered(&row.category))), move || dynamic(move || {
                                    let row = messages.with(|rows| rows.get(at.index()).cloned());
                                    untracked(|| row.map(|row| message_system::notice(&row, actions, Some(target))).unwrap_or_else(|| view().hidden().into_element()))
                                })))
                                .child(when(move || messages.with(|rows| rows.get(at.index()).is_some_and(|row| !message_system::centered(&row.category))), move || view().flex_row().items_start().gap_2().style_when(mine, |s| s.justify_end())
                                    .child(when(move || !selected.with(Vec::is_empty), move || {
                                        let checked = move || messages.with(|rows| rows.get(at.index()).is_some_and(|message| selected.with(|ids| ids.contains(&message.message_id))));
                                        nsh_base::checkbox().checked(checked).size_8().px_1().py_1()
                                            .items_center().justify_center().rounded(move || theme().radius)
                                            .transition_colors().duration(|| crate::motion::duration(crate::motion::FEEDBACK_MS)).ease_out()
                                            .hover(|style| style.bg(move || theme().muted))
                                            .active(|style| style.opacity(0.7))
                                            .focus(|style| style.outline(px(2.0)).outline_color(move || theme().ring))
                                            .disabled_style(|style| style.opacity(0.4))
                                            .accessibility_label(move || Some(message_actions::label(i18n, "Select message", "选择消息").into()))
                                            .indicator_class("size-5 rounded border border-input checked:bg-primary checked:border-primary items-center justify-center")
                                            .indicator(text(move || if checked() { "✓" } else { "" }).text_sm().text_color(move || theme().primary_foreground))
                                            .on_change(move |_: &bool| {
                                                if let Some(id) = messages.with(|rows| rows.get(at.index()).map(|m| m.message_id.clone())) {
                                                    selected.update(|ids| { if ids.contains(&id) { ids.retain(|value| value != &id); } else { ids.push(id); } });
                                                }
                                            })
                                    }))
                                    .child(when(move || sender_group() && appearance.show_avatar(), move || view().size_8().flex_shrink_0().child(when(new_sender, move ||
                                        ui::command().size_8().px_0().py_0().accessibility_label(move || Some(message_actions::label(i18n, "Open profile", "打开个人资料").into()))
                                            .on_press(move || { if let Some(id) = messages.with(|rows| rows.get(at.index()).map(|row| row.sender_id.clone())) { actions.show_user(id); } })
                                            .child(ui::avatar_view(move || messages.with(|rows| rows.get(at.index()).map(|m| m.sender_name.clone().into()).unwrap_or_default()), move || messages.with(|rows| rows.get(at.index()).map(|m| m.sender_avatar_url.clone()).unwrap_or_default())).size_8())))))
                                    .child(view().max_w(ui::bubble_width).min_w_0().flex_shrink(1.0).gap_1()
                                        .child(when(move || sender_group() && new_sender(), move || message_appearance::sender(move || messages.with(|rows| rows.get(at.index()).cloned()), actions)))
                                        .child(view().px_2().py_2().rounded_lg().flex_row().items_end().gap_2()
                                            .style_when(move || messages.with(|rows| rows.get(at.index()).is_some_and(message_media::inline_metadata)), |style| style.p_0())
                                            .bg(move || if mine() { theme().color("bubble-out").unwrap_or(theme().accent) } else { theme().color("bubble-in").unwrap_or(theme().card) })
                                            .child(when(move || timeline.highlight.with(|target| messages.with(|rows| rows.get(at.index()).is_some_and(|m| Some(&m.message_id) == target.as_ref()))), move || view().absolute().inset_0().rounded_lg().border_2().border_color(move || theme().ring).bind_opacity(&timeline.highlight_opacity).pointer_events(nsh_core::PointerEvents::None)))
                                            .child(message_actions::menu(
                                                move || messages.with(|rows| rows.get(at.index()).cloned()),
                                                actions,
                                                dynamic(move || {
                                                    let row = messages.with(|rows| rows.get(at.index()).cloned());
                                                    untracked(|| match row {
                                                        Some(row) => message_content::body(row, actions, Some(target), None),
                                                        None => view().hidden().into_element(),
                                                    })
                                                }),
                                            ))
                                            .child(view().flex_row().flex_shrink_0().items_center().gap_1()
                                                .style_when(move || messages.with(|rows| rows.get(at.index()).is_some_and(message_media::inline_metadata)), |style| style.hidden())
                                                .child(when(encrypted, || ui::icon(ui::Icon::Lock).size_3().text_color(move || theme().muted_foreground)))
                                                .child(text(move || messages.with(|rows| rows.get(at.index()).map(|m| presentation::time(i18n, m.created_at_micros / 1000)).unwrap_or_default())).text_size(px(10.0)).text_color(move || theme().muted_foreground))
                                                .child(when(mine, move || message_appearance::delivery_status(move || messages.with(|rows| rows.get(at.index()).map(|row| row.status.clone()).unwrap_or_default()), actions))))))))
                        },
                    )
                    .footer(when(move || !loading.get() && !failed.get() && !timeline.has_older.get() && encrypted.get() && !header.with(|header| header.is_bot), move || message_system::encryption_notice(actions)))
                    .keys(move || {
                        messages.with(|rows| {
                            rows.iter()
                                .map(|m| m.message_id.clone())
                                .collect::<Vec<_>>()
                        })
                    })
                    .element_ref(&history)
                    .on_viewable_items_change(move |event: &ViewableItems| { sticker_viewable.set(event.viewable); timeline.viewable(event); unread.viewable(event, messages); })
                    .event_throttle(32.0)
                    .on_scroll(move |event: &ScrollEvent| timeline.scrolled(event))
                    .on_load_more(move || timeline.older())
                    .on_load_more_at_start(move || timeline.newer())
                    .inverted()
                    .estimated_item_size(px(80.0))
                    .flex_1()
                    .min_h_0()
                    )
                    .child(view().absolute().right_4().bottom_4().gap_2().items_end()
                        .child(unread.jump_button(i18n)).child(timeline.jump_button(i18n)))
                    .style_when(move || messages.with(Vec::is_empty) || loading.get(), |s| s.hidden()),
                )
                .child(
                    voice.composer(
                    view()
                        .px_4()
                        .py_2()
                        .gap_2()
                        .bg(move || theme().background)
                        .border_t_1()
                        .border_color(move || theme().border)
                        .style_when(move || !can_send.get(), |s| s.hidden())
                        .child(message_quote::reply_bar(actions))
                        .child(when(move || (group || header.with(|row| row.is_bot)) && !input.composing() && input_focused.get(), move || composer::mentions(
                            composer_account.with(|account| account.clone()), draft_id.with(|id| id.clone()), header.with(|row| row.is_bot), draft, selection, apply.clone(), input,
                        )))
                        .child(when(operation_failed, move || text(move || i18n.text(Key::CommandFailed)).text_sm().text_color(move || theme().destructive)))
                        .child(when(draft_writer.failed, move || view().flex_row().gap_2()
                            .child(text(move || message_actions::label(i18n, "Couldn’t save your draft. Try again.", "无法保存草稿，请重试。")).text_sm().text_color(move || theme().destructive))
                            .child(ui::action(move || i18n.text(Key::Retry), move || draft_writer.retry()))))
                    .child(when(send_failed, move || {
                            text(move || i18n.text(Key::SendFailed))
                                .text_sm()
                                .text_color(move || theme().destructive)
                        }))
                        .child(
                            view()
                                .flex_row()
                                .items_end()
                                .gap_2()
                                .min_h_10()
                                .child(composer::attachments(
                                    composer_account.with(|a| a.clone()),
                                    draft_id.with(|s| s.clone()),
                                    operation_failed,
                                    quote,
                                    incoming_files,
                                ))
                                .child(composer::stickers(
                                    composer_account.with(|a| a.clone()),
                                    draft_id.with(|s| s.clone()),
                                    operation_failed,
                                    insert.clone(),
                                    quote,
                                ))
                                .child(
                                    input.view(
                                        move || i18n.text(if encrypted.get() { Key::EncryptedPlaceholder } else { Key::MessagePlaceholder }).to_owned(),
                                        sending,
                                        input_focused,
                                        std::rc::Rc::new(move |event| {
                                            let paths = local_paths(&event.uris);
                                            if !paths.is_empty() { incoming_files.set(paths); }
                                            else if !event.files.is_empty() {
                                                incoming_files.set(event.files.iter().map(|file| composer::AttachmentSource::Clipboard {
                                                    name: file.name.clone(), bytes: Arc::new(file.data.clone()),
                                                }).collect());
                                            }
                                        }),
                                        send.clone(),
                                    ),
                                )
                                .child(
                                    when(
                                        move || {
                                            !draft.with(|s| s.trim().is_empty()) && !input.composing()
                                        },
                                        move || {
                                            let send = send.clone();
                                            ui::command()
                                                .size_10()
                                                .px_1()
                                                .py_1()
                                                .disabled(sending)
                                                .accessibility_label(move || {
                                                    Some(i18n.text(Key::Send).into())
                                                })
                                                .on_press(move || send())
                                                .child(
                                                    ui::icon(ui::Icon::Send)
                                                        .size_6()
                                                        .text_color(move || theme().primary),
                                                )
                                        },
                                    )
                                    .otherwise(move || voice.button()),
                                ),
                        ),
                        ),
                )
                .child(when(move || !can_send.get(), move || view()
                    .min_h_14()
                    .px_4()
                    .justify_center()
                    .items_center()
                    .bg(move || theme().background)
                    .border_t_1()
                    .border_color(move || theme().border)
                    .child(text(move || {
                        if eligibility_failed.get() {
                            i18n.text(Key::CommandFailed)
                        } else if eligibility_loaded.get() {
                            message_actions::label(i18n, "You can't send messages to this group because you're no longer a participant.", "您不能发送消息，因为您已经不再是此群组成员。")
                        } else {
                            message_actions::label(i18n, "Loading…", "加载中…")
                        }
                    }).text_sm().text_color(move || theme().muted_foreground))
                    .child(when(eligibility_failed, move || ui::action(move || i18n.text(Key::Retry), move || eligibility_retry.update(|revision| *revision += 1))))))
        )
        .child(message_forward::forwarding(actions))
        .child(message_detail::preview(actions))
        .child(when(move || panel_presence.mounted(), move || panel_presence.surface(view().flex_row().min_h_0().style_when(narrow_detail, |s| s.flex_1().min_w_0()).child(dynamic(move || {
            let mode = shown_panel.get();
            untracked(|| match mode {
                Some(mode) => inspector::screen(
                    detail_account.with(|a| a.clone()),
                    detail_id.with(|s| s.clone()),
                    mode,
                    move || chat_navigation.back_panel(),
                    target,
                    narrow_detail,
                )
                .into_element(),
                None => view().hidden().into_element(),
            })
        }))))))
}

fn local_paths(uris: &[String]) -> Vec<composer::AttachmentSource> {
    uris.iter()
        .filter_map(|uri| url::Url::parse(uri).ok()?.to_file_path().ok())
        .filter(|path| path.is_absolute())
        .map(|path| composer::AttachmentSource::Path(path.to_string_lossy().into_owned()))
        .collect()
}
