use std::any::Any;
use std::default::Default;
use std::future::Future;
use std::ops::Add;
use std::sync::Arc;

use chrono::TimeDelta;
use log::{debug, error};

use crate::core::crypto::signal_protocol::SignalProtocol;
use crate::core::AnyError;
use crate::db;
use crate::db::mixin::flood_message::FloodMessage;
use crate::db::mixin::job::Job;
use crate::db::mixin::message::Message;
use crate::db::mixin::MixinDatabase;
use crate::sdk::blaze_message::{BlazeMessageData, MessageStatus, ACKNOWLEDGE_MESSAGE_RECEIPTS};
use crate::sdk::message_category;
use crate::sdk::message_category::MessageCategory;

struct ServiceDecryptMessage {
    database: Arc<MixinDatabase>,
    signal_protocol: Arc<SignalProtocol>,
    user_id: String,
}

impl ServiceDecryptMessage {
    pub async fn start(&self) -> Result<(), AnyError> {
        loop {
            let messages = self.database.flood_messages().await?;
            for m in messages {
                match self.process_message(&m).await {
                    Err(err) => {
                        error!("failed to process message: {:?}", err)
                    }
                    _ => {}
                }
            }
        }
    }

    async fn process_message(&self, message: &FloodMessage) -> Result<(), AnyError> {
        let data: BlazeMessageData = serde_json::from_slice(message.data.as_bytes())?;
        if !self.database.is_message_exits(&message.message_id)? {
            // TODO update remote message status
            self.database
                .delete_flood_message(&message.message_id)
                .await?;
            return Ok(());
        }

        let status = match self.parse_flood_message(&data).await {
            Err(err) => {
                error!("failed to handle flood message: {:?}.", err);
                self.handle_invalid_message(&data).await?
            }
            Ok(status) => status,
        };

        Ok(())
    }

    async fn handle_invalid_message(
        &self,
        data: &BlazeMessageData,
    ) -> Result<MessageStatus, AnyError> {
        if data.category == Some(message_category::SIGNAL_KEY.to_string()) {
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.user_id.clone(),
                content: Some(data.data.clone()),
                category: data.category.clone().ok_or("unknown message category")?,
                status: MessageStatus::Unknown.into(),
                ..Message::default()
            };
            self.insert_message(&message, data).await?
        }
        Ok(MessageStatus::Delivered)
    }

    async fn parse_flood_message(
        &self,
        data: &BlazeMessageData,
    ) -> Result<MessageStatus, AnyError> {
        let category = data.category.clone().unwrap_or("".to_string());
        let mut status = MessageStatus::Delivered;
        let handled = if category.is_illegal_message_category() {
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.user_id.clone(),
                category,
                created_at: data.created_at.naive_utc(),
                status: data.status.clone(),
                ..Message::default()
            };
            self.insert_message(&message, &data).await
        } else if category.is_signal() {
            // TODO handle message history
            self.process_signal_message(data).await
        } else if category.is_plain() {
            self.process_plain_message(data).await
        } else if category.is_encrypted() {
            self.process_encrypted_message(data).await
        } else if category.is_system() {
            status = MessageStatus::Read;
            self.process_system_message(data).await
        } else if category.is_app_card() {
            self.process_app_card(data).await
        } else if category.is_app_button_group() {
            self.process_grouped_button(data).await
        } else if category.is_pin() {
            status = MessageStatus::Read;
            self.process_pin(data).await
        } else if category.is_recall() {
            status = MessageStatus::Read;
            self.process_recall(data).await
        } else {
            Ok(())
        };
        if let Err(err) = handled {
            error!("failed to process: {:?}", err);
            status = MessageStatus::Delivered;
            self.handle_invalid_message(data).await?;
        }

        Ok(MessageStatus::Failed)
    }
}

impl ServiceDecryptMessage {
    async fn update_remote_message_status(
        &self,
        message_id: &String,
        status: MessageStatus,
    ) -> Result<(), AnyError> {
        if status != MessageStatus::Delivered && status != MessageStatus::Read {
            Ok(())
        } else {
            self.database
                .insert_job(&Job::create_ack_job(
                    ACKNOWLEDGE_MESSAGE_RECEIPTS,
                    message_id.as_str(),
                    status.into(),
                    None,
                ))
                .await?;
            Ok(())
        }
    }
}

impl ServiceDecryptMessage {
    async fn process_signal_message(&self, data: &BlazeMessageData) -> Result<(), AnyError> {
        let message_data = self.signal_protocol.decode_message_data(&data.data)?;
        Ok(())
    }
}

impl ServiceDecryptMessage {
    async fn process_plain_message(&self, data: &BlazeMessageData) -> Result<(), AnyError> {
        Ok(())
    }
}
impl ServiceDecryptMessage {
    async fn process_encrypted_message(&self, data: &BlazeMessageData) -> Result<(), AnyError> {
        Ok(())
    }
}
impl ServiceDecryptMessage {
    async fn process_system_message(&self, data: &BlazeMessageData) -> Result<(), AnyError> {
        Ok(())
    }
}

impl ServiceDecryptMessage {
    async fn process_grouped_button(&self, data: &BlazeMessageData) -> Result<(), AnyError> {
        Ok(())
    }

    async fn process_app_card(&self, data: &BlazeMessageData) -> Result<(), AnyError> {
        Ok(())
    }

    async fn process_pin(&self, data: &BlazeMessageData) -> Result<(), AnyError> {
        Ok(())
    }

    async fn process_recall(&self, data: &BlazeMessageData) -> Result<(), AnyError> {
        Ok(())
    }
}

impl ServiceDecryptMessage {
    async fn insert_message(
        &self,
        message: &Message,
        data: &BlazeMessageData,
    ) -> Result<(), AnyError> {
        self.database.insert_message(message)?;
        // TODO(BIN): insert fts
        let expire_in = data.expire_in.unwrap_or(0);
        if expire_in > 0 && message.user_id == self.user_id {
            let expire_at = data.created_at + TimeDelta::seconds(expire_in as i64);
            self.database
                .update_message_expired_at(&data.message_id, &expire_at.naive_utc())?
        }
        Ok(())
    }
}
