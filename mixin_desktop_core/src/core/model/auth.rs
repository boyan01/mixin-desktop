use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::anyhow;
use base64ct::{Base64, Encoding};
use libsignal_protocol::KeyPair;
use log::info;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use ring::signature::{Ed25519KeyPair, KeyPair as SignatureKeyPair};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use sdk::{Client, Credential, ProvisioningRequest};

use crate::core::crypto::key_help::generate_registration_id;
use crate::core::crypto::provisioning_cipher::decrypt;
use crate::db::app::{AppDatabase, Auth, AuthDao};

pub struct AuthService {
    auth_dao: AuthDao,
    auth: Arc<Mutex<Option<Auth>>>,
}

pub struct AuthorizationSession {
    device_id: String,
    key_pair: KeyPair,
    auth_url: String,
}

impl AuthorizationSession {
    pub fn auth_url(&self) -> &str {
        &self.auth_url
    }
}

impl AuthService {
    pub fn new(app_db: Arc<AppDatabase>) -> Self {
        AuthService {
            auth_dao: app_db.auth_dao.clone(),
            auth: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn authorize(&self) -> anyhow::Result<AuthResult> {
        let session = self.begin_authorization("rust").await?;
        info!("login url: {}", session.auth_url());

        let time_out = sleep(Duration::from_secs(60));
        tokio::pin!(time_out);
        loop {
            tokio::select! {
                result = self.poll_authorization(&session) => {
                    match result {
                        Ok(auth) => {
                            if let Some(auth) = auth {
                                return Ok(auth);
                            }
                        }
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }
                _ = &mut time_out => {
                    info!("time out");
                    break;
                }
                _ = tokio::signal::ctrl_c() => {
                    break;
                }
            }
            sleep(Duration::from_secs(1)).await;
        }

        Err(anyhow!("need auth"))
    }

    pub async fn begin_authorization(
        &self,
        platform: &str,
    ) -> anyhow::Result<AuthorizationSession> {
        let client = sdk::Client::new(Credential::None);
        let response = client
            .provisioning_api
            .get_provisioning_id(platform)
            .await?;
        let key_pair = KeyPair::generate(&mut rand_core::OsRng);
        let public_key = utf8_percent_encode(
            &Base64::encode_string(&key_pair.public_key.serialize()),
            NON_ALPHANUMERIC,
        )
        .to_string();
        let auth_url = format!(
            "mixin://device/auth?id={}&pub_key={}",
            response.device_id, public_key
        );

        Ok(AuthorizationSession {
            device_id: response.device_id,
            key_pair,
            auth_url,
        })
    }

    pub async fn poll_authorization(
        &self,
        session: &AuthorizationSession,
    ) -> anyhow::Result<Option<AuthResult>> {
        let client = Arc::new(sdk::Client::new(Credential::None));
        check_auth(client, &session.device_id, &session.key_pair).await
    }

    pub async fn complete_authorization(&self, result: AuthResult) -> anyhow::Result<Auth> {
        let identity_number = result.auth.account.identity_number.clone();
        let signal_database = crate::db::SignalDatabase::connect(identity_number)
            .await
            .map_err(|error| anyhow!(error.to_string()))?;
        signal_database
            .init(result.registration_id, Some(&result.identity_key_private))
            .await?;
        self.save_auth(&result.auth).await?;
        Ok(result.auth)
    }

    pub async fn initialize(&self) -> anyhow::Result<()> {
        let auth_list = self.auth_dao.find_all_auth().await?;
        let Some(auth) = auth_list.first() else {
            return Ok(());
        };

        let mut a = self.auth.lock().unwrap();
        *a = Some(auth.clone());

        Ok(())
    }

    pub fn get_auth(&self) -> Option<Auth> {
        self.auth.lock().unwrap().clone()
    }

    pub fn has_auth(&self) -> bool {
        self.auth.lock().unwrap().is_some()
    }

    pub async fn save_auth(&self, auth: &Auth) -> anyhow::Result<()> {
        {
            let mut a = self.auth.lock().unwrap();
            *a = Some(auth.clone());
        }

        self.auth_dao.save_auth(auth).await?;
        Ok(())
    }

    pub async fn clear_auth(&self, id: &str) -> anyhow::Result<()> {
        self.auth_dao.remove_auth(id).await?;

        let mut a = self.auth.lock().unwrap();
        *a = None;

        Ok(())
    }
}

async fn check_auth(
    client: Arc<Client>,
    device_id: &str,
    key_pair: &KeyPair,
) -> anyhow::Result<Option<AuthResult>> {
    info!("check auth: {}", device_id);
    let secret = client.provisioning_api.get_provisioning(device_id).await;

    let Ok(secret) = secret.map(|s| s.secret) else {
        return Ok(None);
    };

    if secret.is_empty() {
        return Ok(None);
    }

    let auth = verify_auth(&client, &secret, key_pair).await?;
    Ok(Some(auth))
}

#[derive(Debug, Serialize, Deserialize)]
struct ProvisioningVerification {
    identity_key_private: String,
    session_id: String,
    provisioning_code: String,
    user_id: String,
}

pub struct AuthResult {
    pub auth: Auth,
    pub registration_id: u32,
    pub identity_key_private: Vec<u8>,
}

async fn verify_auth(
    client: &Client,
    secret: &str,
    key_pair: &KeyPair,
) -> anyhow::Result<AuthResult> {
    let result = decrypt(key_pair.private_key, secret)?;
    let verification: ProvisioningVerification = serde_json::from_slice(&result)?;
    if verification.session_id.trim().is_empty() {
        return Err(anyhow!("provisioning response has no primary session id"));
    }
    let primary_session_id = verification.session_id.clone();

    let mut seed = [0u8; 32];
    rand::fill(&mut seed);
    let pair = Ed25519KeyPair::from_seed_unchecked(&seed)
        .map_err(|_| anyhow!("failed to create Ed25519 session key"))?;

    let private = Base64::decode_vec(&verification.identity_key_private)?;
    let registration_id = generate_registration_id(false) as u32;

    let account = client
        .provisioning_api
        .verify_provisioning(&ProvisioningRequest {
            code: verification.provisioning_code,
            user_id: verification.user_id,
            session_id: verification.session_id,
            session_secret: Base64::encode_string(pair.public_key().as_ref()),
            platform: "Desktop".to_string(),
            platform_version: "MacOS 14.5".to_string(),
            app_version: "1.9.1(200)".to_string(),
            purpose: "SESSION".to_string(),
            registration_id,
        })
        .await?;

    Ok(AuthResult {
        auth: Auth {
            user_id: account.user_id.clone(),
            private_key: seed.to_vec(),
            primary_session_id: Some(primary_session_id),
            account,
        },
        registration_id,
        identity_key_private: private,
    })
}
