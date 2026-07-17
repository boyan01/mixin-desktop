use std::sync::Arc;

use anyhow::{anyhow, Context};
use base64ct::{Base64, Encoding};
use libsignal_protocol::{
    create_sender_key_distribution_message, group_decrypt, group_encrypt, message_decrypt,
    message_encrypt, process_prekey_bundle, process_sender_key_distribution_message,
    CiphertextMessage, CiphertextMessageType, IdentityKey, PreKeyBundle, ProtocolAddress,
    PublicKey, SenderKeyDistributionMessage, SenderKeyName, SignalProtocolError,
};
use log::info;
use rand_core::OsRng;
use uuid::Uuid;

use sdk::message_category;

use crate::core::crypto::compose_message::ComposeMessageData;
use crate::core::crypto::signal_protocol_store::SignalProtocolStore;
use crate::db::SignalDatabase;

pub struct SignalProtocol {
    pub protocol_store: SignalProtocolStore,
    pub signal_database: Arc<SignalDatabase>,
    account_id: String,
}

pub const PRE_KEY_BATCH_SIZE: u32 = 700;
pub const MAX_VALUE: u32 = 0xFFFFFF;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(#[from] anyhow::Error);

impl From<SignalProtocolError> for Error {
    fn from(value: SignalProtocolError) -> Self {
        Self(anyhow!("Signal protocol error: {value}"))
    }
}

impl From<base64ct::Error> for Error {
    fn from(value: base64ct::Error) -> Self {
        Self(anyhow!("Base64 error: {value}"))
    }
}

type Result<T, E = Error> = anyhow::Result<T, E>;

impl SignalProtocol {
    pub fn new(db: Arc<SignalDatabase>, account_id: String) -> Self {
        SignalProtocol {
            signal_database: db.clone(),
            protocol_store: SignalProtocolStore::new(db, account_id.clone()),
            account_id,
        }
    }
}

impl SignalProtocol {
    pub fn device_id(session_id: Option<&str>) -> Result<u32, anyhow::Error> {
        let Some(session_id) = session_id.filter(|value| !value.is_empty()) else {
            return Ok(1);
        };
        let uuid = Uuid::parse_str(session_id)
            .with_context(|| format!("invalid session id: {}", session_id))?;
        let joined_bytes = uuid
            .as_bytes()
            .iter()
            .map(u8::to_string)
            .collect::<String>();
        Ok(dart_vm_string_hash(&joined_bytes))
    }

    pub fn convert_to_cipher_message(key_type: u8, cipher: &[u8]) -> Result<CiphertextMessage> {
        let message_type = match key_type {
            2 => CiphertextMessageType::Whisper,
            3 => CiphertextMessageType::PreKey,
            4 => CiphertextMessageType::SenderKey,
            5 => CiphertextMessageType::SenderKeyDistribution,
            _ => return Err(anyhow!("Invalid key type: {key_type}").into()),
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
            SignalProtocol::device_id(session_id)
                .with_context(|| format!("failed to get device id: {}", sender_id))?,
        );

        let context: libsignal_protocol::Context = None;

        let mut store = self.protocol_store.clone();
        let message = SignalProtocol::convert_to_cipher_message(key_type, &cipher)
            .with_context(|| "failed to convert to cipher message")?;
        info!(
            "decrypt message, category: {}, type: {}",
            category, key_type
        );
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
            .await
            .map_err(|e| anyhow!("signal key decrypt failed: {}, {}", key_type, e))?;
            self.process_group_session(
                group_id,
                address,
                &SenderKeyDistributionMessage::try_from(plain_text.as_ref()).map_err(|e| {
                    anyhow!("failed to convert to sender key distribution message: {e}")
                })?,
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
                    .await
                    .map_err(|e| {
                        anyhow!("Whisper/PreKey message decrypt failed: {}, {}", key_type, e)
                    })?;
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
                    .await
                    .map_err(|e| anyhow!("group decrypt failed: {}, {}", key_type, e))?;
                    Ok(message)
                }
                CiphertextMessageType::SenderKeyDistribution => {
                    Err(anyhow!("Not supported type: {key_type}").into())
                }
            }
        }
    }

    pub async fn process_group_session(
        &self,
        group_id: &str,
        address: ProtocolAddress,
        message: &SenderKeyDistributionMessage,
    ) -> Result<()> {
        let mut store = self.protocol_store.clone();
        process_sender_key_distribution_message(
            &SenderKeyName::new(group_id.to_string(), address.clone())
                .map_err(|e| anyhow!("Failed to create sender key name: {}", e))?,
            message,
            &mut store.sender_key_store,
            None,
        )
        .await
        .map_err(|e| anyhow!("Failed to process sender key distribution message: {}", e))?;
        Ok(())
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
                key.one_time_pre_key.key_id,
                PublicKey::deserialize(&Base64::decode_vec(
                    key.one_time_pre_key
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
    ) -> Result<Option<String>> {
        let mut store = self.protocol_store.clone();
        let remote_address = ProtocolAddress::new(rid.to_string(), did);
        let local_address = ProtocolAddress::new(self.account_id.clone(), 1);
        let message = create_sender_key_distribution_message(
            &SenderKeyName::new(cid.to_string(), local_address)?,
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
            return Ok(None);
        } else {
            cipher_message?
        };
        Ok(Some(encode_message_data(
            cipher_message.message_type() as u8,
            cipher_message.serialize().to_vec(),
            None,
        )))
    }

    pub async fn encrypt_group_message(&self, cid: &str, plaintext: &[u8]) -> Result<String> {
        let mut store = self.protocol_store.clone();
        let sender_key_name = SenderKeyName::new(
            cid.to_string(),
            ProtocolAddress::new(self.account_id.clone(), 1),
        )?;
        let cipher = group_encrypt(
            &mut store.sender_key_store,
            &sender_key_name,
            plaintext,
            &mut OsRng,
            None,
        )
        .await?;
        Ok(encode_message_data(
            CiphertextMessageType::SenderKey as u8,
            cipher,
            None,
        ))
    }

    pub async fn encrypt_session_message(
        &self,
        plaintext: &[u8],
        recipient_id: &str,
        session_id: &str,
        resend_message_id: Option<&str>,
    ) -> Result<String> {
        let cipher = self
            .encrypt_session(
                plaintext,
                recipient_id,
                SignalProtocol::device_id(Some(session_id))?,
            )
            .await?;
        Ok(encode_message_data(
            cipher.message_type() as u8,
            cipher.serialize().to_vec(),
            resend_message_id,
        ))
    }

    pub async fn contains_session(&self, recipient_id: &str, session_id: &str) -> Result<bool> {
        Ok(self
            .signal_database
            .session_dao
            .find_session(recipient_id, SignalProtocol::device_id(Some(session_id))?)
            .await
            .map_err(anyhow::Error::from)?
            .is_some())
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

fn encode_message_data(key_type: u8, cipher: Vec<u8>, resend_message_id: Option<&str>) -> String {
    ComposeMessageData {
        key_type,
        cipher,
        resend_message_id: resend_message_id.map(str::to_string),
    }
    .encode()
}

fn dart_vm_string_hash(value: &str) -> u32 {
    let mut hash = 0u32;
    for code_unit in value.encode_utf16() {
        hash = hash.wrapping_add(u32::from(code_unit));
        hash = hash.wrapping_add(hash << 10);
        hash ^= hash >> 6;
    }
    hash = hash.wrapping_add(hash << 3);
    hash ^= hash >> 11;
    hash = hash.wrapping_add(hash << 15) & ((1 << 30) - 1);
    if hash == 0 {
        1
    } else {
        hash
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use libsignal_protocol::{
        create_sender_key_distribution_message, ProtocolAddress, SenderKeyName,
    };
    use rand_core::OsRng;

    use crate::core::crypto::compose_message::ComposeMessageData;
    use crate::db::SignalDatabase;

    use super::{encode_message_data, SignalProtocol};

    #[test]
    fn device_id_matches_flutter_ulid_hash_code() {
        let fixtures = [
            ("00000000-0000-0000-0000-000000000000", 220_137_401),
            ("00112233-4455-6677-8899-aabbccddeeff", 973_594_656),
            ("ffffffff-ffff-ffff-ffff-ffffffffffff", 22_080_886),
            ("a5c8f9e0-1234-4abc-8def-1234567890ab", 818_292_461),
        ];

        for (session_id, expected) in fixtures {
            assert_eq!(
                SignalProtocol::device_id(Some(session_id)).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn device_id_uses_default_for_missing_session() {
        assert_eq!(SignalProtocol::device_id(None).unwrap(), 1);
        assert_eq!(SignalProtocol::device_id(Some("")).unwrap(), 1);
    }

    #[test]
    fn device_id_rejects_invalid_session() {
        assert!(SignalProtocol::device_id(Some("not-a-uuid")).is_err());
    }

    #[test]
    fn message_data_preserves_cipher_metadata() {
        let message_id = "00112233-4455-6677-8899-aabbccddeeff";
        let encoded = encode_message_data(3, vec![1, 2, 3, 4], Some(message_id));
        let decoded = ComposeMessageData::decode(&encoded).unwrap();

        assert_eq!(decoded.key_type, 3);
        assert_eq!(decoded.cipher, vec![1, 2, 3, 4]);
        assert_eq!(decoded.resend_message_id.as_deref(), Some(message_id));
    }

    #[tokio::test]
    async fn group_message_round_trips_with_local_sender_address() -> anyhow::Result<()> {
        let directory = tempfile::tempdir()?;
        let sender_database = Arc::new(
            SignalDatabase::connect_at(directory.path().join("sender.db"))
                .await
                .map_err(|err| anyhow::anyhow!(err.to_string()))?,
        );
        let receiver_database = Arc::new(
            SignalDatabase::connect_at(directory.path().join("receiver.db"))
                .await
                .map_err(|err| anyhow::anyhow!(err.to_string()))?,
        );
        let sender = SignalProtocol::new(sender_database, "sender-id".into());
        let receiver = SignalProtocol::new(receiver_database, "receiver-id".into());
        let sender_address = ProtocolAddress::new("sender-id".into(), 1);
        let sender_key_name = SenderKeyName::new("group-id".into(), sender_address.clone())
            .map_err(|err| anyhow::anyhow!(err.to_string()))?;
        let mut sender_store = sender.protocol_store.clone();
        let distribution = create_sender_key_distribution_message(
            &sender_key_name,
            &mut sender_store.sender_key_store,
            &mut OsRng,
            None,
        )
        .await
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
        receiver
            .process_group_session("group-id", sender_address, &distribution)
            .await
            .map_err(|err| anyhow::anyhow!(err.to_string()))?;

        let encoded = sender
            .encrypt_group_message("group-id", b"hello")
            .await
            .map_err(|err| anyhow::anyhow!(err.to_string()))?;
        let composed = ComposeMessageData::decode(&encoded)?;
        let plaintext = receiver
            .decrypt(
                "group-id",
                "sender-id",
                composed.key_type,
                composed.cipher,
                sdk::message_category::SIGNAL_TEXT,
                None,
            )
            .await
            .map_err(|err| anyhow::anyhow!(err.to_string()))?;

        assert_eq!(plaintext, b"hello");
        Ok(())
    }
}
