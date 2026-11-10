use std::{sync::Arc, time::Duration};

use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

use super::{UserApi, UserRelationship};
use crate::{client::ClientRef, ApiError, Credential};

async fn endpoint(status: u16, response: Value) -> (UserApi, tokio::task::JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut client = ClientRef::new(Credential::None, None);
    client.base_url = format!("http://{}", listener.local_addr().unwrap());
    client.client = reqwest::Client::builder().no_proxy().build().unwrap();
    let api = UserApi::new(Arc::new(client));
    let server = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(2), async move {
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
                let response = response.to_string();
                let head = format!(
                    "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    response.len()
                );
                connection.write_all(head.as_bytes()).await.unwrap();
                connection.write_all(response.as_bytes()).await.unwrap();
                return request;
            }
        })
        .await
        .expect("relationship fixture timed out")
    });
    (api, server)
}

fn assert_rename(request: Vec<u8>, user_id: &str, full_name: &str) {
    let end = request
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .unwrap();
    let headers = std::str::from_utf8(&request[..end]).unwrap();
    assert_eq!(headers.lines().next(), Some("POST /relationships HTTP/1.1"));
    assert!(headers
        .lines()
        .any(|line| line.eq_ignore_ascii_case("content-type: application/json")));
    assert_eq!(
        serde_json::from_slice::<Value>(&request[end + 4..]).unwrap(),
        json!({"action": "UPDATE", "user_id": user_id, "full_name": full_name})
    );
}

#[tokio::test]
async fn stranger_rename_sends_update_and_full_name_and_returns_server_profile() {
    let user_id = "fixture-stranger";
    let full_name = "NSH \"备注\" 🥭";
    let (api, server) = endpoint(
        200,
        json!({"data": {
            "user_id": user_id,
            "identity_number": "12345",
            "full_name": "Server-normalized nickname",
            "relationship": "STRANGER",
            "biography": "",
            "is_verified": false,
            "is_scam": false,
        }}),
    )
    .await;
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        api.rename_contact(user_id, full_name),
    )
    .await;
    assert_rename(server.await.unwrap(), user_id, full_name);
    let user = result.expect("rename request timed out").unwrap();
    assert_eq!(user.user_id, user_id);
    assert_eq!(user.full_name, "Server-normalized nickname");
    assert!(matches!(
        user.relationship,
        Some(UserRelationship::Stranger)
    ));
}

#[tokio::test]
async fn rejected_rename_preserves_server_error() {
    let user_id = "fixture-stranger";
    let full_name = "Rejected nickname";
    let (api, server) = endpoint(
        400,
        json!({"error": {
            "status": 400,
            "code": 10002,
            "description": "Nickname rejected",
        }}),
    )
    .await;
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        api.rename_contact(user_id, full_name),
    )
    .await;
    assert_rename(server.await.unwrap(), user_id, full_name);
    assert!(matches!(
        result.expect("rename rejection timed out"),
        Err(ApiError::Server(crate::Error { code: 10002, .. }))
    ));
}

#[tokio::test]
async fn friends_reads_the_contact_list_route() {
    let (api, server) = endpoint(
        200,
        json!({"data": [{
            "user_id": "friend", "identity_number": "7000", "relationship": "FRIEND",
            "biography": "", "is_verified": false, "is_scam": false, "app": null,
        }]}),
    )
    .await;

    let users = api.friends().await.unwrap();

    let request = server.await.unwrap();
    assert!(request.starts_with(b"GET /friends HTTP/1.1"));
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].user_id, "friend");
}

#[tokio::test]
async fn blocking_users_reads_the_blocked_list_route() {
    let (api, server) = endpoint(
        200,
        json!({"data": [{
            "user_id": "blocked", "identity_number": "7001", "relationship": "BLOCKING",
            "biography": "", "is_verified": false, "is_scam": false, "app": null,
        }]}),
    )
    .await;

    let users = api.blocking_users().await.unwrap();

    let request = server.await.unwrap();
    assert!(request.starts_with(b"GET /blocking_users HTTP/1.1"));
    assert_eq!(users[0].user_id, "blocked");
    assert!(matches!(
        users[0].relationship,
        Some(UserRelationship::Blocking)
    ));
}
