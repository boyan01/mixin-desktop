use anyhow::{anyhow, bail};
use base64ct::{Base64, Encoding};
use ring::signature::{Ed25519KeyPair, KeyPair};
use sdk::Account;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use super::{PropertyDao, PropertyGroup};

#[cfg(target_arch = "wasm32")]
const AUTHS_KEY: &str = "auths";
#[cfg(target_arch = "wasm32")]
const ACTIVE_USER_ID_KEY: &str = "active_user_id";
/// The Flutter app's `MultiAuthStateNotifier` storage key in `hydrated_box`.
#[cfg(not(target_arch = "wasm32"))]
const MULTI_AUTH_KEY: &str = "MultiAuthCubit";
#[cfg(not(target_arch = "wasm32"))]
const PRIMARY_SESSION_ID_KEY: &str = "primarySessionId";
#[cfg(not(target_arch = "wasm32"))]
const DEVICE_ID_KEY: &str = "deviceId";

/// Saved sessions, most recently active first.
///
/// Native builds share the Flutter app's `MultiAuthCubit` Hive state, whose
/// list keeps the current session last. wasm keeps them in `app.db`.
#[derive(Clone)]
pub struct AuthDao {
    #[cfg(target_arch = "wasm32")]
    properties: PropertyDao,
    #[cfg(not(target_arch = "wasm32"))]
    root: std::path::PathBuf,
    #[cfg(not(target_arch = "wasm32"))]
    hydrated: std::sync::Arc<crate::db::hive::HiveBox>,
}

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
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn new(properties: PropertyDao) -> Self {
        Self { properties }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn new(
        root: std::path::PathBuf,
        hydrated: std::sync::Arc<crate::db::hive::HiveBox>,
    ) -> Self {
        Self { root, hydrated }
    }

    pub async fn find_all_auth(&self) -> anyhow::Result<Vec<Auth>> {
        self.read_auths().await
    }

    pub async fn remove_auth(&self, id: &str) -> anyhow::Result<()> {
        let mut auths = self.find_all_auth().await?;
        auths.retain(|auth| auth.user_id != id);
        self.write_auths(&auths).await
    }

    pub async fn save_auth(&self, auth: &Auth) -> anyhow::Result<()> {
        let mut auths = self.find_all_auth().await?;
        auths.retain(|item| item.user_id != auth.user_id);
        auths.insert(0, auth.clone());
        self.write_auths(&auths).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn account_directory(&self, identity_number: &str) -> anyhow::Result<std::path::PathBuf> {
        crate::db::hive::account_directory(&self.root, identity_number)
    }

    /// Saved `(user_id, identity_number)` pairs, read without decoding the
    /// accounts; the last pair is the current session.
    #[cfg(not(target_arch = "wasm32"))]
    fn saved_identities(&self) -> anyhow::Result<Vec<(String, String)>> {
        let state = self.hydrated.get(MULTI_AUTH_KEY)?;
        Ok(state
            .as_ref()
            .and_then(|state| state.get("auths"))
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|auth| {
                let account = auth.get("account")?;
                Some((
                    account.get("user_id")?.as_str()?.to_owned(),
                    account.get("identity_number")?.as_str()?.to_owned(),
                ))
            })
            .collect())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn account_directory_of(
        &self,
        user_id: &str,
    ) -> anyhow::Result<Option<std::path::PathBuf>> {
        self.saved_identities()?
            .into_iter()
            .find(|(saved, _)| saved == user_id)
            .map(|(_, identity_number)| self.account_directory(&identity_number))
            .transpose()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn active_account_directory(&self) -> anyhow::Result<Option<std::path::PathBuf>> {
        self.saved_identities()?
            .pop()
            .map(|(_, identity_number)| self.account_directory(&identity_number))
            .transpose()
    }

    /// The machine id the account was signed in on (`account_box.deviceId`).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) async fn device_id(&self, identity_number: &str) -> anyhow::Result<Option<String>> {
        Ok(crate::db::hive::read_account_value(
            &self.account_directory(identity_number)?,
            crate::db::hive::ACCOUNT_BOX,
            DEVICE_ID_KEY,
        )?
        .and_then(|value| value.as_str().map(str::to_owned)))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) async fn set_device_id(
        &self,
        identity_number: &str,
        device_id: &str,
    ) -> anyhow::Result<()> {
        crate::db::hive::account_box(
            &self.account_directory(identity_number)?,
            crate::db::hive::ACCOUNT_BOX,
        )?
        .put(DEVICE_ID_KEY, &device_id.into())
    }

    /// Deletes the account's Hive boxes like the Flutter sign-out, keeping
    /// its messages and media.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) async fn clear_account_boxes(&self, identity_number: &str) -> anyhow::Result<()> {
        crate::db::hive::delete_account_boxes(&self.account_directory(identity_number)?)
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn read_auths(&self) -> anyhow::Result<Vec<Auth>> {
        let state = self.hydrated.get(MULTI_AUTH_KEY)?;
        let stored = state
            .as_ref()
            .and_then(|state| state.get("auths"))
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
        let mut auths = serde_json::from_value::<Vec<StoredAuth>>(stored)?
            .into_iter()
            .map(Auth::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        for auth in &mut auths {
            auth.primary_session_id = crate::db::hive::read_account_value(
                &self.account_directory(&auth.account.identity_number)?,
                crate::db::hive::ACCOUNT_BOX,
                PRIMARY_SESSION_ID_KEY,
            )?
            .and_then(|value| value.as_str().map(str::to_owned))
            .filter(|session_id| !session_id.trim().is_empty());
        }
        if !auths.is_empty() {
            auths.rotate_right(1);
        }
        Ok(auths)
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn write_auths(&self, auths: &[Auth]) -> anyhow::Result<()> {
        for auth in auths {
            let Some(primary_session_id) = auth.primary_session_id.as_deref() else {
                continue;
            };
            let account_box = crate::db::hive::account_box(
                &self.account_directory(&auth.account.identity_number)?,
                crate::db::hive::ACCOUNT_BOX,
            )?;
            let primary_session_id = serde_json::Value::from(primary_session_id);
            if account_box.get(PRIMARY_SESSION_ID_KEY)?.as_ref() != Some(&primary_session_id) {
                account_box.put(PRIMARY_SESSION_ID_KEY, &primary_session_id)?;
            }
        }
        // Flutter's MultiAuthState treats the last session as current.
        let mut stored = auths
            .iter()
            .map(|auth| {
                let mut stored = StoredAuth::from(auth);
                stored.primary_session_id = None;
                stored
            })
            .collect::<Vec<_>>();
        if !stored.is_empty() {
            stored.rotate_left(1);
        }
        let state = serde_json::json!({ "auths": stored });
        self.hydrated.put(MULTI_AUTH_KEY, &state)
    }

    #[cfg(target_arch = "wasm32")]
    async fn read_auths(&self) -> anyhow::Result<Vec<Auth>> {
        let stored = self
            .properties
            .get(PropertyGroup::Auth, AUTHS_KEY)
            .await?
            .unwrap_or_else(|| "[]".to_string());
        let active_user_id = self
            .properties
            .get(PropertyGroup::Auth, ACTIVE_USER_ID_KEY)
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

    #[cfg(target_arch = "wasm32")]
    async fn write_auths(&self, auths: &[Auth]) -> anyhow::Result<()> {
        let stored = auths.iter().map(StoredAuth::from).collect::<Vec<_>>();
        let value = serde_json::to_string(&stored)?;
        let active_user_id = auths.first().map(|auth| auth.user_id.as_str());
        self.properties
            .update(&[
                (PropertyGroup::Auth, AUTHS_KEY, Some(value.as_str())),
                (PropertyGroup::Auth, ACTIVE_USER_ID_KEY, active_user_id),
            ])
            .await?;
        Ok(())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::db::hive::{account_box, hydrated_box, ACCOUNT_BOX};
    use serde_json::json;

    fn flutter_auth(user_id: &str, identity_number: &str, seed: u8) -> serde_json::Value {
        // Account.toJson from mixin_bot_sdk_dart 1.5, as Flutter stores it.
        let account = json!({
            "user_id": user_id, "identity_number": identity_number, "phone": "", "full_name": null,
            "biography": "", "avatar_url": null, "relationship": "ME", "mute_until": "",
            "created_at": "2026-01-01T00:00:00.000Z", "is_verified": false, "is_scam": false,
            "session_id": "20000000-0000-4000-8000-000000000001", "pin_token": "", "pin_token_base64": "",
            "code_id": "", "code_url": "", "device_status": "", "has_safe": true, "has_pin": true,
            "has_emergency_contact": false, "receive_message_source": "EVERYBODY",
            "accept_conversation_source": "EVERYBODY", "accept_search_source": "EVERYBODY",
            "fiat_currency": "USD", "transfer_notification_threshold": 0.0,
            "transfer_confirmation_threshold": 100.0, "tip_counter": 1, "tip_key_base64": "",
            "membership": null
        });
        let seed = [seed; 32];
        let pair = Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
        let private_key = [&seed[..], pair.public_key().as_ref()].concat();
        json!({"account": account, "privateKey": Base64::encode_string(&private_key)})
    }

    #[tokio::test]
    async fn flutter_multi_auth_state_reads_current_last_and_saves_it_last() {
        let directory = tempfile::tempdir().unwrap();
        let hydrated = hydrated_box(directory.path()).unwrap();
        hydrated
            .put(
                MULTI_AUTH_KEY,
                &json!({"auths": [flutter_auth("user-a", "7001", 1), flutter_auth("user-b", "7002", 2)]}),
            )
            .unwrap();
        account_box(&directory.path().join("7002"), ACCOUNT_BOX)
            .unwrap()
            .put(
                PRIMARY_SESSION_ID_KEY,
                &json!("30000000-0000-4000-8000-000000000001"),
            )
            .unwrap();
        let dao = AuthDao::new(directory.path().to_path_buf(), hydrated.clone());

        let auths = dao.find_all_auth().await.unwrap();
        assert_eq!(
            auths
                .iter()
                .map(|auth| auth.user_id.as_str())
                .collect::<Vec<_>>(),
            ["user-b", "user-a"]
        );
        assert_eq!(
            auths[0].primary_session_id.as_deref(),
            Some("30000000-0000-4000-8000-000000000001")
        );
        assert_eq!(auths[0].private_key, vec![2; 32]);

        dao.save_auth(&auths[1]).await.unwrap();
        let stored = hydrated.get(MULTI_AUTH_KEY).unwrap().unwrap();
        let stored = stored["auths"].as_array().unwrap();
        assert_eq!(stored[0]["account"]["user_id"], "user-b");
        assert_eq!(stored[1]["account"]["user_id"], "user-a");
        assert_eq!(
            stored[1]["privateKey"],
            flutter_auth("user-a", "7001", 1)["privateKey"]
        );
        assert!(stored[1].get("primarySessionId").is_none());

        dao.remove_auth("user-a").await.unwrap();
        let stored = hydrated.get(MULTI_AUTH_KEY).unwrap().unwrap();
        assert_eq!(stored["auths"].as_array().unwrap().len(), 1);
        assert_eq!(dao.find_all_auth().await.unwrap()[0].user_id, "user-b");
    }
}
