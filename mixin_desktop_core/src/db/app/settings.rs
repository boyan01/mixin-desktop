use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use futures::{Stream, StreamExt};
use sdk::message_category::MessageCategory as _;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex};
use url::Url;

use super::{PropertyDao, PropertyGroup};

macro_rules! settings {
    ($(
        $name:literal: $value_type:ty = $default:expr
            => $getter:ident, $setter:ident, $subscriber:ident;
    )+) => {
        $(
            pub async fn $getter(&self) -> Result<$value_type> {
                self.get_value(&SettingKey::new($name, $default)).await
            }

            pub async fn $setter(&self, value: $value_type) -> Result<()> {
                self.set_value(&SettingKey::new($name, $default), &value).await
            }

            pub fn $subscriber(
                &self,
            ) -> impl Stream<Item = Result<$value_type>> + Send + 'static {
                self.subscribe_value(SettingKey::new($name, $default))
            }
        )+
    };
}

struct SettingKey<T> {
    name: &'static str,
    default: fn() -> T,
}

impl<T> SettingKey<T> {
    const fn new(name: &'static str, default: fn() -> T) -> Self {
        Self { name, default }
    }

    fn encode(&self, value: &T) -> Result<String>
    where
        T: Serialize,
    {
        serde_json::to_string(value).with_context(|| format!("encode setting {}", self.name))
    }
}

#[derive(Clone)]
pub struct SettingDao {
    properties: PropertyDao,
    cache: Arc<Mutex<HashMap<String, Option<String>>>>,
    changes: broadcast::Sender<Vec<String>>,
}

impl SettingDao {
    pub(super) fn new(properties: PropertyDao) -> Self {
        let (changes, _) = broadcast::channel(64);
        Self {
            properties,
            cache: Arc::new(Mutex::new(HashMap::new())),
            changes,
        }
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let mut cache = self.cache.lock().await;
        if let Some(value) = cache.get(key) {
            return Ok(value.clone());
        }
        let value = self.properties.get(PropertyGroup::Setting, key).await?;
        cache.insert(key.to_owned(), value.clone());
        Ok(value)
    }

    pub async fn set(&self, key: &str, value: Option<&str>) -> Result<()> {
        self.set_many(&[(key, value)]).await
    }

    pub(crate) async fn set_many(&self, values: &[(&str, Option<&str>)]) -> Result<()> {
        let mut cache = self.cache.lock().await;
        self.properties
            .update(
                &values
                    .iter()
                    .map(|(key, value)| (PropertyGroup::Setting, *key, *value))
                    .collect::<Vec<_>>(),
            )
            .await?;
        let mut changed = Vec::new();
        for (key, value) in values {
            let value = value.map(str::to_owned);
            if cache.get(*key) != Some(&value) {
                cache.insert((*key).to_owned(), value);
                changed.push((*key).to_owned());
            }
        }
        drop(cache);
        if !changed.is_empty() {
            let _ = self.changes.send(changed);
        }
        Ok(())
    }

    pub fn subscribe(
        &self,
        key: impl Into<String>,
    ) -> impl Stream<Item = Result<Option<String>>> + Send + 'static {
        let key = key.into();
        let query_key = key.clone();
        let settings = self.clone();
        self.subscribe_query(HashSet::from([key]), move || {
            let settings = settings.clone();
            let key = query_key.clone();
            async move { settings.get(&key).await }
        })
    }

    pub(crate) fn subscribe_query<T, Query, QueryFuture>(
        &self,
        keys: HashSet<String>,
        query: Query,
    ) -> impl Stream<Item = Result<T>> + Send + 'static
    where
        T: Clone + PartialEq + Send + 'static,
        Query: Fn() -> QueryFuture + Send + Sync + 'static,
        QueryFuture: Future<Output = Result<T>> + Send + 'static,
    {
        let mut changes = self.changes.subscribe();
        async_stream::stream! {
            let mut previous = None;
            loop {
                match query().await {
                    Ok(value) if previous.as_ref() != Some(&value) => {
                        previous = Some(value.clone());
                        yield Ok(value);
                    }
                    Ok(_) => {}
                    Err(error) => yield Err(error),
                }

                loop {
                    match changes.recv().await {
                        Ok(changed) if changed.iter().any(|key| keys.contains(key)) => break,
                        Ok(_) => {}
                        Err(broadcast::error::RecvError::Lagged(_)) => break,
                        Err(broadcast::error::RecvError::Closed) => return,
                    }
                }
            }
        }
    }

    settings! {
        "photoAutoDownload": bool = || true
            => photo_auto_download, set_photo_auto_download, subscribe_photo_auto_download;
        "videoAutoDownload": bool = || true
            => video_auto_download, set_video_auto_download, subscribe_video_auto_download;
        "fileAutoDownload": bool = || true
            => file_auto_download, set_file_auto_download, subscribe_file_auto_download;
        "proxy_settings": ProxySettings = ProxySettings::default
            => proxy_settings, set_proxy_settings, subscribe_proxy_settings;
    }

    pub async fn should_auto_download(&self, category: &str) -> Result<bool> {
        let category = category.to_string();
        if category.is_image() {
            self.photo_auto_download().await
        } else if category.is_video() {
            self.video_auto_download().await
        } else if category.is_data() {
            self.file_auto_download().await
        } else {
            Ok(true)
        }
    }

    async fn get_value<T>(&self, key: &SettingKey<T>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        decode_setting(key, self.get(key.name).await?.as_deref())
    }

    async fn set_value<T>(&self, key: &SettingKey<T>, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        let value = key.encode(value)?;
        self.set(key.name, Some(&value)).await
    }

    fn subscribe_value<T>(
        &self,
        key: SettingKey<T>,
    ) -> impl Stream<Item = Result<T>> + Send + 'static
    where
        T: DeserializeOwned + Send + 'static,
    {
        self.subscribe(key.name)
            .map(move |value| decode_setting(&key, value?.as_deref()))
    }
}

fn decode_setting<T>(key: &SettingKey<T>, value: Option<&str>) -> Result<T>
where
    T: DeserializeOwned,
{
    let Some(value) = value else {
        return Ok((key.default)());
    };
    serde_json::from_str(value).with_context(|| format!("decode setting {}", key.name))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyType {
    Http,
    Socks5,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub proxy_type: ProxyType,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl ProxyConfig {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            return Err(anyhow!("proxy id is required"));
        }
        if self.host.trim().is_empty() {
            return Err(anyhow!("proxy host is required"));
        }
        if self.port == 0 {
            return Err(anyhow!("proxy port is invalid"));
        }
        self.url()?;
        Ok(())
    }

    pub(crate) fn url(&self) -> Result<Url> {
        let scheme = match self.proxy_type {
            ProxyType::Http => "http",
            ProxyType::Socks5 => "socks5",
        };
        let mut url = Url::parse(&format!("{scheme}://{}:{}", self.host, self.port))?;
        if let Some(username) = self.username.as_deref().filter(|value| !value.is_empty()) {
            url.set_username(username)
                .map_err(|_| anyhow!("proxy username is invalid"))?;
        }
        if let Some(password) = self.password.as_deref().filter(|value| !value.is_empty()) {
            url.set_password(Some(password))
                .map_err(|_| anyhow!("proxy password is invalid"))?;
        }
        Ok(url)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxySettings {
    pub enabled: bool,
    pub selected_proxy_id: Option<String>,
    pub proxies: Vec<ProxyConfig>,
}

impl ProxySettings {
    pub fn validate(&self) -> Result<()> {
        let mut ids = HashSet::new();
        for proxy in &self.proxies {
            proxy.validate()?;
            if !ids.insert(proxy.id.as_str()) {
                return Err(anyhow!("duplicate proxy id"));
            }
        }
        if let Some(selected) = self.selected_proxy_id.as_deref() {
            if !ids.contains(selected) {
                return Err(anyhow!("selected proxy does not exist"));
            }
        }
        if self.enabled && self.active_proxy().is_none() {
            return Err(anyhow!("enabled proxy has no selection"));
        }
        Ok(())
    }

    pub(crate) fn active_proxy(&self) -> Option<&ProxyConfig> {
        if !self.enabled {
            return None;
        }
        let selected = self.selected_proxy_id.as_deref()?;
        self.proxies.iter().find(|proxy| proxy.id == selected)
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt as _;

    use crate::db::app::{AppDatabase, PropertyGroup};

    #[tokio::test]
    async fn attachment_settings_are_independent_and_default_to_true() {
        let directory = tempfile::tempdir().unwrap();
        let database = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();

        assert!(database.setting_dao.photo_auto_download().await.unwrap());
        assert!(database.setting_dao.video_auto_download().await.unwrap());
        assert!(database.setting_dao.file_auto_download().await.unwrap());

        database
            .setting_dao
            .set_photo_auto_download(false)
            .await
            .unwrap();

        assert!(!database.setting_dao.photo_auto_download().await.unwrap());
        assert!(database.setting_dao.video_auto_download().await.unwrap());
        assert!(database.setting_dao.file_auto_download().await.unwrap());
    }

    #[tokio::test]
    async fn setting_subscription_emits_initial_and_persisted_values_across_dao_clones() {
        let directory = tempfile::tempdir().unwrap();
        let database = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        let mut subscription = Box::pin(database.setting_dao.subscribe_photo_auto_download());

        assert!(subscription.next().await.unwrap().unwrap());

        database
            .setting_dao
            .clone()
            .set_photo_auto_download(false)
            .await
            .unwrap();

        assert!(!subscription.next().await.unwrap().unwrap());
        assert!(!database.setting_dao.photo_auto_download().await.unwrap());
    }

    #[tokio::test]
    async fn setting_get_reads_the_database_once_then_uses_the_shared_cache() {
        let directory = tempfile::tempdir().unwrap();
        let database = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        let key = "messagePreview";

        database
            .property_dao
            .set(PropertyGroup::Setting, key, "true")
            .await
            .unwrap();
        assert_eq!(
            database.setting_dao.get(key).await.unwrap().as_deref(),
            Some("true")
        );

        database
            .property_dao
            .set(PropertyGroup::Setting, key, "false")
            .await
            .unwrap();

        assert_eq!(
            database
                .setting_dao
                .clone()
                .get(key)
                .await
                .unwrap()
                .as_deref(),
            Some("true")
        );
    }

    #[tokio::test]
    async fn attachment_categories_read_the_matching_setting() {
        let directory = tempfile::tempdir().unwrap();
        let database = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        database
            .setting_dao
            .set_photo_auto_download(false)
            .await
            .unwrap();
        database
            .setting_dao
            .set_file_auto_download(false)
            .await
            .unwrap();

        assert!(!database
            .setting_dao
            .should_auto_download(sdk::message_category::SIGNAL_IMAGE)
            .await
            .unwrap());
        assert!(database
            .setting_dao
            .should_auto_download(sdk::message_category::PLAIN_VIDEO)
            .await
            .unwrap());
        assert!(!database
            .setting_dao
            .should_auto_download(sdk::message_category::ENCRYPTED_DATA)
            .await
            .unwrap());
        assert!(database
            .setting_dao
            .should_auto_download(sdk::message_category::SIGNAL_AUDIO)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn raw_settings_support_dynamic_keys_and_removal() {
        let directory = tempfile::tempdir().unwrap();
        let database = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        let key = "security.user.passcode";

        database
            .setting_dao
            .set(key, Some("\"123456\""))
            .await
            .unwrap();
        assert_eq!(
            database.setting_dao.get(key).await.unwrap().as_deref(),
            Some("\"123456\"")
        );

        database.setting_dao.set(key, None).await.unwrap();
        assert_eq!(database.setting_dao.get(key).await.unwrap(), None);
    }

    #[tokio::test]
    async fn typed_setting_rejects_an_invalid_persisted_value() {
        let directory = tempfile::tempdir().unwrap();
        let database = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        database
            .setting_dao
            .set("photoAutoDownload", Some("invalid"))
            .await
            .unwrap();

        let error = database
            .setting_dao
            .photo_auto_download()
            .await
            .unwrap_err();

        assert!(error
            .to_string()
            .contains("decode setting photoAutoDownload"));
    }
}
