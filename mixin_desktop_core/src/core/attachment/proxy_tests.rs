use super::*;
use crate::{
    db::app::AppDatabase,
    network::{NetworkService, ProxyConfig, ProxySettings, ProxyType},
};
use base64ct::{Base64, Encoding};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

fn config(port: u16, id: &str) -> ProxyConfig {
    ProxyConfig {
        id: id.into(),
        proxy_type: ProxyType::Http,
        host: "127.0.0.1".into(),
        port,
        username: Some("proxy-user".into()),
        password: Some("proxy-password".into()),
    }
}

async fn read_request(stream: &mut TcpStream) -> (String, Vec<u8>) {
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        assert!(header.len() < 16 * 1024);
        header.push(stream.read_u8().await.unwrap());
    }
    let header = String::from_utf8(header).unwrap();
    let lower = header.to_lowercase();
    let length = lower
        .lines()
        .find_map(|line| {
            line.strip_prefix("content-length:")
                .map(|value| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or_default();
    assert!(length < 128 * 1024);
    let mut body = vec![0; length];
    stream.read_exact(&mut body).await.unwrap();
    (header, body)
}

async fn request(stream: &mut TcpStream) -> (String, Vec<u8>) {
    let (header, body) = read_request(stream).await;
    assert!(header.to_lowercase().contains(&format!(
        "proxy-authorization: basic {}",
        Base64::encode_string(b"proxy-user:proxy-password").to_lowercase()
    )));
    (header, body)
}

async fn respond(stream: &mut TcpStream, body: &[u8]) {
    stream
        .write_all(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    stream.write_all(body).await.unwrap();
}

#[tokio::test]
async fn sdk_and_attachment_bytes_follow_proxy_authentication_and_live_switch() {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let directory = tempfile::tempdir().unwrap();
        let database = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        let first = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let second = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let first_config = config(first.local_addr().unwrap().port(), "first");
        let second_config = config(second.local_addr().unwrap().port(), "second");
        database
            .setting_dao
            .set_proxy_settings(ProxySettings {
                enabled: true,
                selected_proxy_id: Some("first".into()),
                proxies: vec![first_config.clone(), second_config.clone()],
            })
            .await
            .unwrap();
        let network = NetworkService::new(database.setting_dao.clone())
            .await
            .unwrap();
        let sdk = Arc::new(network.sdk_client(sdk::Credential::KeyStore(sdk::KeyStore {
            app_id: "fixture-user".into(),
            session_id: "fixture-session".into(),
            server_public_key: String::new(),
            session_private_key: "11".repeat(32),
            scp: "FULL".into(),
        })));
        let attachment = AttachmentService::new(sdk.clone(), HttpClient::new(), directory.path())
            .with_http_clients(network.http_clients());
        let first_server = tokio::spawn(async move {
            for (method, path, response) in [
                ("GET", "sdk", br#"{"data":{"route":"first"}}"#.as_slice()),
                ("GET", "download", b"download bytes".as_slice()),
                ("PUT", "upload", b"".as_slice()),
            ] {
                let (mut stream, _) = first.accept().await.unwrap();
                let (header, body) = request(&mut stream).await;
                assert!(
                    header.starts_with(&format!("{method} http://network.invalid/{path} HTTP/1.1"))
                );
                if path == "sdk" {
                    let lower = header.to_lowercase();
                    assert!(lower.contains("user-agent: mixin"));
                    assert!(lower.contains("authorization: bearer ey"));
                }
                if method == "PUT" {
                    assert_eq!(body, b"upload bytes");
                } else {
                    assert!(body.is_empty());
                }
                respond(&mut stream, response).await;
            }
        });
        let second_server = tokio::spawn(async move {
            for (path, response) in [
                ("sdk", br#"{"data":{"route":"second"}}"#.as_slice()),
                ("download", b"second download".as_slice()),
            ] {
                let (mut stream, _) = second.accept().await.unwrap();
                let (header, body) = request(&mut stream).await;
                assert!(header.starts_with(&format!("GET http://network.invalid/{path} HTTP/1.1")));
                assert!(body.is_empty());
                respond(&mut stream, response).await;
            }
        });
        let api_request = || {
            reqwest::Request::new(
                reqwest::Method::GET,
                reqwest::Url::parse("http://network.invalid/sdk").unwrap(),
            )
        };
        let value: serde_json::Value = sdk.request(api_request()).await.unwrap();
        assert_eq!(value["route"], "first");
        let downloaded = directory.path().join("download");
        attachment
            .download_to_file(
                &reqwest::Url::parse("http://network.invalid/download").unwrap(),
                &downloaded,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(std::fs::read(&downloaded).unwrap(), b"download bytes");
        let upload = directory.path().join("upload");
        std::fs::write(&upload, b"upload bytes").unwrap();
        let updates = Arc::new(std::sync::Mutex::new(Vec::new()));
        let record = updates.clone();
        attachment
            .upload_file(
                &upload,
                reqwest::Url::parse("http://network.invalid/upload").unwrap(),
                12,
                Some(Arc::new(move |done, total| {
                    record.lock().unwrap().push((done, total))
                })),
            )
            .await
            .unwrap();
        assert!(updates.lock().unwrap().contains(&(12, 12)));
        let mut applied = network.proxy_changes();
        database
            .setting_dao
            .set_proxy_settings(ProxySettings {
                enabled: true,
                selected_proxy_id: Some("second".into()),
                proxies: vec![first_config, second_config.clone()],
            })
            .await
            .unwrap();
        applied.changed().await.unwrap();
        assert_eq!(applied.borrow().as_ref(), Some(&second_config));
        let value: serde_json::Value = sdk.request(api_request()).await.unwrap();
        assert_eq!(value["route"], "second");
        let downloaded_second = directory.path().join("download-second");
        attachment
            .download_to_file(
                &reqwest::Url::parse("http://network.invalid/download").unwrap(),
                &downloaded_second,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(downloaded_second).unwrap(),
            b"second download"
        );
        first_server.await.unwrap();
        second_server.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn provisioning_uses_selected_proxy_before_attempting_the_remote_api() {
    tokio::time::timeout(std::time::Duration::from_secs(5),async {
        let directory=tempfile::tempdir().unwrap();let database=Arc::new(AppDatabase::connect_at(directory.path().join("app.db")).await.unwrap());
        let listener=TcpListener::bind("127.0.0.1:0").await.unwrap();let proxy=config(listener.local_addr().unwrap().port(),"login");
        database.setting_dao.set_proxy_settings(ProxySettings {enabled:true,selected_proxy_id:Some("login".into()),proxies:vec![proxy]}).await.unwrap();
        let network=Arc::new(NetworkService::new(database.setting_dao.clone()).await.unwrap());
        let auth=crate::core::model::auth::AuthService::new(database).with_network(network);
        let server=tokio::spawn(async move {
            let(mut stream,_)=listener.accept().await.unwrap();let(header,body)=request(&mut stream).await;
            assert!(header.starts_with("CONNECT api.mixin.one:443 HTTP/1.1"));assert!(body.is_empty());
            stream.write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
        });
        assert!(auth.begin_authorization("fixture").await.is_err());server.await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn sdk_socks_proxy_authenticates_and_resolves_the_target_at_the_proxy() {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let directory = tempfile::tempdir().unwrap();
        let database = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut proxy = config(listener.local_addr().unwrap().port(), "socks");
        proxy.proxy_type = ProxyType::Socks5;
        database
            .setting_dao
            .set_proxy_settings(ProxySettings {
                enabled: true,
                selected_proxy_id: Some("socks".into()),
                proxies: vec![proxy],
            })
            .await
            .unwrap();
        let network = NetworkService::new(database.setting_dao).await.unwrap();
        let client = network.sdk_client(sdk::Credential::None);
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            assert_eq!(stream.read_u8().await.unwrap(), 5);
            let count = usize::from(stream.read_u8().await.unwrap());
            let mut methods = vec![0; count];
            stream.read_exact(&mut methods).await.unwrap();
            assert!(methods.contains(&2));
            stream.write_all(&[5, 2]).await.unwrap();
            assert_eq!(stream.read_u8().await.unwrap(), 1);
            let length = usize::from(stream.read_u8().await.unwrap());
            let mut user = vec![0; length];
            stream.read_exact(&mut user).await.unwrap();
            assert_eq!(user, b"proxy-user");
            let length = usize::from(stream.read_u8().await.unwrap());
            let mut password = vec![0; length];
            stream.read_exact(&mut password).await.unwrap();
            assert_eq!(password, b"proxy-password");
            stream.write_all(&[1, 0]).await.unwrap();
            let mut header = [0; 4];
            stream.read_exact(&mut header).await.unwrap();
            assert_eq!(header, [5, 1, 0, 3]);
            let length = usize::from(stream.read_u8().await.unwrap());
            let mut target = vec![0; length];
            stream.read_exact(&mut target).await.unwrap();
            assert_eq!(target, b"network.invalid");
            assert_eq!(stream.read_u16().await.unwrap(), 80);
            stream
                .write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 1])
                .await
                .unwrap();
            let (header, body) = read_request(&mut stream).await;
            assert!(header.starts_with("GET /sdk HTTP/1.1"));
            assert!(body.is_empty());
            respond(&mut stream, br#"{"data":{"route":"socks"}}"#).await;
        });
        let response: serde_json::Value = client
            .request(reqwest::Request::new(
                reqwest::Method::GET,
                reqwest::Url::parse("http://network.invalid/sdk").unwrap(),
            ))
            .await
            .unwrap();
        assert_eq!(response["route"], "socks");
        server.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn only_http_not_found_marks_a_download_as_expired() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for status in ["404 Not Found", "500 Internal Server Error"] {
            let (mut stream, _) = listener.accept().await.unwrap();
            read_request(&mut stream).await;
            let response =
                format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            stream.write_all(response.as_bytes()).await.unwrap();
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let attachment = AttachmentService::new(
        Arc::new(MixinClient::new(sdk::Credential::None)),
        reqwest::Client::builder().no_proxy().build().unwrap(),
        directory.path(),
    );
    let url = reqwest::Url::parse(&format!("http://{address}/attachment")).unwrap();
    let target = directory.path().join("download");

    let not_found = attachment
        .download_to_file(&url, &target, None, None)
        .await
        .unwrap_err();
    let server_error = attachment
        .download_to_file(&url, &target, None, None)
        .await
        .unwrap_err();

    assert!(is_attachment_expired(&not_found));
    assert!(!is_attachment_expired(&server_error));
    server.await.unwrap();
}
