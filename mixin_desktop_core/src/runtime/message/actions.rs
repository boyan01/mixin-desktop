//! Account-owned message actions operations.

use super::{
    super::{model, validate_combine_forward_source},
    can_recall_message, validate_forward_source, MessageAccess,
};
use anyhow::{anyhow, Result};
use chrono::Utc;
use sdk::{message_category::MessageCategory as _, ConversationCategory, MessageStatus};

impl MessageAccess {
    /// Returns current business permissions for a bounded message selection.
    /// Commands revalidate these permissions when they execute.
    pub async fn message_actions(
        &self,
        conversation_id: String,
        message_ids: Vec<String>,
    ) -> Result<Vec<model::MessageActions>> {
        self.ensure_active()?;
        if message_ids.len() > 200 {
            return Err(anyhow!("message selection exceeds 200 items"));
        }
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(&conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found"))?;
        let participants = self
            .database
            .participant_dao
            .list_items(&conversation_id)
            .await?;
        let current_role = participants
            .iter()
            .find(|p| p.user_id == self.account_id)
            .and_then(|p| p.role.as_deref());
        let pin_allowed =
            conversation.category != Some(ConversationCategory::Group) || current_role.is_some();
        let now = Utc::now().naive_utc();
        let messages = self.messages_by_ids(&message_ids).await?;
        messages
            .iter()
            .map(|message| {
                if message.conversation_id != conversation_id {
                    return Err(anyhow!("message does not belong to conversation"));
                }
                let sender = participants
                    .iter()
                    .find(|p| p.user_id == message.user_id)
                    .map(|p| crate::db::mixin::participant::Participant {
                        conversation_id: conversation_id.clone(),
                        user_id: p.user_id.clone(),
                        role: p.role.clone(),
                        created_at: p.created_at,
                    });
                let sent = matches!(
                    message.status,
                    MessageStatus::Sent | MessageStatus::Delivered | MessageStatus::Read
                );
                Ok(model::MessageActions {
                    message_id: message.message_id.clone(),
                    can_reply: message.category.can_reply(),
                    can_forward: validate_forward_source(message, "SIGNAL").is_ok(),
                    can_combine_forward: validate_combine_forward_source(message).is_ok(),
                    can_recall: can_recall_message(
                        message,
                        &conversation,
                        &self.account_id,
                        current_role,
                        sender.as_ref(),
                        now,
                    ),
                    can_pin: pin_allowed && sent && message.category.can_reply(),
                })
            })
            .collect()
    }
}
