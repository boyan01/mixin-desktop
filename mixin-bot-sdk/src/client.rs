use std::sync::Arc;

use anyhow::{anyhow, Context};
use bytes::Bytes;
use reqwest::header::HeaderValue;
use reqwest::{Method, Request};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::credential::Credential;
use crate::{
    AccountApi, ApiError, CircleApi, ConversationApi, MessageApi, ProvisioningApi, TokenApi,
    UserApi,
};

pub struct Client {
    inner: Arc<ClientRef>,
    pub user_api: UserApi,
    pub account_api: AccountApi,
    pub provisioning_api: ProvisioningApi,
    pub token_api: TokenApi,
    pub conversation_api: ConversationApi,
    pub circle_api: CircleApi,
    pub message_api: MessageApi,
}

impl Client {
    pub fn new(credential: Credential) -> Self {
        let inner = Arc::new(ClientRef::new(credential));
        Client {
            inner: inner.clone(),
            user_api: UserApi::new(inner.clone()),
            account_api: AccountApi::new(inner.clone()),
            provisioning_api: ProvisioningApi::new(inner.clone()),
            token_api: TokenApi::new(inner.clone()),
            conversation_api: ConversationApi::new(inner.clone()),
            circle_api: CircleApi {
                client: inner.clone(),
            },
            message_api: MessageApi {
                client: inner.clone(),
            },
        }
    }

    pub async fn request<T>(&self, request: Request) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        self.inner.request(request).await
    }
}

pub(crate) struct ClientRef {
    credential: Credential,
    pub(crate) base_url: String,
    pub(crate) client: reqwest::Client,
}

const MIXIN_BASE_URL: &str = "https://api.mixin.one";

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
            client: reqwest::Client::new(),
        }
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

        Ok(resp.bytes().await?)
    }

    pub(crate) async fn request<T>(&self, request: Request) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        let text = self.raw_request(request).await?;
        let result: MixinResponse = serde_json::from_slice(&text)
            .with_context(|| format!("unexpected response: {}", String::from_utf8_lossy(&text)))?;
        match result {
            MixinResponse::Data(data) => Ok(serde_json::from_value(data).with_context(|| {
                format!(
                    "failed to parse response: {}",
                    String::from_utf8_lossy(&text)
                )
            })?),
            MixinResponse::Error(err) => Err(ApiError::Server(err)),
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
}
