use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::client::ClientRef;
use crate::{ApiError, UserSession};

pub struct ConversationApi {
    client: Arc<ClientRef>,
}

impl ConversationApi {
    pub(crate) fn new(client: Arc<ClientRef>) -> Self {
        ConversationApi { client }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(sqlx::Type)]
#[sqlx(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConversationCategory {
    Group,
    Contact,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Participant {
    pub user_id: String,
    pub role: Option<String>,
    #[serde(default)]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ParticipantRequest {
    pub user_id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Conversation {
    pub conversation_id: String,
    pub name: String,
    pub category: Option<ConversationCategory>,
    pub icon_url: String,
    pub code_url: String,
    pub created_at: DateTime<Utc>,
    pub participants: Vec<Participant>,
    pub participant_sessions: Option<Vec<UserSession>>,
    pub mute_until: DateTime<Utc>,
    #[serde(default)]
    pub expire_in: i64,
    pub announcement: String,
    pub creator_id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConversationRequest {
    pub conversation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub random_id: Option<String>,
    pub category: Option<ConversationCategory>,
    pub name: Option<String>,
    pub icon_base64: Option<String>,
    pub announcement: Option<String>,
    pub participants: Option<Vec<ParticipantRequest>>,
    pub duration: Option<i64>,
}

impl ConversationApi {
    pub async fn create_conversation(
        &self,
        request: &ConversationRequest,
    ) -> Result<Conversation, ApiError> {
        self.client.post("conversations", request).await
    }

    pub async fn update(&self, request: &ConversationRequest) -> Result<Conversation, ApiError> {
        self.client
            .post(
                &format!("conversations/{}", request.conversation_id),
                request,
            )
            .await
    }

    pub async fn exit(&self, conversation_id: &str) -> Result<(), ApiError> {
        self.client
            .post(&format!("conversations/{conversation_id}/exit"), "")
            .await
    }

    pub async fn get_conversation(&self, conversation_id: &str) -> Result<Conversation, ApiError> {
        self.client
            .get(&format!("conversations/{conversation_id}"))
            .await
    }

    pub async fn update_participants(
        &self,
        conversation_id: &str,
        action: &str,
        participants: &Vec<Participant>,
    ) -> Result<Conversation, ApiError> {
        self.client
            .post(
                &format!("conversations/{conversation_id}/participants/{action}"),
                participants,
            )
            .await
    }

    pub async fn mute(&self, request: &ConversationRequest) -> Result<Conversation, ApiError> {
        self.client
            .post(
                &format!("conversations/{}/mute", request.conversation_id),
                request,
            )
            .await
    }

    pub async fn rotate(&self, conversation_id: &str) -> Result<Conversation, ApiError> {
        self.client
            .post(&format!("conversations/{}/rotate", conversation_id), "")
            .await
    }

    pub async fn join(&self, code: &str) -> Result<Conversation, ApiError> {
        self.client
            .post(&format!("conversations/{code}/join"), "")
            .await
    }

    // duration: zero to turn off disappearing messages
    pub async fn disappear(&self, conversation_id: &str, duration: i64) -> Result<(), ApiError> {
        self.client
            .post(
                &format!("conversations/{}/disappear", conversation_id),
                &json!({"duration": duration}),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::{ConversationCategory, ConversationRequest, ParticipantRequest};

    #[test]
    fn serializes_create_participant_without_response_fields() {
        let request = ConversationRequest {
            conversation_id: "conversation".into(),
            random_id: None,
            category: Some(ConversationCategory::Contact),
            name: None,
            icon_base64: None,
            announcement: None,
            participants: Some(vec![ParticipantRequest {
                user_id: "recipient".into(),
            }]),
            duration: None,
        };

        let value = serde_json::to_value(request).unwrap();
        assert_eq!(
            value["participants"][0],
            serde_json::json!({"user_id": "recipient"})
        );
        assert!(value.get("random_id").is_none());
    }
}
