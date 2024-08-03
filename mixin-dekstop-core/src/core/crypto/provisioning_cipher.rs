use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use anyhow::anyhow;
use base64ct::{Base64, Encoding};
use libsignal_protocol::{PrivateKey, PublicKey, SignalProtocolError, HKDF};
use ring::hmac;
use ring::hmac::HMAC_SHA256;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const PROVISION: &str = "Mixin Provisioning Message";

#[derive(Debug, Serialize, Deserialize)]
struct ProvisionEnvelope {
    public_key: String,
    body: String,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("json serialization: {0}")]
    Json(#[from] serde_json::Error),
    #[error("base64 conversion: {0}")]
    Base64(#[from] base64ct::Error),
    #[error("error: {0}")]
    Other(#[from] anyhow::Error),
}

impl From<SignalProtocolError> for Error {
    fn from(value: SignalProtocolError) -> Self {
        Self::Other(anyhow!("signal protocol error: {:?}", value))
    }
}

type Result<T> = std::result::Result<T, Error>;

pub fn decrypt(private_key: PrivateKey, content: &str) -> Result<Vec<u8>> {
    let content = Base64::decode_vec(content)?;
    let envelop: ProvisionEnvelope = serde_json::from_slice(&content)?;

    let public_key = Base64::decode_vec(&envelop.public_key)?;
    let public_key = PublicKey::deserialize(&public_key)?;
    let message = Base64::decode_vec(&envelop.body)?;
    if message.first() != Some(&1u8) {
        return Err(Error::Other(anyhow::anyhow!("invalid message type")));
    }

    let iv_and_cipher_text = &message[0..message.len() - 32];
    let mac = &message[message.len() - 32..];

    let iv = &message[1..17];
    let cipher_text = &message[17..message.len() - 32];

    let shared_secret = private_key.calculate_agreement(&public_key)?;

    let derived_secret = HKDF::new(3)?.derive_secrets(&shared_secret, PROVISION.as_bytes(), 64)?;

    let aes_key = &derived_secret[0..32];
    let hmac_key = &derived_secret[32..];

    if !verify_mac(hmac_key, iv_and_cipher_text, mac) {
        return Err(Error::Other(anyhow::anyhow!("invalid mac: {:?}", mac)));
    }

    let plain_text = aes_256_cbc_decrypt(aes_key, iv, cipher_text)?;
    Ok(plain_text)
}

pub fn verify_mac(key: &[u8], message: &[u8], mac: &[u8]) -> bool {
    let key = hmac::Key::new(HMAC_SHA256, key);
    hmac::verify(&key, message, mac).is_ok()
}

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

pub fn aes_256_cbc_encrypt(key: &[u8], iv: &[u8], plain_text: &[u8]) -> Result<Vec<u8>> {
    if key.len() != 32 {
        return Err(Error::Other(anyhow!(
            "invalid aes key length: {}",
            key.len()
        )));
    }
    if iv.len() != 16 {
        return Err(Error::Other(anyhow!("invalid aes iv length: {}", iv.len())));
    }
    Ok(Aes256CbcEnc::new(key.into(), iv.into()).encrypt_padded_vec_mut::<Pkcs7>(plain_text))
}

pub fn aes_256_cbc_decrypt(key: &[u8], iv: &[u8], cipher_text: &[u8]) -> Result<Vec<u8>> {
    if key.len() != 32 {
        return Err(Error::Other(anyhow!(
            "invalid aes key length: {}",
            key.len()
        )));
    }
    if iv.len() != 16 {
        return Err(Error::Other(anyhow!("invalid aes iv length: {}", iv.len())));
    }
    Aes256CbcDec::new(key.into(), iv.into())
        .decrypt_padded_vec_mut::<Pkcs7>(cipher_text)
        .map_err(|e| Error::Other(anyhow!("aes256 failed to decrypt: {:?}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_cbc_test() -> Result<()> {
        let key = hex::decode("4e22eb16d964779994222e82192ce9f747da72dc4abe49dfdeeb71d0ffe3796e")
            .expect("valid hex");
        let iv = hex::decode("6f8a557ddc0a140c878063a6d5f31d3d").expect("valid hex");

        let ptext = hex::decode("30736294a124482a4159").expect("valid hex");

        let ctext = aes_256_cbc_encrypt(&key, &iv, &ptext)?;
        assert_eq!(
            hex::encode(ctext.clone()),
            "dd3f573ab4508b9ed0e45e0baf5608f3"
        );

        let recovered = aes_256_cbc_decrypt(&key, &iv, &ctext)?;
        assert_eq!(hex::encode(ptext), hex::encode(recovered.clone()));

        // padding is invalid:
        assert!(super::aes_256_cbc_decrypt(&key, &iv, &recovered,).is_err());
        assert!(super::aes_256_cbc_decrypt(&ctext, &key, &ctext).is_err());

        // bitflip the IV to cause a change in the recovered text
        let bad_iv = hex::decode("ef8a557ddc0a140c878063a6d5f31d3d").expect("valid hex");
        let recovered = aes_256_cbc_decrypt(&key, &bad_iv, &ctext)?;
        assert_eq!(hex::encode(recovered), "b0736294a124482a4159");

        Ok(())
    }

    #[test]
    fn test_decrypt() {
        let private_key = "GHtZBYTNqbCofFo0keD3jTGoHF6bUAeiW9iV5ad/HHA=";
        let content =
        "eyJwdWJsaWNfa2V5IjoiQmZFOWJFa3EzZ2FsUTFHTnVEMWlJaHBrSkE0RHRTVUxkYXhkS3JiZndMcDgiLCJib2R5IjoiQVd6YWZJSDEyZ2tTQmRjdVplSXRMM3lXVmhZdW1lemppZk9rbFdsV3lnQzVKSXpzMHVxYXFZNnhsdFJzVWJya2N0NGFLazVKSmxwYlBKSXNqTU5qYXZKR1hNSFpOSnQ2SXZ1S1pkUVwvN1RzbjRUUjhmWjNSUXo2aGM3RlpYZENLTUJMVzJHTjAzSDd3aUt1elArZVV4WktCQjFPb2pWaVlSU0Vyc3dUWGwxeWR5cnhcLzM1NVY5MzFCR0N1VlBLVXFuOUJFWCtidVFhYms3YWZYdEVCOUI2YTZGSnFCZXVGcHdcLzlDdWpwYVpXNzNIMmswTmxkNjdPMzB5QkZEM3RuNmtiaXZ3MzNjN0l2Uk9EYlwvQnFTS1NGSlJoMUE2eU1leTAyeHZkNkJpRkxja1FRQk9LXC9ROW5ZOExoM2VlS2FNNTQ3cVV3XC9qUEE0ZGE5TzI0RkJUXC9ON2NvR2dBOVkrN1pvZ0syQ3YzbDZCNG9CN0xyTCtrVlRWTHJ2MFA1aDF3YklrUm10YWRLQmxiTjVvK3RnVUZ5VnVNcWFXQVJ6QnBJNlVDajVaZ0JJVWJWM3N0enVwUXpYU0E5OVBWV3hXOTE2dz09In0=";

        let private_key =
            PrivateKey::deserialize(&Base64::decode_vec(private_key).unwrap()).unwrap();
        let plain_text = decrypt(private_key, content).unwrap();
        let plain_text = String::from_utf8_lossy(&plain_text);

        let r = r#"{"session_id":"304745a1-45af-4045-bd16-ba4d42a03a4e","platform":"iOS","user_id":"f59b9309-70c2-4b69-8fd8-5773dbd10018","identity_key_public":"BTEcMFj5uvP+32z+avKFOjDOrMvmnoDmwMfPZcuxBT08","identity_key_private":"iG5ilNnI8dkqtslK84NWWmPUhzADyUm6odlwA96isEk=","provisioning_code":"7972"}"#;
        assert_eq!(plain_text, r);
    }
}
