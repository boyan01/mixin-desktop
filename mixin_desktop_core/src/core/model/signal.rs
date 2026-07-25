use std::sync::Arc;

use anyhow::{anyhow, Result};
use base64ct::{Base64, Encoding};
use libsignal_protocol::{
    IdentityKeyPair, IdentityKeyStore, KeyPair, PreKeyRecord, SignedPreKeyRecord, SignedPreKeyStore,
};
use rand_core::OsRng;

use sdk::{Client, OneTimePreKey, SignalKeyRequest, SignedPreKey};

use crate::core::crypto::signal_protocol::{SignalProtocol, MAX_VALUE, PRE_KEY_BATCH_SIZE};
use crate::db::signal::pre_key::PreKey;
use crate::db::SignalDatabase;

#[derive(Clone)]
pub struct SignalService {
    pub(crate) signal_protocol: Arc<SignalProtocol>,
    pub(crate) signal_database: Arc<SignalDatabase>,
    client: Arc<Client>,
}

impl SignalService {
    pub fn new(
        protocol: Arc<SignalProtocol>,
        database: Arc<SignalDatabase>,
        client: Arc<Client>,
    ) -> Self {
        Self {
            signal_protocol: protocol,
            signal_database: database,
            client,
        }
    }

    pub(crate) async fn key_count(&self) -> Result<u32> {
        Ok(self
            .client
            .account_api
            .get_signal_key_count()
            .await?
            .one_time_pre_keys_count)
    }

    pub(crate) async fn push_keys(&self) -> Result<()> {
        let keys = self
            .generate_keys()
            .await
            .map_err(|error| anyhow!("failed to generate signal keys: {error}"))?;
        self.client.account_api.push_signal_keys(&keys).await?;
        self.signal_database
            .crypto_key_value
            .set_has_push_signal_keys(true)
            .await?;
        Ok(())
    }

    pub async fn generate_pre_keys(&self) -> Result<Vec<PreKeyRecord>> {
        let crypto_key_value = &self.signal_database.crypto_key_value;

        let mut records = Vec::new();
        let mut pre_keys = Vec::new();
        let pre_key_offset = crypto_key_value.next_pre_key_id();
        for index in pre_key_offset..PRE_KEY_BATCH_SIZE + pre_key_offset {
            let pre_key_record =
                PreKeyRecord::new(index & MAX_VALUE, &KeyPair::generate(&mut OsRng));

            let pre_key = PreKey {
                prekey_id: index & MAX_VALUE,
                record: pre_key_record
                    .serialize()
                    .map_err(|e| anyhow!("failed to serialize pre key: {e}"))?,
            };
            pre_keys.push(pre_key);
            records.push(pre_key_record);
        }

        self.signal_database
            .pre_key_dao
            .insert_pre_key_list(&pre_keys)
            .await?;

        crypto_key_value
            .set_next_pre_key_id((pre_key_offset + PRE_KEY_BATCH_SIZE + 1) & MAX_VALUE)
            .await?;

        Ok(records)
    }

    pub async fn generate_signed_pre_key(
        &self,
        identity_key_pair: &IdentityKeyPair,
    ) -> Result<SignedPreKeyRecord> {
        let crypto_key_value = &self.signal_database.crypto_key_value;
        let signed_pre_key_id = crypto_key_value.next_signed_pre_key_id();
        let key_pair = KeyPair::generate(&mut OsRng);
        let signature = identity_key_pair
            .private_key()
            .calculate_signature(&key_pair.public_key.serialize(), &mut OsRng)
            .map_err(|e| anyhow!("failed to calculate signature: {e}"))?;

        let record = SignedPreKeyRecord::new(
            signed_pre_key_id,
            chrono::Utc::now().timestamp_millis() as u64,
            &key_pair,
            &signature,
        );

        let mut store = self.signal_protocol.protocol_store.clone();
        store
            .signed_pre_key_store
            .save_signed_pre_key(signed_pre_key_id, &record, None)
            .await
            .map_err(|e| anyhow!("failed to save signed pre key: {e}"))?;
        crypto_key_value
            .set_next_signed_pre_key_id((signed_pre_key_id + 1) % MAX_VALUE)
            .await?;

        Ok(record)
    }

    pub async fn generate_keys(&self) -> Result<SignalKeyRequest, Box<dyn std::error::Error>> {
        let identity = self
            .signal_protocol
            .protocol_store
            .identity_store
            .get_identity_key_pair(None)
            .await
            .map_err(|e| anyhow!("failed to get identity key pair: {e}"))?;

        let one_time_pre_keys = self.generate_pre_keys().await?;
        let signed_pre_key = self.generate_signed_pre_key(&identity).await?;

        let mut pks = vec![];
        for pk in one_time_pre_keys {
            pks.push(OneTimePreKey {
                key_id: pk.id()?,
                pub_key: Some(Base64::encode_string(&pk.public_key()?.serialize())),
            });
        }

        Ok(SignalKeyRequest {
            identity_key: Base64::encode_string(&identity.public_key().serialize()),
            signed_pre_key: SignedPreKey {
                key_id: signed_pre_key.id()?,
                pub_key: Some(Base64::encode_string(
                    &signed_pre_key.public_key()?.serialize(),
                )),
                signature: Base64::encode_string(&signed_pre_key.signature()?),
            },
            one_time_pre_keys: pks,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use libsignal_protocol::{IdentityKeyStore, SignedPreKeyRecord};

    use super::*;
    use crate::core::user_agent::generate_user_agent;

    #[tokio::test]
    async fn signed_pre_key_advances_persisted_counter() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let database = Arc::new(
            SignalDatabase::connect_at(directory.path().join("signal.db"))
                .await
                .map_err(|err| anyhow!(err.to_string()))?,
        );
        database.init(7, None).await?;
        let protocol = Arc::new(SignalProtocol::new(database.clone(), "account-id".into()));
        let service = SignalService::new(
            protocol.clone(),
            database.clone(),
            Arc::new(Client::new_with_user_agent(
                sdk::Credential::None,
                Some(generate_user_agent()),
            )),
        );
        let identity = protocol
            .protocol_store
            .identity_store
            .get_identity_key_pair(None)
            .await
            .map_err(|err| anyhow!("get identity key pair: {err}"))?;
        let current = database.crypto_key_value.next_signed_pre_key_id();

        let record: SignedPreKeyRecord = service.generate_signed_pre_key(&identity).await?;

        assert_eq!(
            record
                .id()
                .map_err(|err| anyhow!("get signed pre-key id: {err}"))?,
            current
        );
        assert_eq!(
            database.crypto_key_value.next_signed_pre_key_id(),
            (current + 1) % MAX_VALUE
        );
        drop(service);
        drop(protocol);
        drop(database);

        let reopened = SignalDatabase::connect_at(directory.path().join("signal.db"))
            .await
            .map_err(|err| anyhow!(err.to_string()))?;
        assert_eq!(
            reopened.crypto_key_value.next_signed_pre_key_id(),
            (current + 1) % MAX_VALUE
        );
        Ok(())
    }
}
