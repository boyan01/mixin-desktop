//! Native proxy tunnels. The caller applies its connect deadline and TLS to the target.
use anyhow::{anyhow, bail, Context, Result};
use base64ct::{Base64, Encoding};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use url::{Host, Url};

use crate::network::{ProxyConfig, ProxyType};

const MAX_CONNECT_HEADER: usize = 8192;

pub(super) async fn connect(target: &str, proxy: &ProxyConfig) -> Result<TcpStream> {
    proxy.validate()?;
    let target = Url::parse(target).context("invalid WebSocket target")?;
    if !matches!(target.scheme(), "ws" | "wss") {
        bail!("unsupported WebSocket scheme");
    }
    let port = target
        .port_or_known_default()
        .ok_or_else(|| anyhow!("missing WebSocket port"))?;
    let host = target
        .host()
        .ok_or_else(|| anyhow!("missing WebSocket host"))?;
    let mut stream = TcpStream::connect((proxy.host.trim_matches(['[', ']']), proxy.port))
        .await
        .context("could not connect to selected proxy")?;
    match proxy.proxy_type {
        ProxyType::Http => http_connect(&mut stream, host, port, proxy).await?,
        ProxyType::Socks5 => socks_connect(&mut stream, host, port, proxy).await?,
    }
    Ok(stream)
}

async fn http_connect(
    stream: &mut TcpStream,
    host: Host<&str>,
    port: u16,
    proxy: &ProxyConfig,
) -> Result<()> {
    let authority = format!("{host}:{port}");
    let mut request = format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n");
    if let Some(username) = proxy.username.as_deref().filter(|value| !value.is_empty()) {
        let credentials = format!(
            "{username}:{}",
            proxy.password.as_deref().unwrap_or_default()
        );
        request.push_str(&format!(
            "Proxy-Authorization: Basic {}\r\n",
            Base64::encode_string(credentials.as_bytes())
        ));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).await?;
    let mut response = Vec::new();
    while !response.ends_with(b"\r\n\r\n") {
        if response.len() >= MAX_CONNECT_HEADER {
            bail!("proxy CONNECT response header is too large");
        }
        response.push(
            stream
                .read_u8()
                .await
                .context("proxy CONNECT response ended early")?,
        );
    }
    let response = std::str::from_utf8(&response).context("invalid proxy CONNECT response")?;
    let mut status = response
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    if !matches!(status.next(), Some("HTTP/1.0" | "HTTP/1.1")) {
        bail!("invalid proxy CONNECT status line");
    }
    let status = status
        .next()
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| anyhow!("invalid proxy CONNECT status"))?;
    if !(200..300).contains(&status) {
        bail!("proxy CONNECT rejected with HTTP {status}");
    }
    Ok(())
}

async fn socks_connect(
    stream: &mut TcpStream,
    host: Host<&str>,
    port: u16,
    proxy: &ProxyConfig,
) -> Result<()> {
    let username = proxy.username.as_deref().filter(|value| !value.is_empty());
    stream
        .write_all(if username.is_some() {
            &[5, 2, 0, 2]
        } else {
            &[5, 1, 0]
        })
        .await?;
    let mut response = [0; 2];
    stream.read_exact(&mut response).await?;
    if response[0] != 5 {
        bail!("invalid SOCKS version");
    }
    match response[1] {
        0 => {}
        2 => {
            let username = username.ok_or_else(|| anyhow!("SOCKS proxy requires credentials"))?;
            let password = proxy.password.as_deref().unwrap_or_default();
            if username.len() > 255 || password.len() > 255 {
                bail!("SOCKS credentials are too long");
            }
            let mut auth = vec![1, username.len() as u8];
            auth.extend_from_slice(username.as_bytes());
            auth.push(password.len() as u8);
            auth.extend_from_slice(password.as_bytes());
            stream.write_all(&auth).await?;
            stream.read_exact(&mut response).await?;
            if response != [1, 0] {
                bail!("SOCKS proxy rejected credentials");
            }
        }
        _ => bail!("SOCKS proxy has no acceptable authentication method"),
    }
    let mut request = vec![5, 1, 0];
    match host {
        Host::Ipv4(address) => {
            request.push(1);
            request.extend_from_slice(&address.octets());
        }
        Host::Ipv6(address) => {
            request.push(4);
            request.extend_from_slice(&address.octets());
        }
        Host::Domain(domain) => {
            if domain.is_empty() || domain.len() > 255 {
                bail!("invalid SOCKS target hostname");
            }
            // Do not resolve here: the selected proxy owns target DNS.
            request.extend_from_slice(&[3, domain.len() as u8]);
            request.extend_from_slice(domain.as_bytes());
        }
    }
    request.extend_from_slice(&port.to_be_bytes());
    stream.write_all(&request).await?;
    let mut header = [0; 4];
    stream.read_exact(&mut header).await?;
    if header[0] != 5 || header[2] != 0 {
        bail!("invalid SOCKS CONNECT response");
    }
    if header[1] != 0 {
        bail!("SOCKS CONNECT rejected with code {}", header[1]);
    }
    let length = match header[3] {
        1 => std::mem::size_of::<std::net::Ipv4Addr>(),
        4 => std::mem::size_of::<std::net::Ipv6Addr>(),
        3 => usize::from(stream.read_u8().await?),
        _ => bail!("invalid SOCKS bound address"),
    };
    let mut bound = [0; 257];
    stream.read_exact(&mut bound[..length + 2]).await?;
    Ok(())
}
