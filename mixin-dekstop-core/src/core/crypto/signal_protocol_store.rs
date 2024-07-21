use std::sync::Arc;

use libsignal_protocol::{
    error, Context, Direction, IdentityKey, IdentityKeyPair, IdentityKeyStore, PreKeyRecord,
    PreKeyStore, ProtocolAddress, SenderKeyName, SenderKeyRecord, SenderKeyStore, SessionRecord,
    SessionStore, SignalProtocolError, SignedPreKeyRecord, SignedPreKeyStore,
};

use crate::db;
use crate::db::{Error, SignalDatabase};

struct SignalProtocolStore {
    session_store: MixinSessionStore,
    identity_store: MixinIdentityKeyStore,
    pre_key_store: MixinPreKeyStore,
    signed_pre_key_store: MixinSignedPreKeyStore,
    sender_key_store: MixinSenderKeyStore,
}

impl SignalProtocolStore {
    pub fn new(db: Arc<SignalDatabase>) -> Self {
        SignalProtocolStore {
            session_store: MixinSessionStore { db: db.clone() },
            identity_store: MixinIdentityKeyStore { db: db.clone() },
            pre_key_store: MixinPreKeyStore { db: db.clone() },
            signed_pre_key_store: MixinSignedPreKeyStore { db: db.clone() },
            sender_key_store: MixinSenderKeyStore { db: db.clone() },
        }
    }
}

struct MixinSessionStore {
    db: Arc<SignalDatabase>,
}

impl SessionStore for MixinSessionStore {
    async fn load_session(
        &self,
        address: &ProtocolAddress,
        ctx: Context,
    ) -> error::Result<Option<SessionRecord>> {
        todo!()
    }

    async fn store_session(
        &mut self,
        address: &ProtocolAddress,
        record: &SessionRecord,
        ctx: Context,
    ) -> error::Result<()> {
        todo!()
    }
}

struct MixinIdentityKeyStore {
    db: Arc<SignalDatabase>,
}

impl IdentityKeyStore for MixinIdentityKeyStore {
    async fn get_identity_key_pair(&self, ctx: Context) -> error::Result<IdentityKeyPair> {
        todo!()
    }

    async fn get_local_registration_id(&self, ctx: Context) -> error::Result<u32> {
        todo!()
    }

    async fn save_identity(
        &mut self,
        address: &ProtocolAddress,
        identity: &IdentityKey,
        ctx: Context,
    ) -> error::Result<bool> {
        todo!()
    }

    async fn is_trusted_identity(
        &self,
        address: &ProtocolAddress,
        identity: &IdentityKey,
        direction: Direction,
        ctx: Context,
    ) -> error::Result<bool> {
        todo!()
    }

    async fn get_identity(
        &self,
        address: &ProtocolAddress,
        ctx: Context,
    ) -> error::Result<Option<IdentityKey>> {
        todo!()
    }
}

struct MixinPreKeyStore {
    db: Arc<SignalDatabase>,
}

impl From<db::Error> for SignalProtocolError {
    fn from(value: Error) -> Self {
        SignalProtocolError::InvalidState("db error", value.to_string())
    }
}

impl PreKeyStore for MixinPreKeyStore {
    async fn get_pre_key(&self, prekey_id: u32, ctx: Context) -> error::Result<PreKeyRecord> {
        let pre_key = self.db.pre_key_dao.find_pre_key(prekey_id).await?;
        pre_key
            .map(|r| PreKeyRecord::deserialize(&r.record)?)
            .ok_or(SignalProtocolError::InvalidPreKeyId)
    }

    async fn save_pre_key(
        &mut self,
        prekey_id: u32,
        record: &PreKeyRecord,
        ctx: Context,
    ) -> error::Result<()> {
        self.db
            .pre_key_dao
            .save_pre_key(prekey_id, record.serialize()?)
            .await?;
        Ok(())
    }

    async fn remove_pre_key(&mut self, prekey_id: u32, ctx: Context) -> error::Result<()> {
        self.db.pre_key_dao.delete_pre_key(prekey_id).await?;
        Ok(())
    }
}

struct MixinSignedPreKeyStore {
    db: Arc<SignalDatabase>,
}

impl SignedPreKeyStore for MixinSignedPreKeyStore {
    async fn get_signed_pre_key(
        &self,
        signed_prekey_id: u32,
        ctx: Context,
    ) -> error::Result<SignedPreKeyRecord> {
        let signed_pre_key = self
            .db
            .signed_pre_key_dao
            .find_signed_pre_key(signed_prekey_id)
            .await?;
        signed_pre_key
            .map(|r| SignedPreKeyRecord::deserialize(&r.record)?)
            .ok_or(SignalProtocolError::InvalidSignedPreKeyId)
    }

    async fn save_signed_pre_key(
        &mut self,
        signed_prekey_id: u32,
        record: &SignedPreKeyRecord,
        ctx: Context,
    ) -> error::Result<()> {
        self.db
            .signed_pre_key_dao
            .save_signed_pre_key(signed_prekey_id, record.serialize()?)
            .await?;
        Ok(())
    }
}

struct MixinSenderKeyStore {
    db: Arc<SignalDatabase>,
}

impl SenderKeyStore for MixinSenderKeyStore {
    async fn store_sender_key(
        &mut self,
        sender_key_name: &SenderKeyName,
        record: &SenderKeyRecord,
        ctx: Context,
    ) -> error::Result<()> {
        todo!()
    }

    async fn load_sender_key(
        &mut self,
        sender_key_name: &SenderKeyName,
        ctx: Context,
    ) -> error::Result<Option<SenderKeyRecord>> {
        todo!()
    }
}
