use std::env;

use anyhow::{anyhow, Result};
use futures::StreamExt;
use mixin_desktop_api::{AccountClient, ClientError, DesktopClient};
use qrcode::{render::unicode, QrCode};
use tokio::signal::unix::{signal, SignalKind};

const USAGE: &str = "Usage:
  mixin-headless login   Link this machine to a Mixin account by scanning a QR code
  mixin-headless serve [--port <port>] [--allow-send]
                         Keep the account online and serve on 127.0.0.1
                         (default port 55001): MCP at /mcp, message events at
                         /events. --allow-send enables the mixin_send_text tool.
  mixin-headless token   Print the MCP bearer token

MIXIN_DESKTOP_DATA_DIR must be set so data stays apart from the Mixin desktop app.";

const DATA_DIRECTORY_ENV: &str = "MIXIN_DESKTOP_DATA_DIR";

#[tokio::main]
async fn main() -> Result<()> {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().unwrap_or_else(|| "--help".into());
    if matches!(command.as_str(), "-h" | "--help") {
        println!("{USAGE}");
        return Ok(());
    }
    if !matches!(command.as_str(), "login" | "serve" | "token") {
        return Err(anyhow!("unknown command: {command}\n\n{USAGE}"));
    }
    let mut port = None;
    let mut allow_send = false;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--port" if command == "serve" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| anyhow!("missing value for --port"))?;
                port = Some(
                    value
                        .parse::<u16>()
                        .ok()
                        .filter(|port| *port != 0)
                        .ok_or_else(|| anyhow!("port must be between 1 and 65535"))?,
                );
            }
            "--allow-send" if command == "serve" => allow_send = true,
            _ => return Err(anyhow!("unknown option: {argument}\n\n{USAGE}")),
        }
    }
    // Without it the runtime falls back to the desktop app's data directory
    // and would share its account and database.
    if env::var_os(DATA_DIRECTORY_ENV).is_none_or(|value| value.is_empty()) {
        return Err(anyhow!("set {DATA_DIRECTORY_ENV} to a dedicated directory"));
    }
    let desktop = DesktopClient::open().await?;
    match command.as_str() {
        "login" => login(&desktop).await,
        "serve" => serve(&desktop, port, allow_send).await,
        _ => {
            println!("{}", desktop.settings().mcp_settings().await?.token);
            Ok(())
        }
    }
}

async fn restore(desktop: &DesktopClient) -> Result<Option<AccountClient>> {
    match desktop.restore_account().await {
        Ok(account) => Ok(Some(account)),
        Err(ClientError::NotFound) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

async fn login(desktop: &DesktopClient) -> Result<()> {
    if let Some(account) = restore(desktop).await? {
        println!("already signed in as {}", account.profile().identity_number);
        account.shutdown().await;
        return Ok(());
    }
    loop {
        let login = desktop.begin_login().await?;
        let code = QrCode::new(login.auth_url())?;
        println!(
            "Scan with the Mixin app of the account to link (expires in 60 seconds):\n{}",
            code.render::<unicode::Dense1x2>().quiet_zone(true).build()
        );
        match login.wait().await {
            Ok(account) => {
                let profile = account.profile();
                println!(
                    "signed in as {} ({})",
                    profile.full_name, profile.identity_number
                );
                account.shutdown().await;
                return Ok(());
            }
            // Same recovery as the UI targets: refresh an expired code, and
            // revoke a half-provisioned session before reporting the failure.
            Err(error) if error.to_string().contains("authorization timed out") => continue,
            Err(error) if error.to_string().contains("login_provisioning_error:") => {
                desktop.abort_saved_login().await?;
                return Err(error.into());
            }
            Err(error) => return Err(error.into()),
        }
    }
}

async fn serve(desktop: &DesktopClient, port: Option<u16>, allow_send: bool) -> Result<()> {
    let settings = desktop.settings();
    if let Some(port) = port {
        settings.set_mcp_port(port).await;
    }
    settings.set_mcp_message_send(allow_send).await;
    let account = restore(desktop)
        .await?
        .ok_or_else(|| anyhow!("no signed-in account; run `mixin-headless login` first"))?;
    let result = run(desktop, &account, allow_send).await;
    account.shutdown().await;
    result
}

async fn run(desktop: &DesktopClient, account: &AccountClient, allow_send: bool) -> Result<()> {
    // The MCP server only applies settings once an account runtime exists.
    let settings = desktop.settings();
    let mut mcp = settings.mcp_settings().await?;
    mcp.enabled = true;
    let status = settings.update_mcp_settings(mcp).await?;
    let endpoint = status.endpoint.ok_or_else(|| {
        anyhow!(
            "MCP server failed to start: {}",
            status.last_error.unwrap_or_default()
        )
    })?;
    println!(
        "serving {} and {} as {}{}",
        endpoint,
        endpoint.replace("/mcp", "/events"),
        account.profile().identity_number,
        if allow_send { " (sending enabled)" } else { "" }
    );

    let mut terminate = signal(SignalKind::terminate())?;
    let mut connection = Box::pin(account.connection_status());
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => return Ok(()),
            _ = terminate.recv() => return Ok(()),
            next = connection.next() => if next.is_none() {
                return Err(anyhow!("account runtime stopped"));
            },
        }
    }
}
