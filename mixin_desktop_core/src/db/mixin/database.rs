use std::error::Error;
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

use crate::db::mixin::app::AppDao;
use crate::db::mixin::asset::AssetDao;
use crate::db::mixin::circle::CircleDao;
use crate::db::mixin::circle_conversation_dao::CircleConversationDao;
use crate::db::mixin::conversation::ConversationDao;
use crate::db::mixin::expired_message::ExpiredMessageDao;
use crate::db::mixin::favorite_app::FavoriteAppDao;
use crate::db::mixin::fiat::FiatDao;
use crate::db::mixin::flood_message::FloodMessageDao;
use crate::db::mixin::inscription::InscriptionDao;
use crate::db::mixin::job::JobDao;
use crate::db::mixin::message::MessageDao;
use crate::db::mixin::message_fts::MessageFtsDao;
use crate::db::mixin::message_history::MessageHistoryDao;
use crate::db::mixin::message_mention::MessageMentionDao;
use crate::db::mixin::offset::OffsetDao;
use crate::db::mixin::participant::ParticipantDao;
use crate::db::mixin::participant_session::ParticipantSessionDao;
use crate::db::mixin::pin_message::PinMessageDao;
use crate::db::mixin::safe_snapshot::SafeSnapshotDao;
use crate::db::mixin::snapshot::SnapshotDao;
use crate::db::mixin::sticker::StickerDao;
use crate::db::mixin::transcript_message::TranscriptMessageDao;
use crate::db::mixin::user::UserDao;

pub(crate) const MARK_LIMIT: usize = 999;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const MIN_READ_ONLY_SCHEMA_VERSION: i64 = 27;

#[derive(Clone)]
pub struct MixinDatabase {
    pub user_dao: UserDao,
    pub message_dao: MessageDao,
    pub message_fts_dao: MessageFtsDao,
    pub message_mention_dao: MessageMentionDao,
    pub offset_dao: OffsetDao,
    pub asset_dao: AssetDao,
    pub inscription_dao: InscriptionDao,
    pub sticker_dao: StickerDao,
    pub transcript_message_dao: TranscriptMessageDao,
    pub job_dao: JobDao,
    pub message_history_dao: MessageHistoryDao,
    pub conversation_dao: ConversationDao,
    pub participant_dao: ParticipantDao,
    pub participant_session_dao: ParticipantSessionDao,
    pub circle_dao: CircleDao,
    pub circle_conversation_dao: CircleConversationDao,
    pub snapshot_dao: SnapshotDao,
    pub safe_snapshot_dao: SafeSnapshotDao,
    pub app_dao: AppDao,
    pub pin_message_dao: PinMessageDao,
    pub flood_message_dao: FloodMessageDao,
    pub expired_message_dao: ExpiredMessageDao,
    pub favorite_app_dao: FavoriteAppDao,
    pub fiat_dao: FiatDao,
}

impl MixinDatabase {
    pub async fn new(identity_number: String) -> Result<Self, Box<dyn Error>> {
        let path = crate::db::path::account_database_path(&identity_number, "mixin.db")?;
        Self::connect_at(path).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn connect_at(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let path = path.as_ref();
        crate::db::path::create_parent_directory(path).await?;
        let fts_path = path.with_file_name("fts.db");
        crate::db::fts::migrate(&fts_path).await?;
        let attached_fts_path = fts_path.to_string_lossy().into_owned();
        let pool = SqlitePoolOptions::new()
            .after_connect(move |connection, _| {
                let attached_fts_path = attached_fts_path.clone();
                Box::pin(async move {
                    crate::db::query("ATTACH DATABASE ? AS fts")
                        .bind(attached_fts_path)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
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
        Ok(Self::from_pool(pool))
    }

    /// wasm: open through the Worker bridge and attach the FTS database on
    /// the single pooled connection; OPFS directories and storage-mode
    /// pragmas are owned by the JS open handler.
    #[cfg(target_arch = "wasm32")]
    pub async fn connect_at(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let path = path.as_ref();
        crate::db::path::create_parent_directory(path).await?;
        let fts_path = path.with_file_name("fts.db");
        crate::db::fts::migrate(&fts_path).await?;
        let pool = crate::db::Pool::open(&path.to_string_lossy()).await?;
        crate::db::query("ATTACH DATABASE ? AS fts")
            .bind(fts_path.to_string_lossy().into_owned())
            .execute(&pool)
            .await?;
        super::migration::MIGRATOR.migrate(&pool).await?;
        Ok(Self::from_pool(pool))
    }

    /// Read-only inspection of a local database file (MCP and backup tooling).
    /// The browser has no local file to open read-only, so this stays native.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) async fn connect_read_only_at(
        path: impl AsRef<Path>,
    ) -> anyhow::Result<(Self, i64)> {
        let path = path.as_ref();
        if !path.is_file() {
            anyhow::bail!("Mixin database does not exist: {}", path.display());
        }
        let fts_path = path.with_file_name("fts.db");
        if !fts_path.is_file() {
            anyhow::bail!("Mixin FTS database does not exist: {}", fts_path.display());
        }
        let mut fts_url = url::Url::from_file_path(&fts_path).map_err(|_| {
            anyhow::anyhow!("invalid Mixin FTS database path: {}", fts_path.display())
        })?;
        fts_url.query_pairs_mut().append_pair("mode", "ro");
        let fts_uri: String = fts_url.into();
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .after_connect(move |connection, _| {
                let fts_uri = fts_uri.clone();
                Box::pin(async move {
                    crate::db::query("ATTACH DATABASE ? AS fts")
                        .bind(fts_uri)
                        .execute(&mut *connection)
                        .await?;
                    crate::db::query("PRAGMA query_only = ON")
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(path)
                    .read_only(true)
                    .create_if_missing(false)
                    .busy_timeout(Duration::from_secs(2)),
            )
            .await?;
        let schema_version: i64 = crate::db::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await?;
        if schema_version < MIN_READ_ONLY_SCHEMA_VERSION {
            pool.close().await;
            anyhow::bail!(
                "Mixin database schema {schema_version} is too old; version {MIN_READ_ONLY_SCHEMA_VERSION} or newer is required"
            );
        }
        Ok((Self::from_pool(pool), schema_version))
    }

    fn from_pool(pool: crate::db::Pool) -> Self {
        MixinDatabase {
            user_dao: UserDao::new(pool.clone()),
            message_dao: MessageDao(pool.clone()),
            message_fts_dao: MessageFtsDao(pool.clone()),
            message_mention_dao: MessageMentionDao(pool.clone()),
            offset_dao: OffsetDao::new(pool.clone()),
            asset_dao: AssetDao(pool.clone()),
            inscription_dao: InscriptionDao(pool.clone()),
            sticker_dao: StickerDao(pool.clone()),
            transcript_message_dao: TranscriptMessageDao(pool.clone()),
            job_dao: JobDao(pool.clone()),
            message_history_dao: MessageHistoryDao(pool.clone()),
            conversation_dao: ConversationDao(pool.clone()),
            participant_dao: ParticipantDao(pool.clone()),
            participant_session_dao: ParticipantSessionDao(pool.clone()),
            circle_dao: CircleDao(pool.clone()),
            circle_conversation_dao: CircleConversationDao(pool.clone()),
            snapshot_dao: SnapshotDao(pool.clone()),
            safe_snapshot_dao: SafeSnapshotDao(pool.clone()),
            app_dao: AppDao(pool.clone()),
            pin_message_dao: PinMessageDao(pool.clone()),
            flood_message_dao: FloodMessageDao::new(pool.clone()),
            expired_message_dao: ExpiredMessageDao(pool.clone()),
            favorite_app_dao: FavoriteAppDao(pool.clone()),
            fiat_dao: FiatDao(pool),
        }
    }

    pub async fn close(&self) {
        self.user_dao.0.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn read_only_connection_accepts_newer_schema_and_rejects_writes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mixin.db");
        let writable = MixinDatabase::connect_at(&path).await.unwrap();
        crate::db::query("PRAGMA user_version = 30")
            .execute(&writable.user_dao.0)
            .await
            .unwrap();
        writable.close().await;

        let (database, version) = MixinDatabase::connect_read_only_at(&path).await.unwrap();

        assert_eq!(version, 30);
        let fts_version: i64 = crate::db::query_scalar("PRAGMA fts.user_version")
            .fetch_one(&database.message_fts_dao.0)
            .await
            .unwrap();
        assert_eq!(fts_version, 1);
        let error = database
            .circle_dao
            .update_name("circle", "changed")
            .await
            .unwrap_err();
        assert!(error.to_string().contains("readonly"), "{error:?}");
    }

    #[tokio::test]
    async fn read_only_connection_rejects_incompatible_schema() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mixin.db");
        let writable = MixinDatabase::connect_at(&path).await.unwrap();
        crate::db::query("PRAGMA user_version = 26")
            .execute(&writable.user_dao.0)
            .await
            .unwrap();
        writable.close().await;

        let error = MixinDatabase::connect_read_only_at(&path)
            .await
            .err()
            .unwrap();

        assert!(error.to_string().contains("schema 26"), "{error:?}");
    }
}
