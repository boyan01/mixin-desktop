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
    pub async fn clear_for_sign_out(&self, session_id: &str) -> Result<(), Error> {
        let mut transaction = self.0.begin().await?;
        sqlx::query("DELETE FROM participant_session WHERE session_id = ?")
            .bind(session_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("UPDATE participant_session SET sent_to_server = NULL")
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

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

        if sessions.is_empty() {
            tx.commit().await?;
            return Ok(());
        }

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
        if sessions.is_empty() {
            return Ok(());
        }
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
            "INSERT OR REPLACE INTO participant_session \
             (conversation_id, user_id, session_id, sent_to_server) VALUES (?, ?, ?, ?)",
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

    pub async fn not_sent_participant_sessions(
        &self,
        conversation_id: &str,
        current_session_id: &str,
    ) -> Result<Vec<ParticipantSession>, Error> {
        let sessions = sqlx::query_as::<_, ParticipantSession>(
            "SELECT p.* FROM participant_session AS p \
             LEFT JOIN users AS u ON p.user_id = u.user_id \
             WHERE p.conversation_id = ? AND p.session_id != ? \
             AND u.app_id IS NULL AND p.sent_to_server IS NULL",
        )
        .bind(conversation_id)
        .bind(current_session_id)
        .fetch_all(&self.0)
        .await?;
        Ok(sessions)
    }

    pub async fn participant_session_key_without_self(
        &self,
        conversation_id: &str,
        account_id: &str,
    ) -> Result<Option<ParticipantSession>, Error> {
        let session = sqlx::query_as::<_, ParticipantSession>(
            "SELECT * FROM participant_session \
             WHERE conversation_id = ? AND user_id != ? \
             AND public_key IS NOT NULL AND public_key != '' LIMIT 1",
        )
        .bind(conversation_id)
        .bind(account_id)
        .fetch_optional(&self.0)
        .await?;
        Ok(session)
    }

    pub async fn other_participant_session_key(
        &self,
        conversation_id: &str,
        account_id: &str,
        current_session_id: &str,
    ) -> Result<Option<ParticipantSession>, Error> {
        let session = sqlx::query_as::<_, ParticipantSession>(
            "SELECT * FROM participant_session \
             WHERE conversation_id = ? AND user_id = ? AND session_id != ? \
             AND public_key IS NOT NULL AND public_key != '' \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(conversation_id)
        .bind(account_id)
        .bind(current_session_id)
        .fetch_optional(&self.0)
        .await?;
        Ok(session)
    }

    pub async fn update_status(
        &self,
        conversation_id: &str,
        user_id: &str,
        session_id: &str,
        status: i32,
    ) -> Result<(), Error> {
        sqlx::query(
            "UPDATE participant_session SET sent_to_server = ? \
             WHERE conversation_id = ? AND user_id = ? AND session_id = ?",
        )
        .bind(status)
        .bind(conversation_id)
        .bind(user_id)
        .bind(session_id)
        .execute(&self.0)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::MixinDatabase;

    fn session(
        conversation_id: &str,
        user_id: &str,
        session_id: &str,
        status: Option<i32>,
        public_key: Option<&str>,
    ) -> ParticipantSession {
        ParticipantSession {
            conversation_id: conversation_id.into(),
            user_id: user_id.into(),
            session_id: session_id.into(),
            sent_to_server: status,
            created_at: Some(Utc::now()),
            public_key: public_key.map(str::to_string),
        }
    }

    #[tokio::test]
    async fn sign_out_removes_current_session_and_resets_sender_key_state() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let dao = database.participant_session_dao;
        dao.insert_session("conversation", "me", "current", 1)
            .await
            .unwrap();
        dao.insert_session("conversation", "other", "other-session", 1)
            .await
            .unwrap();

        dao.clear_for_sign_out("current").await.unwrap();

        let sessions = dao.get_participant_sessions("conversation").await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "other-session");
        assert_eq!(sessions[0].sent_to_server, None);
    }

    #[tokio::test]
    async fn selects_only_sessions_requiring_sender_keys() -> Result<(), Box<dyn std::error::Error>>
    {
        let directory = tempfile::tempdir()?;
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db")).await?;
        sqlx::query("INSERT INTO users (user_id, identity_number, app_id) VALUES (?, ?, ?)")
            .bind("app-user")
            .bind("1")
            .bind("app-id")
            .execute(&database.participant_session_dao.0)
            .await?;
        database
            .participant_session_dao
            .replace_all(
                "group",
                &[
                    session("group", "me", "current", None, Some("current-key")),
                    session("group", "me", "other", None, Some("other-key")),
                    session("group", "peer", "peer-session", None, Some("peer-key")),
                    session("group", "app-user", "app-session", None, Some("app-key")),
                    session("group", "sent", "sent-session", Some(1), Some("sent-key")),
                    session("group", "no-key", "no-key-session", None, None),
                ],
            )
            .await?;

        let mut selected = database
            .participant_session_dao
            .not_sent_participant_sessions("group", "current")
            .await?
            .into_iter()
            .map(|session| session.session_id)
            .collect::<Vec<_>>();
        selected.sort();
        assert_eq!(selected, ["no-key-session", "other", "peer-session"]);

        database
            .participant_session_dao
            .update_status("group", "peer", "peer-session", 1)
            .await?;
        let peer = database
            .participant_session_dao
            .get_participant_sessions("group")
            .await?
            .into_iter()
            .find(|session| session.session_id == "peer-session")
            .unwrap();
        assert_eq!(peer.sent_to_server, Some(1));
        assert_eq!(peer.public_key.as_deref(), Some("peer-key"));
        Ok(())
    }

    #[tokio::test]
    async fn empty_replacement_clears_existing_sessions() -> Result<(), Box<dyn std::error::Error>>
    {
        let directory = tempfile::tempdir()?;
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db")).await?;
        database
            .participant_session_dao
            .replace_all(
                "conversation",
                &[session(
                    "conversation",
                    "user",
                    "session",
                    None,
                    Some("key"),
                )],
            )
            .await?;

        database
            .participant_session_dao
            .replace_all("conversation", &[])
            .await?;

        assert!(database
            .participant_session_dao
            .get_participant_sessions("conversation")
            .await?
            .is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn selects_encrypted_protocol_session_keys() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db")).await?;
        database
            .participant_session_dao
            .replace_all(
                "contact",
                &[
                    session("contact", "me", "current", None, Some("current-key")),
                    session("contact", "me", "other", None, Some("other-key")),
                    session("contact", "peer", "peer", None, Some("peer-key")),
                    session("contact", "empty", "empty", None, None),
                ],
            )
            .await?;

        let peer = database
            .participant_session_dao
            .participant_session_key_without_self("contact", "me")
            .await?
            .unwrap();
        assert_eq!(peer.user_id, "peer");
        assert_eq!(peer.public_key.as_deref(), Some("peer-key"));

        let own = database
            .participant_session_dao
            .other_participant_session_key("contact", "me", "current")
            .await?
            .unwrap();
        assert_eq!(own.session_id, "other");
        assert_eq!(own.public_key.as_deref(), Some("other-key"));
        Ok(())
    }
}
