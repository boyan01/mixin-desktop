use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use futures::StreamExt as _;
use log::error;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Method, Proxy};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use url::Url;

use crate::db::app::SettingDao;
pub use crate::db::app::{ProxyConfig, ProxySettings, ProxyType};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_RESPONSE_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

pub struct NetworkService {
    client: watch::Receiver<reqwest::Client>,
    settings_task: JoinHandle<()>,
}

impl NetworkService {
    pub async fn new(setting_dao: SettingDao) -> Result<Self> {
        let mut settings = Box::pin(setting_dao.subscribe_proxy_settings());
        let initial = settings
            .next()
            .await
            .transpose()?
            .ok_or_else(|| anyhow!("proxy settings subscription closed"))?;
        initial.validate()?;
        let client = build_client(initial.active_proxy())?;
        let (client_sender, client) = watch::channel(client);
        let settings_task = tokio::spawn(async move {
            while let Some(next) = settings.next().await {
                let next = match next {
                    Ok(next) => next,
                    Err(error) => {
                        error!("proxy settings subscription failed: {error:?}");
                        return;
                    }
                };
                if let Err(error) = next.validate() {
                    error!("invalid proxy settings update: {error:?}");
                    continue;
                }
                match build_client(next.active_proxy()) {
                    Ok(client) => {
                        client_sender.send_replace(client);
                    }
                    Err(error) => error!("failed to apply proxy settings: {error:?}"),
                }
            }
        });
        Ok(Self {
            client,
            settings_task,
        })
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
        let client = self.client.borrow().clone();
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

impl Drop for NetworkService {
    fn drop(&mut self) {
        self.settings_task.abort();
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
        let service = NetworkService::new(database.setting_dao).await.unwrap();

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
        let settings = database.setting_dao.clone();
        let service = NetworkService::new(database.setting_dao).await.unwrap();
        let mut client_changes = service.client.clone();
        settings
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
        tokio::time::timeout(Duration::from_secs(1), client_changes.changed())
            .await
            .unwrap()
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
