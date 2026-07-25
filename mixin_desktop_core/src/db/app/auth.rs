use anyhow::{anyhow, bail};
use base64ct::{Base64, Encoding};
use log::{info, warn};
use ring::signature::{Ed25519KeyPair, KeyPair};
use sdk::Account;
use serde::{Deserialize, Serialize};

use super::{PropertyDao, PropertyGroup};

const AUTHS_KEY: &str = "auths";
const ACTIVE_USER_ID_KEY: &str = "active_user_id";
const AUTH_MIGRATION_KEY: &str = "auth_migration";

#[derive(Clone)]
pub struct AuthDao(pub(crate) PropertyDao);

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
            private_key: decode_private_key(&value.private_key)?,
            primary_session_id: value
                .primary_session_id
                .filter(|session_id| !session_id.trim().is_empty()),
            account,
        })
    }
}

impl From<&Auth> for StoredAuth {
    fn from(value: &Auth) -> Self {
        Self {
            account: serde_json::to_value(&value.account).expect("account must serialize"),
            private_key: encode_private_key(&value.private_key),
            primary_session_id: value.primary_session_id.clone(),
        }
    }
}

fn decode_private_key(value: &str) -> anyhow::Result<Vec<u8>> {
    let private_key = Base64::decode_vec(value)?;
    let seed = match private_key.len() {
        32 => private_key.as_slice(),
        64 => &private_key[..32],
        length => bail!("invalid session private key length: {length}"),
    };
    let key_pair = Ed25519KeyPair::from_seed_unchecked(seed)
        .map_err(|_| anyhow!("invalid session private key seed"))?;
    if private_key.len() == 64 && key_pair.public_key().as_ref() != &private_key[32..] {
        bail!("session private key public key does not match its seed");
    }
    Ok(seed.to_vec())
}

fn encode_private_key(seed: &[u8]) -> String {
    let key_pair = Ed25519KeyPair::from_seed_unchecked(seed)
        .expect("session private key must be a valid seed");
    let mut private_key = Vec::with_capacity(64);
    private_key.extend_from_slice(seed);
    private_key.extend_from_slice(key_pair.public_key().as_ref());
    Base64::encode_string(&private_key)
}

impl AuthDao {
    pub async fn find_all_auth(&self) -> anyhow::Result<Vec<Auth>> {
        self.migrate_legacy_auths().await?;
        let stored = self
            .0
            .get(PropertyGroup::Auth, AUTHS_KEY)
            .await?
            .unwrap_or_else(|| "[]".to_string());
        let active_user_id = self.0.get(PropertyGroup::Auth, ACTIVE_USER_ID_KEY).await?;

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

    async fn migrate_legacy_auths(&self) -> anyhow::Result<()> {
        let migrated = self.0.get(PropertyGroup::Auth, AUTH_MIGRATION_KEY).await?;
        if migrated.is_some() {
            return Ok(());
        }

        match super::legacy_hive::read_settings().await {
            Ok(Some(settings)) => {
                let mut updates = settings
                    .iter()
                    .map(|(key, value)| {
                        (PropertyGroup::Setting, key.as_str(), Some(value.as_str()))
                    })
                    .collect::<Vec<_>>();
                updates.push((
                    PropertyGroup::Setting,
                    "settingHasMigratedFromHive",
                    Some("true"),
                ));
                self.0.update(&updates).await?;
            }
            Ok(None) => {}
            Err(error) => warn!("failed to migrate legacy settings: {error}"),
        }

        match super::legacy_hive::read_auths().await {
            Ok(legacy_auths) => {
                let mut auths = Vec::with_capacity(legacy_auths.len());
                for legacy_auth in legacy_auths {
                    let stored = StoredAuth {
                        account: legacy_auth.account,
                        private_key: legacy_auth.private_key,
                        primary_session_id: None,
                    };
                    let mut auth = match Auth::try_from(stored) {
                        Ok(auth) => auth,
                        Err(error) => {
                            warn!("failed to migrate legacy authorization: {error}");
                            continue;
                        }
                    };
                    match super::legacy_hive::read_primary_session_id(&auth.account.identity_number)
                        .await
                    {
                        Ok(primary_session_id) => {
                            auth.primary_session_id = primary_session_id;
                        }
                        Err(error) => {
                            warn!("failed to migrate legacy primary session: {error}");
                        }
                    }
                    auths.push(auth);
                }
                if let Some(active_auth) = auths.last() {
                    let active_user_id = active_auth.user_id.clone();
                    self.migrate_legacy_signal_state(active_auth).await?;
                    self.write_auths(&auths, Some(&active_user_id)).await?;
                }
            }
            Err(error) => warn!("failed to migrate legacy authorizations: {error}"),
        }

        self.0
            .set(PropertyGroup::Auth, AUTH_MIGRATION_KEY, "true")
            .await?;
        Ok(())
    }

    async fn migrate_legacy_signal_state(&self, auth: &Auth) -> anyhow::Result<()> {
        let identity_number = &auth.account.identity_number;
        if super::legacy_hive::migrate_signal_database(identity_number).await? {
            info!("migrated legacy signal database for {identity_number}");
        }

        let state = super::legacy_hive::read_signal_state(identity_number).await?;
        let signal_database = crate::db::SignalDatabase::connect(identity_number.clone())
            .await
            .map_err(|error| anyhow!(error.to_string()))?;
        if let Some(value) = state.next_pre_key_id {
            signal_database
                .crypto_key_value
                .set_next_pre_key_id(value)
                .await?;
        }
        if let Some(value) = state.next_signed_pre_key_id {
            signal_database
                .crypto_key_value
                .set_next_signed_pre_key_id(value)
                .await?;
        }
        if let Some(value) = state.has_push_signal_keys {
            signal_database
                .crypto_key_value
                .set_has_push_signal_keys(value)
                .await?;
        }
        signal_database.close().await;
        Ok(())
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
        self.0
            .update(&[
                (PropertyGroup::Auth, AUTHS_KEY, Some(value.as_str())),
                (PropertyGroup::Auth, ACTIVE_USER_ID_KEY, active_user_id),
            ])
            .await?;
        Ok(())
    }
}
