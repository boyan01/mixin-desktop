//! Dart Hive box files shared with the Flutter desktop app.
//!
//! The Flutter app keeps credentials, Signal counters, and preferences in
//! Hive boxes (`<directory>/<name>.hive`). This module reads and appends the
//! same frame format so both apps can open one data directory. Like Hive, an
//! open box holds an exclusive lock on `<name>.lock`; the Flutter app waits on
//! that lock, and this module refuses to open a box another process holds.
//!
//! Frame: `u32 length | key | value | u32 crc32`, little endian. A frame
//! without a value deletes its key. Integers are stored as f64.
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{Map, Number, Value};

const KEY_UINT: u8 = 0;
const KEY_STRING: u8 = 1;
const VALUE_NULL: u8 = 0;
const VALUE_INT: u8 = 1;
const VALUE_DOUBLE: u8 = 2;
const VALUE_BOOL: u8 = 3;
const VALUE_STRING: u8 = 4;
const VALUE_LIST: u8 = 10;
const VALUE_MAP: u8 = 11;
const MAX_DEPTH: usize = 64;

/// Process-wide state of the Flutter app: `MultiAuthCubit`, `SettingCubit`.
pub const HYDRATED_BOX: &str = "hydrated_box";
pub const ACCOUNT_BOX: &str = "account_box";
pub const CRYPTO_BOX: &str = "crypto_box";
pub const PRIVACY_BOX: &str = "privacy_box";
pub const SESSION_BOX: &str = "session_box";
pub const SECURITY_BOX: &str = "security_box";
pub const SHOW_PIN_MESSAGE_BOX: &str = "show_pin_message_box";
pub const SCAM_WARNING_BOX: &str = "scam_warning_key_value";
pub const DOWNLOAD_BOX: &str = "download_box";
/// Per-account boxes the Flutter app deletes on sign-out (`clearKeyValues`).
pub const ACCOUNT_BOXES: [&str; 8] = [
    PRIVACY_BOX,
    CRYPTO_BOX,
    ACCOUNT_BOX,
    SHOW_PIN_MESSAGE_BOX,
    SCAM_WARNING_BOX,
    DOWNLOAD_BOX,
    SESSION_BOX,
    SECURITY_BOX,
];

pub fn hydrated_box(root: &Path) -> Result<Arc<HiveBox>> {
    HiveBox::open(root, HYDRATED_BOX)
}

/// `<root>/<identity_number>`, the account directory holding its boxes.
pub fn account_directory(root: &Path, identity_number: &str) -> Result<PathBuf> {
    if identity_number.is_empty() || !identity_number.chars().all(|c| c.is_ascii_digit()) {
        bail!("invalid identity number");
    }
    Ok(root.join(identity_number))
}

/// Opens `<account_directory>/<name>/<name>.hive`.
pub fn account_box(account_directory: &Path, name: &str) -> Result<Arc<HiveBox>> {
    HiveBox::open(&account_directory.join(name), name)
}

/// Reads one key without creating a box that does not exist yet.
pub fn read_account_value(
    account_directory: &Path,
    name: &str,
    key: &str,
) -> Result<Option<Value>> {
    let directory = account_directory.join(name);
    if !directory.join(format!("{name}.hive")).exists()
        && !directory.join(format!("{name}.hivec")).exists()
    {
        return Ok(None);
    }
    HiveBox::open(&directory, name)?.get(key)
}

/// Deletes the boxes the Flutter app deletes on sign-out (`clearKeyValues`).
pub fn delete_account_boxes(account_directory: &Path) -> Result<()> {
    for name in ACCOUNT_BOXES {
        delete_box(&account_directory.join(name), name)?;
    }
    Ok(())
}

/// Raised when another process, normally the Flutter app, holds a box.
#[derive(Debug)]
pub struct HiveBoxLocked(pub PathBuf);

impl std::fmt::Display for HiveBoxLocked {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} is in use by another Mixin process",
            self.0.display()
        )
    }
}

impl std::error::Error for HiveBoxLocked {}

pub struct HiveBox {
    path: PathBuf,
    state: Mutex<Option<State>>,
    #[cfg(test)]
    fail_writes: std::sync::atomic::AtomicBool,
}

struct State {
    file: File,
    entries: HashMap<String, Entry>,
    _lock: File,
}

#[derive(Clone)]
enum Entry {
    Value(Value),
    /// A value type this module does not decode. It is kept on disk as is.
    Unsupported(u16),
}

fn registry() -> &'static Mutex<HashMap<PathBuf, Weak<HiveBox>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<PathBuf, Weak<HiveBox>>>> = OnceLock::new();
    REGISTRY.get_or_init(Default::default)
}

impl HiveBox {
    /// Opens `<directory>/<name>.hive`. One process shares one handle per
    /// file because POSIX record locks belong to the process, not the handle.
    pub fn open(directory: &Path, name: &str) -> Result<Arc<Self>> {
        let path = directory.join(format!("{name}.hive"));
        let mut registry = registry().lock().unwrap();
        if let Some(open) = registry.get(&path).and_then(Weak::upgrade) {
            if open.state.lock().unwrap().is_some() {
                return Ok(open);
            }
        }
        let state = State::open(directory, name)?;
        let hive_box = Arc::new(Self {
            path: path.clone(),
            state: Mutex::new(Some(state)),
            #[cfg(test)]
            fail_writes: Default::default(),
        });
        registry.retain(|_, open| open.strong_count() > 0);
        registry.insert(path, Arc::downgrade(&hive_box));
        Ok(hive_box)
    }

    pub fn get(&self, key: &str) -> Result<Option<Value>> {
        let state = self.state.lock().unwrap();
        let state = state.as_ref().ok_or_else(|| self.closed())?;
        match state.entries.get(key) {
            None => Ok(None),
            Some(Entry::Value(value)) => Ok(Some(value.clone())),
            Some(Entry::Unsupported(value_type)) => bail!(
                "{} key {key} has unsupported Hive type {value_type}",
                self.path.display()
            ),
        }
    }

    pub fn keys(&self) -> Result<Vec<String>> {
        let state = self.state.lock().unwrap();
        let state = state.as_ref().ok_or_else(|| self.closed())?;
        Ok(state.entries.keys().cloned().collect())
    }

    pub fn put(&self, key: &str, value: &Value) -> Result<()> {
        self.write(&[(key, Some(value))])
    }

    pub fn delete(&self, key: &str) -> Result<()> {
        self.write(&[(key, None)])
    }

    /// Appends all changes with one write, as Hive's `putAll` does.
    pub fn write(&self, changes: &[(&str, Option<&Value>)]) -> Result<()> {
        #[cfg(test)]
        if self.fail_writes.load(std::sync::atomic::Ordering::SeqCst) {
            bail!("test Hive write rejected");
        }
        let mut state = self.state.lock().unwrap();
        let state = state.as_mut().ok_or_else(|| self.closed())?;
        let mut bytes = Vec::new();
        for (key, value) in changes {
            if value.is_none() && !state.entries.contains_key(*key) {
                continue;
            }
            encode_frame(&mut bytes, key, *value)?;
        }
        if bytes.is_empty() {
            return Ok(());
        }
        state
            .file
            .write_all(&bytes)
            .and_then(|()| state.file.sync_data())
            .with_context(|| format!("write {}", self.path.display()))?;
        for (key, value) in changes {
            match value {
                Some(value) => state
                    .entries
                    .insert((*key).to_owned(), Entry::Value((*value).clone())),
                None => state.entries.remove(*key),
            };
        }
        Ok(())
    }

    /// Makes later writes fail, standing in for a full or read-only disk.
    #[cfg(test)]
    pub(crate) fn fail_writes(&self, fail: bool) {
        self.fail_writes
            .store(fail, std::sync::atomic::Ordering::SeqCst);
    }

    fn closed(&self) -> anyhow::Error {
        anyhow!("{} is closed", self.path.display())
    }
}

/// Deletes a box from disk like Hive's `deleteFromDisk`, closing any open
/// handle first so a later [`HiveBox::open`] starts empty.
pub fn delete_box(directory: &Path, name: &str) -> Result<()> {
    let path = directory.join(format!("{name}.hive"));
    let open = registry().lock().unwrap().remove(&path);
    if let Some(open) = open.and_then(|open| open.upgrade()) {
        open.state.lock().unwrap().take();
    }
    for extension in ["hive", "hivec", "lock"] {
        let path = directory.join(format!("{name}.{extension}"));
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error).with_context(|| format!("remove {}", path.display())),
        }
    }
    Ok(())
}

impl State {
    fn open(directory: &Path, name: &str) -> Result<Self> {
        fs::create_dir_all(directory)
            .with_context(|| format!("create Hive directory {}", directory.display()))?;
        let lock_path = directory.join(format!("{name}.lock"));
        let lock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .with_context(|| format!("open {}", lock_path.display()))?;
        try_lock(&lock).map_err(|error| match error.kind() {
            ErrorKind::WouldBlock => anyhow::Error::new(HiveBoxLocked(lock_path.clone())),
            _ => anyhow::Error::new(error).context(format!("lock {}", lock_path.display())),
        })?;

        let path = directory.join(format!("{name}.hive"));
        let compacted = directory.join(format!("{name}.hivec"));
        // Hive's backend manager finishes or discards an interrupted compaction.
        if compacted.exists() {
            if path.exists() {
                fs::remove_file(&compacted)?;
            } else {
                fs::rename(&compacted, &path)?;
            }
        }
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
        };
        let (entries, valid_length) = parse_frames(&bytes)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("open {}", path.display()))?;
        if valid_length < bytes.len() {
            // Hive's crash recovery drops a torn tail; appending after it
            // would hide the new frames from the Flutter app.
            log::warn!("recovering corrupted Hive box {}", path.display());
            file.set_len(valid_length as u64)?;
        }
        Ok(Self {
            file,
            entries,
            _lock: lock,
        })
    }
}

#[cfg(unix)]
fn try_lock(file: &File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    // Dart's `RandomAccessFile.lock` takes a POSIX record lock, which does
    // not conflict with `flock`, so `File::try_lock` would not see it.
    let mut lock: libc::flock = unsafe { std::mem::zeroed() };
    lock.l_type = libc::F_WRLCK as _;
    lock.l_whence = libc::SEEK_SET as _;
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETLK, &lock) } == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    match error.raw_os_error() {
        Some(libc::EAGAIN) | Some(libc::EACCES) => Err(ErrorKind::WouldBlock.into()),
        _ => Err(error),
    }
}

#[cfg(windows)]
fn try_lock(file: &File) -> std::io::Result<()> {
    file.try_lock().map_err(|error| match error {
        std::fs::TryLockError::WouldBlock => ErrorKind::WouldBlock.into(),
        std::fs::TryLockError::Error(error) => error,
    })
}

/// Returns the live entries and the length of the valid frame prefix.
fn parse_frames(bytes: &[u8]) -> Result<(HashMap<String, Entry>, usize)> {
    let mut entries = HashMap::new();
    let mut offset = 0;
    while bytes.len() - offset >= 4 {
        let length = u32::from_le_bytes(bytes[offset..offset + 4].try_into()?) as usize;
        if length < 10 || length > bytes.len() - offset {
            break;
        }
        let frame = &bytes[offset..offset + length];
        let checksum = u32::from_le_bytes(frame[length - 4..].try_into()?);
        if crc32fast::hash(&frame[..length - 4]) != checksum {
            break;
        }
        let mut reader = Reader::new(&frame[4..length - 4]);
        let key = reader.read_key()?;
        if reader.remaining() == 0 {
            entries.remove(&key);
        } else {
            let entry = match reader.read_value(0) {
                Ok(value) => Entry::Value(value),
                Err(DecodeError::Unsupported(value_type)) => Entry::Unsupported(value_type),
                Err(DecodeError::Invalid(error)) => {
                    return Err(error.context(format!("decode Hive key {key}")))
                }
            };
            entries.insert(key, entry);
        }
        offset += length;
    }
    Ok((entries, offset))
}

fn encode_frame(bytes: &mut Vec<u8>, key: &str, value: Option<&Value>) -> Result<()> {
    if key.len() > 0xFF {
        bail!("Hive keys are limited to 255 bytes");
    }
    let start = bytes.len();
    bytes.extend_from_slice(&[0; 4]);
    bytes.push(KEY_STRING);
    bytes.push(key.len() as u8);
    bytes.extend_from_slice(key.as_bytes());
    if let Some(value) = value {
        encode_value(bytes, value, 0)?;
    }
    let length = (bytes.len() - start + 4) as u32;
    bytes[start..start + 4].copy_from_slice(&length.to_le_bytes());
    let checksum = crc32fast::hash(&bytes[start..]);
    bytes.extend_from_slice(&checksum.to_le_bytes());
    Ok(())
}

/// Lists are written untyped (`List<dynamic>`) and maps with string keys,
/// which every Flutter reader in the app accepts.
fn encode_value(bytes: &mut Vec<u8>, value: &Value, depth: usize) -> Result<()> {
    if depth > MAX_DEPTH {
        bail!("Hive value nesting is too deep");
    }
    match value {
        Value::Null => bytes.push(VALUE_NULL),
        Value::Bool(value) => bytes.extend_from_slice(&[VALUE_BOOL, u8::from(*value)]),
        Value::Number(number) => {
            let (value_type, value) = if let Some(value) = number.as_i64() {
                (VALUE_INT, value as f64)
            } else if let Some(value) = number.as_u64() {
                (VALUE_INT, value as f64)
            } else {
                let value = number.as_f64().ok_or_else(|| anyhow!("invalid number"))?;
                (VALUE_DOUBLE, value)
            };
            bytes.push(value_type);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        Value::String(value) => {
            bytes.push(VALUE_STRING);
            encode_string(bytes, value);
        }
        Value::Array(values) => {
            bytes.push(VALUE_LIST);
            bytes.extend_from_slice(&(values.len() as u32).to_le_bytes());
            for value in values {
                encode_value(bytes, value, depth + 1)?;
            }
        }
        Value::Object(values) => {
            bytes.push(VALUE_MAP);
            bytes.extend_from_slice(&(values.len() as u32).to_le_bytes());
            for (key, value) in values {
                bytes.push(VALUE_STRING);
                encode_string(bytes, key);
                encode_value(bytes, value, depth + 1)?;
            }
        }
    }
    Ok(())
}

fn encode_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

enum DecodeError {
    Unsupported(u16),
    Invalid(anyhow::Error),
}

impl From<anyhow::Error> for DecodeError {
    fn from(error: anyhow::Error) -> Self {
        Self::Invalid(error)
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn read_key(&mut self) -> Result<String> {
        match self.read_byte()? {
            KEY_STRING => {
                let length = self.read_byte()? as usize;
                self.read_string(length)
            }
            KEY_UINT => Ok(self.read_u32()?.to_string()),
            key_type => bail!("unsupported Hive key type {key_type}"),
        }
    }

    fn read_value(&mut self, depth: usize) -> Result<Value, DecodeError> {
        if depth > MAX_DEPTH {
            return Err(anyhow!("Hive value nesting is too deep").into());
        }
        let mut value_type = self.read_byte()? as u16;
        // hive_ce writes adapter ids above 255 behind an extension marker.
        if value_type == 21 {
            value_type = self.read_u16()?;
        }
        let value = match value_type {
            0 => Value::Null,
            1 => Value::Number(self.read_integer()?.into()),
            2 => Value::Number(self.read_double()?),
            3 => Value::Bool(self.read_byte()? != 0),
            4 => {
                let length = self.read_u32()? as usize;
                Value::String(self.read_string(length)?)
            }
            5 => {
                let length = self.read_u32()? as usize;
                Value::Array(
                    self.read_bytes(length)?
                        .iter()
                        .map(|value| Value::Number((*value).into()))
                        .collect(),
                )
            }
            6 | 13 => self.read_list(|reader| Ok(Value::Number(reader.read_integer()?.into())))?,
            7 | 14 => self.read_list(|reader| Ok(Value::Number(reader.read_double()?)))?,
            8 => self.read_list(|reader| Ok(Value::Bool(reader.read_byte()? != 0)))?,
            9 | 15 => self.read_list(|reader| {
                let length = reader.read_u32()? as usize;
                Ok(Value::String(reader.read_string(length)?))
            })?,
            10 | 19 => {
                let length = self.read_u32()? as usize;
                let mut values = Vec::with_capacity(length.min(1024));
                for _ in 0..length {
                    values.push(self.read_value(depth + 1)?);
                }
                Value::Array(values)
            }
            11 => {
                let length = self.read_u32()? as usize;
                let mut values = Map::new();
                for _ in 0..length {
                    let key = match self.read_value(depth + 1)? {
                        Value::String(key) => key,
                        Value::Number(key) => key.to_string(),
                        _ => return Err(anyhow!("unsupported Hive map key").into()),
                    };
                    values.insert(key, self.read_value(depth + 1)?);
                }
                Value::Object(values)
            }
            value_type => return Err(DecodeError::Unsupported(value_type)),
        };
        Ok(value)
    }

    fn read_list(&mut self, mut read: impl FnMut(&mut Self) -> Result<Value>) -> Result<Value> {
        let length = self.read_u32()? as usize;
        let mut values = Vec::with_capacity(length.min(1024));
        for _ in 0..length {
            values.push(read(self)?);
        }
        Ok(Value::Array(values))
    }

    fn read_integer(&mut self) -> Result<i64> {
        let value = f64::from_le_bytes(self.read_bytes(8)?.try_into()?);
        if !value.is_finite() || value.fract() != 0.0 {
            bail!("invalid Hive integer");
        }
        Ok(value as i64)
    }

    fn read_double(&mut self) -> Result<Number> {
        let value = f64::from_le_bytes(self.read_bytes(8)?.try_into()?);
        Number::from_f64(value).ok_or_else(|| anyhow!("invalid Hive double"))
    }

    fn read_byte(&mut self) -> Result<u8> {
        Ok(self.read_bytes(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.read_bytes(2)?.try_into()?))
    }

    fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.read_bytes(4)?.try_into()?))
    }

    fn read_string(&mut self, length: usize) -> Result<String> {
        Ok(std::str::from_utf8(self.read_bytes(length)?)?.to_owned())
    }

    fn read_bytes(&mut self, length: usize) -> Result<&'a [u8]> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Bytes Hive 2.2.3 writes for `box.put('k', 1)` then `box.put('k', 'v')`.
    fn dart_frames() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [&[VALUE_INT][..], &[VALUE_STRING][..]] {
            let mut frame = vec![0, 0, 0, 0, KEY_STRING, 1, b'k'];
            frame.extend_from_slice(value);
            if value[0] == VALUE_INT {
                frame.extend_from_slice(&1.0_f64.to_le_bytes());
            } else {
                frame.extend_from_slice(&1_u32.to_le_bytes());
                frame.push(b'v');
            }
            let length = (frame.len() + 4) as u32;
            frame[..4].copy_from_slice(&length.to_le_bytes());
            let checksum = crc32fast::hash(&frame);
            frame.extend_from_slice(&checksum.to_le_bytes());
            bytes.extend_from_slice(&frame);
        }
        bytes
    }

    #[test]
    fn reopened_box_returns_the_latest_frames_written_by_dart_and_rust() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("box.hive"), dart_frames()).unwrap();
        let hive_box = HiveBox::open(directory.path(), "box").unwrap();
        assert_eq!(hive_box.get("k").unwrap(), Some(json!("v")));

        let value = json!({"list": [1, 1.5, "a", null, true], "nested": {"n": -2}});
        hive_box.put("other", &value).unwrap();
        hive_box.delete("k").unwrap();
        drop(hive_box);

        let reopened = HiveBox::open(directory.path(), "box").unwrap();
        assert_eq!(reopened.get("other").unwrap(), Some(value));
        assert_eq!(reopened.get("k").unwrap(), None);
    }

    #[test]
    fn integers_and_doubles_keep_their_dart_types() {
        let mut bytes = Vec::new();
        encode_frame(&mut bytes, "n", Some(&json!(2))).unwrap();
        encode_frame(&mut bytes, "d", Some(&json!(2.0))).unwrap();
        let (entries, _) = parse_frames(&bytes).unwrap();
        let value = |key: &str| match &entries[key] {
            Entry::Value(value) => value.clone(),
            Entry::Unsupported(_) => panic!("unsupported"),
        };
        assert!(value("n").is_i64());
        assert!(value("d").is_f64());
        assert_eq!(bytes[7], VALUE_INT);
    }

    #[test]
    fn opening_a_box_with_a_torn_tail_truncates_it_before_appending() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("box.hive");
        let mut bytes = dart_frames();
        let valid = bytes.len();
        bytes.extend_from_slice(&[9, 0, 0, 0, 1]);
        fs::write(&path, &bytes).unwrap();

        let hive_box = HiveBox::open(directory.path(), "box").unwrap();
        assert_eq!(fs::metadata(&path).unwrap().len() as usize, valid);
        hive_box.put("after", &json!(true)).unwrap();
        drop(hive_box);

        let (entries, length) = parse_frames(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(length, fs::metadata(&path).unwrap().len() as usize);
        assert!(entries.contains_key("after"));
    }

    #[test]
    fn unsupported_values_fail_only_their_own_key() {
        let directory = tempfile::tempdir().unwrap();
        let mut frame = vec![0, 0, 0, 0, KEY_STRING, 1, b'd', 18];
        frame.extend_from_slice(&0.0_f64.to_le_bytes());
        frame.push(1);
        let length = (frame.len() + 4) as u32;
        frame[..4].copy_from_slice(&length.to_le_bytes());
        let checksum = crc32fast::hash(&frame);
        frame.extend_from_slice(&checksum.to_le_bytes());
        fs::write(directory.path().join("box.hive"), frame).unwrap();

        let hive_box = HiveBox::open(directory.path(), "box").unwrap();
        assert!(hive_box.get("d").is_err());
        hive_box.put("ok", &json!(1)).unwrap();
        assert_eq!(hive_box.get("ok").unwrap(), Some(json!(1)));
    }

    #[test]
    fn deleted_box_reopens_empty() {
        let directory = tempfile::tempdir().unwrap();
        let hive_box = HiveBox::open(directory.path(), "box").unwrap();
        hive_box.put("k", &json!(1)).unwrap();
        delete_box(directory.path(), "box").unwrap();
        assert!(hive_box.get("k").is_err());
        assert!(!directory.path().join("box.hive").exists());
        assert_eq!(
            HiveBox::open(directory.path(), "box")
                .unwrap()
                .get("k")
                .unwrap(),
            None
        );
    }

    const LOCK_HOLDER_ENV: &str = "MIXIN_HIVE_LOCK_HOLDER";

    /// Child-process half of the lock test; a no-op in normal test runs.
    #[test]
    fn lock_holder_process() {
        let Some(directory) = std::env::var_os(LOCK_HOLDER_ENV) else {
            return;
        };
        let _box = HiveBox::open(Path::new(&directory), "box").unwrap();
        println!("locked");
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
    }

    #[test]
    fn a_box_held_by_another_process_cannot_be_opened_until_released() {
        use std::io::BufRead;
        let directory = tempfile::tempdir().unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "db::hive::tests::lock_holder_process",
                "--nocapture",
            ])
            .env(LOCK_HOLDER_ENV, directory.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut lines = std::io::BufReader::new(child.stdout.take().unwrap()).lines();
        assert!(lines.any(|line| line.unwrap().trim() == "locked"));

        let error = HiveBox::open(directory.path(), "box").err().unwrap();
        assert!(error.downcast_ref::<HiveBoxLocked>().is_some());

        drop(child.stdin.take());
        lines.for_each(drop);
        assert!(child.wait().unwrap().success());
        HiveBox::open(directory.path(), "box").unwrap();
    }
}
