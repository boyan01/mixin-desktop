//! The composer bar: send eligibility, draft, send and its failure states.
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, ConversationListData, MessageListView};
use nsh_core::prelude::*;

use super::{input_edit, message_actions::Actions, message_quote, reedit::Reedit};
use crate::{
    composer,
    i18n::{self, I18n, Key},
    runtime,
    ui::{self, Typography},
};

/// Whether the account can send here, and the conversation row that says so.
#[derive(Clone, Copy)]
pub(super) struct Eligibility {
    pub can_send: Signal<bool>,
    loaded: Signal<bool>,
    failed: Signal<bool>,
    retry: Signal<u64>,
}

impl Eligibility {
    /// Follows the conversation. Each message changes it too, so a refresh must
    /// keep the last answer: clearing it would unmount the composer mid-typing.
    pub fn mount(
        account: Arc<AccountClient>,
        conversation_id: String,
        header: Signal<ConversationListData>,
    ) -> Self {
        let state = Self {
            can_send: signal(false),
            loaded: signal(false),
            failed: signal(false),
            retry: signal(0_u64),
        };
        effect(move || {
            state.retry.get();
            state.failed.set(false);
            let account = account.clone();
            let id = conversation_id.clone();
            spawn(async move {
                let mut changes = Box::pin(runtime::observe(account.conversation_changes()));
                loop {
                    let client = account.clone();
                    let conversation = id.clone();
                    let result = runtime::run(async move {
                        let allowed = client.conversation().can_send(conversation.clone()).await?;
                        let row = client
                            .conversation()
                            .conversation_items_by_ids(vec![conversation])
                            .await?
                            .into_iter()
                            .next();
                        Ok((allowed, row))
                    })
                    .await;
                    match result {
                        Ok((allowed, row)) => batch(|| {
                            state.can_send.set(allowed);
                            state.loaded.set(true);
                            state.failed.set(false);
                            if let Some(row) = row {
                                header.set(row);
                            }
                        }),
                        Err(error) => {
                            state.failed.set(!state.loaded.get_untracked());
                            log::warn!("Could not load conversation send permission: {error}");
                        }
                    }
                    loop {
                        let Some(event) = changes.next().await else {
                            return;
                        };
                        if event.reload_all || event.conversation_ids.contains(&id) {
                            break;
                        }
                    }
                }
            });
        });
        state
    }

    fn revoke(self) {
        batch(|| {
            self.can_send.set(false);
            self.loaded.set(true);
        });
    }
}

pub(super) struct Props {
    pub account: Arc<AccountClient>,
    pub conversation_id: String,
    pub header: Signal<ConversationListData>,
    pub group: bool,
    pub encrypted: Signal<bool>,
    pub actions: Actions,
    pub eligibility: Eligibility,
    pub target: Signal<Option<String>>,
    pub drafts: Signal<HashMap<String, String>>,
    pub pending: Signal<HashSet<String>>,
    pub session_owner: Owner,
    pub incoming_files: Signal<Vec<composer::AttachmentSource>>,
    /// (narrow, covered by a panel, panel closed): a change may move focus back.
    pub focus_layout: Memo<(bool, bool, bool)>,
}

pub(super) fn bar(props: Props) -> impl IntoElement {
    let Props {
        account,
        conversation_id,
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
    } = props;
    let i18n = i18n::current();
    let can_send = eligibility.can_send;
    let quote = actions.quote;
    let operation_failed = actions.failed;
    let voice = composer::Voice::mount(
        account.clone(),
        conversation_id.clone(),
        operation_failed,
        quote,
    );
    let draft_writer = use_context::<crate::drafts::Drafts>().expect("Home provides drafts");
    let draft_id = stored(conversation_id.clone());
    drafts.update(|rows| {
        rows.entry(conversation_id.clone())
            .or_insert_with(|| header.with_untracked(|row| row.draft.clone()));
    });
    let draft = computed(move || {
        drafts.with(|rows| {
            rows.get(&draft_id.with(|id| id.clone()))
                .cloned()
                .unwrap_or_default()
        })
    });
    let sending =
        computed(move || pending.with(|ids| ids.contains(&draft_id.with(|id| id.clone()))));
    let send_failed = signal(false);
    let input_focused = signal(false);
    let initial_caret = draft.get_untracked().encode_utf16().count() as u32;
    let selection = signal((initial_caret, initial_caret));
    let input = composer::Input::mount(
        account.clone(),
        draft,
        selection,
        Rc::new(move |value| {
            draft_writer.save(draft_id.with(|id| id.clone()), value.clone());
            drafts.update(|rows| {
                rows.insert(draft_id.with(|id| id.clone()), value);
            });
        }),
    );
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
    let send = send_command(SendContext {
        account: account.clone(),
        conversation_id: conversation_id.clone(),
        can_send,
        sending,
        draft,
        input,
        quote,
        target,
        drafts,
        pending,
        draft_writer,
        send_failed,
        session_owner,
        eligibility,
    });
    let apply: Rc<dyn Fn(String, (u32, u32))> = Rc::new(move |text, caret| {
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
    let reedit = use_context::<Reedit>().expect("Home provides recall editing");
    let apply_reedit = apply.clone();
    effect(move || {
        if sending.get() {
            return;
        }
        if let Some(content) = reedit.take_request(&draft_id.with(|id| id.clone())) {
            let apply = apply_reedit.clone();
            input.after_composition(Rc::new(move || {
                let mut text = draft.get_untracked();
                text.push_str(&content);
                let caret = text.encode_utf16().count() as u32;
                apply(text, (caret, caret));
            }));
        }
    });
    let apply_insertion = apply.clone();
    let insert: Rc<dyn Fn(String)> = Rc::new(move |text| {
        let apply = apply_insertion.clone();
        input.after_composition(Rc::new(move || {
            let (value, caret) =
                input_edit::insert(&draft.get_untracked(), selection.get_untracked(), &text);
            apply(value, caret);
        }));
    });
    let mention_account = stored(account.clone());
    let editor = view()
        .px_4()
        .pt_2()
        .pb_3()
        .gap_2()
        .style_when(move || !can_send.get(), |style| style.hidden())
        .child(message_quote::reply_bar(actions))
        .child(when(
            move || {
                (group || header.with(|row| row.is_bot))
                    && !input.composing()
                    && input_focused.get()
            },
            move || {
                composer::mentions(
                    mention_account.with(|account| account.clone()),
                    draft_id.with(|id| id.clone()),
                    header.with(|row| row.is_bot),
                    draft,
                    selection,
                    apply.clone(),
                    input,
                )
            },
        ))
        .child(when(operation_failed, move || {
            ui::error_text(move || i18n.text(Key::CommandFailed))
        }))
        .child(when(draft_writer.failed, move || {
            view()
                .flex_row()
                .gap_2()
                .child(ui::error_text(move || {
                    i18n.tr(
                        "Couldn’t save your draft. Try again.",
                        "无法保存草稿，请重试。",
                    )
                }))
                .child(ui::action(
                    move || i18n.text(Key::Retry),
                    move || draft_writer.retry(),
                ))
        }))
        .child(when(send_failed, move || {
            ui::error_text(move || i18n.text(Key::SendFailed))
        }))
        .child(
            // Inner controls are 4pt inside this outline and share its curvature.
            view()
                .flex_row()
                .items_end()
                .gap_1()
                .p_1()
                .rounded(ui::radius_composer())
                .bg(ui::bubble_in)
                .border_1()
                .border_color(ui::hairline)
                .child(composer::attachments(
                    account.clone(),
                    conversation_id.clone(),
                    operation_failed,
                    quote,
                    incoming_files,
                ))
                .child(composer::stickers(
                    account,
                    conversation_id,
                    operation_failed,
                    insert,
                    quote,
                ))
                .child(input.view(
                    move || {
                        i18n.text(if encrypted.get() {
                            Key::EncryptedPlaceholder
                        } else {
                            Key::MessagePlaceholder
                        })
                        .to_owned()
                    },
                    sending,
                    input_focused,
                    Rc::new(move |event| {
                        let paths = super::local_paths(&event.uris);
                        if !paths.is_empty() {
                            incoming_files.set(paths);
                        } else if !event.files.is_empty() {
                            incoming_files.set(
                                event
                                    .files
                                    .iter()
                                    .map(|file| composer::AttachmentSource::Clipboard {
                                        name: file.name.clone(),
                                        bytes: Arc::new(file.data.clone()),
                                    })
                                    .collect(),
                            );
                        }
                    }),
                    send.clone(),
                ))
                .child(
                    when(
                        move || !draft.with(|s| s.trim().is_empty()) && !input.composing(),
                        move || {
                            let send = send.clone();
                            ui::primary()
                                .size_10()
                                .min_h_0()
                                .px_0()
                                .py_0()
                                .rounded(ui::radius_full())
                                .disabled(sending)
                                .accessibility_label(move || Some(i18n.text(Key::Send).into()))
                                .on_press(move || send())
                                .child(ui::icon(ui::Icon::Send).text_color(ui::fg_on_accent))
                        },
                    )
                    .otherwise(move || voice.button()),
                ),
        );
    view().child(voice.composer(editor)).child(when(
        move || !can_send.get(),
        move || unavailable(eligibility, i18n),
    ))
}

struct SendContext {
    account: Arc<AccountClient>,
    conversation_id: String,
    can_send: Signal<bool>,
    sending: Memo<bool>,
    draft: Memo<String>,
    input: composer::Input,
    quote: Signal<Option<MessageListView>>,
    target: Signal<Option<String>>,
    drafts: Signal<HashMap<String, String>>,
    pending: Signal<HashSet<String>>,
    draft_writer: crate::drafts::Drafts,
    send_failed: Signal<bool>,
    session_owner: Owner,
    eligibility: Eligibility,
}

fn send_command(context: SendContext) -> Rc<dyn Fn()> {
    let SendContext {
        account,
        conversation_id,
        can_send,
        sending,
        draft,
        input,
        quote,
        target,
        drafts,
        pending,
        draft_writer,
        send_failed,
        session_owner,
        eligibility,
    } = context;
    let owner = Owner::current();
    Rc::new(move || {
        if !can_send.get_untracked() || sending.get_untracked() || input.composing() {
            return;
        }
        let body = draft.get_untracked();
        if body.trim().is_empty() {
            return;
        }
        let client = account.clone();
        let quote_id = quote.with(|q| q.as_ref().map(|m| m.message_id.clone()));
        let sent_quote_id = quote_id.clone();
        let id = conversation_id.clone();
        let draft_key = id.clone();
        pending.update(|ids| {
            ids.insert(draft_key.clone());
        });
        send_failed.set(false);
        // The session owns the send: the draft must clear after leaving this chat.
        session_owner.run(|| {
            spawn(async move {
                let text = body.clone();
                let result = runtime::commit(async move {
                    client.message().send_text(id, text, quote_id, false).await
                })
                .await;
                pending.update(|ids| {
                    ids.remove(&draft_key);
                });
                match result {
                    Ok(_) => {
                        drafts.update(|rows| {
                            if rows.get(&draft_key) == Some(&body) {
                                rows.insert(draft_key.clone(), String::new());
                                draft_writer.save(draft_key.clone(), String::new());
                            }
                        });
                        if !owner.is_live() {
                            return;
                        }
                        if quote
                            .with(|q| q.as_ref().map(|m| &m.message_id) == sent_quote_id.as_ref())
                        {
                            quote.set(None);
                        }
                        // Writing the jump target shows the latest rows.
                        target.set(None);
                    }
                    Err(error) => {
                        if !owner.is_live() {
                            return;
                        }
                        if matches!(error, mixin_desktop_api::ClientError::PermissionDenied) {
                            eligibility.revoke();
                        }
                        send_failed.set(true);
                    }
                }
            })
        });
    })
}

/// Blank while the first answer is on its way: it arrives in a moment, and a
/// passing "Loading…" would only flash.
fn unavailable(eligibility: Eligibility, i18n: I18n) -> impl IntoElement {
    view()
        .min_h_14()
        .px_4()
        .justify_center()
        .items_center()
        .bg(ui::surface_list)
        .border_t_1()
        .border_color(ui::border)
        .child(
            text(move || {
                if eligibility.failed.get() {
                    i18n.text(Key::CommandFailed)
                } else if eligibility.loaded.get() {
                    i18n.tr(
                        "You can't send messages to this group because you're no longer a participant.",
                        "您不能发送消息，因为您已经不再是此群组成员。",
                    )
                } else {
                    ""
                }
            })
            .text_role(ui::TextRole::Body)
            .text_color(ui::fg_secondary),
        )
        .child(when(eligibility.failed, move || {
            ui::action(
                move || i18n.text(Key::Retry),
                move || eligibility.retry.update(|revision| *revision += 1),
            )
        }))
}
