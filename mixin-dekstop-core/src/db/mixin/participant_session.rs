use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, Sqlite};

pub struct ParticipantSessionDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(sqlx::FromRow)]
pub struct ParticipantSession {
    pub conversation_id: String,
    pub user_id: String,
    pub session_id: String,
    pub sent_to_server: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
    pub public_key: Option<String>,
}

impl ParticipantSessionDao {
    pub async fn replace_all(
        &self,
        conversation_id: &str,
        sessions: &[ParticipantSession],
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.0.begin().await?;

        sqlx::query("DELETE FROM participant_sessions WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&mut *tx)
            .await?;

        let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
            "INSERT OR REPLACE INTO participant_sessions (conversation_id, user_id, session_id, sent_to_server, created_at, public_key)",
        );
        qb.push_values(sessions.iter(), |mut b, session| {
            b.push_bind(&session.conversation_id)
                .push_bind(&session.user_id)
                .push_bind(&session.session_id)
                .push_bind(&session.sent_to_server)
                .push_bind(&session.created_at)
                .push_bind(&session.public_key);
        });
        qb.build().execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(())
    }
}
