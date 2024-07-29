use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::anyhow;
use base64ct::{Base64, Encoding};
use libsignal_protocol::KeyPair;
use log::{info, trace};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rand::rngs::OsRng;
use rand::Rng;
use ring::signature::{Ed25519KeyPair, KeyPair as SignatureKeyPair};
use ring_compat::signature::ecdsa::SigningKey;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use sdk::{Client, Credential, ProvisioningRequest};

use crate::core::crypto::key_help::generate_registration_id;
use crate::core::crypto::provisioning_cipher::decrypt;
use crate::db::app::{AppDatabase, Auth, AuthDao};

pub struct AuthService {
    app_db: Arc<AppDatabase>,
    auth_dao: AuthDao,
    auth: Arc<Mutex<Option<Auth>>>,
}

impl AuthService {
    pub fn new(app_db: Arc<AppDatabase>) -> Self {
        AuthService {
            auth_dao: app_db.auth_dao.clone(),
            app_db,
            auth: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn authorize(&self) -> anyhow::Result<AuthResult> {
        let client = Arc::new(sdk::Client::new(Credential::None));
        let resp = client.provisioning_api.get_provisioning_id("rust").await?;
        let key_pair = KeyPair::generate(&mut rand_core::OsRng);

        let pub_key = utf8_percent_encode(
            &Base64::encode_string(&key_pair.public_key.serialize()),
            NON_ALPHANUMERIC,
        )
        .to_string();

        let url = format!(
            "mixin://device/auth?id={}&pub_key={}",
            resp.device_id, pub_key
        );
        info!("login url: {}", url);

        let time_out = sleep(Duration::from_secs(60));
        tokio::pin!(time_out);
        loop {
            tokio::select! {
                result = check_auth(client.clone(), &resp.device_id, &key_pair) => {
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

    let mut seed = [0u8; 32];
    OsRng.fill(&mut seed);
    let pair = Ed25519KeyPair::from_seed_unchecked(&mut seed)?;

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
            account,
        },
        registration_id,
        identity_key_private: private,
    })
}
