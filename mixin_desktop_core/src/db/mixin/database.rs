use std::error::Error;
use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

use crate::db::mixin::app::AppDao;
use crate::db::mixin::asset::AssetDao;
use crate::db::mixin::circle::CircleDao;
use crate::db::mixin::circle_conversation_dao::CircleConversationDao;
use crate::db::mixin::conversation::ConversationDao;
use crate::db::mixin::expired_message::ExpiredMessageDao;
use crate::db::mixin::favorite_app::FavoriteAppDao;
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
}

impl MixinDatabase {
    pub async fn new(identity_number: String) -> Result<Self, Box<dyn Error>> {
        let path = crate::db::path::account_database_path(&identity_number, "mixin.db")?;
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
        let migrator = sqlx::migrate!("./src/db/mixin/migrations");
        migrator.run(&pool).await?;
        Ok(MixinDatabase {
            user_dao: UserDao(pool.clone()),
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
        })
    }
}
