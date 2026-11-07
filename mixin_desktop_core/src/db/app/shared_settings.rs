//! Settings the Flutter app also owns. Their NSH keys keep the string
//! encodings the UI already uses, while the values live where the Flutter
//! app keeps them:
//!
//! - `SettingCubit` in `hydrated_box`: appearance and chat preferences.
//! - `show_pin_message_box`: pinned-message banner visibility per account.
//! - `account_box.recentUsedEmoji`: recent emoji per account.
//! - The active account's `mixin.db` `properties`: proxy settings.
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use serde_json::{Map, Number, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use super::AuthDao;
use crate::db::hive::{self, HiveBox};

const SETTING_CUBIT: &str = "SettingCubit";
pub(super) const PROXY_KEY: &str = "proxy_settings";
const PINNED_PREFIX: &str = "show_pin_message_box_";
const EMOJI_PREFIX: &str = "desktop_emoji_preferences_";
const RECENT_EMOJI: &str = "recentUsedEmoji";
/// `PropertyGroup.setting` and the keys of Flutter's `SettingPropertyStorage`.
const PROPERTY_GROUP: &str = "setting";
const ENABLE_PROXY: &str = "enable_proxy";
const SELECTED_PROXY: &str = "selected_proxy";
const PROXY_LIST: &str = "proxy_list";

#[derive(Clone, Copy)]
enum Kind {
    Bool,
    Double,
    /// Flutter: 0 follows the system, 1 is dark, 2 is light.
    Brightness,
}

/// NSH key, `SettingCubit` field.
const CUBIT_FIELDS: [(&str, &str, Kind); 8] = [
    ("photoAutoDownload", "photoAutoDownload", Kind::Bool),
    ("videoAutoDownload", "videoAutoDownload", Kind::Bool),
    ("fileAutoDownload", "fileAutoDownload", Kind::Bool),
    (
        "desktop_ui_message_show_avatar",
        "messageShowAvatar",
        Kind::Bool,
    ),
    (
        "desktop_ui_message_show_identity_number",
        "messageShowIdentityNumber",
        Kind::Bool,
    ),
    (
        "desktop_ui_chat_font_size_delta",
        "chatFontSizeDelta",
        Kind::Double,
    ),
    ("desktop_notification_preview", "messagePreview", Kind::Bool),
    (
        "desktop_ui_sidebar_collapsed",
        "collapsedSidebar",
        Kind::Bool,
    ),
];
const APPEARANCE: (&str, &str) = ("desktop_ui_appearance", "brightness");

enum Route<'a> {
    Cubit(&'static str, Kind),
    Pinned {
        user_id: &'a str,
        conversation_id: &'a str,
    },
    Emoji {
        user_id: &'a str,
    },
    Proxy,
}

fn route(key: &str) -> Option<Route<'_>> {
    if let Some((_, field, kind)) = CUBIT_FIELDS.iter().find(|(name, ..)| *name == key) {
        return Some(Route::Cubit(field, *kind));
    }
    if key == APPEARANCE.0 {
        return Some(Route::Cubit(APPEARANCE.1, Kind::Brightness));
    }
    if key == PROXY_KEY {
        return Some(Route::Proxy);
    }
    if let Some(rest) = key.strip_prefix(PINNED_PREFIX) {
        let (user_id, conversation_id) = rest.split_once('_')?;
        return Some(Route::Pinned {
            user_id,
            conversation_id,
        });
    }
    key.strip_prefix(EMOJI_PREFIX)
        .map(|user_id| Route::Emoji { user_id })
}

/// Whether `key` lives in the Flutter app's storage.
pub(super) fn is_shared(key: &str) -> bool {
    route(key).is_some()
}

/// Shared keys whose full NSH value also stays in `app.db`: emoji skin tones
/// have no Flutter equivalent, and the proxy applies before sign-in.
pub(super) fn keeps_app_copy(key: &str) -> bool {
    matches!(route(key), Some(Route::Emoji { .. } | Route::Proxy))
}

#[derive(Clone)]
pub(super) struct SharedSettings {
    hydrated: Arc<HiveBox>,
    auth: AuthDao,
}

impl SharedSettings {
    pub(super) fn new(hydrated: Arc<HiveBox>, auth: AuthDao) -> Self {
        Self { hydrated, auth }
    }

    /// Reads a shared key. `fallback` is its `app.db` value, used for NSH-only
    /// parts (emoji skin tones) and the proxy before any account signs in.
    pub(super) async fn get(&self, key: &str, fallback: Option<String>) -> Result<Option<String>> {
        match route(key).context("not a shared setting")? {
            Route::Cubit(field, kind) => {
                let state = self.hydrated.get(SETTING_CUBIT)?;
                Ok(state
                    .as_ref()
                    .and_then(|state| state.get(field))
                    .and_then(|value| encode(kind, value)))
            }
            Route::Pinned {
                user_id,
                conversation_id,
            } => {
                let Some(directory) = self.auth.account_directory_of(user_id)? else {
                    return Ok(None);
                };
                Ok(hive::read_account_value(
                    &directory,
                    hive::SHOW_PIN_MESSAGE_BOX,
                    conversation_id,
                )?
                .and_then(|value| value.as_bool())
                .map(|shown| shown.to_string()))
            }
            Route::Emoji { user_id } => {
                let Some(directory) = self.auth.account_directory_of(user_id)? else {
                    return Ok(fallback);
                };
                let Some(recent) =
                    hive::read_account_value(&directory, hive::ACCOUNT_BOX, RECENT_EMOJI)?
                else {
                    return Ok(fallback);
                };
                let mut preferences = fallback
                    .and_then(|value| serde_json::from_str::<Map<String, Value>>(&value).ok())
                    .unwrap_or_default();
                preferences.insert("recent".into(), recent);
                Ok(Some(Value::Object(preferences).to_string()))
            }
            Route::Proxy => {
                let Some(database) = self.active_database()? else {
                    return Ok(fallback);
                };
                read_proxy(&database).await.map(|value| value.or(fallback))
            }
        }
    }

    /// Writes shared keys. NSH-only parts were already saved to `app.db`.
    pub(super) async fn set(&self, values: &[(&str, Option<&str>)]) -> Result<()> {
        let mut cubit = None::<Map<String, Value>>;
        for (key, value) in values {
            match route(key).context("not a shared setting")? {
                Route::Cubit(field, kind) => {
                    let state = match &mut cubit {
                        Some(state) => state,
                        None => cubit.insert(
                            self.hydrated
                                .get(SETTING_CUBIT)?
                                .and_then(|state| match state {
                                    Value::Object(state) => Some(state),
                                    _ => None,
                                })
                                .unwrap_or_default(),
                        ),
                    };
                    state.insert(field.to_owned(), decode(kind, *value)?);
                }
                Route::Pinned {
                    user_id,
                    conversation_id,
                } => {
                    let Some(directory) = self.auth.account_directory_of(user_id)? else {
                        continue;
                    };
                    // `ShowPinMessageKeyValue.show` stores true; dismiss stores false.
                    let shown = *value != Some("false");
                    hive::account_box(&directory, hive::SHOW_PIN_MESSAGE_BOX)?
                        .put(conversation_id, &Value::Bool(shown))?;
                }
                Route::Emoji { user_id } => {
                    let Some(directory) = self.auth.account_directory_of(user_id)? else {
                        continue;
                    };
                    let recent = value
                        .and_then(|value| serde_json::from_str::<Value>(value).ok())
                        .and_then(|value| value.get("recent").cloned())
                        .filter(Value::is_array);
                    let account_box = hive::account_box(&directory, hive::ACCOUNT_BOX)?;
                    match recent {
                        Some(recent) => account_box.put(RECENT_EMOJI, &recent)?,
                        None => account_box.delete(RECENT_EMOJI)?,
                    }
                }
                Route::Proxy => {
                    if let Some(database) = self.active_database()? {
                        write_proxy(&database, *value).await?;
                    }
                }
            }
        }
        if let Some(state) = cubit {
            self.hydrated.put(SETTING_CUBIT, &Value::Object(state))?;
        }
        Ok(())
    }

    fn active_database(&self) -> Result<Option<PathBuf>> {
        Ok(self
            .auth
            .active_account_directory()?
            .map(|directory| directory.join("mixin.db"))
            .filter(|database| database.exists()))
    }
}

/// Encodes a `SettingCubit` value the way the NSH UI writes it.
fn encode(kind: Kind, value: &Value) -> Option<String> {
    match kind {
        Kind::Bool => value.as_bool().map(|value| value.to_string()),
        Kind::Double => value.as_f64().map(|value| value.to_string()),
        Kind::Brightness => match value.as_i64()? {
            1 => Some("dark".into()),
            2 => Some("light".into()),
            _ => None,
        },
    }
}

/// `chatFontSizeDelta` must stay a Dart double and `brightness` an int.
fn decode(kind: Kind, value: Option<&str>) -> Result<Value> {
    Ok(match (kind, value) {
        (Kind::Brightness, Some("dark")) => Value::from(1),
        (Kind::Brightness, Some("light")) => Value::from(2),
        (Kind::Brightness, _) => Value::from(0),
        (_, None) => Value::Null,
        (Kind::Bool, Some(value)) => Value::Bool(value.parse().context("invalid boolean setting")?),
        (Kind::Double, Some(value)) => Value::Number(
            Number::from_f64(value.parse().context("invalid number setting")?)
                .context("invalid number setting")?,
        ),
    })
}

async fn connect(database: &Path) -> Result<crate::db::Pool> {
    Ok(SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new().filename(database))
        .await?)
}

async fn read_proxy(database: &Path) -> Result<Option<String>> {
    let pool = connect(database).await?;
    let rows: Result<Vec<(String, String)>, _> = crate::db::query_as(
        r#"SELECT "key", "value" FROM properties WHERE "group" = ? AND "key" IN (?, ?, ?)"#,
    )
    .bind(PROPERTY_GROUP)
    .bind(ENABLE_PROXY)
    .bind(SELECTED_PROXY)
    .bind(PROXY_LIST)
    .fetch_all(&pool)
    .await;
    pool.close().await;
    let rows = rows?;
    if rows.is_empty() {
        return Ok(None);
    }
    let value = |key: &str| {
        rows.iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    };
    // Mirrors `SettingPropertyStorage`: a bool is the string "true", and an
    // unreadable list is empty.
    let proxies = value(PROXY_LIST)
        .and_then(|list| serde_json::from_str::<Vec<Value>>(list).ok())
        .unwrap_or_default();
    Ok(Some(
        serde_json::json!({
            "enabled": value(ENABLE_PROXY) == Some("true"),
            "selected_proxy_id": value(SELECTED_PROXY),
            "proxies": proxies,
        })
        .to_string(),
    ))
}

async fn write_proxy(database: &Path, value: Option<&str>) -> Result<()> {
    let settings = value
        .map(serde_json::from_str::<Value>)
        .transpose()?
        .unwrap_or(Value::Null);
    let enabled = settings["enabled"].as_bool().unwrap_or(false).to_string();
    let selected = settings["selected_proxy_id"].as_str();
    let proxies = settings
        .get("proxies")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()))
        .to_string();
    let pool = connect(database).await?;
    let result = async {
        let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
        for (key, value) in [
            (ENABLE_PROXY, Some(enabled.as_str())),
            (SELECTED_PROXY, selected),
            (PROXY_LIST, Some(proxies.as_str())),
        ] {
            match value {
                Some(value) => {
                    crate::db::query(
                        r#"INSERT INTO properties ("key", "group", "value") VALUES (?, ?, ?)
                           ON CONFLICT("key", "group") DO UPDATE SET "value" = excluded."value""#,
                    )
                    .bind(key)
                    .bind(PROPERTY_GROUP)
                    .bind(value)
                    .execute(&mut *transaction)
                    .await?;
                }
                None => {
                    crate::db::query(r#"DELETE FROM properties WHERE "key" = ? AND "group" = ?"#)
                        .bind(key)
                        .bind(PROPERTY_GROUP)
                        .execute(&mut *transaction)
                        .await?;
                }
            }
        }
        transaction.commit().await?;
        Ok::<_, anyhow::Error>(())
    }
    .await;
    pool.close().await;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::hive::hydrated_box;
    use serde_json::json;

    async fn settings(root: &Path) -> SharedSettings {
        let hydrated = hydrated_box(root).unwrap();
        hydrated
            .put(
                "MultiAuthCubit",
                &json!({"auths": [{"account": {"user_id": "user", "identity_number": "7000"}, "privateKey": ""}]}),
            )
            .unwrap();
        SharedSettings::new(hydrated.clone(), AuthDao::new(root.to_path_buf(), hydrated))
    }

    #[tokio::test]
    async fn flutter_setting_cubit_values_map_to_nsh_encodings_and_back() {
        let directory = tempfile::tempdir().unwrap();
        let settings = settings(directory.path()).await;
        settings
            .hydrated
            .put(
                SETTING_CUBIT,
                &json!({"brightness": 1, "messagePreview": false, "chatFontSizeDelta": 2.0, "collapsedSidebar": null}),
            )
            .unwrap();

        assert_eq!(
            settings
                .get("desktop_ui_appearance", None)
                .await
                .unwrap()
                .as_deref(),
            Some("dark")
        );
        assert_eq!(
            settings
                .get("desktop_notification_preview", None)
                .await
                .unwrap()
                .as_deref(),
            Some("false")
        );
        assert_eq!(
            settings
                .get("desktop_ui_chat_font_size_delta", None)
                .await
                .unwrap()
                .as_deref(),
            Some("2")
        );
        assert_eq!(
            settings
                .get("desktop_ui_sidebar_collapsed", None)
                .await
                .unwrap(),
            None
        );

        settings
            .set(&[
                ("desktop_ui_appearance", None),
                ("desktop_ui_chat_font_size_delta", Some("1")),
                ("photoAutoDownload", Some("false")),
            ])
            .await
            .unwrap();
        let state = settings.hydrated.get(SETTING_CUBIT).unwrap().unwrap();
        assert_eq!(state["brightness"], json!(0));
        assert!(state["chatFontSizeDelta"].is_f64());
        assert_eq!(state["photoAutoDownload"], json!(false));
        assert_eq!(state["messagePreview"], json!(false));
    }

    #[tokio::test]
    async fn pinned_banner_and_recent_emoji_use_the_account_boxes() {
        let directory = tempfile::tempdir().unwrap();
        let settings = settings(directory.path()).await;
        let pinned = "show_pin_message_box_user_conversation";

        settings.set(&[(pinned, Some("false"))]).await.unwrap();
        let account = directory.path().join("7000");
        assert_eq!(
            hive::read_account_value(&account, hive::SHOW_PIN_MESSAGE_BOX, "conversation").unwrap(),
            Some(json!(false))
        );
        settings.set(&[(pinned, None)]).await.unwrap();
        assert_eq!(
            settings.get(pinned, None).await.unwrap().as_deref(),
            Some("true")
        );

        let emoji = "desktop_emoji_preferences_user";
        settings
            .set(&[(emoji, Some(r#"{"recent":["😀","👍"],"skins":{"👍":2}}"#))])
            .await
            .unwrap();
        assert_eq!(
            hive::read_account_value(&account, hive::ACCOUNT_BOX, RECENT_EMOJI).unwrap(),
            Some(json!(["😀", "👍"]))
        );
        let read: Value = serde_json::from_str(
            &settings
                .get(emoji, Some(r#"{"recent":[],"skins":{"👍":2}}"#.into()))
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(read, json!({"recent": ["😀", "👍"], "skins": {"👍": 2}}));
    }

    #[tokio::test]
    async fn proxy_settings_round_trip_through_the_account_properties() {
        let directory = tempfile::tempdir().unwrap();
        let settings = settings(directory.path()).await;
        assert_eq!(
            settings
                .get(PROXY_KEY, Some("app".into()))
                .await
                .unwrap()
                .as_deref(),
            Some("app")
        );
        crate::db::MixinDatabase::connect_at(directory.path().join("7000/mixin.db"))
            .await
            .unwrap()
            .close()
            .await;

        let proxy = r#"{"enabled":true,"selected_proxy_id":"p","proxies":[{"id":"p","type":"socks5","host":"127.0.0.1","port":1080,"username":null,"password":null}]}"#;
        settings.set(&[(PROXY_KEY, Some(proxy))]).await.unwrap();
        let pool = connect(&directory.path().join("7000/mixin.db"))
            .await
            .unwrap();
        let rows: Vec<(String, String)> = crate::db::query_as(
            r#"SELECT "key", "value" FROM properties WHERE "group" = 'setting' ORDER BY "key""#,
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        pool.close().await;
        assert_eq!(rows[0], ("enable_proxy".into(), "true".into()));
        assert_eq!(
            serde_json::from_str::<Value>(&rows[1].1).unwrap()[0]["type"],
            "socks5"
        );
        assert_eq!(rows[2], ("selected_proxy".into(), "p".into()));

        let read: Value =
            serde_json::from_str(&settings.get(PROXY_KEY, None).await.unwrap().unwrap()).unwrap();
        assert_eq!(read, serde_json::from_str::<Value>(proxy).unwrap());
    }
}
