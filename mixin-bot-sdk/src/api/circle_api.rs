use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::client::ClientRef;
use crate::ApiError;

pub struct CircleApi {
    pub(crate) client: Arc<ClientRef>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Circle {
    pub circle_id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct CircleConversation {
    pub conversation_id: String,
    pub circle_id: String,
    pub user_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub pin_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "action", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CircleConversationRequest {
    Add {
        conversation_id: String,
        user_id: Option<String>,
    },
    Remove {
        conversation_id: String,
        user_id: Option<String>,
    },
}

impl CircleApi {
    pub async fn get_circles(&self) -> Result<Vec<Circle>, ApiError> {
        self.client.get("circles").await
    }

    pub async fn get_circle(&self, id: &str) -> Result<Circle, ApiError> {
        self.client.get(&format!("circles/{id}")).await
    }

    pub async fn create_circle(&self, name: &str) -> Result<Circle, ApiError> {
        self.client
            .post("circles", &serde_json::json!({"name": name}))
            .await
    }

    pub async fn update_circle(&self, id: &str, name: &str) -> Result<Circle, ApiError> {
        self.client
            .post(&format!("circles/{id}"), &serde_json::json!({"name": name}))
            .await
    }

    pub async fn delete_circle(&self, id: &str) -> Result<(), ApiError> {
        self.client.post(&format!("circles/{id}/delete"), "").await
    }

    pub async fn update_circle_conversation(
        &self,
        id: &str,
        request: &CircleConversationRequest,
    ) -> Result<CircleConversation, ApiError> {
        self.client
            .post(&format!("circles/{id}/conversations"), request)
            .await
    }

    pub async fn get_circle_conversations(
        &self,
        id: &str,
        offset: impl Into<Option<&str>>,
        limit: impl Into<Option<u64>>,
    ) -> Result<Vec<CircleConversation>, ApiError> {
        let offset = offset.into().unwrap_or_default();
        let limit = limit.into().unwrap_or(500);
        self.client
            .get(&format!(
                "circles/{id}/conversations?offset={offset}&limit={limit}"
            ))
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::CircleConversationRequest;

    #[tokio::test]
    async fn test() {
        let action = CircleConversationRequest::Add {
            conversation_id: "1".to_string(),
            user_id: Some("2".to_string()),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(
            json,
            r#"{"action":"ADD","conversation_id":"1","user_id":"2"}"#
        );
    }
}
