use std::{collections::HashMap, sync::Arc};

use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, MessageListView};
use nsh_core::prelude::*;

use crate::runtime;

/// A presentation map for this bounded message window, rebuilt from the account.
#[derive(Clone, Copy)]
pub(super) struct MentionNames(pub Signal<HashMap<String, String>>);

impl MentionNames {
    pub fn mount(
        account: Arc<AccountClient>,
        messages: Signal<Vec<MessageListView>>,
        reply: Option<Signal<Option<MessageListView>>>,
    ) {
        let names = signal(HashMap::<String, String>::new());
        provide_context(Self(names));
        let refresh = signal(0_u64);
        let observer = account.clone();
        spawn(async move {
            let mut changes = Box::pin(runtime::observe(observer.conversation_changes()));
            while let Some(change) = changes.next().await {
                // Profile updates and stream lag both request a complete reload.
                if change.reload_all {
                    refresh.update(|revision| *revision = revision.wrapping_add(1));
                }
            }
        });
        effect(move || {
            refresh.get();
            let mut contents =
                messages.with(|rows| {
                    let mut contents = Vec::new();
                    for row in rows {
                        if row.category.ends_with("_TEXT") {
                            contents.push(row.content.clone());
                        }
                        if let Some(caption) =
                            row.caption.as_ref().filter(|caption| !caption.is_empty())
                        {
                            contents.push(caption.clone());
                        }
                        if let Some(quote) = row.quote_content.as_deref().and_then(|content| {
                            serde_json::from_str::<serde_json::Value>(content).ok()
                        }) && quote["type"]
                            .as_str()
                            .is_some_and(|category| category.ends_with("_TEXT"))
                            && let Some(content) = quote["content"].as_str()
                        {
                            contents.push(content.to_owned());
                        }
                    }
                    contents
                });
            if let Some(reply) = reply {
                reply.with(|row| {
                    if let Some(row) = row
                        && row.category.ends_with("_TEXT")
                    {
                        contents.push(row.content.clone());
                    }
                });
            }
            if contents.is_empty() {
                names.set(HashMap::new());
                return;
            }
            let account = account.clone();
            // NSH disposes this effect's task before querying the next window.
            spawn(async move {
                match runtime::run(async move { account.user().mention_names(contents).await })
                    .await
                {
                    Ok(values) => names.set(values),
                    Err(error) => log::warn!("Could not resolve message mentions: {error}"),
                }
            });
        });
    }
}
