use chrono::{NaiveDateTime, Utc};
use diesel::{Insertable, Queryable, RunQueryDsl, Selectable};
use diesel::dsl::insert_into;
use serde_json::json;
use uuid::Uuid;

use crate::core::util::unique_object_id;
use crate::db::{Error, MixinDatabase};
use crate::sdk::blaze_message::{CREATE_MESSAGE, PIN_MESSAGE, RECALL_MESSAGE};
use crate::sdk::message::{BlazeAckMessage, RecallMessage};

pub struct Job {
    pub job_id: String,
    pub action: String,
    pub created_at: NaiveDateTime,
    pub order_id: Option<i32>,
    pub priority: i32,
    pub user_id: Option<String>,
    pub blaze_message: Option<String>,
    pub conversation_id: Option<String>,
    pub resend_message_id: Option<String>,
    pub run_count: i32,
}

const UPDATE_STICKER: &str = "LOCAL_UPDATE_STICKER";
const UPDATE_ASSET: &str = "LOCAL_UPDATE_ASSET";
const UPDATE_TOKEN: &str = "LOCAL_UPDATE_TOKEN";
const SYNC_INSCRIPTION_MESSAGE: &str = "LOCAL_SYNC_INSCRIPTION_MESSAGE";

impl Job {
    fn new() -> Self {
        Job {
            job_id: Uuid::new_v4().to_string(),
            action: Default::default(),
            created_at: Utc::now().naive_utc(),
            order_id: None,
            priority: 5,
            user_id: None,
            blaze_message: None,
            conversation_id: None,
            resend_message_id: None,
            run_count: 0,
        }
    }

    pub fn create_ack_job(act: &str, message_id: &str, status: String, expire_at: Option<i32>) -> Job {
        let m = BlazeAckMessage {
            message_id: message_id.to_string(),
            status: status.to_uppercase(),
            expire_at,
        };
        let j_id = unique_object_id(&vec![
            m.message_id.as_str(),
            m.status.as_str(),
            act,
        ]).to_string();
        Job {
            job_id: j_id,
            action: act.to_string(),
            blaze_message: serde_json::to_string(&m).ok(),
            ..Job::new()
        }
    }

    pub fn create_mention_read_ack_job(cid: &str, message_id: &str) -> Job {
        Job {
            action: CREATE_MESSAGE.to_string(),
            conversation_id: Some(cid.to_string()),
            blaze_message: serde_json::to_string(&BlazeAckMessage {
                message_id: message_id.to_string(),
                status: "MENTION_READ".to_string(),
                expire_at: None,
            }).ok(),
            ..Self::new()
        }
    }

    pub fn create_send_pin_job(conversation_id: &str, encoded: &str) -> Job {
        Job {
            action: PIN_MESSAGE.to_string(),
            conversation_id: Some(conversation_id.to_string()),
            blaze_message: Some(encoded.to_string()),
            ..Self::new()
        }
    }

    pub fn create_send_recall_job(conversation_id: &str, message_id: &str) -> Job {
        let a = json!({
                "message_id": message_id
            });
        Job {
            conversation_id: Some(conversation_id.to_string()),
            action: RECALL_MESSAGE.to_string(),
            blaze_message: serde_json::to_string(&RecallMessage {
                message_id: message_id.to_string(),
            }).ok(),
            ..Self::new()
        }
    }

    pub fn create_update_sticker_job(sticker_id: &str) -> Job {
        Job {
            action: UPDATE_STICKER.to_string(),
            blaze_message: Some(sticker_id.to_string()),
            ..Self::new()
        }
    }

    pub fn create_update_asset_job(asset_id: &str) -> Job {
        Job {
            action: UPDATE_ASSET.to_string(),
            blaze_message: Some(asset_id.to_string()),
            ..Self::new()
        }
    }

    pub fn create_update_token_job(asset_id: &str) -> Job {
        Job {
            action: UPDATE_TOKEN.to_string(),
            blaze_message: Some(asset_id.to_string()),
            ..Self::new()
        }
    }

    pub fn create_sync_inscription_message_job(message_id: &str) -> Job {
        Job {
            action: SYNC_INSCRIPTION_MESSAGE.to_string(),
            blaze_message: Some(message_id.to_string()),
            ..Self::new()
        }
    }
}


impl MixinDatabase {
    pub async fn insert_job(&self, j: &Job) -> Result<(), Error> {
        // insert_into(jobs).values(j).execute(&mut self.get_connection()?)?;
        Ok(())
    }
}