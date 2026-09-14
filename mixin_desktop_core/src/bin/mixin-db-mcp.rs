use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use directories::BaseDirs;
use mixin_desktop_core::runtime::mcp::{
    inspect_read_only_database, serve_read_only_http, DEFAULT_PORT,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    port: u16,
    token: String,
    data_dir: PathBuf,
    identity_number: String,
}

impl Config {
    fn read(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("cannot read config {}", path.display()))?;
        // TOML parse errors can include the secret-bearing source line.
        let config: Self = toml::from_str(&contents).map_err(|_| {
            anyhow!("invalid MCP config: check field names, types, and TOML syntax")
        })?;
        if config.token.trim().is_empty()
            || !config.token.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(anyhow!(
                "token must contain nonempty visible ASCII without spaces"
            ));
        }
        if config.port == 0 {
            return Err(anyhow!("port must be between 1 and 65535"));
        }
        if config.data_dir.as_os_str().is_empty() || config.identity_number.trim().is_empty() {
            return Err(anyhow!(
                "set data_dir and identity_number in {}",
                path.display()
            ));
        }
        Ok(config)
    }

    fn create(&self, path: &Path) -> Result<()> {
        let contents = toml::to_string_pretty(self)?;
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("cannot create config directory {}", parent.display()))?;
        }
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(path)
            .with_context(|| format!("cannot create config {}", path.display()))?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut logging = simplelog::ConfigBuilder::new();
    logging.add_filter_allow_str("mixin_desktop_core::db::mixin::message_fts");
    logging.add_filter_allow_str("mixin_desktop_core::runtime::mcp");
    simplelog::WriteLogger::init(log::LevelFilter::Info, logging.build(), std::io::stderr())?;
    let mut arguments = env::args().skip(1);
    let command = arguments.next().unwrap_or_else(|| "--help".into());
    if matches!(command.as_str(), "-h" | "--help") {
        println!(
            "Usage:
  mixin-db-mcp init [--config <path>] [--data-dir <path>] [--identity-number <number>]
  mixin-db-mcp serve [--config <path>]
  mixin-db-mcp doctor [--config <path>]

The default config is ~/.config/mixin-db-mcp/config.toml.
init creates a private config with a random bearer token and never overwrites a file.
Set data_dir and identity_number before serving. Only 127.0.0.1 is used."
        );
        return Ok(());
    }
    if !matches!(command.as_str(), "init" | "serve" | "doctor") {
        return Err(anyhow!("unknown command: {command}"));
    }
    let mut path = None;
    let mut data_dir = PathBuf::new();
    let mut identity_number = String::new();
    while let Some(argument) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| anyhow!("missing value for {argument}"))?;
        match argument.as_str() {
            "--config" => path = Some(PathBuf::from(value)),
            "--data-dir" if command == "init" => data_dir = PathBuf::from(value),
            "--identity-number" if command == "init" => identity_number = value,
            _ => return Err(anyhow!("unknown option: {argument}")),
        }
    }
    let path = match path {
        Some(path) => path,
        None => BaseDirs::new()
            .ok_or_else(|| anyhow!("cannot determine home directory; use --config"))?
            .home_dir()
            .join(".config/mixin-db-mcp/config.toml"),
    };
    if command == "init" {
        Config {
            port: DEFAULT_PORT,
            token: Uuid::new_v4().simple().to_string(),
            data_dir,
            identity_number,
        }
        .create(&path)?;
        println!(
            "Created {}. Set data_dir and identity_number before serving.",
            path.display()
        );
        return Ok(());
    }
    let config = Config::read(&path)?;
    match command.as_str() {
        "doctor" => {
            let info = inspect_read_only_database(config.data_dir, config.identity_number).await?;
            println!("status: ok");
            println!("mode: read-only");
            println!("database: {}", info.database_path.display());
            println!("identity_number: {}", info.identity_number);
            println!("user_id: {}", info.user_id);
            println!("schema_version: {}", info.schema_version);
        }
        "serve" => {
            serve_read_only_http(
                config.data_dir,
                config.identity_number,
                config.port,
                config.token,
            )
            .await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_private_config_and_refuses_to_replace_existing_token() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested/mixin-db-mcp/config.toml");
        let config = Config {
            port: DEFAULT_PORT,
            token: Uuid::new_v4().simple().to_string(),
            data_dir: directory.path().to_owned(),
            identity_number: "123".into(),
        };
        config.create(&path).unwrap();
        assert_eq!(Config::read(&path).unwrap().token, config.token);
        assert!(config.create(&path).is_err());
        assert_eq!(Config::read(&path).unwrap().token, config.token);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn invalid_credentials_and_incomplete_config_are_rejected_without_echoing_secrets() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        for token in ["", " ", "secret with space", "secret\nvalue"] {
            let config = Config {
                port: DEFAULT_PORT,
                token: token.into(),
                data_dir: directory.path().to_owned(),
                identity_number: "123".into(),
            };
            fs::write(&path, toml::to_string(&config).unwrap()).unwrap();
            assert!(Config::read(&path).is_err());
        }
        for (port, data_dir, identity_number) in [
            (0, "/data", "123"),
            (DEFAULT_PORT, "", "123"),
            (DEFAULT_PORT, "/data", ""),
        ] {
            let config = Config {
                port,
                token: "valid-token".into(),
                data_dir: data_dir.into(),
                identity_number: identity_number.into(),
            };
            fs::write(&path, toml::to_string(&config).unwrap()).unwrap();
            assert!(Config::read(&path).is_err());
        }
        fs::write(&path, "token = \"private-secret").unwrap();
        let error = Config::read(&path).err().unwrap().to_string();
        assert!(!error.contains("private-secret"));
    }
}
