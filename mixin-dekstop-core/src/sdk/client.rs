use reqwest::Method;
use serde_json::Value;
use sha2::Digest;

use sdk::ApiError;
use sdk::credential::Credential;

use crate::sdk;

pub struct Client {
    credential: Credential,
    base_url: String,
    client: reqwest::Client,
}

const MIXIN_BASE_URL: &str = "https://api.mixin.one";


impl Client {
    pub fn new(credential: Credential) -> Self {
        return Client {
            credential,
            base_url: MIXIN_BASE_URL.to_string(),
            client: reqwest::Client::new(),
        };
    }

    fn get() {}
}

impl Client {
    pub async fn get_me(&self) -> Result<String, ApiError> {
        let c = reqwest::Client::new();
        let url = format!("{}/{}", self.base_url, "me");
        let sign = self.credential.sign_authentication_token(Method::GET, &"/me".to_string(), &"".to_string())?;
        let response = c.get(url)
            .header("Authorization", format!("Bearer {}", sign))
            .header("Content-Type", "application/json")
            .send().await?;
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            let err: sdk::Error = serde_json::from_slice(text.as_bytes())?;
            return Err(err.into());
        }
        let value: Value = serde_json::from_slice(text.as_bytes())?;
        let err = &value["error"];
        println!("value {}", serde_json::to_string(err).unwrap());
        if !err.is_null() {
            let err: sdk::Error = serde_json::from_value(err.clone())?;
            return Err(err.into());
        }
        Ok(text)
    }
}

