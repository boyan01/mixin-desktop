use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};

use aes::cipher::{Block, BlockCipherDecrypt, BlockCipherEncrypt, KeyInit};
use aes::Aes256;
use anyhow::{anyhow, bail, Context, Result};
use futures::StreamExt;
use hmac::{Hmac, KeyInit as HmacKeyInit, Mac};
use reqwest::header::{CONNECTION, CONTENT_LENGTH, CONTENT_TYPE};
use reqwest::Client as HttpClient;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, ReadBuf};
use tokio_util::io::ReaderStream;
use tokio_util::sync::CancellationToken;

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

#[derive(Debug)]
pub struct AttachmentUploadResult {
    pub attachment_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub key: Option<Vec<u8>>,
    pub digest: Option<Vec<u8>>,
}

struct ProgressReader<R> {
    reader: R,
    completed: u64,
    total: u64,
    progress: Option<Arc<dyn Fn(u64, u64) + Send + Sync>>,
}

impl<R> ProgressReader<R> {
    fn new(reader: R, total: u64, progress: Option<Arc<dyn Fn(u64, u64) + Send + Sync>>) -> Self {
        Self {
            reader,
            completed: 0,
            total,
            progress,
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for ProgressReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut TaskContext<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buffer.filled().len();
        let result = Pin::new(&mut self.reader).poll_read(context, buffer);
        if let Poll::Ready(Ok(())) = result {
            self.completed = self
                .completed
                .saturating_add((buffer.filled().len() - before) as u64);
            if let Some(progress) = &self.progress {
                progress(self.completed, self.total);
            }
        }
        result
    }
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
        self.download_to(message, extra, false, None, None).await
    }

    pub async fn download_public(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.http_client.get(url).send().await?.error_for_status()?;
        Ok(response.bytes().await?.to_vec())
    }

    pub async fn download_cancellable(
        &self,
        message: &Message,
        extra: &AttachmentExtra,
        cancellation: &CancellationToken,
        progress: &(dyn Fn(u64, u64) + Send + Sync),
    ) -> Result<AttachmentDownloadResult> {
        self.download_to(message, extra, false, Some(cancellation), Some(progress))
            .await
    }

    pub async fn download_transcript(
        &self,
        message: &Message,
        extra: &AttachmentExtra,
    ) -> Result<AttachmentDownloadResult> {
        self.download_to(message, extra, true, None, None).await
    }

    pub async fn download_transcript_cancellable(
        &self,
        message: &Message,
        extra: &AttachmentExtra,
        cancellation: &CancellationToken,
        progress: &(dyn Fn(u64, u64) + Send + Sync),
    ) -> Result<AttachmentDownloadResult> {
        self.download_to(message, extra, true, Some(cancellation), Some(progress))
            .await
    }

    pub async fn copy_for_forward(
        &self,
        source: &Path,
        target_message: &Message,
    ) -> Result<(PathBuf, i64)> {
        let account_dir = tokio::fs::canonicalize(&self.account_data_dir)
            .await
            .context("canonicalize account data directory")?;
        let source = tokio::fs::canonicalize(source)
            .await
            .with_context(|| format!("resolve attachment source {}", source.display()))?;
        if !source.starts_with(&account_dir) {
            bail!("attachment source is outside the account data directory");
        }
        if !tokio::fs::metadata(&source).await?.is_file() {
            bail!("attachment source is not a file");
        }
        validate_path_component("message id", &target_message.message_id)?;
        validate_path_component("conversation id", &target_message.conversation_id)?;
        let target = attachment_path(&self.account_data_dir, target_message)?;
        let directory = target
            .parent()
            .ok_or_else(|| anyhow!("attachment target has no parent directory"))?;
        ensure_safe_target_directory(&account_dir, directory).await?;
        let copy_temp = temp_path(&target, "copy")?;
        let operation = async {
            let size = tokio::fs::copy(&source, &copy_temp).await?;
            let size = i64::try_from(size).context("attachment is too large")?;
            tokio::fs::rename(&copy_temp, &target).await?;
            Ok::<_, anyhow::Error>(size)
        }
        .await;
        if operation.is_err() {
            let _ = tokio::fs::remove_file(&copy_temp).await;
        }
        Ok((target, operation?))
    }

    pub async fn import_local(
        &self,
        source: &Path,
        target_message: &Message,
    ) -> Result<(PathBuf, i64)> {
        let source = tokio::fs::canonicalize(source)
            .await
            .with_context(|| format!("resolve local attachment {}", source.display()))?;
        if !tokio::fs::metadata(&source).await?.is_file() {
            bail!("local attachment source is not a file");
        }
        validate_path_component("message id", &target_message.message_id)?;
        validate_path_component("conversation id", &target_message.conversation_id)?;
        let account_dir = tokio::fs::canonicalize(&self.account_data_dir)
            .await
            .context("canonicalize account data directory")?;
        let target = attachment_path(&self.account_data_dir, target_message)?;
        let directory = target
            .parent()
            .ok_or_else(|| anyhow!("attachment target has no parent directory"))?;
        ensure_safe_target_directory(&account_dir, directory).await?;
        let copy_temp = temp_path(&target, "import")?;
        let operation = async {
            let size = tokio::fs::copy(&source, &copy_temp).await?;
            let size = i64::try_from(size).context("attachment is too large")?;
            tokio::fs::rename(&copy_temp, &target).await?;
            Ok::<_, anyhow::Error>(size)
        }
        .await;
        if operation.is_err() {
            let _ = tokio::fs::remove_file(&copy_temp).await;
        }
        Ok((target, operation?))
    }

    pub async fn read_account_file(&self, path: &Path, max_size: u64) -> Result<Vec<u8>> {
        let account_dir = tokio::fs::canonicalize(&self.account_data_dir)
            .await
            .context("canonicalize account data directory")?;
        let path = tokio::fs::canonicalize(path)
            .await
            .with_context(|| format!("resolve local file {}", path.display()))?;
        if !path.starts_with(&account_dir) {
            bail!("local file is outside the account data directory");
        }
        if !tokio::fs::metadata(&path).await?.is_file() {
            bail!("local path is not a file");
        }
        let mut bytes = Vec::new();
        tokio::fs::File::open(&path)
            .await?
            .take(max_size.saturating_add(1))
            .read_to_end(&mut bytes)
            .await?;
        if bytes.len() as u64 > max_size {
            bail!("local file exceeds the size limit");
        }
        Ok(bytes)
    }

    pub async fn upload(
        &self,
        path: &Path,
        encrypted: bool,
        cancellation: Option<&CancellationToken>,
        progress: Option<Arc<dyn Fn(u64, u64) + Send + Sync>>,
    ) -> Result<AttachmentUploadResult> {
        let attachment = self.mixin_client.attachment_api.create_attachment().await?;
        if attachment.attachment_id.trim().is_empty() {
            bail!("attachment response has no attachment ID");
        }
        let upload_url = attachment
            .upload_url
            .as_deref()
            .filter(|url| !url.trim().is_empty())
            .ok_or_else(|| anyhow!("attachment has no upload URL"))?;
        let parsed_url = https_url(upload_url, "attachment upload URL")?;

        let encrypted_temp = encrypted.then(|| temp_path(path, "upload")).transpose()?;
        let mut key = None;
        let mut digest = None;
        let upload_path = if let Some(temp) = encrypted_temp.as_ref() {
            let source = path.to_path_buf();
            let output = temp.clone();
            let encrypted = match tokio::task::spawn_blocking(move || {
                encrypt_attachment_file(&source, &output)
            })
            .await
            {
                Ok(result) => result,
                Err(error) => {
                    let _ = tokio::fs::remove_file(temp).await;
                    return Err(error).context("attachment encrypt task failed");
                }
            };
            let (generated_key, generated_digest) = match encrypted {
                Ok(result) => result,
                Err(error) => {
                    let _ = tokio::fs::remove_file(temp).await;
                    return Err(error);
                }
            };
            key = Some(generated_key);
            digest = Some(generated_digest);
            temp.as_path()
        } else {
            path
        };

        let operation = async {
            let file = tokio::fs::File::open(upload_path).await?;
            let length = file.metadata().await?.len();
            progress.as_ref().map(|callback| callback(0, length));
            let stream = ReaderStream::new(ProgressReader::new(file, length, progress.clone()));
            let request = self
                .http_client
                .put(parsed_url)
                .header(CONTENT_TYPE, "application/octet-stream")
                .header(CONTENT_LENGTH, length)
                .header(CONNECTION, "close")
                .header("x-amz-acl", "public-read")
                .body(reqwest::Body::wrap_stream(stream))
                .send();
            match cancellation {
                Some(cancellation) => tokio::select! {
                    _ = cancellation.cancelled() => bail!("attachment upload canceled"),
                    result = request => {
                        result?.error_for_status()?;
                    }
                },
                None => {
                    request.await?.error_for_status()?;
                }
            }
            progress.as_ref().map(|callback| callback(1, 1));
            Ok::<_, anyhow::Error>(())
        }
        .await;
        if let Some(temp) = encrypted_temp {
            let _ = tokio::fs::remove_file(temp).await;
        }
        operation?;

        Ok(AttachmentUploadResult {
            attachment_id: attachment.attachment_id,
            created_at: attachment.created_at,
            key,
            digest,
        })
    }

    async fn download_to(
        &self,
        message: &Message,
        extra: &AttachmentExtra,
        transcript: bool,
        cancellation: Option<&CancellationToken>,
        progress: Option<&(dyn Fn(u64, u64) + Send + Sync)>,
    ) -> Result<AttachmentDownloadResult> {
        validate_message(message, extra)?;
        ensure_not_cancelled(cancellation)?;
        let get_attachment = self
            .mixin_client
            .attachment_api
            .get_attachment(&extra.attachment_id);
        let attachment = match cancellation {
            Some(cancellation) => tokio::select! {
                _ = cancellation.cancelled() => bail!("attachment download canceled"),
                result = get_attachment => result?,
            },
            None => get_attachment.await?,
        };
        if attachment.attachment_id != extra.attachment_id {
            bail!("attachment response id does not match the requested id");
        }
        let view_url = attachment
            .view_url
            .as_deref()
            .filter(|url| !url.trim().is_empty())
            .ok_or_else(|| anyhow!("attachment has no view URL"))?;
        let view_url = https_url(view_url, "attachment view URL")?;

        let target = if transcript {
            transcript_attachment_path(&self.account_data_dir, message)?
        } else {
            attachment_path(&self.account_data_dir, message)?
        };
        let directory = target
            .parent()
            .ok_or_else(|| anyhow!("attachment target has no parent directory"))?;
        let account_dir = tokio::fs::canonicalize(&self.account_data_dir)
            .await
            .context("canonicalize account data directory")?;
        ensure_safe_target_directory(&account_dir, directory).await?;

        let download_temp = temp_path(&target, "download")?;
        let output_temp = temp_path(&target, "part")?;
        let operation = async {
            match (&message.media_key, &message.media_digest) {
                (Some(key), Some(digest)) => {
                    validate_encryption_material(key, digest)?;
                    self.download_to_file(&view_url, &download_temp, cancellation, progress)
                        .await?;

                    let input = download_temp.clone();
                    let output = output_temp.clone();
                    let key = key.clone();
                    let digest = digest.clone();
                    tokio::task::spawn_blocking(move || {
                        decrypt_attachment_file(&input, &output, &key, &digest)
                    })
                    .await
                    .context("attachment decrypt task failed")??;
                    ensure_not_cancelled(cancellation)?;
                    tokio::fs::remove_file(&download_temp).await?;
                }
                (None, None) => {
                    self.download_to_file(&view_url, &output_temp, cancellation, progress)
                        .await?
                }
                _ => bail!("attachment key and digest must be provided together"),
            }

            let size = tokio::fs::metadata(&output_temp).await?.len();
            let size = i64::try_from(size).context("attachment is too large")?;
            ensure_not_cancelled(cancellation)?;
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

    async fn download_to_file(
        &self,
        view_url: &reqwest::Url,
        path: &Path,
        cancellation: Option<&CancellationToken>,
        progress: Option<&(dyn Fn(u64, u64) + Send + Sync)>,
    ) -> Result<()> {
        let request = self
            .http_client
            .get(view_url.clone())
            .header(CONTENT_TYPE, "application/octet-stream")
            .send();
        let response = match cancellation {
            Some(cancellation) => tokio::select! {
                _ = cancellation.cancelled() => bail!("attachment download canceled"),
                result = request => result?,
            },
            None => request.await?,
        }
        .error_for_status()?;
        let total = response.content_length().unwrap_or_default();
        progress.map(|callback| callback(0, total));
        let mut stream = response.bytes_stream();
        let mut received = 0_u64;
        let mut file = tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .await
            .with_context(|| format!("create attachment temporary file {}", path.display()))?;

        loop {
            let chunk = match cancellation {
                Some(cancellation) => tokio::select! {
                    _ = cancellation.cancelled() => bail!("attachment download canceled"),
                    chunk = stream.next() => chunk,
                },
                None => stream.next().await,
            };
            let Some(chunk) = chunk else { break };
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            received = received.saturating_add(chunk.len() as u64);
            progress.map(|callback| callback(received, total));
        }
        file.flush().await?;
        file.sync_all().await?;
        progress.map(|callback| callback(1, 1));
        Ok(())
    }
}

fn https_url(value: &str, label: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(value)?;
    if url.scheme() != "https" {
        bail!("{label} must use HTTPS");
    }
    Ok(url)
}

async fn ensure_safe_target_directory(account_dir: &Path, directory: &Path) -> Result<()> {
    tokio::fs::create_dir_all(directory).await?;
    let directory = tokio::fs::canonicalize(directory)
        .await
        .with_context(|| format!("canonicalize attachment directory {}", directory.display()))?;
    if !directory.starts_with(account_dir) {
        bail!("attachment target is outside the account data directory");
    }
    Ok(())
}

fn ensure_not_cancelled(cancellation: Option<&CancellationToken>) -> Result<()> {
    if cancellation.is_some_and(CancellationToken::is_cancelled) {
        bail!("attachment download canceled");
    }
    Ok(())
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

pub(crate) fn attachment_path(account_data_dir: &Path, message: &Message) -> Result<PathBuf> {
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

pub(crate) fn attachment_file_name(path: &Path) -> Result<&str> {
    path.file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("attachment path has no valid UTF-8 file name"))
}

pub(crate) fn transcript_attachment_path(
    account_data_dir: &Path,
    message: &Message,
) -> Result<PathBuf> {
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

fn encrypt_attachment_file(input: &Path, output: &Path) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut key = vec![0_u8; ATTACHMENT_KEY_SIZE];
    let mut iv = [0_u8; CBC_BLOCK_SIZE];
    rand::fill(key.as_mut_slice());
    rand::fill(&mut iv);

    let cipher = Aes256::new_from_slice(&key[..AES_KEY_SIZE])
        .map_err(|_| anyhow!("invalid attachment AES key"))?;
    let mut hmac =
        <Hmac<Sha256> as HmacKeyInit>::new_from_slice(&key[AES_KEY_SIZE..ATTACHMENT_KEY_SIZE])
            .map_err(|_| anyhow!("invalid attachment MAC key"))?;
    let mut digest_hasher = Sha256::new();
    let mut reader = BufReader::with_capacity(IO_BUFFER_SIZE, File::open(input)?);
    let mut writer = BufWriter::with_capacity(
        IO_BUFFER_SIZE,
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(output)?,
    );
    writer.write_all(&iv)?;
    hmac.update(&iv);
    digest_hasher.update(iv);

    let mut previous = iv;
    loop {
        let mut plaintext = [0_u8; CBC_BLOCK_SIZE];
        let mut count = 0;
        while count < CBC_BLOCK_SIZE {
            let read = reader.read(&mut plaintext[count..])?;
            if read == 0 {
                break;
            }
            count += read;
        }
        if count < CBC_BLOCK_SIZE {
            let padding = (CBC_BLOCK_SIZE - count) as u8;
            plaintext[count..].fill(padding);
        }
        for (byte, previous_byte) in plaintext.iter_mut().zip(previous) {
            *byte ^= previous_byte;
        }
        let mut block = Block::<Aes256>::from(plaintext);
        cipher.encrypt_block(&mut block);
        writer.write_all(&block)?;
        hmac.update(&block);
        digest_hasher.update(block);
        previous.copy_from_slice(&block);
        if count < CBC_BLOCK_SIZE {
            break;
        }
    }

    let mac = hmac.finalize().into_bytes();
    writer.write_all(&mac)?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    digest_hasher.update(mac);
    Ok((key, digest_hasher.finalize().to_vec()))
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
        let mut block = Block::<Aes256>::from(encrypted);
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
    let mut hmac = <Hmac<Sha256> as HmacKeyInit>::new_from_slice(mac_key)
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

    #[tokio::test]
    async fn reading_an_upload_body_reports_transferred_bytes_and_total_size() {
        let (mut writer, reader) = tokio::io::duplex(64);
        tokio::spawn(async move {
            writer.write_all(b"hello").await.unwrap();
        });
        let updates = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured_updates = updates.clone();
        let progress = Arc::new(move |completed, total| {
            captured_updates.lock().unwrap().push((completed, total));
        });
        let mut stream = ReaderStream::new(ProgressReader::new(reader, 5, Some(progress)));

        while let Some(chunk) = stream.next().await {
            chunk.unwrap();
        }

        assert_eq!(updates.lock().unwrap().last(), Some(&(5, 5)));
    }

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
    fn encrypts_and_decrypts_attachment_file() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("input");
        let encrypted = directory.path().join("encrypted");
        let output = directory.path().join("output");
        let content = vec![0x5a; IO_BUFFER_SIZE + 37];
        std::fs::write(&input, &content).unwrap();

        let (key, digest) = encrypt_attachment_file(&input, &encrypted).unwrap();
        decrypt_attachment_file(&encrypted, &output, &key, &digest).unwrap();

        assert_eq!(std::fs::read(output).unwrap(), content);
    }

    #[test]
    fn encrypts_and_decrypts_empty_attachment_file() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("input");
        let encrypted = directory.path().join("encrypted");
        let output = directory.path().join("output");
        std::fs::write(&input, []).unwrap();

        let (key, digest) = encrypt_attachment_file(&input, &encrypted).unwrap();
        decrypt_attachment_file(&encrypted, &output, &key, &digest).unwrap();

        assert!(std::fs::read(output).unwrap().is_empty());
    }

    #[test]
    fn rejects_insecure_attachment_urls() {
        assert!(https_url("http://example.com/file", "attachment URL").is_err());
        assert!(https_url("https://example.com/file", "attachment URL").is_ok());
    }

    #[tokio::test]
    async fn copies_forwarded_attachment_inside_account_directory() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.png");
        std::fs::write(&source, b"image").unwrap();
        let service = AttachmentService::new(
            Arc::new(MixinClient::new(sdk::Credential::None)),
            HttpClient::new(),
            directory.path(),
        );
        let message = Message {
            message_id: "message-id".to_string(),
            conversation_id: "conversation-id".to_string(),
            category: sdk::message_category::SIGNAL_IMAGE.to_string(),
            media_mime_type: Some("image/png".to_string()),
            ..Message::default()
        };

        let (target, size) = service.copy_for_forward(&source, &message).await.unwrap();

        assert_eq!(size, 5);
        assert_eq!(std::fs::read(target).unwrap(), b"image");
    }

    #[tokio::test]
    async fn rejects_forwarded_attachment_outside_account_directory() {
        let account = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let source = outside.path().join("source.png");
        std::fs::write(&source, b"image").unwrap();
        let service = AttachmentService::new(
            Arc::new(MixinClient::new(sdk::Credential::None)),
            HttpClient::new(),
            account.path(),
        );
        let message = Message {
            message_id: "message-id".to_string(),
            conversation_id: "conversation-id".to_string(),
            category: sdk::message_category::SIGNAL_IMAGE.to_string(),
            ..Message::default()
        };

        let error = service
            .copy_for_forward(&source, &message)
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "attachment source is outside the account data directory"
        );
    }

    #[tokio::test]
    async fn imports_recorded_audio_from_temporary_directory() {
        let account = tempfile::tempdir().unwrap();
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("recording.ogg");
        std::fs::write(&source, b"ogg audio").unwrap();
        let service = AttachmentService::new(
            Arc::new(MixinClient::new(sdk::Credential::None)),
            HttpClient::new(),
            account.path(),
        );
        let message = Message {
            message_id: "message-id".to_string(),
            conversation_id: "conversation-id".to_string(),
            category: sdk::message_category::SIGNAL_AUDIO.to_string(),
            media_mime_type: Some("audio/ogg".to_string()),
            ..Message::default()
        };

        let (target, size) = service.import_local(&source, &message).await.unwrap();

        assert_eq!(size, 9);
        assert_eq!(std::fs::read(&target).unwrap(), b"ogg audio");
        assert!(target.starts_with(account.path().join("Media/Audios")));
        assert_eq!(
            target.extension().and_then(|value| value.to_str()),
            Some("ogg")
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn rejects_forward_target_through_symlinked_directory() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let account = root.path().join("account");
        let outside = root.path().join("outside");
        std::fs::create_dir_all(account.join("Media/Images")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        symlink(&outside, account.join("Media/Images/conversation-id")).unwrap();
        let source = account.join("source.png");
        std::fs::write(&source, b"image").unwrap();
        let service = AttachmentService::new(
            Arc::new(MixinClient::new(sdk::Credential::None)),
            HttpClient::new(),
            &account,
        );
        let message = Message {
            message_id: "message-id".to_string(),
            conversation_id: "conversation-id".to_string(),
            category: sdk::message_category::SIGNAL_IMAGE.to_string(),
            ..Message::default()
        };

        let error = service
            .copy_for_forward(&source, &message)
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "attachment target is outside the account data directory"
        );
        assert!(!outside.join("message-id.jpg").exists());
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
