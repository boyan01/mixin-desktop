//! Bounded window queries; SQLite defines message ordering, never the UI.
use std::{collections::HashSet, sync::Arc};

use mixin_desktop_api::{AccountClient, ClientResult, MessageListView};

pub(super) const PAGE: usize = 50;
pub(super) const RECENT: usize = 100;
pub(super) const HALF: usize = 99;
pub(super) const CAPACITY: usize = HALF * 2 + 1;

#[derive(Clone)]
pub(super) enum Load {
    Latest,
    Jump(String),
    InitialUnread(String),
    Older,
    Newer,
    Refresh {
        anchor: Option<String>,
        follow_tail: bool,
    },
}

pub(super) struct Window {
    pub rows: Vec<MessageListView>,
    pub older: bool,
    pub newer: bool,
    pub missing: bool,
    pub scroll: Option<String>,
    pub tail: bool,
    pub deleted: HashSet<String>,
}

pub(super) async fn load(
    account: Arc<AccountClient>,
    conversation: String,
    request: Load,
    previous: Window,
) -> ClientResult<Window> {
    let access = account.message();
    let explicit_jump = matches!(&request, Load::Jump(_));
    match request {
        Load::Latest => {
            let mut window = recent(&account, &conversation).await?;
            window.tail = true;
            Ok(window)
        }
        Load::Jump(id) | Load::InitialUnread(id) => {
            if previous.rows.iter().any(|row| row.message_id == id) {
                return Ok(Window {
                    scroll: Some(id),
                    missing: false,
                    ..previous
                });
            }
            let mut window = around(&account, &conversation, &id).await?;
            if window.rows.is_empty() {
                window = recent(&account, &conversation).await?;
                window.missing = explicit_jump;
                window.tail = true;
            } else {
                window.scroll = Some(id);
            }
            Ok(window)
        }
        Load::Older | Load::Newer => {
            let older = matches!(request, Load::Older);
            let anchor = if older {
                previous.rows.last()
            } else {
                previous.rows.first()
            };
            let Some(anchor) = anchor else {
                return recent(&account, &conversation).await;
            };
            let mut page = access
                .messages_relative(
                    conversation,
                    anchor.message_id.clone(),
                    older,
                    (PAGE + 1) as i64,
                )
                .await?;
            let more = page.len() > PAGE;
            if older {
                page.truncate(PAGE);
            } else if more {
                // The nearest newer page is returned newest-first. Its extra
                // lookahead is at the front, not next to the current window.
                page.remove(0);
            }
            let mut window = previous;
            if older {
                window.older = more;
                window.rows.extend(page);
            } else {
                window.newer = more;
                page.extend(window.rows);
                window.rows = page;
            }
            deduplicate(&mut window.rows);
            if window.rows.len() > CAPACITY {
                if older {
                    window.rows.drain(..window.rows.len() - CAPACITY);
                    window.newer = true;
                } else {
                    window.rows.truncate(CAPACITY);
                    window.older = true;
                }
            }
            window.scroll = None;
            window.missing = false;
            Ok(window)
        }
        Load::Refresh {
            anchor,
            follow_tail,
        } => {
            if follow_tail {
                // Refresh the actual contiguous latest window. Appending cached
                // rows would preserve deleted messages or leave holes after sync.
                let limit = previous.rows.len().clamp(RECENT, CAPACITY);
                let rows = access
                    .messages(conversation, None, None, limit as i64)
                    .await?;
                let deleted = deleted_rows(&account, &previous.rows, &rows).await?;
                return Ok(Window {
                    older: rows.len() == limit,
                    rows,
                    newer: false,
                    missing: false,
                    scroll: None,
                    tail: true,
                    deleted,
                });
            }
            // Find a surviving visible neighbor before reloading. A delete or
            // expiration must not strand a cursor on a row that no longer exists.
            let ids = previous
                .rows
                .iter()
                .map(|row| row.message_id.clone())
                .collect::<Vec<_>>();
            let existing = access
                .message_items_by_ids(ids)
                .await?
                .into_iter()
                .map(|row| row.message_id)
                .collect::<HashSet<_>>();
            let deleted = previous
                .rows
                .iter()
                .map(|row| row.message_id.clone())
                .filter(|id| !existing.contains(id))
                .collect();
            let position = anchor
                .as_ref()
                .and_then(|id| previous.rows.iter().position(|row| &row.message_id == id))
                .unwrap_or_default();
            let anchor = anchor.filter(|id| existing.contains(id)).or_else(|| {
                previous
                    .rows
                    .iter()
                    .enumerate()
                    .filter(|(_, row)| existing.contains(&row.message_id))
                    .min_by_key(|(index, _)| index.abs_diff(position))
                    .map(|(_, row)| row.message_id.clone())
            });
            let mut window = match anchor {
                Some(anchor) => around(&account, &conversation, &anchor).await?,
                None => recent(&account, &conversation).await?,
            };
            window.deleted = deleted;
            Ok(window)
        }
    }
}

async fn recent(account: &AccountClient, conversation: &str) -> ClientResult<Window> {
    let rows = account
        .message()
        .messages(conversation.to_owned(), None, None, RECENT as i64)
        .await?;
    Ok(Window {
        older: rows.len() == RECENT,
        rows,
        newer: false,
        missing: false,
        scroll: None,
        tail: false,
        deleted: HashSet::new(),
    })
}

async fn around(account: &AccountClient, conversation: &str, anchor: &str) -> ClientResult<Window> {
    let mut rows = account
        .message()
        .messages_around(
            conversation.to_owned(),
            anchor.to_owned(),
            HALF as i64,
            HALF as i64,
        )
        .await?;
    rows.reverse();
    let at = rows
        .iter()
        .position(|row| row.message_id == anchor)
        .unwrap_or_default();
    Ok(Window {
        newer: at >= HALF,
        older: rows.len().saturating_sub(at + 1) >= HALF,
        rows,
        missing: false,
        scroll: None,
        tail: false,
        deleted: HashSet::new(),
    })
}

async fn deleted_rows(
    account: &AccountClient,
    previous: &[MessageListView],
    next: &[MessageListView],
) -> ClientResult<HashSet<String>> {
    let present = next
        .iter()
        .map(|row| row.message_id.as_str())
        .collect::<HashSet<_>>();
    let mut missing = previous
        .iter()
        .filter(|row| !present.contains(row.message_id.as_str()))
        .map(|row| row.message_id.clone())
        .collect::<HashSet<_>>();
    if !missing.is_empty() {
        let existing = account
            .message()
            .message_items_by_ids(missing.iter().cloned().collect())
            .await?;
        for row in existing {
            missing.remove(&row.message_id);
        }
    }
    Ok(missing)
}

fn deduplicate(rows: &mut Vec<MessageListView>) {
    let mut seen = HashSet::with_capacity(rows.len());
    rows.retain(|row| seen.insert(row.message_id.clone()));
}
