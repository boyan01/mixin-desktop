use aes::cipher::{
    block_padding::Pkcs7, consts::U12, BlockModeDecrypt, BlockModeEncrypt, KeyIvInit,
};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes128Gcm, Nonce};
use anyhow::{anyhow, bail, Result};
use cbc::{Decryptor, Encryptor};
use curve25519_dalek::edwards::CompressedEdwardsY;
use ring::signature::{Ed25519KeyPair, KeyPair};
use sha2::{Digest, Sha512};
use uuid::Uuid;
use x25519_dalek::{PublicKey, StaticSecret};

const VERSION: u8 = 1;
const HEADER_SIZE: usize = 35;
const SESSION_ENTRY_SIZE: usize = 64;
const SESSION_ID_SIZE: usize = 16;
const ENCRYPTED_KEY_SIZE: usize = 48;
const CBC_IV_SIZE: usize = 16;
const GCM_NONCE_SIZE: usize = 12;

pub fn decrypt_message(
    private_key: &[u8],
    session_id: &str,
    cipher_text: &[u8],
) -> Result<Vec<u8>> {
    if cipher_text.len() < HEADER_SIZE || cipher_text[0] != VERSION {
        bail!("invalid encrypted message header");
    }

    let session_count = u16::from_le_bytes([cipher_text[1], cipher_text[2]]) as usize;
    let payload_offset = HEADER_SIZE
        .checked_add(
            session_count
                .checked_mul(SESSION_ENTRY_SIZE)
                .ok_or_else(|| anyhow!("invalid session count"))?,
        )
        .ok_or_else(|| anyhow!("invalid encrypted message size"))?;
    if cipher_text.len() < payload_offset + GCM_NONCE_SIZE + 16 {
        bail!("truncated encrypted message");
    }

    let expected_session_id = *Uuid::parse_str(session_id)?.as_bytes();
    let encrypted_key = (0..session_count).find_map(|index| {
        let offset = HEADER_SIZE + index * SESSION_ENTRY_SIZE;
        let id = &cipher_text[offset..offset + SESSION_ID_SIZE];
        (id == expected_session_id).then_some(
            &cipher_text[offset + SESSION_ID_SIZE..offset + SESSION_ID_SIZE + ENCRYPTED_KEY_SIZE],
        )
    });
    let encrypted_key =
        encrypted_key.ok_or_else(|| anyhow!("message has no key for this session"))?;

    let sender_public_key: [u8; 32] = cipher_text[3..HEADER_SIZE]
        .try_into()
        .map_err(|_| anyhow!("invalid sender public key"))?;
    let message_key = decrypt_message_key(private_key, &sender_public_key, encrypted_key)?;

    let nonce_bytes: [u8; GCM_NONCE_SIZE] = cipher_text
        [payload_offset..payload_offset + GCM_NONCE_SIZE]
        .try_into()
        .expect("validated encrypted message nonce length");
    let nonce = Nonce::<U12>::from(nonce_bytes);
    Aes128Gcm::new_from_slice(&message_key)
        .map_err(|error| anyhow!("invalid message key: {error}"))?
        .decrypt(&nonce, &cipher_text[payload_offset + GCM_NONCE_SIZE..])
        .map_err(|_| anyhow!("failed to authenticate encrypted message"))
}

pub fn encrypt_message(
    private_key: &[u8],
    plain_text: &[u8],
    sessions: &[(&str, &[u8])],
) -> Result<Vec<u8>> {
    if sessions.is_empty() || sessions.len() > u16::MAX as usize {
        bail!("encrypted message requires at least one session");
    }

    let mut message_key = [0u8; 16];
    let mut nonce = [0u8; GCM_NONCE_SIZE];
    rand::fill(&mut message_key);
    rand::fill(&mut nonce);

    let nonce_value = Nonce::<U12>::from(nonce);
    let encrypted_payload = Aes128Gcm::new_from_slice(&message_key)
        .map_err(|error| anyhow!("invalid message key: {error}"))?
        .encrypt(&nonce_value, plain_text)
        .map_err(|_| anyhow!("failed to encrypt message"))?;

    let sender_public_key = ed25519_public_to_x25519(private_key)?;
    let mut output = Vec::with_capacity(
        HEADER_SIZE
            + sessions.len() * SESSION_ENTRY_SIZE
            + GCM_NONCE_SIZE
            + encrypted_payload.len(),
    );
    output.push(VERSION);
    output.extend_from_slice(&(sessions.len() as u16).to_le_bytes());
    output.extend_from_slice(&sender_public_key);
    for (session_id, public_key) in sessions {
        output.extend_from_slice(Uuid::parse_str(session_id)?.as_bytes());
        output.extend_from_slice(&encrypt_message_key(private_key, public_key, &message_key)?);
    }
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&encrypted_payload);
    Ok(output)
}

fn decrypt_message_key(
    private_key: &[u8],
    sender_public_key: &[u8; 32],
    encrypted_key: &[u8],
) -> Result<Vec<u8>> {
    if encrypted_key.len() != ENCRYPTED_KEY_SIZE {
        bail!("invalid encrypted message key");
    }
    let secret = shared_secret(private_key, sender_public_key)?;
    Decryptor::<aes::Aes256>::new_from_slices(&secret, &encrypted_key[..CBC_IV_SIZE])
        .map_err(|error| anyhow!("invalid key cipher: {error}"))?
        .decrypt_padded_vec::<Pkcs7>(&encrypted_key[CBC_IV_SIZE..])
        .map_err(|_| anyhow!("failed to decrypt message key"))
}

fn encrypt_message_key(
    private_key: &[u8],
    public_key: &[u8],
    message_key: &[u8],
) -> Result<Vec<u8>> {
    let public_key: [u8; 32] = public_key
        .try_into()
        .map_err(|_| anyhow!("invalid recipient public key"))?;
    let secret = shared_secret(private_key, &public_key)?;
    let mut iv = [0u8; CBC_IV_SIZE];
    rand::fill(&mut iv);
    let encrypted = Encryptor::<aes::Aes256>::new_from_slices(&secret, &iv)
        .map_err(|error| anyhow!("invalid key cipher: {error}"))?
        .encrypt_padded_vec::<Pkcs7>(message_key);
    Ok([iv.as_slice(), encrypted.as_slice()].concat())
}

fn shared_secret(private_key: &[u8], public_key: &[u8; 32]) -> Result<[u8; 32]> {
    let private_key = ed25519_private_to_x25519(private_key)?;
    Ok(StaticSecret::from(private_key)
        .diffie_hellman(&PublicKey::from(*public_key))
        .to_bytes())
}

fn ed25519_private_to_x25519(private_key: &[u8]) -> Result<[u8; 32]> {
    let seed: &[u8; 32] = private_key
        .get(..32)
        .ok_or_else(|| anyhow!("invalid Ed25519 private key"))?
        .try_into()
        .map_err(|_| anyhow!("invalid Ed25519 private key"))?;
    let digest = Sha512::digest(seed);
    let mut key: [u8; 32] = digest[..32].try_into().expect("SHA-512 prefix length");
    key[0] &= 248;
    key[31] &= 127;
    key[31] |= 64;
    Ok(key)
}

fn ed25519_public_to_x25519(private_key: &[u8]) -> Result<[u8; 32]> {
    let seed = private_key
        .get(..32)
        .ok_or_else(|| anyhow!("invalid Ed25519 private key"))?;
    let pair = Ed25519KeyPair::from_seed_unchecked(seed)
        .map_err(|_| anyhow!("invalid Ed25519 private key"))?;
    let compressed: [u8; 32] = pair
        .public_key()
        .as_ref()
        .try_into()
        .map_err(|_| anyhow!("invalid Ed25519 public key"))?;
    CompressedEdwardsY(compressed)
        .decompress()
        .map(|point| point.to_montgomery().to_bytes())
        .ok_or_else(|| anyhow!("invalid Ed25519 public key"))
}

#[cfg(test)]
mod tests {
    use base64ct::{Base64, Encoding};

    use super::*;

    #[test]
    fn encrypted_message_round_trip_for_selected_session() {
        let alice = [7u8; 32];
        let bob = [9u8; 32];
        let bob_public = ed25519_public_to_x25519(&bob).unwrap();
        let session_id = "11111111-2222-4333-8444-555555555555";

        let encrypted =
            encrypt_message(&alice, b"hello mixin", &[(session_id, &bob_public)]).unwrap();
        let decrypted = decrypt_message(&bob, session_id, &encrypted).unwrap();

        assert_eq!(decrypted, b"hello mixin");
        assert!(decrypt_message(&bob, "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", &encrypted).is_err());
    }

    #[test]
    fn decrypts_flutter_encrypted_protocol_fixture() {
        // Generated by flutter-app's EncryptedProtocol with fixed Ed25519 seeds.
        let cipher = Base64::decode_vec(
            "AQEAdh2I7IMEE5Gd/p1NHVbxfmU8jJlAgt9bE3uQoK5u33QRERERIiJDM4REVVVVVVVVgM31Zj4d/k7b4NPv6N70PjaC+26FhO+gIR6N5iqP4PkqKzXnJMrbDrme5tfyLBzWHthkCsEPuTPoeLZX493L25NmW+EQ7ujM1pkSra2r6FXKMZGodqdu",
        )
        .unwrap();
        let bob_private = Base64::decode_vec(
            "CQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQn9FyQ4WqDHW2T7eM1gL6HZkf3r92sTxY7XAurINen2GA==",
        )
        .unwrap();

        let decrypted = decrypt_message(
            &bob_private,
            "11111111-2222-4333-8444-555555555555",
            &cipher,
        )
        .unwrap();

        assert_eq!(decrypted, b"hello mixin");
    }
}
