use std::hash::{DefaultHasher, Hash, Hasher};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::anyhow;
use base64ct::{Base64, Encoding};
use libsignal_protocol::{
    create_sender_key_distribution_message, group_decrypt, message_decrypt, message_encrypt,
    process_prekey_bundle, CiphertextMessage, CiphertextMessageType, IdentityKey, PreKeyBundle,
    ProtocolAddress, PublicKey, SenderKeyName, SignalProtocolError,
};
use rand_core::OsRng;
use ulid::Ulid;

use sdk::message_category;

use crate::core::crypto::compose_message::ComposeMessageData;
use crate::core::crypto::signal_protocol_store::SignalProtocolStore;
use crate::db::SignalDatabase;

pub struct SignalProtocol {
    pub protocol_store: SignalProtocolStore,
}

type Result<T, E = Box<dyn std::error::Error>> = anyhow::Result<T, E>;

impl SignalProtocol {
    pub fn new(db: Arc<SignalDatabase>, account_id: String) -> Self {
        SignalProtocol {
            protocol_store: SignalProtocolStore::new(db, account_id),
        }
    }
}

impl SignalProtocol {
    pub fn device_id(session_id: Option<&str>) -> Result<u32, anyhow::Error> {
        if let Some(session_id) = session_id {
            let mut hash = DefaultHasher::new();
            Ulid::from_str(session_id)?.hash(&mut hash);
            let code = hash.finish();
            Ok(code as u32)
        } else {
            Ok(1)
        }
    }

    pub fn convert_to_cipher_message(
        key_type: u8,
        cipher: &[u8],
    ) -> anyhow::Result<CiphertextMessage, SignalProtocolError> {
        let message_type = match key_type {
            2 => CiphertextMessageType::Whisper,
            3 => CiphertextMessageType::PreKey,
            4 => CiphertextMessageType::SenderKey,
            5 => CiphertextMessageType::SenderKeyDistribution,
            _ => return Err(SignalProtocolError::InvalidCiphertext),
        };
        let message = match message_type {
            CiphertextMessageType::Whisper => CiphertextMessage::SignalMessage(cipher.try_into()?),
            CiphertextMessageType::PreKey => {
                CiphertextMessage::PreKeySignalMessage(cipher.try_into()?)
            }
            CiphertextMessageType::SenderKey => {
                CiphertextMessage::SenderKeyMessage(cipher.try_into()?)
            }
            CiphertextMessageType::SenderKeyDistribution => {
                CiphertextMessage::SenderKeyDistributionMessage(cipher.try_into()?)
            }
        };
        Ok(message)
    }

    pub async fn decrypt(
        &self,
        group_id: &str,
        sender_id: &str,
        key_type: u8,
        cipher: Vec<u8>,
        category: &str,
        session_id: Option<&str>,
    ) -> Result<Vec<u8>> {
        let address = ProtocolAddress::new(
            sender_id.to_string(),
            SignalProtocol::device_id(session_id)?,
        );

        let context: libsignal_protocol::Context = None;

        let mut store = self.protocol_store.clone();
        let message = SignalProtocol::convert_to_cipher_message(key_type, &cipher)?;
        if category == message_category::SIGNAL_KEY {
            let plain_text = message_decrypt(
                &message,
                &address,
                &mut store.session_store,
                &mut store.identity_store,
                &mut store.pre_key_store,
                &mut store.signed_pre_key_store,
                &mut OsRng,
                context,
            )
            .await?;
            Ok(plain_text)
        } else {
            match message.message_type() {
                CiphertextMessageType::Whisper | CiphertextMessageType::PreKey => {
                    let plain_text = message_decrypt(
                        &message,
                        &address,
                        &mut store.session_store,
                        &mut store.identity_store,
                        &mut store.pre_key_store,
                        &mut store.signed_pre_key_store,
                        &mut OsRng,
                        context,
                    )
                    .await?;
                    Ok(plain_text)
                }
                CiphertextMessageType::SenderKey => {
                    let sender_key_id = SenderKeyName::new(group_id.to_string(), address)?;
                    let message = group_decrypt(
                        &cipher,
                        &mut store.sender_key_store,
                        &sender_key_id,
                        context,
                    )
                    .await?;
                    Ok(message)
                }
                CiphertextMessageType::SenderKeyDistribution => {
                    Err(anyhow!("Not supported type: {key_type}").into())
                }
            }
        }
    }

    pub async fn process_session(&self, recipient_id: &str, key: &sdk::SignalKey) -> Result<()> {
        let mut store = self.protocol_store.clone();
        let address = ProtocolAddress::new(
            recipient_id.to_string(),
            SignalProtocol::device_id(Some(&key.session_id))?,
        );
        let pre_key_bundle = PreKeyBundle::new(
            key.registration_id,
            SignalProtocol::device_id(Some(&key.session_id))?,
            Some((
                key.ont_time_pre_key.key_id,
                PublicKey::deserialize(&Base64::decode_vec(
                    key.ont_time_pre_key
                        .pub_key
                        .as_ref()
                        .ok_or(anyhow!("Failed to deserialize public key"))?,
                )?)
                .map_err(|_| anyhow!("Failed to deserialize public key"))?,
            )),
            key.signed_pre_key.key_id,
            PublicKey::deserialize(&Base64::decode_vec(
                key.signed_pre_key
                    .pub_key
                    .as_ref()
                    .ok_or(anyhow!("Failed to deserialize public key"))?,
            )?)
            .map_err(|e| anyhow!("Failed to deserialize public key: {e}"))?,
            Base64::decode_vec(&key.signed_pre_key.signature)?,
            IdentityKey::decode(&Base64::decode_vec(&key.identity_key)?)
                .map_err(|e| anyhow!("Failed to decode identity key: {}", e))?,
        )
        .map_err(|e| anyhow!("Failed to create prekey bundle: {}", e))?;

        let result = process_prekey_bundle(
            &address,
            &mut store.session_store,
            &mut store.identity_store,
            &pre_key_bundle,
            &mut OsRng,
            None,
        )
        .await;
        if let Err(SignalProtocolError::UntrustedIdentity(address)) = result {
            store.identity_store.delete_identity(&address).await?;
            process_prekey_bundle(
                &address,
                &mut store.session_store,
                &mut store.identity_store,
                &pre_key_bundle,
                &mut OsRng,
                None,
            )
            .await?;
        } else {
            result?;
        }
        Ok(())
    }

    pub async fn encrypt_sender_key(
        &self,
        cid: &str,
        rid: &str,
        did: u32,
    ) -> Result<(String, bool)> {
        let mut store = self.protocol_store.clone();
        let remote_address = ProtocolAddress::new(rid.to_string(), did);
        let message = create_sender_key_distribution_message(
            &SenderKeyName::new(cid.to_string(), remote_address.clone())?,
            &mut store.sender_key_store,
            &mut OsRng,
            None,
        )
        .await?;

        let cipher_message = self.encrypt_session(message.serialized(), rid, did).await;
        let cipher_message = if let Err(SignalProtocolError::UntrustedIdentity(_)) = cipher_message
        {
            store
                .identity_store
                .delete_identity(&remote_address)
                .await?;
            store.session_store.delete_session(&remote_address).await?;
            return Ok(("".to_string(), false));
        } else {
            cipher_message?
        };
        let data = ComposeMessageData {
            key_type: cipher_message.message_type() as u8,
            cipher: cipher_message.serialize().to_vec(),
            resend_message_id: None,
        };

        Ok((data.encode(), true))
    }

    pub async fn encrypt_session(
        &self,
        content: &[u8],
        dest: &str,
        did: u32,
    ) -> Result<CiphertextMessage, SignalProtocolError> {
        let remote_address = ProtocolAddress::new(dest.to_string(), did);
        let mut store = self.protocol_store.clone();
        message_encrypt(
            content,
            &remote_address,
            &mut store.session_store,
            &mut store.identity_store,
            None,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_signal_protocol() {}
}
