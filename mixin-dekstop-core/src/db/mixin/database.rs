use std::error::Error;

use sqlx::{Pool, Sqlite};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::db::mixin::app::AppDao;
use crate::db::mixin::circle::CircleDao;
use crate::db::mixin::circle_conversation_dao::CircleConversationDao;
use crate::db::mixin::conversation::ConversationDao;
use crate::db::mixin::job::JobDao;
use crate::db::mixin::message::MessageDao;
use crate::db::mixin::message_history::MessageHistoryDao;
use crate::db::mixin::message_mention::MessageMentionDao;
use crate::db::mixin::participant::ParticipantDao;
use crate::db::mixin::participant_session::ParticipantSessionDao;
use crate::db::mixin::pin_message::PinMessageDao;
use crate::db::mixin::safe_snapshot::SafeSnapshotDao;
use crate::db::mixin::snapshot::SnapshotDao;
use crate::db::mixin::sticker::StickerDao;
use crate::db::mixin::user::UserDao;

pub struct MixinDatabase {
    pub(crate) pool: Pool<Sqlite>,
    pub user_dao: UserDao,
    pub message_dao: MessageDao,
    pub message_mention_dao: MessageMentionDao,
    pub sticker_dao: StickerDao,
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
}

impl MixinDatabase {
    pub async fn new(identity_number: String) -> Result<Self, Box<dyn Error>> {
        let pool = SqlitePoolOptions::new()
            .connect_with(
                SqliteConnectOptions::new()
                    .filename("mixin.db")
                    .create_if_missing(true),
            )
            .await?;
        let migrator = sqlx::migrate!("./src/db/mixin/migrations");
        migrator.run(&pool).await?;
        Ok(MixinDatabase {
            pool: pool.clone(),
            user_dao: UserDao(pool.clone()),
            message_dao: MessageDao(pool.clone()),
            message_mention_dao: MessageMentionDao(pool.clone()),
            sticker_dao: StickerDao(pool.clone()),
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
        })
    }
}

impl MixinDatabase {}

struct User {
    id: String,
    identity_number: String,
    relationship: Option<String>,
    full_name: Option<String>,
    avatar_url: Option<String>,
    phone: Option<String>,
    is_verified: Option<bool>,
    created_at: Option<i32>,
    mute_until: Option<i32>,
    has_pin: Option<i32>,
    biography: Option<String>,
    is_scam: Option<i32>,
    code_url: Option<String>,
    code_id: Option<String>,
    is_deactivated: Option<bool>,
}
