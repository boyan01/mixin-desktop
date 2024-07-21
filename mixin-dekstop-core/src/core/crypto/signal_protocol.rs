use std::error::Error;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::str::FromStr;
use std::sync::Arc;
use std::u8;

use base64ct::Encoding;
use libsignal_protocol::{
    group_decrypt, message_decrypt, CiphertextMessage, CiphertextMessageType, Context,
    IdentityKeyStore, PreKeyStore, ProtocolAddress, SenderKeyName, SenderKeyStore, SessionStore,
    SignedPreKeyStore,
};
use rand_core::OsRng;
use ulid::Ulid;

use crate::db::SignalDatabase;
use crate::sdk::message_category;

pub struct SignalProtocol {
    session_store: Box<dyn SessionStore>,
    identity_store: Box<dyn IdentityKeyStore>,
    pre_key_store: Box<dyn PreKeyStore>,
    signed_pre_key_store: Box<dyn SignedPreKeyStore>,
    sender_key_store: Box<dyn SenderKeyStore>,
}

impl SignalProtocol {
    fn new(db: Arc<SignalDatabase>) -> Self {
        SignalProtocol {
            session_store: Box::new(db.clone()),
            identity_store: Box::new(db.clone()),
            pre_key_store: Box::new(db.clone()),
            signed_pre_key_store: Box::new(db.clone()),
            sender_key_store: Box::new(db.clone()),
        }
    }
}

struct ComposeMessageData {
    resend_message_id: Option<String>,
    message: CiphertextMessage,
}

const CIPHERTEXT_MESSAGE_TYPE_WHISPER: u8 = CiphertextMessageType::Whisper as u8;
const CIPHERTEXT_MESSAGE_TYPE_PRE_KEY: u8 = CiphertextMessageType::PreKey as u8;
const CIPHERTEXT_MESSAGE_TYPE_SENDER_KEY: u8 = CiphertextMessageType::SenderKey as u8;
const CIPHERTEXT_MESSAGE_TYPE_SENDER_KEY_DISTRIBUTION: u8 =
    CiphertextMessageType::SenderKeyDistribution as u8;

impl SignalProtocol {
    pub fn decode_message_data(
        &self,
        encoded: &String,
    ) -> Result<ComposeMessageData, Box<dyn Error>> {
        if encoded.is_empty() {}
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
        group_id: String,
        sender_id: String,
        data: ComposeMessageData,
        category: String,
        session_id: Option<&str>,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let address = ProtocolAddress::new(sender_id, SignalProtocol::device_id(session_id)?);

        let context: Context = None;

        let mut session = *self.session_store;
        if category == message_category::SIGNAL_KEY {
            let rng = &mut OsRng;
            let message = message_decrypt(
                &data.message,
                &address,
                &mut session,
                &self.identity_store,
                &self.pre_key_store,
                &self.signed_pre_key_store,
                rng,
                context,
            )
            .await?;
            return Ok(message);
        } else {
            let sender_key_id = SenderKeyName::new(group_id, address)?;
            let message = group_decrypt(
                data.message.serialize(),
                &self.sender_key_store,
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
        let protocol = SignalProtocol::new(db);
        protocol.decode_message_data("".to_string()).unwrap();
    }
}
