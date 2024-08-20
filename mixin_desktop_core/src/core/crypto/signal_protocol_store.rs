use std::sync::Arc;

use async_trait::async_trait;
use libsignal_protocol::{
    error, Context, Direction, IdentityKey, IdentityKeyPair, IdentityKeyStore, PreKeyRecord,
    PreKeyStore, PrivateKey, ProtocolAddress, SenderKeyName, SenderKeyRecord, SenderKeyStore,
    SessionRecord, SessionStore, SignalProtocolError, SignedPreKeyRecord, SignedPreKeyStore,
};
use log::info;

use crate::db;
use crate::db::{Error, SignalDatabase};

#[derive(Clone)]
pub struct SignalProtocolStore {
    pub session_store: MixinSessionStore,
    pub identity_store: MixinIdentityKeyStore,
    pub pre_key_store: MixinPreKeyStore,
    pub signed_pre_key_store: MixinSignedPreKeyStore,
    pub sender_key_store: MixinSenderKeyStore,
}

impl SignalProtocolStore {
    pub fn new(db: Arc<SignalDatabase>, account_id: String) -> Self {
        SignalProtocolStore {
            session_store: MixinSessionStore { db: db.clone() },
            identity_store: MixinIdentityKeyStore {
                db: db.clone(),
                account_id,
            },
            pre_key_store: MixinPreKeyStore { db: db.clone() },
            signed_pre_key_store: MixinSignedPreKeyStore { db: db.clone() },
            sender_key_store: MixinSenderKeyStore { db: db.clone() },
        }
    }
}

#[derive(Clone)]
pub struct MixinSessionStore {
    db: Arc<SignalDatabase>,
}

impl MixinSessionStore {
    pub async fn delete_session(&self, address: &ProtocolAddress) -> anyhow::Result<()> {
        self.db
            .session_dao
            .delete_session(address.name(), address.device_id())
            .await
            .map_err(anyhow::Error::from)
    }

    pub async fn contain_user_session(&self, recipient_id: &str) -> anyhow::Result<bool> {
        self.db
            .session_dao
            .has_session(recipient_id)
            .await
            .map_err(anyhow::Error::from)
    }
}

#[async_trait(?Send)]
impl SessionStore for MixinSessionStore {
    async fn load_session(
        &self,
        address: &ProtocolAddress,
        _ctx: Context,
    ) -> error::Result<Option<SessionRecord>> {
        let result = self
            .db
            .session_dao
            .find_session(address.name(), address.device_id())
            .await?;
        if let Some(session) = result {
            Ok(Some(SessionRecord::deserialize(&session)?))
        } else {
            Ok(None)
        }
    }

    async fn store_session(
        &mut self,
        address: &ProtocolAddress,
        record: &SessionRecord,
        _ctx: Context,
    ) -> error::Result<()> {
        self.db
            .session_dao
            .save_session(address.name(), address.device_id(), record.serialize()?)
            .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct MixinIdentityKeyStore {
    db: Arc<SignalDatabase>,
    account_id: String,
}

impl MixinIdentityKeyStore {
    pub async fn delete_identity(&self, address: &ProtocolAddress) -> anyhow::Result<()> {
        self.db
            .identity_dao
            .delete_identity(address.name())
            .await
            .map_err(anyhow::Error::from)
    }
}

#[async_trait(?Send)]
impl IdentityKeyStore for MixinIdentityKeyStore {
    async fn get_identity_key_pair(&self, _ctx: Context) -> error::Result<IdentityKeyPair> {
        let identity = self.db.identity_dao.get_local_identity().await?;
        if let Some(i) = identity {
            Ok(IdentityKeyPair::new(
                IdentityKey::decode(&i.public_key)?,
                PrivateKey::deserialize(&i.private_key.ok_or(
                    SignalProtocolError::InternalError("identity private key not found"),
                )?)?,
            ))
        } else {
            Err(SignalProtocolError::NoKeyTypeIdentifier)
        }
    }

    async fn get_local_registration_id(&self, _ctx: Context) -> error::Result<u32> {
        let identity = self.db.identity_dao.get_local_identity().await?;
        identity
            .and_then(|i| i.registration_id)
            .ok_or(SignalProtocolError::NoKeyTypeIdentifier)
    }

    async fn save_identity(
        &mut self,
        address: &ProtocolAddress,
        identity: &IdentityKey,
        _ctx: Context,
    ) -> error::Result<bool> {
        let address = address.name();
        let exists = self
            .db
            .identity_dao
            .find_identity_by_address(address)
            .await?;
        match exists {
            Some(exits) => {
                let exits = IdentityKey::decode(&exits.public_key)?;
                if &exits != identity {
                    info!("replace exits identity: {}", address);
                    let identity = db::signal::identity::Identity {
                        address: address.to_string(),
                        registration_id: None,
                        public_key: identity.serialize().to_vec(),
                        private_key: None,
                        timestamp: chrono::Utc::now(),
                    };
                    self.db.identity_dao.save_identity(&identity).await?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            None => {
                info!("save new identity: {}", address);
                let identity = db::signal::identity::Identity {
                    address: address.to_string(),
                    registration_id: None,
                    public_key: identity.serialize().to_vec(),
                    private_key: None,
                    timestamp: chrono::Utc::now(),
                };
                self.db.identity_dao.save_identity(&identity).await?;
                Ok(true)
            }
        }
    }

    async fn is_trusted_identity(
        &self,
        address: &ProtocolAddress,
        identity: &IdentityKey,
        direction: Direction,
        ctx: Context,
    ) -> error::Result<bool> {
        let their_address = address.name();
        if self.account_id == their_address {
            let local = self.get_identity_key_pair(ctx).await?;
            return Ok(identity == local.identity_key());
        }
        match direction {
            Direction::Sending => {
                let find = self.get_identity(address, ctx).await?;
                match find {
                    Some(find) => Ok(identity == &find),
                    None => Ok(false),
                }
            }
            Direction::Receiving => Ok(true),
        }
    }

    async fn get_identity(
        &self,
        address: &ProtocolAddress,
        _ctx: Context,
    ) -> error::Result<Option<IdentityKey>> {
        let result = self
            .db
            .identity_dao
            .find_identity_by_address(address.name())
            .await?;
        if let Some(identity) = result {
            Ok(Some(IdentityKey::decode(&identity.public_key)?))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone)]
pub struct MixinPreKeyStore {
    db: Arc<SignalDatabase>,
}

impl From<Error> for SignalProtocolError {
    fn from(value: Error) -> Self {
        SignalProtocolError::InvalidState("db error", value.to_string())
    }
}

#[async_trait(?Send)]
impl PreKeyStore for MixinPreKeyStore {
    async fn get_pre_key(&self, prekey_id: u32, _ctx: Context) -> error::Result<PreKeyRecord> {
        let pre_key = self.db.pre_key_dao.find_pre_key(prekey_id).await?;
        pre_key
            .and_then(|bytes| PreKeyRecord::deserialize(&bytes).ok())
            .ok_or(SignalProtocolError::InvalidSignedPreKeyId)
    }

    async fn save_pre_key(
        &mut self,
        prekey_id: u32,
        record: &PreKeyRecord,
        _ctx: Context,
    ) -> error::Result<()> {
        self.db
            .pre_key_dao
            .save_pre_key(prekey_id, record.serialize()?)
            .await?;
        Ok(())
    }

    async fn remove_pre_key(&mut self, prekey_id: u32, _ctx: Context) -> error::Result<()> {
        self.db.pre_key_dao.delete_pre_key(prekey_id).await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct MixinSignedPreKeyStore {
    db: Arc<SignalDatabase>,
}

#[async_trait(?Send)]
impl SignedPreKeyStore for MixinSignedPreKeyStore {
    async fn get_signed_pre_key(
        &self,
        signed_prekey_id: u32,
        _ctx: Context,
    ) -> error::Result<SignedPreKeyRecord> {
        let signed_pre_key = self
            .db
            .signed_pre_key_dao
            .find_signed_pre_key(signed_prekey_id)
            .await?;
        signed_pre_key
            .and_then(|bytes| SignedPreKeyRecord::deserialize(&bytes).ok())
            .ok_or(SignalProtocolError::InvalidSignedPreKeyId)
    }

    async fn save_signed_pre_key(
        &mut self,
        signed_prekey_id: u32,
        record: &SignedPreKeyRecord,
        _ctx: Context,
    ) -> error::Result<()> {
        self.db
            .signed_pre_key_dao
            .save_signed_pre_key(signed_prekey_id, record.serialize()?)
            .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct MixinSenderKeyStore {
    db: Arc<SignalDatabase>,
}

impl MixinSenderKeyStore {
    pub async fn exists_sender_key(&self, group_id: &str, sender_id: &str) -> anyhow::Result<bool> {
        let result = self
            .db
            .sender_key_dao
            .has_sender_key(group_id, sender_id, 1)
            .await?;
        Ok(result)
    }
}

#[async_trait(?Send)]
impl SenderKeyStore for MixinSenderKeyStore {
    async fn store_sender_key(
        &mut self,
        sender_key_name: &SenderKeyName,
        record: &SenderKeyRecord,
        _ctx: Context,
    ) -> error::Result<()> {
        self.db
            .sender_key_dao
            .save_sender_key(
                sender_key_name.group_id().unwrap().as_str(),
                sender_key_name.sender_name().unwrap().as_str(),
                sender_key_name.sender_device_id().unwrap(),
                record.serialize().unwrap(),
            )
            .await?;
        Ok(())
    }

    async fn load_sender_key(
        &mut self,
        sender_key_name: &SenderKeyName,
        _ctx: Context,
    ) -> error::Result<Option<SenderKeyRecord>> {
        let result = self
            .db
            .sender_key_dao
            .find_sender_key(
                sender_key_name.group_id().unwrap().as_str(),
                sender_key_name.sender_name().unwrap().as_str(),
                sender_key_name.sender_device_id().unwrap(),
            )
            .await?;
        if let Some(record) = result {
            Ok(Some(SenderKeyRecord::deserialize(&record)?))
        } else {
            Ok(None)
        }
    }
}
