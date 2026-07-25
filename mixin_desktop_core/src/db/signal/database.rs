use std::error::Error;
use std::path::Path;

use anyhow::anyhow;
use libsignal_protocol::{IdentityKeyPair, PrivateKey};
use rand_core::OsRng;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

use crate::db;
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
        super::migration::MIGRATOR.migrate(&pool).await?;

        let database = SignalDatabase {
            pre_key_dao: PreKeyDao(pool.clone()),
            signed_pre_key_dao: SignedPreKeyDao(pool.clone()),
            session_dao: SessionDao(pool.clone()),
            sender_key_dao: SenderKeyDao(pool.clone()),
            identity_dao: IdentityDao(pool.clone()),
            crypto_key_value: CryptoKeyValue::new(pool.clone()),
            ratchet_sender_key_dao: RatchetSenderKeyDao(pool.clone()),
        };
        database.crypto_key_value.init().await?;
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
        let mut transaction = self.pre_key_dao.0.begin_with("BEGIN IMMEDIATE").await?;
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
    async fn persists_and_reads_pre_keys_after_connect() -> Result<(), Box<dyn Error>> {
        let _ = TestLogger::init(LevelFilter::Info, Config::default());
        let directory = tempfile::tempdir()?;
        let db = SignalDatabase::connect_at(directory.path().join("signal.db")).await?;
        let mut csprng = OsRng;
        let key_pair = KeyPair::generate(&mut csprng);
        let serialized = PreKeyRecord::new(0, &key_pair).serialize().unwrap();
        db.pre_key_dao
            .save_pre_key(0, serialized.clone())
            .await
            .expect("save prekey error");
        let stored = db
            .pre_key_dao
            .find_pre_key(0)
            .await
            .expect("get prekey error")
            .expect("missing prekey");

        assert_eq!(stored, serialized);
        Ok(())
    }

    #[tokio::test]
    async fn clear_removes_all_signal_state() -> Result<(), Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let db = SignalDatabase::connect_at(directory.path().join("signal.db")).await?;
        let pool = &db.pre_key_dao.0;
        for statement in [
            "INSERT INTO sender_keys VALUES ('group', 'sender', X'01')",
            "INSERT INTO identities (address, registration_id, public_key, private_key, next_prekey_id, timestamp) VALUES ('address', 1, X'01', X'02', NULL, 1)",
            "INSERT INTO prekeys (prekey_id, record) VALUES (1, X'01')",
            "INSERT INTO signed_prekeys (prekey_id, record, timestamp) VALUES (1, X'01', 1)",
            "INSERT INTO sessions (address, device, record, timestamp) VALUES ('address', 1, X'01', 1)",
            "INSERT INTO ratchet_sender_keys VALUES ('group', 'sender', 'SENT', 'message', '2026-07-16T00:00:00Z')",
            "INSERT INTO properties VALUES ('has_push_signal_keys', 'true')",
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

    #[tokio::test]
    async fn v1_upgrade_copies_legacy_app_counters_into_signal_properties(
    ) -> Result<(), Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let account_directory = directory.path().join("7000");
        tokio::fs::create_dir(&account_directory).await?;
        let path = account_directory.join("signal.db");
        let pool = SqlitePoolOptions::new()
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true),
            )
            .await?;
        sqlx::raw_sql(include_str!("schema.sql"))
            .execute(&pool)
            .await?;
        sqlx::query("DROP TABLE properties").execute(&pool).await?;
        sqlx::query("PRAGMA user_version = 1")
            .execute(&pool)
            .await?;
        pool.close().await;

        let app_pool = SqlitePoolOptions::new()
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(directory.path().join("app.db"))
                    .create_if_missing(true),
            )
            .await?;
        sqlx::raw_sql(
            r#"CREATE TABLE properties (
                "key" TEXT NOT NULL,
                "group" TEXT NOT NULL,
                "value" TEXT NOT NULL,
                PRIMARY KEY ("key", "group")
            );
            INSERT INTO properties VALUES ('next_pre_key_id', 'crypto:7000', '123');
            INSERT INTO properties VALUES ('next_signed_pre_key_id', 'crypto:7000', '456');
            INSERT INTO properties VALUES ('has_push_signal_keys', 'crypto:7000', 'true');"#,
        )
        .execute(&app_pool)
        .await?;
        app_pool.close().await;

        let database = SignalDatabase::connect_at(&path).await?;
        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&database.pre_key_dao.0)
            .await?;
        let properties: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'properties'",
        )
        .fetch_one(&database.pre_key_dao.0)
        .await?;

        assert_eq!(version, super::super::migration::SCHEMA_VERSION);
        assert_eq!(properties, 1);
        assert_eq!(database.crypto_key_value.next_pre_key_id(), 123);
        assert_eq!(database.crypto_key_value.next_signed_pre_key_id(), 456);
        assert!(database.crypto_key_value.has_push_signal_keys());
        Ok(())
    }
}
