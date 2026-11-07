use anyhow::{anyhow, Result};
use base64ct::{Base64UrlUnpadded, Encoding};
use hmac::{Hmac, KeyInit as HmacKeyInit, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

use crate::core::crypto::encrypted_protocol::ed25519_private_to_x25519;

pub(super) fn matches_resource(page: &str, patterns: &[String]) -> bool {
    let Some(page) = super::web_url(page) else {
        return false;
    };
    patterns.iter().any(|pattern| {
        super::web_url(pattern.trim()).is_some_and(|rule| {
            rule.scheme() == page.scheme() && rule.host_str() == page.host_str()
                // Flutter currently omits the port. Match the actual origin here.
                && rule.port_or_known_default() == page.port_or_known_default()
                && page.path().starts_with(rule.path())
        })
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn sign(
    account_id: &str,
    private_key: &[u8],
    bot_public_key: &[u8; 32],
    timestamp: i64,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<String> {
    let private_key = Zeroizing::new(ed25519_private_to_x25519(private_key)?);
    let shared = Zeroizing::new(
        StaticSecret::from(*private_key)
            .diffie_hellman(&PublicKey::from(*bot_public_key))
            .to_bytes(),
    );
    if bool::from(shared.as_ref().ct_eq(&[0_u8; 32])) {
        return Err(anyhow!("bot public key has no contributory agreement"));
    }
    let mut mac = <Hmac<Sha256> as HmacKeyInit>::new_from_slice(shared.as_ref())
        .map_err(|_| anyhow!("invalid bot agreement key"))?;
    mac.update(timestamp.to_string().as_bytes());
    mac.update(method.as_bytes());
    mac.update(path.as_bytes());
    if let Some(body) = body.filter(|body| !body.trim().is_empty()) {
        mac.update(body.as_bytes());
    }
    let mut payload = Vec::with_capacity(account_id.len() + 32);
    payload.extend_from_slice(account_id.as_bytes());
    payload.extend_from_slice(&mac.finalize().into_bytes());
    Ok(Base64UrlUnpadded::encode_string(&payload))
}
