use base64ct::{Base64, Encoding};
use sdk::Account;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};

const GROUP: &str = "auth";
const AUTHS_KEY: &str = "auths";
const ACTIVE_USER_ID_KEY: &str = "active_user_id";

#[derive(Clone)]
pub struct AuthDao(pub(crate) Pool<Sqlite>);

#[derive(Debug, Clone)]
pub struct Auth {
    pub user_id: String,
    pub private_key: Vec<u8>,
    pub primary_session_id: Option<String>,
    pub account: Account,
}

#[derive(Deserialize, Serialize)]
struct StoredAuth {
    account: serde_json::Value,
    #[serde(rename = "privateKey")]
    private_key: String,
    #[serde(
        rename = "primarySessionId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    primary_session_id: Option<String>,
}

impl TryFrom<StoredAuth> for Auth {
    type Error = anyhow::Error;

    fn try_from(value: StoredAuth) -> Result<Self, Self::Error> {
        let mut account = value.account;
        if let Some(account) = account.as_object_mut() {
            for (key, default) in [
                ("app", serde_json::Value::Null),
                ("has_safe", false.into()),
                ("is_deactivated", false.into()),
                ("membership", serde_json::Value::Null),
                ("salt_base64", "".into()),
                ("spend_public_key", "".into()),
                ("tip_counter", 0.into()),
                ("tip_key_base64", "".into()),
            ] {
                account.entry(key).or_insert(default);
            }
            for key in [
                "transfer_confirmation_threshold",
                "transfer_notification_threshold",
            ] {
                if let Some(number) = account.get(key).and_then(|value| value.as_f64()) {
                    account.insert(key.to_string(), (number as i64).into());
                }
            }
        }
        let account: Account = serde_json::from_value(account)?;
        Ok(Self {
            user_id: account.user_id.clone(),
            private_key: Base64::decode_vec(&value.private_key)?,
            primary_session_id: value.primary_session_id,
            account,
        })
    }
}

impl From<&Auth> for StoredAuth {
    fn from(value: &Auth) -> Self {
        Self {
            account: serde_json::to_value(&value.account).expect("account must serialize"),
            private_key: Base64::encode_string(&value.private_key),
            primary_session_id: value.primary_session_id.clone(),
        }
    }
}

impl AuthDao {
    pub async fn find_all_auth(&self) -> anyhow::Result<Vec<Auth>> {
        let stored = sqlx::query_scalar::<_, String>(
            "SELECT value FROM properties WHERE \"group\" = ? AND \"key\" = ?",
        )
        .bind(GROUP)
        .bind(AUTHS_KEY)
        .fetch_optional(&self.0)
        .await?
        .unwrap_or_else(|| "[]".to_string());
        let active_user_id = sqlx::query_scalar::<_, String>(
            "SELECT value FROM properties WHERE \"group\" = ? AND \"key\" = ?",
        )
        .bind(GROUP)
        .bind(ACTIVE_USER_ID_KEY)
        .fetch_optional(&self.0)
        .await?;

        let mut auths = serde_json::from_str::<Vec<StoredAuth>>(&stored)?
            .into_iter()
            .map(Auth::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(active_user_id) = active_user_id {
            if let Some(index) = auths.iter().position(|auth| auth.user_id == active_user_id) {
                auths.rotate_left(index);
            }
        }
        Ok(auths)
    }

    pub async fn remove_auth(&self, id: &str) -> anyhow::Result<()> {
        let mut auths = self.find_all_auth().await?;
        auths.retain(|auth| auth.user_id != id);
        let active_user_id = auths.first().map(|auth| auth.user_id.as_str());
        self.write_auths(&auths, active_user_id).await
    }

    pub async fn save_auth(&self, auth: &Auth) -> anyhow::Result<()> {
        let mut auths = self.find_all_auth().await?;
        auths.retain(|item| item.user_id != auth.user_id);
        auths.insert(0, auth.clone());
        self.write_auths(&auths, Some(&auth.user_id)).await
    }

    async fn write_auths(
        &self,
        auths: &[Auth],
        active_user_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let stored = auths.iter().map(StoredAuth::from).collect::<Vec<_>>();
        let value = serde_json::to_string(&stored)?;
        let mut transaction = self.0.begin().await?;
        sqlx::query(
            r#"INSERT INTO properties ("group", "key", value) VALUES (?, ?, ?)
               ON CONFLICT("key", "group") DO UPDATE SET value = excluded.value"#,
        )
        .bind(GROUP)
        .bind(AUTHS_KEY)
        .bind(value)
        .execute(&mut *transaction)
        .await?;
        if let Some(active_user_id) = active_user_id {
            sqlx::query(
                r#"INSERT INTO properties ("group", "key", value) VALUES (?, ?, ?)
                   ON CONFLICT("key", "group") DO UPDATE SET value = excluded.value"#,
            )
            .bind(GROUP)
            .bind(ACTIVE_USER_ID_KEY)
            .bind(active_user_id)
            .execute(&mut *transaction)
            .await?;
        } else {
            sqlx::query("DELETE FROM properties WHERE \"group\" = ? AND \"key\" = ?")
                .bind(GROUP)
                .bind(ACTIVE_USER_ID_KEY)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }
}
