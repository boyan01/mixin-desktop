//! Account notification batches and bounded mention projection.
use super::{model, AccountRuntime};
use crate::db::MixinDatabase;
use anyhow::Result;

async fn resolve_mentions(
    database: &MixinDatabase,
    events: &mut [model::NotificationEvent],
) -> Result<()> {
    let mut positions = Vec::new();
    let mut contents = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if event.category.ends_with("_TEXT") {
            positions.push((index, None));
            contents.push(event.content.clone());
        } else if event.category == sdk::message_category::MESSAGE_PIN {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&event.content) {
                if value["category"]
                    .as_str()
                    .is_some_and(|kind| kind.ends_with("_TEXT"))
                {
                    contents.push(value["content"].as_str().unwrap_or_default().to_owned());
                    positions.push((index, Some(value)));
                }
            }
        }
    }
    for ((index, pin), content) in positions
        .into_iter()
        .zip(database.user_dao.replace_mentions(&contents).await?)
    {
        events[index].content = if let Some(mut pin) = pin {
            pin["content"] = content.into();
            pin.to_string()
        } else {
            content
        };
    }
    Ok(())
}

impl AccountRuntime {
    pub async fn notification_event_batch(
        &self,
        after_created_at_micros: i64,
        after_row_id: i64,
        limit: i64,
    ) -> Result<model::NotificationEventBatch> {
        let limit = limit.clamp(1, 200);
        let messages = self
            .database
            .message_dao
            .notification_items_after(&self.account_id, after_row_id, limit)
            .await?;
        let (next_created_at_micros, next_row_id) = messages
            .last()
            .map(|message| {
                (
                    message.created_at.and_utc().timestamp_micros(),
                    message.row_id,
                )
            })
            .unwrap_or((after_created_at_micros, after_row_id));
        let has_more = messages.len() == limit as usize;
        let identity_number = self.account().identity_number;
        let mut events = messages
            .into_iter()
            .filter_map(|message| {
                model::NotificationEvent::from_message(message, &self.account_id, &identity_number)
            })
            .collect::<Vec<_>>();
        resolve_mentions(&self.database, &mut events).await?;
        Ok(model::NotificationEventBatch {
            events,
            next_created_at_micros,
            next_row_id,
            has_more,
        })
    }

    pub async fn latest_notification_row_id(&self) -> Result<i64> {
        Ok(self.database.message_dao.latest_row_id().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::mixin::message::Message;

    #[tokio::test]
    async fn notification_batch_retains_system_actor_and_resolves_text_and_pin_mentions() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let pool = &database.message_dao.0;
        crate::db::query("INSERT INTO users (user_id,identity_number,full_name,created_at,mute_until) VALUES ('alice','7000','Alice',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP),('bob','7001','Bob',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)").execute(pool).await.unwrap();
        crate::db::query("INSERT INTO conversations (conversation_id,owner_id,name,category,created_at,status) VALUES ('chat','alice','Group','GROUP',0,0)").execute(pool).await.unwrap();
        for (id, category, content, action, participant) in [
            ("text", "SIGNAL_TEXT", "Hello @7001", None, None),
            (
                "pin",
                "MESSAGE_PIN",
                r#"{"category":"SIGNAL_TEXT","message_id":"text","content":"Hello @7001"}"#,
                None,
                None,
            ),
            (
                "system",
                "SYSTEM_CONVERSATION",
                "",
                Some("ADD"),
                Some("bob"),
            ),
        ] {
            database
                .message_dao
                .insert_message(&Message {
                    message_id: id.into(),
                    conversation_id: "chat".into(),
                    user_id: "alice".into(),
                    category: category.into(),
                    content: Some(content.into()),
                    status: sdk::MessageStatus::Read,
                    action: action.map(str::to_owned),
                    participant_id: participant.map(str::to_owned),
                    created_at: chrono::Utc::now().naive_utc(),
                    ..Message::default()
                })
                .await
                .unwrap();
        }
        let rows = database
            .message_dao
            .notification_items_after("me", 0, 10)
            .await
            .unwrap();
        let mut events: Vec<_> = rows
            .into_iter()
            .filter_map(|row| model::NotificationEvent::from_message(row, "me", "9999"))
            .collect();
        resolve_mentions(&database, &mut events).await.unwrap();
        let text = events
            .iter()
            .find(|event| event.message_id == "text")
            .unwrap();
        assert_eq!(text.content, "Hello @Bob");
        let pin = events
            .iter()
            .find(|event| event.message_id == "pin")
            .unwrap();
        let pin: serde_json::Value = serde_json::from_str(&pin.content).unwrap();
        assert_eq!(pin["content"], "Hello @Bob");
        assert_eq!(pin["message_id"], "text");
        let system = events
            .iter()
            .find(|event| event.message_id == "system")
            .unwrap();
        assert_eq!(system.sender_id, "alice");
        assert_eq!(system.sender_name, "Alice");
        assert_eq!(system.participant_id.as_deref(), Some("bob"));
        assert_eq!(system.participant_name.as_deref(), Some("Bob"));
        assert_eq!(system.action.as_deref(), Some("ADD"));
        assert_eq!(system.conversation_name, "Group");
        let raw = database
            .message_dao
            .find_message_by_id(&"text".into())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(raw.content.as_deref(), Some("Hello @7001"));
        database.close().await;
    }
}
