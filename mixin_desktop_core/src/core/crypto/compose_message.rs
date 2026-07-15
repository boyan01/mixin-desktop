use anyhow::Result;
use anyhow::{anyhow, bail};
use base64ct::{Base64, Encoding};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposeMessageData {
    pub key_type: u8,
    pub cipher: Vec<u8>,
    pub resend_message_id: Option<String>,
}

const CURRENT_VERSION: u8 = 3;

impl ComposeMessageData {
    pub fn decode(encoded: &str) -> Result<ComposeMessageData> {
        if encoded.is_empty() {
            bail!("Empty message");
        }
        let cipher_text = Base64::decode_vec(encoded)?;

        let header = cipher_text
            .get(0..8)
            .ok_or_else(|| anyhow!("Invalid header length"))?;

        if header[0] != CURRENT_VERSION {
            bail!("Invalid message version: {}", header[0]);
        }

        let data_type = header[1];
        let is_resend_message = header[2] == 1;

        let (resend_message_id, data) = if is_resend_message {
            let message_id = cipher_text
                .get(8..44)
                .ok_or_else(|| anyhow!("Invalid resend message length"))?;
            let data = cipher_text
                .get(44..)
                .ok_or_else(|| anyhow!("Invalid resend message length"))?;
            (Some(String::from_utf8_lossy(message_id).to_string()), data)
        } else {
            (None, &cipher_text[8..])
        };

        Ok(ComposeMessageData {
            key_type: data_type,
            resend_message_id,
            cipher: data.to_vec(),
        })
    }

    pub fn encode(&self) -> String {
        let mut message = vec![CURRENT_VERSION, self.key_type];
        if let Some(resend_message_id) = &self.resend_message_id {
            message.extend([1, 0, 0, 0, 0, 0]);
            message.extend_from_slice(resend_message_id.as_bytes());
        } else {
            message.extend([0, 0, 0, 0, 0, 0]);
        }
        message.extend_from_slice(&self.cipher);
        Base64::encode_string(&message)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_compose_message() {
        let data = ComposeMessageData {
            key_type: 0,
            cipher: vec![1, 2, 3, 4],
            resend_message_id: None,
        };
        let encoded = data.encode();
        let decoded = ComposeMessageData::decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn rejects_truncated_resend_message_without_panicking() {
        let encoded = Base64::encode_string(&[CURRENT_VERSION, 3, 1, 0, 0, 0, 0, 0, 1]);

        assert!(ComposeMessageData::decode(&encoded).is_err());
    }
}
