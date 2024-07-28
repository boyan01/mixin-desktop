use std::sync::Arc;

use anyhow::anyhow;
use log::debug;
use reqwest::header::HeaderValue;
use reqwest::{Method, Request};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use sdk::credential::Credential;
use sdk::ApiError;

use crate::sdk;
use crate::sdk::api::account_api::AccountApi;
use crate::sdk::api::user_api::UserApi;

pub struct Client {
    inner: Arc<ClientRef>,
    pub user_api: UserApi,
    pub account_api: AccountApi,
}

impl Client {
    pub fn new(credential: Credential) -> Self {
        let inner = Arc::new(ClientRef::new(credential));
        return Client {
            inner: inner.clone(),
            user_api: UserApi::new(inner.clone()),
            account_api: AccountApi::new(inner.clone()),
        };
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
    base_url: String,
    client: reqwest::Client,
}

const MIXIN_BASE_URL: &str = "https://api.mixin.one";

#[derive(Serialize, Deserialize)]
pub(crate) struct MixinResponse {
    data: Value,
    error: Option<sdk::Error>,
}

impl ClientRef {
    pub fn new(credential: Credential) -> Self {
        return ClientRef {
            credential,
            base_url: MIXIN_BASE_URL.to_string(),
            client: reqwest::Client::new(),
        };
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

    pub(crate) async fn request<T>(&self, mut request: Request) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        let path = match request.method() {
            &Method::GET => format!(
                "{}?{}",
                request.url().path(),
                request.url().query().or(Some("")).unwrap()
            ),
            _ => request.url().path().to_string(),
        };
        let body: &[u8] = match request.method() {
            &Method::POST => request
                .body()
                .map(|body| -> &[u8] {
                    match body.as_bytes() {
                        None => &[],
                        Some(bytes) => bytes,
                    }
                })
                .unwrap_or(&[]),
            _ => &[],
        };
        let signature = self
            .credential
            .sign_authentication_token(request.method(), &path.to_string(), &body)
            .map_err(|e| anyhow!("can not sign request: {}", e))?;

        let header = request.headers_mut();
        header.append("Content-Type", HeaderValue::from_static("application/json"));
        let auth = HeaderValue::from_bytes(format!("Bearer {}", signature).as_bytes());
        match auth {
            Ok(h) => header.append("Authorization", h),
            Err(err) => return Err(anyhow!("can not set auth header: {}", err).into()),
        };

        let resp = self.client.execute(request).await?;

        let text = resp.bytes().await?;

        debug!("resp: {}", String::from_utf8_lossy(&text));

        let result: MixinResponse = serde_json::from_slice(&text)?;
        if result.error.is_some() {
            return Err(ApiError::Server(result.error.unwrap()));
        }
        Ok(serde_json::from_value(result.data)?)
    }
}

#[cfg(test)]
pub mod tests {
    use log::LevelFilter;
    use simplelog::{Config, TestLogger};
    use tokio::fs;

    use crate::sdk::KeyStore;

    use super::*;

    pub async fn new_test_client() -> Client {
        let _ = TestLogger::init(LevelFilter::Info, Config::default());
        let file = fs::read("./keystore.json").await.expect("no keystore file");
        let keystore: KeyStore = serde_json::from_slice(&file).expect("failed to read keystore");
        Client::new(Credential::KeyStore(keystore))
    }
}
