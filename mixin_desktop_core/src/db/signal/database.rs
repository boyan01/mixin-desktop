use std::error::Error;
use std::path::Path;

use anyhow::anyhow;
use libsignal_protocol::{IdentityKeyPair, PrivateKey};
use rand_core::OsRng;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

use crate::db;
use crate::db::key_value::KeyValue;
use crate::db::signal::crypto_store::CryptoKeyValue;
use crate::db::signal::identity::{Identity, IdentityDao};
use crate::db::signal::pre_key::PreKeyDao;
use crate::db::signal::ratchet_sender_key::RatchetSenderKeyDao;
use crate::db::signal::sender_key::SenderKeyDao;
use crate::db::signal::session::SessionDao;
use crate::db::signal::signed_pre_key::SignedPreKeyDao;

pub struct SignalDatabase {
    pub pre_key_dao: PreKeyDao,
    pub signed_pre_key_dao: SignedPreKeyDao,
    pub session_dao: SessionDao,
    pub sender_key_dao: SenderKeyDao,
    pub identity_dao: IdentityDao,
    pub crypto_key_value: CryptoKeyValue,
    pub ratchet_sender_key_dao: RatchetSenderKeyDao,
}

impl SignalDatabase {
    pub async fn connect(identity_number: String) -> Result<Self, Box<dyn Error>> {
        let path = crate::db::path::account_database_path(&identity_number, "signal.db")?;
        Self::connect_at(path).await
    }

    pub async fn connect_at(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let path = path.as_ref();
        crate::db::path::create_parent_directory(path).await?;
        let pool = SqlitePoolOptions::new()
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(path)
                    .journal_mode(SqliteJournalMode::Wal)
                    .synchronous(SqliteSynchronous::Normal)
                    .foreign_keys(true)
                    .create_if_missing(true),
            )
            .await?;
        let migrator = sqlx::migrate!("./src/db/signal/migrations");
        migrator.run(&pool).await?;

        let key_value = KeyValue(pool.clone());

        let database = SignalDatabase {
            pre_key_dao: PreKeyDao(pool.clone()),
            signed_pre_key_dao: SignedPreKeyDao(pool.clone()),
            session_dao: SessionDao(pool.clone()),
            sender_key_dao: SenderKeyDao(pool.clone()),
            identity_dao: IdentityDao(pool.clone()),
            crypto_key_value: CryptoKeyValue::new(key_value),
            ratchet_sender_key_dao: RatchetSenderKeyDao(pool.clone()),
        };
        database.crypto_key_value.init().await;
        Ok(database)
    }

    pub async fn init(
        &self,
        registration_id: u32,
        private_key: Option<&[u8]>,
    ) -> Result<(), db::Error> {
        let key = if let Some(private_key) = private_key {
            let private_key = PrivateKey::deserialize(private_key)
                .map_err(|e| anyhow!("deserialize private key error: {}", e))?;
            IdentityKeyPair::try_from(private_key).map_err(|e| anyhow!("key pair error: {}", e))?
        } else {
            IdentityKeyPair::generate(&mut OsRng)
        };
        self.identity_dao
            .save_identity(&Identity {
                address: "-1".to_string(),
                registration_id: Some(registration_id),
                public_key: key.public_key().serialize().to_vec(),
                private_key: Some(key.private_key().serialize()),
                timestamp: chrono::Utc::now(),
            })
            .await
    }

    pub async fn clear(&self) -> Result<(), db::Error> {
        let mut transaction = self.pre_key_dao.0.begin().await?;
        for table in [
            "sender_keys",
            "identities",
            "prekeys",
            "signed_prekeys",
            "sessions",
            "ratchet_sender_keys",
            "properties",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {table}")))
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn close(&self) {
        self.pre_key_dao.0.close().await;
    }
}

#[cfg(test)]
mod tests {
    use libsignal_protocol::{KeyPair, PreKeyRecord};
    use log::LevelFilter;
    use rand_core::OsRng;
    use simplelog::{Config, TestLogger};

    use super::*;

    #[tokio::test]
    async fn test_connect() -> Result<(), Box<dyn Error>> {
        let _ = TestLogger::init(LevelFilter::Info, Config::default());
        let directory = tempfile::tempdir()?;
        let db = SignalDatabase::connect_at(directory.path().join("signal.db")).await?;
        let mut csprng = OsRng;
        let key_pair = KeyPair::generate(&mut csprng);
        db.pre_key_dao
            .save_pre_key(0, PreKeyRecord::new(0, &key_pair).serialize().unwrap())
            .await
            .expect("save prekey error");
        let a = db
            .pre_key_dao
            .find_pre_key(0)
            .await
            .expect("get prekey error");
        println!("{:x?}", a.unwrap());
        Ok(())
    }

    #[tokio::test]
    async fn clear_removes_all_signal_state() -> Result<(), Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let db = SignalDatabase::connect_at(directory.path().join("signal.db")).await?;
        let pool = &db.pre_key_dao.0;
        for statement in [
            "INSERT INTO sender_keys VALUES ('group', 'sender', X'01')",
            "INSERT INTO identities VALUES ('address', 1, X'01', X'02', NULL, 1)",
            "INSERT INTO prekeys VALUES (1, X'01')",
            "INSERT INTO signed_prekeys VALUES (1, X'01', 1)",
            "INSERT INTO sessions VALUES ('address', 1, X'01', 1)",
            "INSERT INTO ratchet_sender_keys VALUES ('group', 'sender', 'SENT', 'message', '2026-07-16T00:00:00Z')",
            "INSERT INTO properties VALUES ('key', 'group', 'value')",
        ] {
            sqlx::query(statement).execute(pool).await?;
        }

        db.clear().await?;

        for table in [
            "sender_keys",
            "identities",
            "prekeys",
            "signed_prekeys",
            "sessions",
            "ratchet_sender_keys",
            "properties",
        ] {
            let count: i64 =
                sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
                    .fetch_one(pool)
                    .await?;
            assert_eq!(count, 0, "{table} was not cleared");
        }
        Ok(())
    }
}
