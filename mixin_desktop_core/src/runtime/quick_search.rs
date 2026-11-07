//! Account-scoped, globally ranked command-palette pages.
use std::collections::HashMap;

use super::{model::QuickSearchItem, AccountRuntime};

impl AccountRuntime {
    pub async fn quick_search(
        &self,
        query: String,
        offset: i64,
        limit: i64,
    ) -> crate::Result<Vec<QuickSearchItem>> {
        self.ensure_active()?;
        let ids = crate::db::mixin::quick_search::ranked_ids(&self.database, &query, offset, limit)
            .await?;
        let user_ids = ids
            .iter()
            .filter(|(kind, _)| *kind == 0)
            .map(|(_, id)| id.clone())
            .collect::<Vec<_>>();
        let conversation_ids = ids
            .iter()
            .filter(|(kind, _)| *kind == 1)
            .map(|(_, id)| id.clone())
            .collect::<Vec<_>>();
        let mut users = if user_ids.is_empty() {
            HashMap::new()
        } else {
            self.database
                .user_dao
                .find_users(&user_ids)
                .await?
                .into_iter()
                .map(|user| (user.user_id.clone(), user))
                .collect()
        };
        let mut conversations = if conversation_ids.is_empty() {
            HashMap::new()
        } else {
            self.database
                .conversation_dao
                .list_items_by_ids(&conversation_ids)
                .await?
                .into_iter()
                .map(|conversation| (conversation.conversation_id.clone(), conversation))
                .collect()
        };
        Ok(ids
            .into_iter()
            .filter_map(|(kind, id)| {
                if kind == 0 {
                    users
                        .remove(&id)
                        .map(|user| QuickSearchItem::User(Box::new(user.into())))
                } else {
                    conversations.remove(&id).map(|conversation| {
                        QuickSearchItem::Conversation(Box::new(conversation.into()))
                    })
                }
            })
            .collect())
    }
}
