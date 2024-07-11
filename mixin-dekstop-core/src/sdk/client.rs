use reqwest::{Method, Request};
use reqwest::header::HeaderValue;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use serde_json::Value;

use sdk::ApiError;
use sdk::credential::Credential;

use crate::sdk;

pub struct Client {
    credential: Credential,
    pub(crate) base_url: String,
    pub(crate) client: reqwest::Client,
}

const MIXIN_BASE_URL: &str = "https://api.mixin.one";

#[derive(Serialize, Deserialize)]
pub(crate) struct MixinResponse {
    data: Value,
    error: Option<sdk::Error>,
}

impl Client {
    pub fn new(credential: Credential) -> Self {
        return Client {
            credential,
            base_url: MIXIN_BASE_URL.to_string(),
            client: reqwest::Client::new(),
        };
    }

    pub(crate) async fn request<T>(&self, mut request: Request) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        let path = match request.method() {
            &Method::GET => format!("{}?{}", request.url().path(), request.url().query().or(Some("")).unwrap()),
            _ => request.url().path().to_string(),
        };
        let body: &[u8] = match request.method() {
            &Method::POST => request.body().map(|body| -> &[u8]{
                match body.as_bytes() {
                    None => &[],
                    Some(bytes) => bytes,
                }
            }).unwrap_or(&[]),
            _ => &[],
        };
        let signature = self.credential.sign_authentication_token(request.method(), &path.to_string(), &body)?;

        let header = request.headers_mut();
        header.append("Content-Type", HeaderValue::from_static("application/json"));
        let auth = HeaderValue::from_bytes(format!("Bearer {}", signature).as_bytes());
        match auth {
            Ok(h) => header.append("Authorization", h),
            Err(_) => return Err(ApiError::Unknown("can not set auth header".to_string())),
        };

        let resp = self.client.execute(request).await?;
        let text = resp.bytes().await?;

        let result: MixinResponse = serde_json::from_slice(&text)?;
        if result.error.is_some() {
            return Err(ApiError::Server(result.error.unwrap()));
        }
        Ok(serde_json::from_value(result.data)?)
    }
}
