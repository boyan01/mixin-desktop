use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use aes::cipher::{Block, BlockDecrypt, KeyInit};
use aes::Aes256;
use anyhow::{anyhow, bail, Context, Result};
use futures::StreamExt;
use hmac::{Hmac, Mac};
use reqwest::header::CONTENT_TYPE;
use reqwest::Client as HttpClient;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use sdk::message_category::MessageCategory;
use sdk::Client as MixinClient;

use crate::core::model::AttachmentExtra;
use crate::db::mixin::message::{MediaStatus, Message};

const AES_KEY_SIZE: usize = 32;
const MAC_KEY_SIZE: usize = 32;
const ATTACHMENT_KEY_SIZE: usize = AES_KEY_SIZE + MAC_KEY_SIZE;
const CBC_BLOCK_SIZE: usize = 16;
const MAC_SIZE: usize = 32;
const IO_BUFFER_SIZE: usize = 64 * 1024;

pub struct AttachmentService {
    mixin_client: Arc<MixinClient>,
    http_client: HttpClient,
    account_data_dir: PathBuf,
}

#[derive(Debug)]
pub struct AttachmentDownloadResult {
    pub path: PathBuf,
    pub size: i64,
    pub status: MediaStatus,
    pub attachment: AttachmentExtra,
}

impl AttachmentService {
    pub fn new(
        mixin_client: Arc<MixinClient>,
        http_client: HttpClient,
        account_data_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            mixin_client,
            http_client,
            account_data_dir: account_data_dir.into(),
        }
    }

    pub async fn download(
        &self,
        message: &Message,
        extra: &AttachmentExtra,
    ) -> Result<AttachmentDownloadResult> {
        self.download_to(message, extra, false).await
    }

    pub async fn download_transcript(
        &self,
        message: &Message,
        extra: &AttachmentExtra,
    ) -> Result<AttachmentDownloadResult> {
        self.download_to(message, extra, true).await
    }

    async fn download_to(
        &self,
        message: &Message,
        extra: &AttachmentExtra,
        transcript: bool,
    ) -> Result<AttachmentDownloadResult> {
        validate_message(message, extra)?;
        let attachment = self
            .mixin_client
            .attachment_api
            .get_attachment(&extra.attachment_id)
            .await?;
        if attachment.attachment_id != extra.attachment_id {
            bail!("attachment response id does not match the requested id");
        }
        let view_url = attachment
            .view_url
            .as_deref()
            .filter(|url| !url.trim().is_empty())
            .ok_or_else(|| anyhow!("attachment has no view URL"))?;

        let target = if transcript {
            transcript_attachment_path(&self.account_data_dir, message)?
        } else {
            attachment_path(&self.account_data_dir, message)?
        };
        let directory = target
            .parent()
            .ok_or_else(|| anyhow!("attachment target has no parent directory"))?;
        tokio::fs::create_dir_all(directory).await?;

        let download_temp = temp_path(&target, "download")?;
        let output_temp = temp_path(&target, "part")?;
        let operation = async {
            match (&message.media_key, &message.media_digest) {
                (Some(key), Some(digest)) => {
                    validate_encryption_material(key, digest)?;
                    self.download_to_file(view_url, &download_temp).await?;

                    let input = download_temp.clone();
                    let output = output_temp.clone();
                    let key = key.clone();
                    let digest = digest.clone();
                    tokio::task::spawn_blocking(move || {
                        decrypt_attachment_file(&input, &output, &key, &digest)
                    })
                    .await
                    .context("attachment decrypt task failed")??;
                    tokio::fs::remove_file(&download_temp).await?;
                }
                (None, None) => self.download_to_file(view_url, &output_temp).await?,
                _ => bail!("attachment key and digest must be provided together"),
            }

            let size = tokio::fs::metadata(&output_temp).await?.len();
            let size = i64::try_from(size).context("attachment is too large")?;
            tokio::fs::rename(&output_temp, &target)
                .await
                .with_context(|| format!("move attachment into {}", target.display()))?;
            Ok::<_, anyhow::Error>(size)
        }
        .await;

        let _ = tokio::fs::remove_file(&download_temp).await;
        if operation.is_err() {
            let _ = tokio::fs::remove_file(&output_temp).await;
        }
        let size = operation?;

        Ok(AttachmentDownloadResult {
            path: target,
            size,
            status: MediaStatus::Done,
            attachment: AttachmentExtra {
                attachment_id: attachment.attachment_id,
                message_id: message.message_id.clone(),
                shareable: extra.shareable,
                created_at: Some(attachment.created_at),
            },
        })
    }

    async fn download_to_file(&self, view_url: &str, path: &Path) -> Result<()> {
        let response = self
            .http_client
            .get(view_url)
            .header(CONTENT_TYPE, "application/octet-stream")
            .send()
            .await?
            .error_for_status()?;
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .await
            .with_context(|| format!("create attachment temporary file {}", path.display()))?;

        while let Some(chunk) = stream.next().await {
            file.write_all(&chunk?).await?;
        }
        file.flush().await?;
        file.sync_all().await?;
        Ok(())
    }
}

fn validate_message(message: &Message, extra: &AttachmentExtra) -> Result<()> {
    if !message.category.is_attachment() {
        bail!("message category is not an attachment");
    }
    validate_path_component("message id", &message.message_id)?;
    validate_path_component("conversation id", &message.conversation_id)?;
    if extra.attachment_id.trim().is_empty() {
        bail!("attachment id is empty");
    }
    if extra.message_id != message.message_id {
        bail!("attachment message id does not match the message");
    }
    Ok(())
}

fn validate_path_component(label: &str, value: &str) -> Result<()> {
    let mut components = Path::new(value).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        bail!("invalid {label}");
    }
    Ok(())
}

fn attachment_path(account_data_dir: &Path, message: &Message) -> Result<PathBuf> {
    let media_type = if message.category.is_image() {
        "Images"
    } else if message.category.is_video() {
        "Videos"
    } else if message.category.is_audio() {
        "Audios"
    } else if message.category.is_data() {
        "Files"
    } else {
        bail!("message category is not an attachment");
    };
    let suffix = attachment_suffix(message);
    Ok(account_data_dir
        .join("Media")
        .join(media_type)
        .join(&message.conversation_id)
        .join(format!("{}{}", message.message_id, suffix)))
}

fn transcript_attachment_path(account_data_dir: &Path, message: &Message) -> Result<PathBuf> {
    let suffix = attachment_suffix(message);
    Ok(account_data_dir
        .join("Media")
        .join("Transcripts")
        .join(format!("{}{}", message.message_id, suffix)))
}

fn attachment_suffix(message: &Message) -> String {
    if message.category.is_image() {
        return match message
            .media_mime_type
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "image/png" => ".png",
            "image/gif" => ".gif",
            "image/webp" => ".webp",
            _ => ".jpg",
        }
        .to_string();
    }
    if message.category.is_video() {
        return ".mp4".to_string();
    }
    if message.category.is_audio() {
        return ".ogg".to_string();
    }

    message
        .name
        .as_deref()
        .and_then(|name| Path::new(name).extension())
        .and_then(|extension| extension.to_str())
        .filter(|extension| {
            !extension.is_empty()
                && extension
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        })
        .map(|extension| format!(".{extension}"))
        .unwrap_or_default()
}

fn temp_path(target: &Path, kind: &str) -> Result<PathBuf> {
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("attachment target has no file name"))?;
    Ok(target.with_file_name(format!(".{name}.{}.{}", uuid::Uuid::new_v4(), kind)))
}

fn validate_encryption_material(key: &[u8], digest: &[u8]) -> Result<()> {
    if key.len() < ATTACHMENT_KEY_SIZE {
        bail!("attachment key must contain 64 bytes");
    }
    if digest.len() != Sha256::output_size() {
        bail!("attachment digest must contain 32 bytes");
    }
    Ok(())
}

fn decrypt_attachment_file(input: &Path, output: &Path, key: &[u8], digest: &[u8]) -> Result<()> {
    validate_encryption_material(key, digest)?;
    let input_size = File::open(input)?.metadata()?.len();
    let minimum_size = CBC_BLOCK_SIZE + CBC_BLOCK_SIZE + MAC_SIZE;
    if input_size < minimum_size as u64 {
        bail!("encrypted attachment is truncated");
    }
    let input_size_usize = usize::try_from(input_size).context("attachment is too large")?;
    let ciphertext_size = input_size_usize - CBC_BLOCK_SIZE - MAC_SIZE;
    if ciphertext_size == 0 || !ciphertext_size.is_multiple_of(CBC_BLOCK_SIZE) {
        bail!("encrypted attachment has invalid CBC length");
    }

    verify_attachment(
        input,
        input_size,
        &key[AES_KEY_SIZE..ATTACHMENT_KEY_SIZE],
        digest,
    )?;

    let mut reader = BufReader::with_capacity(IO_BUFFER_SIZE, File::open(input)?);
    let mut previous = [0u8; CBC_BLOCK_SIZE];
    reader.read_exact(&mut previous)?;
    let cipher = Aes256::new_from_slice(&key[..AES_KEY_SIZE])
        .map_err(|_| anyhow!("invalid attachment AES key"))?;
    let block_count = ciphertext_size / CBC_BLOCK_SIZE;
    let mut writer = BufWriter::with_capacity(
        IO_BUFFER_SIZE,
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(output)?,
    );

    for index in 0..block_count {
        let mut encrypted = [0u8; CBC_BLOCK_SIZE];
        reader.read_exact(&mut encrypted)?;
        let mut block = Block::<Aes256>::clone_from_slice(&encrypted);
        cipher.decrypt_block(&mut block);
        for (byte, previous_byte) in block.iter_mut().zip(previous) {
            *byte ^= previous_byte;
        }

        if index + 1 == block_count {
            let padding = *block.last().unwrap() as usize;
            if padding == 0
                || padding > CBC_BLOCK_SIZE
                || !block[CBC_BLOCK_SIZE - padding..]
                    .iter()
                    .all(|byte| *byte as usize == padding)
            {
                bail!("attachment has invalid PKCS7 padding");
            }
            writer.write_all(&block[..CBC_BLOCK_SIZE - padding])?;
        } else {
            writer.write_all(&block)?;
        }
        previous = encrypted;
    }

    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok(())
}

fn verify_attachment(input: &Path, input_size: u64, mac_key: &[u8], digest: &[u8]) -> Result<()> {
    let authenticated_size = input_size - MAC_SIZE as u64;
    let mut reader = BufReader::with_capacity(IO_BUFFER_SIZE, File::open(input)?);
    let mut hmac = <Hmac<Sha256> as Mac>::new_from_slice(mac_key)
        .map_err(|_| anyhow!("invalid attachment MAC key"))?;
    let mut digest_hasher = Sha256::new();
    let mut remaining = authenticated_size;
    let mut buffer = [0u8; IO_BUFFER_SIZE];

    while remaining > 0 {
        let count = usize::try_from(remaining.min(buffer.len() as u64)).unwrap();
        reader.read_exact(&mut buffer[..count])?;
        hmac.update(&buffer[..count]);
        digest_hasher.update(&buffer[..count]);
        remaining -= count as u64;
    }

    let mut expected_mac = [0u8; MAC_SIZE];
    reader.read_exact(&mut expected_mac)?;
    hmac.verify_slice(&expected_mac)
        .map_err(|_| anyhow!("attachment MAC does not match"))?;
    digest_hasher.update(expected_mac);
    if digest_hasher.finalize().as_slice() != digest {
        bail!("attachment digest does not match");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLUTTER_ATTACHMENT: &str = "101112131415161718191a1b1c1d1e1f7301839ee28e3d217404ef7b47ecaf7a82e1940f786b844d26d5cb2fd579b6b51871648d317bb4428c9962bc0ea88684d3ac624a099a9a445f2eb0eeaea59129";
    const FLUTTER_DIGEST: &str = "a0aac4cbc19d3f1d946b3677da7da8618a84dad087ffe7080c122bc830ad366a";

    #[test]
    fn decrypts_flutter_attachment_fixture() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("encrypted");
        let output = directory.path().join("plain");
        let key = (0u8..64).collect::<Vec<_>>();
        std::fs::write(&input, hex::decode(FLUTTER_ATTACHMENT).unwrap()).unwrap();

        decrypt_attachment_file(&input, &output, &key, &hex::decode(FLUTTER_DIGEST).unwrap())
            .unwrap();

        assert_eq!(
            std::fs::read(output).unwrap(),
            b"Mixin attachment fixture\n"
        );
    }

    #[test]
    fn rejects_tampered_attachment_without_plaintext() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("encrypted");
        let output = directory.path().join("plain");
        let key = (0u8..64).collect::<Vec<_>>();
        let mut encrypted = hex::decode(FLUTTER_ATTACHMENT).unwrap();
        encrypted[20] ^= 1;
        std::fs::write(&input, encrypted).unwrap();

        let error =
            decrypt_attachment_file(&input, &output, &key, &hex::decode(FLUTTER_DIGEST).unwrap())
                .unwrap_err();

        assert_eq!(error.to_string(), "attachment MAC does not match");
        assert!(!output.exists());
    }

    #[test]
    fn rejects_wrong_attachment_digest_without_plaintext() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("encrypted");
        let output = directory.path().join("plain");
        let key = (0u8..64).collect::<Vec<_>>();
        let mut digest = hex::decode(FLUTTER_DIGEST).unwrap();
        digest[0] ^= 1;
        std::fs::write(&input, hex::decode(FLUTTER_ATTACHMENT).unwrap()).unwrap();

        let error = decrypt_attachment_file(&input, &output, &key, &digest).unwrap_err();

        assert_eq!(error.to_string(), "attachment digest does not match");
        assert!(!output.exists());
    }

    #[test]
    fn builds_flutter_compatible_media_path() {
        let message = Message {
            message_id: "message-id".to_string(),
            conversation_id: "conversation-id".to_string(),
            category: sdk::message_category::SIGNAL_IMAGE.to_string(),
            media_mime_type: Some("image/png".to_string()),
            ..Message::default()
        };

        assert_eq!(
            attachment_path(Path::new("/account"), &message).unwrap(),
            Path::new("/account/Media/Images/conversation-id/message-id.png")
        );
        assert_eq!(
            transcript_attachment_path(Path::new("/account"), &message).unwrap(),
            Path::new("/account/Media/Transcripts/message-id.png")
        );
    }
}
