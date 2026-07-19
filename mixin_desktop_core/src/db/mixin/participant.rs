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
    #[sqlx(try_from = "crate::db::datetime::DatabaseDateTime")]
    pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub struct ParticipantListItem {
    pub user_id: String,
    pub role: Option<String>,
    #[sqlx(try_from = "crate::db::datetime::DatabaseDateTime")]
    pub created_at: DateTime<Utc>,
    pub identity_number: String,
    pub full_name: String,
    pub avatar_url: String,
    pub biography: String,
    pub is_verified: bool,
    pub is_bot: bool,
    pub membership: Option<String>,
    pub relationship: String,
}

impl ParticipantDao {
    pub async fn list_items(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ParticipantListItem>, Error> {
        Ok(sqlx::query_as::<_, ParticipantListItem>(
            r#"SELECT participant.user_id, participant.role, participant.created_at,
                      COALESCE(user.identity_number, '') AS identity_number,
                      COALESCE(user.full_name, '') AS full_name,
                      COALESCE(user.avatar_url, '') AS avatar_url,
                      COALESCE(user.biography, '') AS biography,
                      COALESCE(user.is_verified, FALSE) AS is_verified,
                      COALESCE(user.app_id, '') != '' AS is_bot,
                      user.membership AS membership,
                      COALESCE(user.relationship, '') AS relationship
               FROM participants participant
               LEFT JOIN users user ON user.user_id = participant.user_id
               WHERE participant.conversation_id = ?
               ORDER BY participant.created_at ASC, participant.user_id ASC"#,
        )
        .bind(conversation_id)
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn replace_all(
        &self,
        conversation_id: &str,
        participants: &[Participant],
    ) -> Result<(), Error> {
        let mut tx = self.0.begin_with("BEGIN IMMEDIATE").await?;

        sqlx::query("DELETE FROM participants WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&mut *tx)
            .await?;

        if participants.is_empty() {
            tx.commit().await?;
            return Ok(());
        }

        let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
            "INSERT OR REPLACE INTO participants (conversation_id, user_id, role, created_at)",
        );
        qb.push_values(participants.iter(), |mut b, participant| {
            b.push_bind(&participant.conversation_id)
                .push_bind(&participant.user_id)
                .push_bind(&participant.role)
                .push_bind(participant.created_at.timestamp_millis());
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
        .bind(participant.created_at.timestamp_millis())
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
        let sql = "SELECT p.conversation_id FROM participants p, conversations c \
                   WHERE p.user_id = ? AND p.conversation_id = c.conversation_id \
                   AND c.status = 2 LIMIT 1";
        let result = sqlx::query_scalar::<_, String>(sql)
            .bind(uid)
            .fetch_optional(&self.0)
            .await?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::Participant;
    use crate::db::MixinDatabase;

    #[tokio::test]
    async fn accepts_empty_participant_replacement() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();

        database
            .participant_dao
            .replace_all("conversation", &[])
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn finds_joined_conversation_with_positional_parameter() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO conversations (conversation_id, created_at, status) VALUES (?, ?, ?)",
        )
        .bind("conversation")
        .bind(Utc::now().timestamp_millis())
        .bind(2)
        .execute(&database.participant_dao.0)
        .await
        .unwrap();
        database
            .participant_dao
            .insert_participant(&Participant {
                conversation_id: "conversation".to_string(),
                user_id: "user".to_string(),
                role: None,
                created_at: Utc::now(),
            })
            .await
            .unwrap();

        assert_eq!(
            database
                .participant_dao
                .find_any_joined_conversation_id("user")
                .await
                .unwrap()
                .as_deref(),
            Some("conversation")
        );
    }
}
