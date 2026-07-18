use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context};
use bytes::Bytes;
use reqwest::header::HeaderValue;
use reqwest::{Method, Request};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::watch;

use crate::credential::Credential;
use crate::{
    AccountApi, ApiError, AssetApi, AttachmentApi, CircleApi, ConversationApi, MessageApi,
    ProvisioningApi, SnapshotApi, TokenApi, UserApi,
};

pub struct Client {
    inner: Arc<ClientRef>,
    pub user_api: UserApi,
    pub account_api: AccountApi,
    pub attachment_api: AttachmentApi,
    pub asset_api: AssetApi,
    pub provisioning_api: ProvisioningApi,
    pub token_api: TokenApi,
    pub conversation_api: ConversationApi,
    pub circle_api: CircleApi,
    pub message_api: MessageApi,
    pub snapshot_api: SnapshotApi,
}

impl Client {
    pub fn new(credential: Credential) -> Self {
        let inner = Arc::new(ClientRef::new(credential));
        Client {
            inner: inner.clone(),
            user_api: UserApi::new(inner.clone()),
            account_api: AccountApi::new(inner.clone()),
            attachment_api: AttachmentApi::new(inner.clone()),
            asset_api: AssetApi::new(inner.clone()),
            provisioning_api: ProvisioningApi::new(inner.clone()),
            token_api: TokenApi::new(inner.clone()),
            conversation_api: ConversationApi::new(inner.clone()),
            circle_api: CircleApi {
                client: inner.clone(),
            },
            message_api: MessageApi {
                client: inner.clone(),
            },
            snapshot_api: SnapshotApi::new(inner.clone()),
        }
    }

    pub async fn request<T>(&self, request: Request) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        self.inner.request(request).await
    }

    pub fn subscribe_authentication_errors(&self) -> watch::Receiver<bool> {
        self.inner.authentication_failed.subscribe()
    }

    pub fn subscribe_server_error_codes(&self) -> watch::Receiver<Option<i64>> {
        self.inner.server_error_code.subscribe()
    }
}

pub(crate) struct ClientRef {
    credential: Credential,
    pub(crate) base_url: String,
    pub(crate) client: reqwest::Client,
    authentication_failed: watch::Sender<bool>,
    server_error_code: watch::Sender<Option<i64>>,
}

const MIXIN_BASE_URL: &str = "https://api.mixin.one";
const API_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum MixinResponse {
    Data(Value),
    Error(crate::Error),
}

impl ClientRef {
    pub fn new(credential: Credential) -> Self {
        ClientRef {
            credential,
            base_url: MIXIN_BASE_URL.to_string(),
            client: reqwest::Client::builder()
                .connect_timeout(API_TIMEOUT)
                .read_timeout(API_TIMEOUT)
                .build()
                .expect("failed to build Mixin HTTP client"),
            authentication_failed: watch::channel(false).0,
            server_error_code: watch::channel(None).0,
        }
    }

    fn notify_authentication_failed(&self) {
        self.authentication_failed.send_replace(true);
    }

    pub(crate) async fn get<T>(&self, path: &str) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        let request = self
            .client
            .request(Method::GET, format!("{}/{}", self.base_url, path))
            .build()?;
        self.request(request).await
    }

    pub(crate) async fn get_with_query<T>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        let mut url = reqwest::Url::parse(&format!("{}/{}", self.base_url, path))
            .map_err(|error| anyhow!("invalid request url: {error}"))?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let request = self.client.request(Method::GET, url).build()?;
        self.request(request).await
    }

    pub(crate) async fn post<T, B>(&self, path: &str, body: &B) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
        B: ?Sized + Serialize,
    {
        let request = self
            .client
            .request(Method::POST, format!("{}/{}", self.base_url, path))
            .body(
                serde_json::to_string(&body)
                    .map_err(|e| anyhow!("can not serialize body: {}", e))?,
            )
            .build()?;
        self.request(request).await
    }

    pub(crate) async fn raw_request(&self, mut request: Request) -> Result<Bytes, ApiError> {
        let path = match request.method() {
            &Method::GET => {
                let path = request.url().path();
                if let Some(query) = request.url().query() {
                    format!("{}?{}", path, query)
                } else {
                    path.to_string()
                }
            }
            _ => request.url().path().to_string(),
        };
        let body: &[u8] = match request.method() {
            &Method::POST => request
                .body()
                .map(|body| body.as_bytes().unwrap_or_default())
                .unwrap_or_default(),
            _ => &[],
        };
        let signature = self
            .credential
            .sign_authentication_token(request.method(), &path.to_string(), body)
            .map_err(|e| anyhow!("can not sign request: {}", e))?;

        let header = request.headers_mut();
        header.append("Content-Type", HeaderValue::from_static("application/json"));
        let auth = HeaderValue::from_bytes(format!("Bearer {}", signature).as_bytes());
        match auth {
            Ok(h) => header.append("Authorization", h),
            Err(err) => return Err(anyhow!("can not set auth header: {}", err).into()),
        };

        let resp = self.client.execute(request).await?;
        let status = resp.status();
        if status.as_u16() == crate::err::error_code::AUTHENTICATION as u16 {
            self.notify_authentication_failed();
        }
        let bytes = resp.bytes().await?;
        if !status.is_success() {
            return match self.parse_response::<Value>(&bytes) {
                Err(error) => Err(error),
                Ok(_) => Err(anyhow!("unexpected response status {status}").into()),
            };
        }

        Ok(bytes)
    }

    pub(crate) async fn request<T>(&self, request: Request) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        let text = self.raw_request(request).await?;
        self.parse_response(&text)
    }

    pub(crate) fn parse_response<T>(&self, text: &[u8]) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        let result: MixinResponse = serde_json::from_slice(text)
            .with_context(|| format!("unexpected response: {}", String::from_utf8_lossy(text)))?;
        match result {
            MixinResponse::Data(data) => Ok(serde_json::from_value(data).with_context(|| {
                format!(
                    "failed to parse response: {}",
                    String::from_utf8_lossy(text)
                )
            })?),
            MixinResponse::Error(err) => {
                if matches!(
                    err.code,
                    crate::err::error_code::TIME_INACCURATE | crate::err::error_code::OLD_VERSION
                ) {
                    self.server_error_code.send_replace(Some(err.code));
                }
                if err.code == crate::err::error_code::AUTHENTICATION {
                    self.notify_authentication_failed();
                }
                Err(ApiError::Server(err))
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use log::LevelFilter;
    use simplelog::{Config, TestLogger};
    use tokio::fs;

    use crate::KeyStore;

    use super::*;

    pub async fn new_test_client() -> Client {
        let _ = TestLogger::init(LevelFilter::Trace, Config::default());
        let file = fs::read("../keystore.json")
            .await
            .expect("no keystore file");
        let keystore: KeyStore = serde_json::from_slice(&file).expect("failed to read keystore");
        Client::new(Credential::KeyStore(keystore))
    }

    #[tokio::test]
    async fn authentication_envelope_notifies_subscribers() {
        let client = Client::new(Credential::None);
        let mut authentication_errors = client.subscribe_authentication_errors();

        let error = client
            .inner
            .parse_response::<Value>(
                br#"{"error":{"status":401,"code":401,"description":"Unauthorized"}}"#,
            )
            .unwrap_err();

        assert!(matches!(
            error,
            ApiError::Server(crate::Error { code: 401, .. })
        ));
        authentication_errors.changed().await.unwrap();
        assert!(*authentication_errors.borrow());
    }

    #[tokio::test]
    async fn account_health_errors_notify_subscribers() {
        let client = Client::new(Credential::None);
        let mut server_errors = client.subscribe_server_error_codes();

        let error = client
            .inner
            .parse_response::<Value>(
                br#"{"error":{"status":400,"code":911,"description":"Clock"}}"#,
            )
            .unwrap_err();

        assert!(matches!(
            error,
            ApiError::Server(crate::Error { code: 911, .. })
        ));
        server_errors.changed().await.unwrap();
        assert_eq!(*server_errors.borrow(), Some(911));
    }
}
