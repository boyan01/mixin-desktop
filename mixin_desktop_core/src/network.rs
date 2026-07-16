use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Method, Proxy};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use url::Url;

use crate::db::app::PropertyDao;

const PROPERTY_GROUP: &str = "network";
const PROXY_SETTINGS_KEY: &str = "proxy_settings";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_RESPONSE_BYTES: usize = 32 * 1024 * 1024;

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
    fn validate(&self) -> Result<()> {
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

    fn url(&self) -> Result<Url> {
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
    fn validate(&self) -> Result<()> {
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

    pub fn active_proxy(&self) -> Option<&ProxyConfig> {
        if !self.enabled {
            return None;
        }
        let selected = self.selected_proxy_id.as_deref()?;
        self.proxies.iter().find(|proxy| proxy.id == selected)
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

struct NetworkState {
    settings: ProxySettings,
    client: reqwest::Client,
}

pub struct NetworkService {
    property_dao: PropertyDao,
    state: RwLock<NetworkState>,
}

impl NetworkService {
    pub async fn new(property_dao: PropertyDao) -> Result<Self> {
        let settings = match property_dao.get(PROPERTY_GROUP, PROXY_SETTINGS_KEY).await? {
            Some(value) => serde_json::from_str::<ProxySettings>(&value)
                .context("invalid persisted proxy settings")?,
            None => ProxySettings::default(),
        };
        settings.validate()?;
        let client = build_client(settings.active_proxy())?;
        Ok(Self {
            property_dao,
            state: RwLock::new(NetworkState { settings, client }),
        })
    }

    pub async fn proxy_settings(&self) -> ProxySettings {
        self.state.read().await.settings.clone()
    }

    pub async fn set_proxy_settings(&self, settings: ProxySettings) -> Result<()> {
        settings.validate()?;
        let client = build_client(settings.active_proxy())?;
        let encoded = serde_json::to_string(&settings)?;
        self.property_dao
            .set(PROPERTY_GROUP, PROXY_SETTINGS_KEY, &encoded)
            .await?;
        let mut state = self.state.write().await;
        state.settings = settings;
        state.client = client;
        Ok(())
    }

    pub async fn request(
        &self,
        method: &str,
        url: &str,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
        timeout_millis: Option<u64>,
        max_response_bytes: Option<usize>,
    ) -> Result<HttpResponse> {
        let method = Method::from_bytes(method.as_bytes()).context("invalid HTTP method")?;
        let url = Url::parse(url).context("invalid HTTP URL")?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(anyhow!("unsupported HTTP URL scheme"));
        }
        let mut request_headers = HeaderMap::new();
        for (name, value) in headers {
            request_headers.insert(
                HeaderName::from_bytes(name.as_bytes())?,
                HeaderValue::from_str(&value)?,
            );
        }
        let client = self.state.read().await.client.clone();
        let mut request = client
            .request(method, url)
            .headers(request_headers)
            .timeout(Duration::from_millis(
                timeout_millis.unwrap_or(DEFAULT_TIMEOUT.as_millis() as u64),
            ));
        if let Some(body) = body {
            request = request.body(body);
        }
        let response = request.send().await?;
        let status_code = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (name.to_string(), value.to_string()))
            })
            .collect();
        let limit = max_response_bytes.unwrap_or(MAX_RESPONSE_BYTES);
        if response
            .content_length()
            .is_some_and(|content_length| content_length > limit as u64)
        {
            return Err(anyhow!("HTTP response is too large"));
        }
        let mut body = Vec::new();
        let mut response = response;
        while let Some(chunk) = response.chunk().await? {
            if body.len() + chunk.len() > limit {
                return Err(anyhow!("HTTP response is too large"));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(HttpResponse {
            status_code,
            headers,
            body,
        })
    }
}

fn build_client(proxy: Option<&ProxyConfig>) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(150));
    if let Some(config) = proxy {
        builder = builder.proxy(Proxy::all(config.url()?.as_str())?);
    }
    Ok(builder.build()?)
}

pub type SharedNetworkService = Arc<NetworkService>;

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;
    use crate::db::app::AppDatabase;

    #[tokio::test]
    async fn persists_proxy_settings_in_app_database() {
        let directory = tempfile::tempdir().unwrap();
        let database = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        let service = NetworkService::new(database.property_dao.clone())
            .await
            .unwrap();
        let settings = ProxySettings {
            enabled: false,
            selected_proxy_id: Some("local".to_owned()),
            proxies: vec![ProxyConfig {
                id: "local".to_owned(),
                proxy_type: ProxyType::Http,
                host: "127.0.0.1".to_owned(),
                port: 7890,
                username: Some("user".to_owned()),
                password: Some("password".to_owned()),
            }],
        };

        service.set_proxy_settings(settings.clone()).await.unwrap();
        let restored = NetworkService::new(database.property_dao)
            .await
            .unwrap()
            .proxy_settings()
            .await;

        assert_eq!(restored, settings);
    }

    #[tokio::test]
    async fn performs_http_request_and_returns_response_metadata() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let count = stream.read(&mut request).await.unwrap();
            assert!(String::from_utf8_lossy(&request[..count]).starts_with("POST /asset HTTP/1.1"));
            stream
                .write_all(
                    b"HTTP/1.1 201 Created\r\nContent-Type: image/gif\r\nContent-Length: 4\r\nConnection: close\r\n\r\nGIF8",
                )
                .await
                .unwrap();
        });
        let directory = tempfile::tempdir().unwrap();
        let database = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        let service = NetworkService::new(database.property_dao).await.unwrap();

        let response = service
            .request(
                "POST",
                &format!("http://{address}/asset"),
                HashMap::from([("x-test".to_owned(), "yes".to_owned())]),
                Some(b"request".to_vec()),
                Some(5_000),
                Some(16),
            )
            .await
            .unwrap();
        server.await.unwrap();

        assert_eq!(response.status_code, 201);
        assert_eq!(response.headers.get("content-type").unwrap(), "image/gif");
        assert_eq!(response.body, b"GIF8");
    }

    #[tokio::test]
    async fn routes_requests_through_the_active_http_proxy() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 2048];
            let count = stream.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..count]).to_string();
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nproxied",
                )
                .await
                .unwrap();
            request
        });
        let directory = tempfile::tempdir().unwrap();
        let database = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        let service = NetworkService::new(database.property_dao).await.unwrap();
        service
            .set_proxy_settings(ProxySettings {
                enabled: true,
                selected_proxy_id: Some("proxy".to_owned()),
                proxies: vec![ProxyConfig {
                    id: "proxy".to_owned(),
                    proxy_type: ProxyType::Http,
                    host: address.ip().to_string(),
                    port: address.port(),
                    username: Some("user".to_owned()),
                    password: Some("password".to_owned()),
                }],
            })
            .await
            .unwrap();

        let response = service
            .request(
                "GET",
                "http://example.invalid/proxied",
                HashMap::new(),
                None,
                Some(5_000),
                None,
            )
            .await
            .unwrap();
        let request = server.await.unwrap().to_lowercase();

        assert_eq!(response.body, b"proxied");
        assert!(request.starts_with("get http://example.invalid/proxied http/1.1"));
        assert!(request.contains("proxy-authorization: basic dxnlcjpwyxnzd29yza=="));
    }

    #[test]
    fn rejects_enabled_settings_without_an_active_proxy() {
        let settings = ProxySettings {
            enabled: true,
            selected_proxy_id: None,
            proxies: Vec::new(),
        };

        assert_eq!(
            settings.validate().unwrap_err().to_string(),
            "enabled proxy has no selection"
        );
    }
}
