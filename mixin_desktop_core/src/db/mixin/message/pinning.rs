//! Transactional pin commands and stable ordering coordinates.
use super::*;

impl MessageDao {
    pub async fn can_unpin_messages(
        &self,
        conversation_id: &str,
        account_id: &str,
    ) -> Result<bool, Error> {
        Self::can_unpin_with(&self.0, conversation_id, account_id).await
    }

    async fn can_unpin_with<E: Executor>(
        executor: E,
        conversation_id: &str,
        account_id: &str,
    ) -> Result<bool, Error> {
        Ok(crate::db::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM conversations c WHERE c.conversation_id = ?1 \
             AND (c.category = 'CONTACT' OR (c.category = 'GROUP' AND c.status != ?2 AND EXISTS(\
             SELECT 1 FROM participants p WHERE p.conversation_id = c.conversation_id AND p.user_id = ?3 AND p.role IS NOT NULL))))")
            .bind(conversation_id).bind(super::super::conversation::ConversationStatus::QUIT.0).bind(account_id)
            .fetch_one(executor).await?)
    }

    pub async fn pinned_message_offset(
        &self,
        conversation_id: &str,
        message_id: &str,
    ) -> Result<Option<i64>, Error> {
        Ok(crate::db::query_scalar(
            "SELECT (SELECT COUNT(*) FROM pin_messages p INNER JOIN messages m ON m.message_id = p.message_id \
             WHERE p.conversation_id = ?1 AND (m.created_at > anchor.created_at OR \
             (m.created_at = anchor.created_at AND m.message_id > anchor.message_id))) \
             FROM pin_messages pin INNER JOIN messages anchor ON anchor.message_id = pin.message_id \
             WHERE pin.conversation_id = ?1 AND pin.message_id = ?2")
            .bind(conversation_id).bind(message_id).fetch_optional(&self.0).await?)
    }

    pub async fn unpin_all_with_jobs(
        &self,
        conversation_id: &str,
        account_id: &str,
    ) -> Result<(), Error> {
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        let allowed = Self::can_unpin_with(&mut *transaction, conversation_id, account_id).await?;
        if !allowed {
            return Err(Error::PermissionDenied);
        }
        loop {
            let ids: Vec<String> = crate::db::query_scalar("SELECT message_id FROM pin_messages WHERE conversation_id = ? ORDER BY message_id LIMIT 100")
                .bind(conversation_id).fetch_all(&mut *transaction).await?;
            if ids.is_empty() {
                break;
            }
            let payload = serde_json::to_string(&sdk::PinMessagePayload::Unpin(ids.clone()))
                .map_err(anyhow::Error::from)?;
            Self::insert_job_with(
                &mut transaction,
                &Job::create_send_pin_job(conversation_id, &payload),
            )
            .await?;
            for id in ids {
                crate::db::query(
                    "DELETE FROM pin_messages WHERE conversation_id = ? AND message_id = ?",
                )
                .bind(conversation_id)
                .bind(id)
                .execute(&mut *transaction)
                .await?;
            }
        }
        transaction.commit().await?;
        Ok(())
    }
}
