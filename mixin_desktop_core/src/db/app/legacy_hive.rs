use std::io::ErrorKind;

use anyhow::{anyhow, bail, Context};
use serde_json::{Map, Number, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

const MULTI_AUTH_KEY: &str = "MultiAuthCubit";
const SETTING_KEY: &str = "SettingCubit";
const PRIMARY_SESSION_ID_KEY: &str = "primarySessionId";
const STRING_KEY_TYPE: u8 = 1;

pub(super) struct LegacyAuth {
    pub account: Value,
    pub private_key: String,
}

pub(super) struct LegacySignalState {
    pub next_pre_key_id: Option<u32>,
    pub next_signed_pre_key_id: Option<u32>,
    pub has_push_signal_keys: Option<bool>,
}

pub(super) async fn read_settings() -> anyhow::Result<Option<Vec<(String, String)>>> {
    let path = crate::db::path::app_database_path("hydrated_box.hive")?;
    let Some(bytes) = read_optional(&path).await? else {
        return Ok(None);
    };
    let Some(value) = parse_value_for_key(&bytes, SETTING_KEY)? else {
        return Ok(None);
    };
    let settings = value
        .as_object()
        .ok_or_else(|| anyhow!("legacy SettingCubit is not an object"))?
        .iter()
        .filter_map(|(key, value)| {
            (!value.is_null())
                .then(|| serde_json::to_string(value).map(|value| (key.clone(), value)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(settings))
}

pub(super) async fn read_auths() -> anyhow::Result<Vec<LegacyAuth>> {
    let path = crate::db::path::app_database_path("hydrated_box.hive")?;
    let Some(bytes) = read_optional(&path).await? else {
        return Ok(Vec::new());
    };
    let Some(value) = parse_value_for_key(&bytes, MULTI_AUTH_KEY)? else {
        return Ok(Vec::new());
    };
    let auths = value
        .as_object()
        .and_then(|state| state.get("auths"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("legacy MultiAuthCubit has no auth list"))?;

    auths
        .iter()
        .map(|value| {
            let value = value
                .as_object()
                .ok_or_else(|| anyhow!("legacy auth is not an object"))?;
            Ok(LegacyAuth {
                account: value
                    .get("account")
                    .cloned()
                    .ok_or_else(|| anyhow!("legacy auth has no account"))?,
                private_key: value
                    .get("privateKey")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("legacy auth has no private key"))?
                    .to_string(),
            })
        })
        .collect()
}

pub(super) async fn read_primary_session_id(
    identity_number: &str,
) -> anyhow::Result<Option<String>> {
    let path = crate::db::path::account_data_directory(identity_number)?
        .join("account_box")
        .join("account_box.hive");
    let Some(bytes) = read_optional(&path).await? else {
        return Ok(None);
    };
    let Some(value) = parse_value_for_key(&bytes, PRIMARY_SESSION_ID_KEY)? else {
        return Ok(None);
    };
    let value = value
        .as_str()
        .ok_or_else(|| anyhow!("legacy primary session is not a string"))?;
    Ok(Some(
        uuid::Uuid::parse_str(value)
            .context("legacy primary session is not a UUID")?
            .to_string(),
    ))
}

pub(super) async fn read_signal_state(identity_number: &str) -> anyhow::Result<LegacySignalState> {
    let account_directory = crate::db::path::account_data_directory(identity_number)?;
    let crypto_path = account_directory.join("crypto_box").join("crypto_box.hive");
    let privacy_path = account_directory
        .join("privacy_box")
        .join("privacy_box.hive");

    let crypto = read_optional(&crypto_path).await?;
    let privacy = read_optional(&privacy_path).await?;
    Ok(LegacySignalState {
        next_pre_key_id: read_u32_value(crypto.as_deref(), "next_pre_key_id")?,
        next_signed_pre_key_id: read_u32_value(crypto.as_deref(), "next_signed_pre_key_id")?,
        has_push_signal_keys: read_bool_value(privacy.as_deref(), "has_push_signal_keys")?,
    })
}

pub(super) async fn migrate_signal_database(identity_number: &str) -> anyhow::Result<bool> {
    let source = crate::db::path::app_database_path("signal.db")?;
    match tokio::fs::metadata(&source).await {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(error).with_context(|| format!("read metadata for {}", source.display()));
        }
    }

    let destination = crate::db::path::account_database_path(identity_number, "signal.db")?;
    crate::db::path::create_parent_directory(&destination).await?;
    let temporary = destination.with_extension("db-migration");
    remove_optional(&temporary).await?;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new().filename(&source))
        .await
        .with_context(|| format!("open legacy signal database {}", source.display()))?;
    let migration_result = sqlx::query("VACUUM INTO ?")
        .bind(temporary.to_string_lossy().as_ref())
        .execute(&pool)
        .await;
    pool.close().await;
    migration_result.with_context(|| {
        format!(
            "copy legacy signal database {} to {}",
            source.display(),
            destination.display()
        )
    })?;

    remove_optional(&destination.with_extension("db-shm")).await?;
    remove_optional(&destination.with_extension("db-wal")).await?;
    remove_optional(&destination).await?;
    tokio::fs::rename(&temporary, &destination)
        .await
        .with_context(|| format!("install migrated signal database {}", destination.display()))?;
    Ok(true)
}

fn read_u32_value(bytes: Option<&[u8]>, key: &str) -> anyhow::Result<Option<u32>> {
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    parse_value_for_key(bytes, key)?
        .map(|value| {
            value
                .as_u64()
                .and_then(|value| u32::try_from(value).ok())
                .ok_or_else(|| anyhow!("legacy {key} is not a u32"))
        })
        .transpose()
}

fn read_bool_value(bytes: Option<&[u8]>, key: &str) -> anyhow::Result<Option<bool>> {
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    parse_value_for_key(bytes, key)?
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| anyhow!("legacy {key} is not a bool"))
        })
        .transpose()
}

async fn remove_optional(path: &std::path::Path) -> anyhow::Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("remove {}", path.display())),
    }
}

async fn read_optional(path: &std::path::Path) -> anyhow::Result<Option<Vec<u8>>> {
    match tokio::fs::read(path).await {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("read {}", path.display())),
    }
}

fn parse_value_for_key(bytes: &[u8], target_key: &str) -> anyhow::Result<Option<Value>> {
    let mut offset = 0;
    let mut value = None;

    while bytes.len().saturating_sub(offset) >= 4 {
        let frame_length = read_u32(&bytes[offset..])? as usize;
        if frame_length < 8 || frame_length > bytes.len() - offset {
            break;
        }

        let frame = &bytes[offset..offset + frame_length];
        let checksum_offset = frame_length - 4;
        let checksum = read_u32(&frame[checksum_offset..])?;
        if crc32fast::hash(&frame[..checksum_offset]) != checksum {
            break;
        }

        let mut reader = HiveReader::new(&frame[4..checksum_offset]);
        let key = reader.read_key()?;
        if key == target_key {
            value = if reader.remaining() == 0 {
                None
            } else {
                Some(reader.read_value(0)?)
            };
        }
        offset += frame_length;
    }

    Ok(value)
}

fn read_u32(bytes: &[u8]) -> anyhow::Result<u32> {
    Ok(u32::from_le_bytes(
        bytes
            .get(..4)
            .ok_or_else(|| anyhow!("truncated Hive value"))?
            .try_into()?,
    ))
}

struct HiveReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> HiveReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn read_key(&mut self) -> anyhow::Result<String> {
        if self.read_byte()? != STRING_KEY_TYPE {
            bail!("unsupported Hive frame key type");
        }
        let length = self.read_byte()? as usize;
        self.read_string(length)
    }

    fn read_value(&mut self, depth: usize) -> anyhow::Result<Value> {
        if depth > 64 {
            bail!("Hive value nesting is too deep");
        }
        let mut value_type = self.read_byte()? as u16;
        if value_type == 21 {
            value_type = self.read_u16()?;
        }
        match value_type {
            0 => Ok(Value::Null),
            1 => Ok(Value::Number(self.read_integer()?.into())),
            2 => Number::from_f64(self.read_f64()?)
                .map(Value::Number)
                .ok_or_else(|| anyhow!("invalid Hive double")),
            3 => Ok(Value::Bool(self.read_byte()? != 0)),
            4 => {
                let length = self.read_u32()? as usize;
                Ok(Value::String(self.read_string(length)?))
            }
            5 => {
                let length = self.read_u32()? as usize;
                Ok(Value::Array(
                    self.read_bytes(length)?
                        .iter()
                        .map(|value| Value::Number((*value).into()))
                        .collect(),
                ))
            }
            6 | 13 => {
                let length = self.read_u32()? as usize;
                let mut values = Vec::with_capacity(length);
                for _ in 0..length {
                    values.push(Value::Number(self.read_integer()?.into()));
                }
                Ok(Value::Array(values))
            }
            7 | 14 => {
                let length = self.read_u32()? as usize;
                let mut values = Vec::with_capacity(length);
                for _ in 0..length {
                    let value = Number::from_f64(self.read_f64()?)
                        .ok_or_else(|| anyhow!("invalid Hive double"))?;
                    values.push(Value::Number(value));
                }
                Ok(Value::Array(values))
            }
            8 => {
                let length = self.read_u32()? as usize;
                let mut values = Vec::with_capacity(length);
                for _ in 0..length {
                    values.push(Value::Bool(self.read_byte()? != 0));
                }
                Ok(Value::Array(values))
            }
            9 | 15 => {
                let length = self.read_u32()? as usize;
                let mut values = Vec::with_capacity(length);
                for _ in 0..length {
                    let string_length = self.read_u32()? as usize;
                    values.push(Value::String(self.read_string(string_length)?));
                }
                Ok(Value::Array(values))
            }
            10 | 19 => {
                let length = self.read_u32()? as usize;
                let mut values = Vec::with_capacity(length);
                for _ in 0..length {
                    values.push(self.read_value(depth + 1)?);
                }
                Ok(Value::Array(values))
            }
            11 => {
                let length = self.read_u32()? as usize;
                let mut values = Map::new();
                for _ in 0..length {
                    let key = self
                        .read_value(depth + 1)?
                        .as_str()
                        .ok_or_else(|| anyhow!("Hive map key is not a string"))?
                        .to_string();
                    values.insert(key, self.read_value(depth + 1)?);
                }
                Ok(Value::Object(values))
            }
            value_type => bail!("unsupported Hive value type: {value_type}"),
        }
    }

    fn read_integer(&mut self) -> anyhow::Result<i64> {
        let value = self.read_f64()?;
        if !value.is_finite() || value.fract() != 0.0 {
            bail!("invalid Hive integer");
        }
        Ok(value as i64)
    }

    fn read_byte(&mut self) -> anyhow::Result<u8> {
        Ok(self.read_bytes(1)?[0])
    }

    fn read_u16(&mut self) -> anyhow::Result<u16> {
        Ok(u16::from_le_bytes(self.read_bytes(2)?.try_into()?))
    }

    fn read_u32(&mut self) -> anyhow::Result<u32> {
        Ok(u32::from_le_bytes(self.read_bytes(4)?.try_into()?))
    }

    fn read_f64(&mut self) -> anyhow::Result<f64> {
        Ok(f64::from_le_bytes(self.read_bytes(8)?.try_into()?))
    }

    fn read_string(&mut self, length: usize) -> anyhow::Result<String> {
        Ok(std::str::from_utf8(self.read_bytes(length)?)?.to_string())
    }

    fn read_bytes(&mut self, length: usize) -> anyhow::Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(length)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| anyhow!("truncated Hive value"))?;
        let bytes = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }
}
