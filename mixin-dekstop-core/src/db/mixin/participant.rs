use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, Sqlite};

use crate::db::Error;

#[derive(Clone)]
pub struct ParticipantDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(sqlx::FromRow)]
pub struct Participant {
    pub conversation_id: String,
    pub user_id: String,
    pub role: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ParticipantDao {
    pub async fn replace_all(
        &self,
        conversation_id: &str,
        participants: &[Participant],
    ) -> Result<(), Error> {
        let mut tx = self.0.begin().await?;

        sqlx::query("DELETE FROM participants WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&mut *tx)
            .await?;

        let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
            "INSERT OR REPLACE INTO participants (conversation_id, user_id, role, created_at)",
        );
        qb.push_values(participants.iter(), |mut b, participant| {
            b.push_bind(&participant.conversation_id)
                .push_bind(&participant.user_id)
                .push_bind(&participant.role)
                .push_bind(participant.created_at);
        });
        qb.build().execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn find_participant_by_id(
        &self,
        conversation_id: &str,
        user_id: &str,
    ) -> Result<Option<Participant>, Error> {
        let result = sqlx::query_as::<_, Participant>(
            "SELECT * FROM participants WHERE conversation_id = ? AND user_id = ?",
        )
        .bind(conversation_id)
        .bind(user_id)
        .fetch_optional(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn insert_participant(&self, participant: &Participant) -> Result<(), Error> {
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO participants (conversation_id, user_id, role, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(&participant.conversation_id)
        .bind(&participant.user_id)
        .bind(&participant.role)
        .bind(participant.created_at)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn update_participant_role(
        &self,
        cid: &str,
        pid: &str,
        role: &Option<String>,
    ) -> Result<(), Error> {
        let _ = sqlx::query(
            "UPDATE participants SET role = ? WHERE conversation_id = ? AND user_id = ?",
        )
        .bind(role)
        .bind(cid)
        .bind(pid)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn remove_participant(&self, cid: &str, pid: &str) -> Result<(), Error> {
        let _ = sqlx::query("DELETE FROM participants WHERE conversation_id = ? AND user_id = ?")
            .bind(cid)
            .bind(pid)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn find_any_joined_conversation_id(
        &self,
        uid: &str,
    ) -> Result<Option<String>, Error> {
        let sql = "SELECT p.conversation_id FROM participants p, conversations c WHERE p.user_id = :userId AND p.conversation_id = c.conversation_id AND c.status = 2 LIMIT 1";
        let result = sqlx::query_scalar::<_, String>(sql)
            .bind(uid)
            .fetch_optional(&self.0)
            .await?;
        Ok(result)
    }
}
