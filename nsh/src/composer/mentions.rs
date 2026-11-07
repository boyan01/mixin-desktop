use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, ConversationParticipantItem};
use nsh_base::theme;
use nsh_core::prelude::*;
use std::{rc::Rc, sync::Arc};

use super::{failure, tr};
use crate::{i18n, runtime, ui};

pub(crate) fn mentions(
    account: Arc<AccountClient>,
    conversation: String,
    bot: bool,
    draft: Memo<String>,
    selection: Signal<(u32, u32)>,
    apply: Rc<dyn Fn(String, (u32, u32))>,
    input: super::Input,
) -> impl IntoElement {
    const ROW_HEIGHT: f32 = 48.0;
    const MAX_HEIGHT: f32 = ROW_HEIGHT * 3.5;
    let i18n = i18n::current();
    let candidate = computed(move || candidate(&draft.get(), selection.get()));
    let dismissed = signal(None::<(usize, usize, String)>);
    let account = stored(account);
    let conversation = stored(conversation);
    let apply = stored(apply);
    when(
        move || candidate.get().is_some() && candidate.get() != dismissed.get(),
        move || {
            cleanup(move || input.picker(false));
            let rows = signal_with(
                Vec::<ConversationParticipantItem>::new(),
                SignalEquals::Never,
            );
            let loading = signal(false);
            let failed = signal(false);
            let selected = signal(0_usize);
            let loaded = signal(None::<(usize, usize, String)>);
            // Group messages refresh a settled query; only a new query is "searching".
            let searching = computed(move || loading.get() && loaded.get() != candidate.get());
            // The picker owns editor keys only while there is something to pick.
            // A failed lookup leaves Enter free to send the draft.
            let picking = computed(move || searching.get() || !rows.with(Vec::is_empty));
            effect(move || input.picker(picking.get()));
            let refresh = signal(0_u64);
            let observer = account.with(|account| account.clone());
            spawn(async move {
                let mut changes = Box::pin(runtime::observe(observer.conversation_changes()));
                while let Some(change) = changes.next().await {
                    if change.reload_all
                        || conversation.with(|id| change.conversation_ids.contains(id))
                    {
                        refresh.update(|revision| *revision = revision.wrapping_add(1));
                    }
                }
            });
            let list = element_ref();
            effect(move || {
                let index = selected.get();
                after_answer(move || {
                    list.scroll_to_index(index as u32, percentage(0.5), px(0.0), false)
                });
            });
            effect(move || {
                refresh.get();
                let Some(request) = candidate.get() else {
                    return;
                };
                let query = request.2.clone();
                let client = account.with(|a| a.clone());
                let id = conversation.with(|id| id.clone());
                loading.set(true);
                failed.set(false);
                // Rows stay until their replacement arrives: every group message
                // refreshes this query, and `loaded` still guards choosing.
                spawn(async move {
                    let result = runtime::run(async move {
                        if bot {
                            client
                                .conversation()
                                .search_bot_group_users(id, query)
                                .await
                        } else {
                            client.conversation().search_group_users(id, query).await
                        }
                    })
                    .await;
                    match result {
                        Ok(values) => {
                            let same_query = loaded.get_untracked().as_ref() == Some(&request);
                            let last = values.len().saturating_sub(1);
                            batch(|| {
                                selected.update(|at| {
                                    *at = if same_query { (*at).min(last) } else { 0 }
                                });
                                rows.set(values);
                                loaded.set(Some(request));
                            });
                        }
                        Err(_) => {
                            rows.set(Vec::new());
                            failed.set(true);
                        }
                    }
                    loading.set(false);
                });
            });
            let choose = move |index: usize| {
                if input.composing() {
                    return;
                }
                if candidate.get_untracked() != loaded.get_untracked() {
                    return;
                }
                let Some(user) = rows.with(|rows| rows.get(index).cloned()) else {
                    return;
                };
                let Some((start, end, _)) = candidate.get_untracked() else {
                    return;
                };
                let mut value = draft.get_untracked();
                let replacement = format!("@{} ", user.identity_number);
                value.replace_range(start..end, &replacement);
                let caret = value[..start + replacement.len()].encode_utf16().count() as u32;
                apply.with(|apply| apply(value, (caret, caret)));
            };
            let seen = stored(std::cell::Cell::new(input.keys.get_untracked().0));
            effect(move || {
                let (revision, key) = input.keys.get();
                if seen.with(|seen| seen.replace(revision)) == revision
                    || input.composing()
                    || !picking.get_untracked()
                {
                    return;
                }
                match key.as_str() {
                    "Escape" => dismissed.set(candidate.get_untracked()),
                    "ArrowDown" => {
                        let last = rows.with_untracked(Vec::len).saturating_sub(1);
                        selected.update(|at| *at = (*at + 1).min(last));
                    }
                    "ArrowUp" => selected.update(|at| *at = at.saturating_sub(1)),
                    "Enter" | "Tab" => choose(selected.get_untracked()),
                    _ => {}
                }
            });
            view()
                .style_when(
                    move || !picking.get() && !failed.get(),
                    |style| style.hidden(),
                )
                .role(AccessibilityRole::Listbox)
                .min_h_0()
                .border_b_1()
                .border_color(move || theme().border)
                .bg(move || theme().popover)
                .child(when(
                    move || searching.get() && rows.with(Vec::is_empty),
                    move || {
                        text(move || tr(i18n, "Loading members…", "正在加载群成员…"))
                            .px_3()
                            .py_2()
                            .text_sm()
                            .text_color(move || theme().muted_foreground)
                    },
                ))
                .child(when(failed, move || {
                    view().px_3().py_2().child(failure(move || {
                        tr(
                            i18n,
                            "Couldn’t load members. Type again to retry.",
                            "无法加载群成员，请重新输入重试。",
                        )
                    }))
                }))
                .child(
                    virtual_list(
                        move || rows.with(Vec::len),
                        move |at| {
                            // A passive listbox option keeps editing focus on the existing
                            // chat input, as NSH's combobox options do. A button would move it.
                            view()
                                .role(AccessibilityRole::Option)
                                .focusable(false)
                                .accessibility_state(move || nsh_core::AccessibilityState {
                                    selected: selected.get() == at.index(),
                                    ..Default::default()
                                })
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .hover(|s| s.bg(move || theme().accent))
                                .flex_row()
                                .justify_start()
                                .gap_2()
                                .style_when(
                                    move || selected.get() == at.index(),
                                    |s| s.bg(move || theme().accent),
                                )
                                .child(
                                    ui::avatar_view(
                                        move || {
                                            rows.with(|rows| {
                                                rows.get(at.index())
                                                    .map(|u| u.full_name.clone().into())
                                                    .unwrap_or_default()
                                            })
                                        },
                                        move || {
                                            rows.with(|rows| {
                                                rows.get(at.index())
                                                    .map(|u| u.avatar_url.clone())
                                                    .unwrap_or_default()
                                            })
                                        },
                                    )
                                    .size_8(),
                                )
                                .child(
                                    text(move || {
                                        rows.with(|rows| {
                                            rows.get(at.index())
                                                .map(|u| {
                                                    format!(
                                                        "{} @{}",
                                                        u.full_name, u.identity_number
                                                    )
                                                })
                                                .unwrap_or_default()
                                        })
                                    })
                                    .text_sm(),
                                )
                                .on_press_down(move |_: &()| {
                                    if is_desktop() {
                                        choose(at.index());
                                    }
                                })
                                .on_press(move |_: &()| choose(at.index()))
                        },
                    )
                    .keys(move || {
                        rows.with(|rows| rows.iter().map(|u| u.user_id.clone()).collect::<Vec<_>>())
                    })
                    .element_ref(&list)
                    .estimated_item_size(px(ROW_HEIGHT))
                    .h(move || px((rows.with(Vec::len) as f32 * ROW_HEIGHT).min(MAX_HEIGHT))),
                )
        },
    )
}

fn candidate(text: &str, (start, end): (u32, u32)) -> Option<(usize, usize, String)> {
    if start != end {
        return None;
    }
    let mut units = 0_u32;
    let mut byte = 0;
    for ch in text.chars() {
        if units == end {
            break;
        }
        units += ch.len_utf16() as u32;
        byte += ch.len_utf8();
    }
    // Reject stale ranges and a caret in the middle of a UTF-16 surrogate pair.
    if units != end {
        return None;
    }
    let end = byte;
    let prefix = &text[..end];
    let start = prefix.rfind('@')?;
    let query = &text[start + 1..end];
    if query.chars().any(char::is_whitespace) {
        return None;
    }
    Some((start, end, query.to_owned()))
}
