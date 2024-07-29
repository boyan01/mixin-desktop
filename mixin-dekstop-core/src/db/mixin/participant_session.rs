use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, Sqlite};

use crate::db::Error;

#[derive(Clone)]
pub struct ParticipantSessionDao(pub(crate) sqlx::Pool<Sqlite>);

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

        sqlx::query("DELETE FROM participant_session WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&mut *tx)
            .await?;

        let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
            "INSERT OR REPLACE INTO participant_session (conversation_id, user_id, session_id, sent_to_server, created_at, public_key)",
        );
        qb.push_values(sessions.iter(), |mut b, session| {
            b.push_bind(&session.conversation_id)
                .push_bind(&session.user_id)
                .push_bind(&session.session_id)
                .push_bind(session.sent_to_server)
                .push_bind(session.created_at)
                .push_bind(&session.public_key);
        });
        qb.build().execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn remove_participant(
        &self,
        conversation_id: &str,
        user_id: &str,
    ) -> Result<(), sqlx::Error> {
        let _ = sqlx::query(
            "DELETE FROM participant_session WHERE conversation_id = ? AND user_id = ?",
        )
        .bind(conversation_id)
        .bind(user_id)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn clear_status(&self, cid: &str) -> Result<(), Error> {
        let _ = sqlx::query(
            "UPDATE participant_session SET sent_to_server = null WHERE conversation_id = ?",
        )
        .bind(cid)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn insert(&self, cid: &str, sessions: &[sdk::UserSession]) -> Result<(), Error> {
        let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "INSERT OR REPLACE INTO participant_session (conversation_id, user_id, session_id, public_key)",
        );
        query_builder.push_values(sessions.iter(), |mut b, session| {
            b.push_bind(cid)
                .push_bind(&session.user_id)
                .push_bind(&session.session_id)
                .push_bind(&session.public_key);
        });
        query_builder.build().execute(&self.0).await?;
        Ok(())
    }

    pub async fn insert_session(
        &self,
        cid: &str,
        uid: &str,
        sid: &str,
        sent_to_server: i32,
    ) -> Result<(), Error> {
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO participant_session (conversation_id, user_id, session_id, sent_to_server)",
        )
        .bind(cid)
        .bind(uid)
        .bind(sid)
        .bind(sent_to_server)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn get_participant_sessions(
        &self,
        cid: &str,
    ) -> Result<Vec<ParticipantSession>, Error> {
        let result = sqlx::query_as::<_, ParticipantSession>(
            "SELECT * FROM participant_session WHERE conversation_id = ?",
        )
        .bind(cid)
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }
}
