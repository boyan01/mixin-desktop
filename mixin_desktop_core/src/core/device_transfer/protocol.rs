//! Version-three transfer wire codec. Attachment memory is bounded by one chunk.
use super::{
    TransferCommand, HMAC_SIZE, IV_SIZE, JSON_PACKET_LIMIT, KEY_SIZE, PACKET_COMMAND, PACKET_FILE,
    PACKET_JSON,
};
use aes::{
    cipher::{Block, BlockCipherDecrypt, BlockCipherEncrypt, KeyInit},
    Aes256,
};
use anyhow::{anyhow, bail, Context, Result};
use hmac::{Hmac, KeyInit as HmacKeyInit, Mac};
use ring::rand::{SecureRandom, SystemRandom};
use serde_json::{Map, Value};
use sha2::Sha256;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use uuid::Uuid;

pub(super) const CHUNK_SIZE: usize = 64 * 1024;

pub(super) enum Packet {
    Command(TransferCommand),
    Json {
        kind: String,
        data: Map<String, Value>,
        size: usize,
    },
    File {
        message_id: String,
        file: StagedFile,
        size: usize,
    },
}
impl Packet {
    pub(super) fn wire_size(&self) -> usize {
        match self {
            Self::Command(command) => serde_json::to_vec(command).map_or(0, |data| data.len()),
            Self::Json { size, .. } | Self::File { size, .. } => *size,
        }
    }
}

/// No final media path exists until authentication and padding validation finish.
/// Synchronous creation makes ownership established before the first await, so
/// cancellation cannot leave a late spawn_blocking file creation behind.
pub(super) struct StagedFile {
    path: PathBuf,
}
impl StagedFile {
    pub(super) fn create(directory: &Path) -> Result<(Self, tokio::fs::File)> {
        std::fs::create_dir_all(directory)?;
        let path = directory.join(format!(".transfer-{}", Uuid::new_v4()));
        let mut options = std::fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let file = options.open(&path)?;
        Ok((Self { path }, tokio::fs::File::from_std(file)))
    }
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}
impl Drop for StagedFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub(super) async fn write_command<W: AsyncWrite + Unpin>(
    writer: &mut W,
    key: &[u8; KEY_SIZE],
    command: &TransferCommand,
) -> Result<usize> {
    write_encrypted_packet(writer, key, PACKET_COMMAND, &serde_json::to_vec(command)?).await
}

pub(super) async fn write_json_record<W: AsyncWrite + Unpin>(
    writer: &mut W,
    key: &[u8; KEY_SIZE],
    kind: &str,
    data: &Map<String, Value>,
) -> Result<usize> {
    let bytes = serde_json::to_vec(&serde_json::json!({"data": data, "type": kind}))?;
    if bytes.len() > JSON_PACKET_LIMIT {
        bail!("device transfer JSON packet exceeds 500KB");
    }
    write_encrypted_packet(writer, key, PACKET_JSON, &bytes).await
}

async fn write_encrypted_packet<W: AsyncWrite + Unpin>(
    writer: &mut W,
    key: &[u8; KEY_SIZE],
    packet_type: u8,
    data: &[u8],
) -> Result<usize> {
    if data.len() > JSON_PACKET_LIMIT {
        bail!("transfer control payload exceeds 500KB");
    }
    let mut iv = [0u8; IV_SIZE];
    SystemRandom::new()
        .fill(&mut iv)
        .map_err(|_| anyhow!("secure random failed"))?;
    let encrypted = encrypt_cbc(&key[..32], &iv, data)?;
    let mut body = Vec::with_capacity(IV_SIZE + encrypted.len());
    body.extend_from_slice(&iv);
    body.extend_from_slice(&encrypted);
    write_packet_body(writer, key, packet_type, &body).await
}

async fn write_packet_body<W: AsyncWrite + Unpin>(
    writer: &mut W,
    key: &[u8; KEY_SIZE],
    packet_type: u8,
    body: &[u8],
) -> Result<usize> {
    let body_length = i32::try_from(body.len()).context("transfer packet too large")?;
    let mut hmac = <Hmac<Sha256> as HmacKeyInit>::new_from_slice(&key[32..])
        .map_err(|_| anyhow!("invalid transfer HMAC key"))?;
    hmac.update(body);
    let digest = hmac.finalize().into_bytes();
    writer.write_u8(packet_type).await?;
    writer.write_i32(body_length).await?;
    writer.write_all(body).await?;
    writer.write_all(&digest).await?;
    writer.flush().await?;
    Ok(5 + body.len() + HMAC_SIZE)
}

pub(super) fn encrypt_cbc(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256::new_from_slice(key).map_err(|_| anyhow!("invalid AES key"))?;
    let padding = 16 - data.len() % 16;
    let mut padded = Vec::with_capacity(data.len() + padding);
    padded.extend_from_slice(data);
    padded.resize(data.len() + padding, padding as u8);
    let mut previous: [u8; 16] = iv.try_into().map_err(|_| anyhow!("invalid AES IV"))?;
    for chunk in padded.chunks_exact_mut(16) {
        for (byte, previous) in chunk.iter_mut().zip(previous) {
            *byte ^= previous;
        }
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(chunk);
        let mut block = Block::<Aes256>::from(bytes);
        cipher.encrypt_block(&mut block);
        chunk.copy_from_slice(&block);
        previous.copy_from_slice(chunk);
    }
    Ok(padded)
}

pub(super) fn decrypt_cbc(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    if data.is_empty() || !data.len().is_multiple_of(16) {
        bail!("invalid AES ciphertext length");
    }
    let cipher = Aes256::new_from_slice(key).map_err(|_| anyhow!("invalid AES key"))?;
    let mut output = data.to_vec();
    let mut previous: [u8; 16] = iv.try_into().map_err(|_| anyhow!("invalid AES IV"))?;
    for chunk in output.chunks_exact_mut(16) {
        let encrypted: [u8; 16] = chunk.try_into().expect("AES block length");
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(chunk);
        let mut block = Block::<Aes256>::from(bytes);
        cipher.decrypt_block(&mut block);
        for (byte, previous) in block.iter_mut().zip(previous) {
            *byte ^= previous;
        }
        chunk.copy_from_slice(&block);
        previous = encrypted;
    }
    let padding = *output.last().unwrap() as usize;
    if padding == 0
        || padding > 16
        || !output[output.len() - padding..]
            .iter()
            .all(|byte| *byte as usize == padding)
    {
        bail!("invalid AES padding");
    }
    output.truncate(output.len() - padding);
    Ok(output)
}

fn encrypt_blocks(cipher: &Aes256, previous: &mut [u8; 16], data: &mut [u8]) {
    for chunk in data.chunks_exact_mut(16) {
        let mut block = Block::<Aes256>::default();
        for ((target, byte), prior) in block.iter_mut().zip(chunk.iter()).zip(*previous) {
            *target = *byte ^ prior;
        }
        cipher.encrypt_block(&mut block);
        chunk.copy_from_slice(&block);
        previous.copy_from_slice(chunk);
    }
}

fn decrypt_blocks(cipher: &Aes256, previous: &mut [u8; 16], data: &mut [u8]) {
    for chunk in data.chunks_exact_mut(16) {
        let encrypted: [u8; 16] = chunk.try_into().expect("AES block length");
        let mut block = Block::<Aes256>::from(encrypted);
        cipher.decrypt_block(&mut block);
        for (byte, prior) in block.iter_mut().zip(*previous) {
            *byte ^= prior;
        }
        chunk.copy_from_slice(&block);
        *previous = encrypted;
    }
}

fn unpadded_length(data: &[u8]) -> Result<usize> {
    let padding = usize::from(*data.last().ok_or_else(|| anyhow!("missing AES padding"))?);
    if padding == 0
        || padding > 16
        || padding > data.len()
        || !data[data.len() - padding..]
            .iter()
            .all(|byte| usize::from(*byte) == padding)
    {
        bail!("invalid AES padding");
    }
    Ok(data.len() - padding)
}

pub(super) async fn write_file_packet<W: AsyncWrite + Unpin>(
    writer: &mut W,
    key: &[u8; KEY_SIZE],
    message_id: &str,
    path: &Path,
) -> Result<usize> {
    let mut source = tokio::fs::File::open(path).await?;
    let length = source.metadata().await?.len();
    if length == 0 {
        return Ok(0);
    }
    let encrypted_length = length
        .checked_add(16 - length % 16)
        .ok_or_else(|| anyhow!("transfer attachment is too large"))?;
    let body_length = encrypted_length
        .checked_add(32)
        .and_then(|length| i32::try_from(length).ok())
        .ok_or_else(|| anyhow!("transfer attachment is too large for version three"))?;
    let id = Uuid::parse_str(message_id)?;
    let mut iv = [0_u8; 16];
    SystemRandom::new()
        .fill(&mut iv)
        .map_err(|_| anyhow!("secure random failed"))?;
    let cipher = Aes256::new_from_slice(&key[..32]).map_err(|_| anyhow!("invalid AES key"))?;
    let mut previous = iv;
    let mut mac = <Hmac<Sha256> as HmacKeyInit>::new_from_slice(&key[32..])
        .map_err(|_| anyhow!("invalid transfer HMAC key"))?;
    mac.update(id.as_bytes());
    mac.update(&iv);
    writer.write_u8(PACKET_FILE).await?;
    writer.write_i32(body_length).await?;
    writer.write_all(id.as_bytes()).await?;
    writer.write_all(&iv).await?;
    let mut buffer = vec![0_u8; CHUNK_SIZE];
    let mut remaining = length / 16 * 16;
    while remaining > 0 {
        let count = remaining.min(CHUNK_SIZE as u64) as usize;
        source.read_exact(&mut buffer[..count]).await?;
        encrypt_blocks(&cipher, &mut previous, &mut buffer[..count]);
        mac.update(&buffer[..count]);
        writer.write_all(&buffer[..count]).await?;
        remaining -= count as u64;
    }
    let remainder = (length % 16) as usize;
    let mut final_block = [0_u8; 16];
    source.read_exact(&mut final_block[..remainder]).await?;
    final_block[remainder..].fill((16 - remainder) as u8);
    // Do not authenticate a short prefix if a source was replaced or grew while
    // it was being read. The peer sees EOF rather than a successful finish.
    if source.read(&mut [0_u8; 1]).await? != 0 {
        bail!("transfer attachment length changed");
    }
    encrypt_blocks(&cipher, &mut previous, &mut final_block);
    mac.update(&final_block);
    writer.write_all(&final_block).await?;
    writer.write_all(&mac.finalize().into_bytes()).await?;
    writer.flush().await?;
    Ok(5 + body_length as usize + HMAC_SIZE)
}

pub(super) async fn read_packet<R: AsyncRead + Unpin>(
    reader: &mut R,
    key: &[u8; KEY_SIZE],
    account_directory: &Path,
) -> Result<Packet> {
    let packet_type = reader.read_u8().await?;
    let body_length = reader.read_i32().await?;
    if body_length <= 0 {
        bail!("invalid transfer packet length");
    }
    let body_length = body_length as usize;
    if packet_type == PACKET_FILE {
        if body_length < 48 || !(body_length - 32).is_multiple_of(16) {
            bail!("invalid attachment transfer length");
        }
        return read_file_packet(reader, key, body_length, account_directory).await;
    }
    if !matches!(packet_type, PACKET_COMMAND | PACKET_JSON) {
        bail!("unknown transfer packet type");
    }
    // Bound control data before allocating or reading the advertised body.
    if !(32..=JSON_PACKET_LIMIT + IV_SIZE + 16).contains(&body_length)
        || !(body_length - IV_SIZE).is_multiple_of(16)
    {
        bail!("invalid control transfer length");
    }
    let mut body = vec![0_u8; body_length];
    reader.read_exact(&mut body).await?;
    let mut expected = [0_u8; HMAC_SIZE];
    reader.read_exact(&mut expected).await?;
    let mut mac = <Hmac<Sha256> as HmacKeyInit>::new_from_slice(&key[32..])
        .map_err(|_| anyhow!("invalid transfer HMAC key"))?;
    mac.update(&body);
    mac.verify_slice(&expected)
        .map_err(|_| anyhow!("transfer packet HMAC mismatch"))?;
    let decrypted = decrypt_cbc(&key[..32], &body[..IV_SIZE], &body[IV_SIZE..])?;
    if decrypted.len() > JSON_PACKET_LIMIT {
        bail!("transfer control payload exceeds 500KB");
    }
    if packet_type == PACKET_COMMAND {
        Ok(Packet::Command(
            serde_json::from_slice(&decrypted)
                .map_err(|_| anyhow!("invalid transfer command payload"))?,
        ))
    } else {
        let wrapper: Value = serde_json::from_slice(&decrypted)
            .map_err(|_| anyhow!("invalid transfer JSON payload"))?;
        Ok(Packet::Json {
            kind: wrapper
                .get("type")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("transfer JSON packet has no type"))?
                .into(),
            data: wrapper
                .get("data")
                .and_then(Value::as_object)
                .cloned()
                .ok_or_else(|| anyhow!("transfer JSON packet has no data"))?,
            size: 5 + body_length + HMAC_SIZE,
        })
    }
}

async fn read_file_packet<R: AsyncRead + Unpin>(
    reader: &mut R,
    key: &[u8; KEY_SIZE],
    body_length: usize,
    account_directory: &Path,
) -> Result<Packet> {
    let mut id = [0_u8; 16];
    let mut iv = [0_u8; 16];
    reader.read_exact(&mut id).await?;
    reader.read_exact(&mut iv).await?;
    let message_id = Uuid::from_bytes(id).to_string();
    let cipher = Aes256::new_from_slice(&key[..32]).map_err(|_| anyhow!("invalid AES key"))?;
    let mut previous = iv;
    let mut mac = <Hmac<Sha256> as HmacKeyInit>::new_from_slice(&key[32..])
        .map_err(|_| anyhow!("invalid transfer HMAC key"))?;
    mac.update(&id);
    mac.update(&iv);
    let account = std::fs::canonicalize(account_directory)?;
    let staging_directory = account.join(".device-transfer");
    std::fs::create_dir_all(&staging_directory)?;
    let staging_directory = std::fs::canonicalize(staging_directory)?;
    if !staging_directory.starts_with(account) {
        bail!("transfer staging is outside the account");
    }
    let (staged, mut file) = StagedFile::create(&staging_directory)?;
    let mut buffer = vec![0_u8; CHUNK_SIZE];
    let mut remaining = body_length - 32;
    let mut final_block = [0_u8; 16];
    while remaining > 0 {
        let count = remaining.min(CHUNK_SIZE);
        reader.read_exact(&mut buffer[..count]).await?;
        mac.update(&buffer[..count]);
        decrypt_blocks(&cipher, &mut previous, &mut buffer[..count]);
        remaining -= count;
        let write_count = if remaining == 0 {
            final_block.copy_from_slice(&buffer[count - 16..count]);
            count - 16
        } else {
            count
        };
        file.write_all(&buffer[..write_count]).await?;
    }
    let mut expected = [0_u8; HMAC_SIZE];
    reader.read_exact(&mut expected).await?;
    mac.verify_slice(&expected)
        .map_err(|_| anyhow!("transfer packet HMAC mismatch"))?;
    let final_length = unpadded_length(&final_block)?;
    file.write_all(&final_block[..final_length]).await?;
    file.flush().await?;
    drop(file);
    Ok(Packet::File {
        message_id,
        file: staged,
        size: 5 + body_length + HMAC_SIZE,
    })
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
