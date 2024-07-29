use sdk::Account;
use sqlx::{Error, Sqlite};

#[derive(Clone)]
pub struct AuthDao(pub(crate) sqlx::Pool<Sqlite>);

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct Auth {
    pub user_id: String,
    pub private_key: Vec<u8>,
    #[sqlx(json)]
    pub account: Account,
}

impl AuthDao {
    pub async fn find_all_auth(&self) -> Result<Vec<Auth>, Error> {
        sqlx::query_as::<_, Auth>("SELECT * FROM auths")
            .fetch_all(&self.0)
            .await
    }

    pub async fn remove_auth(&self, id: &str) -> Result<(), Error> {
        let _ = sqlx::query("DELETE FROM auths WHERE user_id = ?")
            .bind(id)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn save_auth(&self, auth: &Auth) -> anyhow::Result<()> {
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO auths (user_id, private_key, account) VALUES (?, ?, ?)",
        )
        .bind(&auth.user_id)
        .bind(&auth.private_key)
        .bind(serde_json::to_string(&auth.account)?)
        .execute(&self.0)
        .await?;
        Ok(())
    }
}
