use std::error::Error;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::str::FromStr;
use std::sync::Arc;
use std::u8;

use base64ct::Encoding;
use libsignal_protocol::{
    CiphertextMessage, CiphertextMessageType, Context, group_decrypt, message_decrypt,
    ProtocolAddress, SenderKeyName, SignalProtocolError,
};
use rand_core::OsRng;
use ulid::Ulid;

use crate::core::crypto::signal_protocol_store::SignalProtocolStore;
use crate::db::SignalDatabase;
use crate::sdk::message_category;

pub struct SignalProtocol {
    protocol_store: SignalProtocolStore,
}

impl SignalProtocol {
    pub fn new(db: Arc<SignalDatabase>, account_id: String) -> Self {
        SignalProtocol {
            protocol_store: SignalProtocolStore::new(db, account_id),
        }
    }
}

pub struct ComposeMessageData {
    pub resend_message_id: Option<String>,
    pub message: CiphertextMessage,
}

const CIPHERTEXT_MESSAGE_TYPE_WHISPER: u8 = CiphertextMessageType::Whisper as u8;
const CIPHERTEXT_MESSAGE_TYPE_PRE_KEY: u8 = CiphertextMessageType::PreKey as u8;
const CIPHERTEXT_MESSAGE_TYPE_SENDER_KEY: u8 = CiphertextMessageType::SenderKey as u8;
const CIPHERTEXT_MESSAGE_TYPE_SENDER_KEY_DISTRIBUTION: u8 =
    CiphertextMessageType::SenderKeyDistribution as u8;

impl SignalProtocol {
    pub fn decode_message_data(&self, encoded: &str) -> Result<ComposeMessageData, Box<dyn Error>> {
        if encoded.is_empty() {
            return Err(SignalProtocolError::InvalidArgument("Empty message".into()).into());
        }
        let cipher_text = base64ct::Base64::decode_vec(&encoded)?;
        let message_type = cipher_text[1] >> 4;

        let is_resend_message = cipher_text[1] == 1;
        let (resend_message_id, data) = if is_resend_message {
            let message_id = String::from_utf8_lossy(&cipher_text[8..44]);
            (Some(message_id.to_string()), &cipher_text[44..])
        } else {
            (None, &cipher_text[8..])
        };

        let message: CiphertextMessage = match message_type {
            CIPHERTEXT_MESSAGE_TYPE_WHISPER => CiphertextMessage::SignalMessage(data.try_into()?),
            CIPHERTEXT_MESSAGE_TYPE_PRE_KEY => {
                CiphertextMessage::PreKeySignalMessage(data.try_into()?)
            }
            CIPHERTEXT_MESSAGE_TYPE_SENDER_KEY => {
                CiphertextMessage::SenderKeyMessage(data.try_into()?)
            }
            CIPHERTEXT_MESSAGE_TYPE_SENDER_KEY_DISTRIBUTION => {
                CiphertextMessage::SenderKeyDistributionMessage(data.try_into()?)
            }
            _ => return Err("Invalid message type".into()),
        };

        Ok(ComposeMessageData {
            resend_message_id,
            message,
        })
    }

    fn device_id(session_id: Option<&str>) -> Result<u32, Box<dyn Error>> {
        if let Some(session_id) = session_id {
            let mut hash = DefaultHasher::new();
            Ulid::from_str(session_id)?.hash(&mut hash);
            let code = hash.finish();
            Ok(code as u32)
        } else {
            Ok(1)
        }
    }

    pub async fn decrypt(
        &self,
        group_id: &str,
        sender_id: &str,
        data: &ComposeMessageData,
        category: &str,
        session_id: Option<&str>,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let address = ProtocolAddress::new(sender_id.to_string(), SignalProtocol::device_id(session_id)?);

        let context: Context = None;

        let mut store = self.protocol_store.clone();
        if category == message_category::SIGNAL_KEY {
            let rng = &mut OsRng;
            let message = message_decrypt(
                &data.message,
                &address,
                &mut store.session_store,
                &mut store.identity_store,
                &mut store.pre_key_store,
                &mut store.signed_pre_key_store,
                rng,
                context,
            )
            .await?;
            return Ok(message);
        } else {
            let sender_key_id = SenderKeyName::new(group_id.to_string(), address)?;
            let message = group_decrypt(
                data.message.serialize(),
                &mut store.sender_key_store,
                &sender_key_id,
                context,
            )
            .await?;
            Ok(message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_signal_protocol() {
        let db = Arc::new(SignalDatabase::connect("".to_string()).await.unwrap());
        let protocol = SignalProtocol::new(db, "".to_string());
        protocol.decode_message_data("").unwrap();
    }
}
