use std::ops::Add;
use std::time::{Duration, SystemTime};

use base64ct::{Base64UrlUnpadded, Encoding};
use jsonwebtoken::{Algorithm, Header};
use reqwest::Method;
use ring::signature::Ed25519KeyPair;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Clone)]
pub enum Credential {
    KeyStore(KeyStore),
    None,
}

#[derive(Clone)]
#[derive(Serialize, Deserialize)]
pub struct KeyStore {
    pub app_id: String,
    pub session_id: String,
    pub server_public_key: String,
    pub session_private_key: String,
}

impl Credential {
    pub(crate) fn sign_authentication_token(&self, method: &Method, path: &String, body: impl AsRef<[u8]>) -> Result<String, String> {
        match self {
            Credential::KeyStore(key_store) => {
                let now = SystemTime::now();
                let expire = now.add(Duration::from_secs(60 * 60 * 24 * 30 * 3));
                let mut cipher = Sha256::new();
                cipher.update(method.as_str());
                cipher.update(path);
                cipher.update(body);
                let sum = cipher.finalize();
                let claims = Claims {
                    uid: key_store.app_id.clone(),
                    sid: key_store.session_id.clone(),
                    iat: now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64,
                    exp: expire.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64,
                    jti: Uuid::new_v4().to_string(),
                    sig: base16ct::lower::encode_string(sum.as_ref()),
                    scp: "FULL".to_string(),
                };
                let header = Header::new(Algorithm::EdDSA);
                let pks = match base16ct::lower::decode_vec(&key_store.session_private_key) {
                    Ok(pk) => pk,
                    Err(err) => return Err(format!("invalid session private key: {}", err))
                };
                let pk = match Ed25519KeyPair::from_seed_unchecked(&pks) {
                    Ok(pk) => pk,
                    Err(err) => return Err(format!("invalid session private key: {}", err))
                };
                let encoded_header = b64_encode_part(&header)?;
                let encoded_claims = b64_encode_part(&claims)?;
                let message = [encoded_header, encoded_claims].join(".");
                let signature = pk.sign(message.as_bytes());
                Ok([message, b64_encode(signature.as_ref())].join("."))
            }
            Credential::None => Ok("".to_string())
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    uid: String,
    sid: String,
    iat: i64,
    exp: i64,
    jti: String,
    sig: String,
    scp: String,
}

pub(crate) fn b64_encode(input: &[u8]) -> String {
    Base64UrlUnpadded::encode_string(input)
}

/// Serializes a struct to JSON and encodes it in base64
pub(crate) fn b64_encode_part<T: Serialize>(input: &T) -> Result<String, String> {
    match serde_json::to_vec(input) {
        Ok(json) => Ok(b64_encode(&json)),
        Err(err) => Err(err.to_string())
    }
}


