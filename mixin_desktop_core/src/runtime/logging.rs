use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use log::{LevelFilter, Record};
use log4rs::append::console::ConsoleAppender;
use log4rs::append::Append;
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::encode::writer::simple::SimpleWriter;
use log4rs::encode::Encode;
use log4rs::Handle;

use crate::db::path;

const MAX_LOG_FILE_SIZE: u64 = 10 * 1024 * 1024;
const MAX_LOG_FILES: usize = 10;
const LOG_PATTERN: &str = "{d(%Y-%m-%dT%H:%M:%S%.3f%:z)} {l:<5} {t} - {m}{n}";

static LOGGER: Mutex<Option<Handle>> = Mutex::new(None);

struct IndexedFileState {
    index: u64,
    writer: BufWriter<File>,
    size: u64,
    has_records: bool,
}

struct IndexedFileAppender {
    directory: PathBuf,
    header_prefix: String,
    max_file_size: u64,
    max_files: usize,
    encoder: PatternEncoder,
    state: Mutex<IndexedFileState>,
}

impl fmt::Debug for IndexedFileAppender {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IndexedFileAppender")
            .field("directory", &self.directory)
            .field("max_file_size", &self.max_file_size)
            .field("max_files", &self.max_files)
            .finish_non_exhaustive()
    }
}

impl IndexedFileAppender {
    fn new(
        directory: PathBuf,
        header_prefix: String,
        max_file_size: u64,
        max_files: usize,
    ) -> Result<Self> {
        if max_files == 0 {
            return Err(anyhow!("max log files must be greater than zero"));
        }
        fs::create_dir_all(&directory)?;
        cleanup_unmanaged_log_files(&directory)?;
        prune_old_log_files(&directory, max_files)?;
        let index = indexed_log_files(&directory)?
            .last()
            .map(|(index, _)| *index)
            .unwrap_or(0);
        let (writer, size, created) = open_log_file(&directory, index, &header_prefix)?;
        Ok(Self {
            directory,
            header_prefix,
            max_file_size,
            max_files,
            encoder: PatternEncoder::new(LOG_PATTERN),
            state: Mutex::new(IndexedFileState {
                index,
                writer,
                size,
                has_records: !created,
            }),
        })
    }

    fn rotate(&self, state: &mut IndexedFileState) -> Result<()> {
        state.writer.flush()?;
        let index = state
            .index
            .checked_add(1)
            .ok_or_else(|| anyhow!("log file index overflow"))?;
        let (writer, size, _) = open_log_file(&self.directory, index, &self.header_prefix)?;
        state.index = index;
        state.writer = writer;
        state.size = size;
        state.has_records = false;
        prune_old_log_files(&self.directory, self.max_files)
    }
}

impl Append for IndexedFileAppender {
    fn append(&self, record: &Record<'_>) -> Result<()> {
        let mut encoded = Vec::new();
        self.encoder
            .encode(&mut SimpleWriter(&mut encoded), record)?;

        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let next_size = state.size.saturating_add(encoded.len() as u64);
        if state.has_records && next_size > self.max_file_size {
            self.rotate(&mut state)?;
        }
        state.writer.write_all(&encoded)?;
        state.writer.flush()?;
        state.size = state.size.saturating_add(encoded.len() as u64);
        state.has_records = true;
        Ok(())
    }

    fn flush(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = state.writer.flush();
    }
}

pub fn init(app_name: String, app_version: String, build_number: String) -> Result<()> {
    let mut logger = LOGGER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if logger.is_some() {
        return Ok(());
    }

    let directory = path::log_directory()?;
    let header_prefix = log_header_prefix(&app_name, &app_version, &build_number);
    let file = IndexedFileAppender::new(
        directory.clone(),
        header_prefix,
        MAX_LOG_FILE_SIZE,
        MAX_LOG_FILES,
    )?;
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(LOG_PATTERN)))
        .build();
    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(file)))
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .logger(Logger::builder().build("flutter", LevelFilter::Trace))
        .build(
            Root::builder()
                .appender("file")
                .appender("stdout")
                .build(LevelFilter::Info),
        )?;
    *logger = Some(log4rs::init_config(config)?);
    log::info!(target: "logger", "initialized at {}", directory.display());
    Ok(())
}

pub fn directory() -> Result<String> {
    Ok(path::log_directory()?.to_string_lossy().into_owned())
}

fn log_header_prefix(app_name: &str, app_version: &str, build_number: &str) -> String {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    format!(
        "# app: {}\n# version: {} ({})\n# core_version: {}\n# target: {}/{}\n# profile: {}\n",
        sanitize_header_value(app_name),
        sanitize_header_value(app_version),
        sanitize_header_value(build_number),
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        profile,
    )
}

fn log_header(prefix: &str) -> String {
    format!(
        "{}# created_at: {}\n",
        prefix,
        chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, false),
    )
}

fn sanitize_header_value(value: &str) -> String {
    value.replace(['\r', '\n'], " ")
}

fn open_log_file(
    directory: &Path,
    index: u64,
    header_prefix: &str,
) -> Result<(BufWriter<File>, u64, bool)> {
    let path = directory.join(format!("log_{index}.log"));
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut size = file.metadata()?.len();
    let created = size == 0;
    if created {
        let header = log_header(header_prefix);
        file.write_all(header.as_bytes())?;
        file.flush()?;
        size = header.len() as u64;
    }
    Ok((BufWriter::new(file), size, created))
}

fn cleanup_unmanaged_log_files(directory: &Path) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry.file_name().to_string_lossy().ends_with(".log")
            && log_file_index(&entry.file_name()).is_none()
        {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

fn prune_old_log_files(directory: &Path, max_files: usize) -> Result<()> {
    let files = indexed_log_files(directory)?;
    for (_, path) in files.iter().take(files.len().saturating_sub(max_files)) {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn indexed_log_files(directory: &Path) -> Result<Vec<(u64, PathBuf)>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        if let Some(index) = log_file_index(&entry.file_name()) {
            files.push((index, entry.path()));
        }
    }
    files.sort_by_key(|(index, _)| *index);
    Ok(files)
}

fn log_file_index(name: &std::ffi::OsStr) -> Option<u64> {
    let name = name.to_str()?;
    let index = name
        .strip_prefix("log_")?
        .strip_suffix(".log")?
        .parse::<u64>()
        .ok()?;
    (name == format!("log_{index}.log")).then_some(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_increments_index_without_renaming_existing_files() {
        let directory = tempfile::tempdir().unwrap();
        for index in 0..=10 {
            fs::write(
                directory.path().join(format!("log_{index}.log")),
                format!("file {index}\n"),
            )
            .unwrap();
        }
        let appender = IndexedFileAppender::new(
            directory.path().to_path_buf(),
            "# app: Mixin\n".to_string(),
            MAX_LOG_FILE_SIZE,
            MAX_LOG_FILES,
        )
        .unwrap();

        let mut state = appender.state.lock().unwrap();
        appender.rotate(&mut state).unwrap();
        drop(state);

        assert!(!directory.path().join("log_0.log").exists());
        assert!(!directory.path().join("log_1.log").exists());
        assert_eq!(
            fs::read_to_string(directory.path().join("log_2.log")).unwrap(),
            "file 2\n"
        );
        assert!(fs::read_to_string(directory.path().join("log_11.log"))
            .unwrap()
            .starts_with("# app: Mixin\n# created_at: "));
        assert_eq!(indexed_log_files(directory.path()).unwrap().len(), 10);
    }

    #[test]
    fn removes_only_legacy_log_files() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("log_0.log"), "current").unwrap();
        fs::write(directory.path().join("log_10.log"), "indexed").unwrap();
        fs::write(directory.path().join("log_01.log"), "invalid index").unwrap();
        fs::write(directory.path().join("2026-07-19.log"), "legacy").unwrap();
        fs::write(directory.path().join("note.txt"), "keep").unwrap();

        cleanup_unmanaged_log_files(directory.path()).unwrap();

        assert!(directory.path().join("log_0.log").exists());
        assert!(directory.path().join("log_10.log").exists());
        assert!(!directory.path().join("log_01.log").exists());
        assert!(!directory.path().join("2026-07-19.log").exists());
        assert!(directory.path().join("note.txt").exists());
    }
}
