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
    pub category: Option<ConversationCategory>,
    pub name: Option<String>,
    pub icon_base64: Option<String>,
    pub announcement: Option<String>,
    pub participants: Option<Vec<Participant>>,
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
                &format!("conversations/{}", &request.conversation_id),
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
                &format!("conversations/{}/mute", &request.conversation_id),
                request,
            )
            .await
    }

    pub async fn rotate(&self, conversation_id: &str) -> Result<Conversation, ApiError> {
        self.client
            .post(&format!("conversations/{}/rotate", conversation_id), "")
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
    use crate::client::tests::new_test_client;

    #[tokio::test]
    async fn test_get_conversation() {
        let client = new_test_client().await;
        let result = client
            .conversation_api
            .get_conversation("131d9290-0298-4dd5-b0f7-9ded04753ef9")
            .await;
        println!("result: {:?}", result);
    }

    #[tokio::test]
    async fn test_quit_conversation() {
        let client = new_test_client().await;
        let result = client
            .conversation_api
            .exit("131d9290-0298-4dd5-b0f7-9ded04753ef9")
            .await;
        println!("result: {:?}", result);
    }
}
