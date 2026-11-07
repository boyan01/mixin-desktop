use std::{sync::Arc, time::Duration};

use serde_json::Value;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

use super::AccountApi;
use crate::{client::ClientRef, ApiError, Credential, SignalKeyRequest, SignedPreKey};

async fn endpoint(
    status: u16,
    response: &'static str,
) -> (AccountApi, tokio::task::JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut client = ClientRef::new(Credential::None, None);
    client.base_url = format!("http://{}", listener.local_addr().unwrap());
    client.client = reqwest::Client::builder().no_proxy().build().unwrap();
    let api = AccountApi::new(Arc::new(client));
    let server = tokio::spawn(async move {
        let (mut connection, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut chunk = [0_u8; 1024];
        loop {
            let count = connection.read(&mut chunk).await.unwrap();
            assert!(count != 0, "request ended before its complete body");
            request.extend_from_slice(&chunk[..count]);
            assert!(request.len() <= 64 * 1024, "request exceeded fixture bound");
            let Some(end) = request.windows(4).position(|part| part == b"\r\n\r\n") else {
                continue;
            };
            let headers = std::str::from_utf8(&request[..end]).unwrap();
            let length = headers
                .lines()
                .find_map(|line| {
                    line.split_once(':').and_then(|(name, value)| {
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                })
                .unwrap_or(0);
            if request.len() < end + 4 + length {
                continue;
            }
            assert_eq!(request.len(), end + 4 + length);
            let head = format!(
                "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                response.len()
            );
            connection.write_all(head.as_bytes()).await.unwrap();
            connection.write_all(response.as_bytes()).await.unwrap();
            return request;
        }
    });
    (api, server)
}

fn keys() -> SignalKeyRequest {
    SignalKeyRequest {
        identity_key: "fixture-identity".into(),
        signed_pre_key: SignedPreKey {
            key_id: 7,
            pub_key: Some("fixture-public-key".into()),
            signature: "fixture-signature".into(),
        },
        one_time_pre_keys: vec![crate::OneTimePreKey {
            key_id: 8,
            pub_key: Some("fixture-pre-key".into()),
        }],
    }
}

fn assert_upload(request: Vec<u8>, keys: &SignalKeyRequest) {
    let end = request
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .unwrap();
    let headers = std::str::from_utf8(&request[..end]).unwrap();
    assert_eq!(headers.lines().next(), Some("POST /signal/keys HTTP/1.1"));
    assert!(headers
        .lines()
        .any(|line| line.eq_ignore_ascii_case("content-type: application/json")));
    assert_eq!(
        serde_json::from_slice::<Value>(&request[end + 4..]).unwrap(),
        serde_json::to_value(keys).unwrap()
    );
}

#[tokio::test]
async fn empty_successful_signal_upload_response_completes() {
    for body in ["{}", "", r#"{"data":null}"#] {
        let (api, server) = endpoint(200, body).await;
        let keys = keys();
        let result = tokio::time::timeout(Duration::from_secs(2), api.push_signal_keys(&keys))
            .await
            .expect("signal upload fixture timed out");
        assert_upload(server.await.unwrap(), &keys);
        assert!(
            result.is_ok(),
            "successful void response {body:?}: {result:?}"
        );
    }
}

#[tokio::test]
async fn rejected_signal_upload_response_preserves_server_error() {
    for status in [200, 401] {
        let (api, server) = endpoint(
            status,
            r#"{"error":{"status":401,"code":401,"description":"Unauthorized"}}"#,
        )
        .await;
        let keys = keys();
        let result = tokio::time::timeout(Duration::from_secs(2), api.push_signal_keys(&keys))
            .await
            .expect("signal rejection fixture timed out");
        assert_upload(server.await.unwrap(), &keys);
        assert!(matches!(
            result,
            Err(ApiError::Server(crate::Error { code: 401, .. }))
        ));
    }
}

#[tokio::test]
async fn empty_signal_key_count_response_is_rejected() {
    let (api, server) = endpoint(200, "{}").await;
    let result = tokio::time::timeout(Duration::from_secs(2), api.get_signal_key_count())
        .await
        .expect("signal count fixture timed out");
    let request = server.await.unwrap();
    assert!(request.starts_with(b"GET /signal/keys/count HTTP/1.1\r\n"));
    assert!(result.is_err());
}
