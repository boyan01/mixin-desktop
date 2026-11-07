use super::*;
use crate::core::{
    attachment::AttachmentService,
    conversation_change::ConversationChangeNotifier,
    crypto::signal_protocol::SignalProtocol,
    message::{blaze::Blaze, sender::MessageSender},
    model::{signal::SignalService, AppService, ConversationService},
};
use crate::db::{app::Auth, MixinDatabase};
use crate::runtime::{
    attachment_progress::AttachmentProgresses, lifecycle::monitor_session, runner, AccountState,
};
use sdk::{Account, Client, Credential};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Mutex as SyncMutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{watch, RwLock},
};
use tokio_util::sync::CancellationToken;

const USER: &str = "10000000-0000-4000-8000-000000000001";
const SESSION: &str = "10000000-0000-4000-8000-000000000002";

fn auth(session_id: &str, seed: u8) -> Auth {
    let account: Account = serde_json::from_value(serde_json::json!({
        "user_id": USER, "app": null, "avatar_url": null, "biography": "", "code_id": "", "code_url": "",
        "created_at": "", "device_status": "", "fiat_currency": "USD", "full_name": "Synthetic lifecycle fixture",
        "has_emergency_contact": false, "accept_search_source": "EVERYBODY", "accept_conversation_source": "EVERYBODY",
        "receive_message_source": "EVERYBODY", "has_pin": false, "has_safe": false, "identity_number": "7000000",
        "is_deactivated": false, "is_scam": false, "is_verified": false, "mute_until": "", "phone": "", "pin_token": "",
        "pin_token_base64": "", "salt_base64": "", "session_id": session_id, "spend_public_key": "", "tip_counter": 0,
        "tip_key_base64": "", "transfer_confirmation_threshold": 0, "transfer_notification_threshold": 0
    })).unwrap();
    Auth {
        user_id: USER.into(),
        account,
        private_key: vec![seed; 32],
        primary_session_id: None,
    }
}

struct Fixture {
    _directory: tempfile::TempDir,
    database: Arc<AppDatabase>,
    desktop: DesktopRuntime,
    runtime: Arc<AccountRuntime>,
    monitor: tokio::task::JoinHandle<()>,
    http: tokio::task::JoinHandle<()>,
    stop_http: CancellationToken,
    address: std::net::SocketAddr,
}

impl Fixture {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let account_directory = directory.path().join("account");
        std::fs::create_dir(&account_directory).unwrap();
        let database = Arc::new(
            AppDatabase::connect_at(directory.path().join("app.db"))
                .await
                .unwrap(),
        );
        let auth_service = Arc::new(AuthService::new(database.clone()));
        let session = auth(SESSION, 1);
        auth_service.save_auth(&session).await.unwrap();
        let settings = database.setting_dao.clone();
        let network = Arc::new(NetworkService::new(settings.clone()).await.unwrap());
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let stop_http = CancellationToken::new();
        let stopping = stop_http.clone();
        let http = tokio::spawn(async move {
            loop {
                let (mut stream, _) = tokio::select! {
                    _ = stopping.cancelled() => return,
                    accepted = listener.accept() => accepted.unwrap(),
                };
                let mut request = Vec::new();
                loop {
                    let mut byte = [0_u8; 1];
                    if stream.read(&mut byte).await.unwrap() == 0 {
                        break;
                    }
                    request.push(byte[0]);
                    assert!(request.len() < 8192, "unbounded fixture request headers");
                    if request.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8(request).unwrap();
                let status = if request.starts_with("CONNECT api.mixin.one:443 HTTP/1.1\r\n") {
                    // Sign-out revocation is exercised, without opening any remote socket.
                    "502 Bad Gateway"
                } else {
                    assert!(
                        request.starts_with(&format!(
                            "GET http://{address}/unauthorized HTTP/1.1\r\n"
                        )),
                        "unexpected fixture request"
                    );
                    "401 Unauthorized"
                };
                let body =
                    r#"{"error":{"status":401,"code":401,"description":"test session rejected"}}"#;
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        });
        let transport = reqwest::Client::builder()
            .no_proxy()
            .proxy(reqwest::Proxy::all(format!("http://{address}")).unwrap())
            .build()
            .unwrap();
        let (_, transports) = watch::channel(transport.clone());
        let client = Arc::new(Client::new_with_http_clients(
            Credential::None,
            None,
            transports,
        ));
        let account_db = Arc::new(
            MixinDatabase::connect_at(account_directory.join("mixin.db"))
                .await
                .unwrap(),
        );
        let signal = Arc::new(
            SignalDatabase::connect_at(account_directory.join("signal.db"))
                .await
                .unwrap(),
        );
        let protocol = Arc::new(SignalProtocol::new(signal.clone(), USER.into()));
        let blaze = Arc::new(Blaze::new(
            account_db.clone(),
            client.clone(),
            Credential::None,
            USER.into(),
            None,
        ));
        let sender = Arc::new(MessageSender::new(
            blaze.clone(),
            ConversationService::new(account_db.clone(), client.clone(), USER.into()),
            account_db.clone(),
            USER.into(),
            SESSION.into(),
            protocol.clone(),
            SignalService::new(protocol, signal.clone(), client.clone()),
        ));
        let attachment = Arc::new(AttachmentService::new(
            client.clone(),
            transport,
            account_directory.clone(),
        ));
        let changes = ConversationChangeNotifier::new();
        let app_service = Arc::new(AppService::new(
            account_db.clone(),
            client.clone(),
            &session,
            sender.clone(),
            attachment,
            Some(changes.clone()),
        ));
        let transfer = crate::core::device_transfer::DeviceTransferService::new(
            account_db.clone(),
            sender,
            USER.into(),
            SESSION.into(),
            None,
            account_directory.clone(),
            changes.clone(),
        );
        let (shutdown, receiver) = watch::channel(false);
        let runner = runner::Runner::observing_shutdown(receiver.clone());
        let state = Arc::new(AccountState {
            account_id: USER.into(),
            account_data_dir: account_directory,
            profile: watch::channel(session.account.clone()).0,
            auth_service: auth_service.clone(),
            client,
            database: account_db,
            signal_database: signal,
            app_service,
            conversation_changes: changes,
            shutdown: receiver,
            notification_changes: watch::channel(0).0,
            blaze,
            device_transfer: transfer,
            screen_lock: tokio::sync::OnceCell::new(),
            account_health: watch::channel("ready".into()).0,
            active: AtomicBool::new(true),
            mutation_gate: RwLock::new(()),
            attachment_downloads: SyncMutex::new(HashMap::new()),
            attachment_progresses: AttachmentProgresses::default(),
            bot_public_keys: Mutex::new(HashMap::new()),
            network: network.clone(),
            settings: settings.clone(),
        });
        let monitor = tokio::spawn(monitor_session(
            state.clone(),
            session.clone(),
            shutdown.clone(),
            runner.completion(),
        ));
        let runtime = Arc::new(AccountRuntime {
            state,
            shutdown,
            runner,
        });
        let mcp_server = McpServer::new(settings.clone()).await.unwrap();
        mcp_server.set_port(0).await;
        let desktop = DesktopRuntime {
            auth_service,
            settings,
            network_service: network,
            account: Mutex::new(Some(runtime.clone())),
            mcp_server,
        };
        Self {
            _directory: directory,
            database,
            desktop,
            runtime,
            monitor,
            http,
            stop_http,
            address,
        }
    }

    async fn start_mcp(&self) -> (String, String) {
        self.desktop
            .mcp_server
            .start(self.runtime.clone())
            .await
            .unwrap();
        let settings = self.desktop.mcp_server.settings().await.unwrap();
        let token = settings.token.clone();
        let status = self
            .desktop
            .mcp_server
            .update_settings(super::super::mcp::McpSettings {
                enabled: true,
                ..settings
            })
            .await
            .unwrap();
        assert!(status.running);
        (status.endpoint.unwrap().replace("/mcp", "/events"), token)
    }

    /// Saved sessions live in the shared Hive state next to `app.db`.
    fn reject_auth_writes(&self, reject: bool) {
        crate::db::hive::hydrated_box(self._directory.path())
            .unwrap()
            .fail_writes(reject);
    }

    async fn unauthorized(&self) {
        let request = reqwest::Request::new(
            reqwest::Method::GET,
            format!("http://{}/unauthorized", self.address)
                .parse()
                .unwrap(),
        );
        let result = self
            .runtime
            .client
            .request::<serde_json::Value>(request)
            .await;
        assert!(matches!(result, Err(sdk::ApiError::Server(error)) if error.code == 401));
    }

    async fn wait_retired(&self) {
        let mut health = self.runtime.subscribe_account_health();
        tokio::time::timeout(Duration::from_secs(3), async {
            while health.borrow_and_update().as_str() != "unauthorized" {
                health.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
        self.runtime.runner.wait().await;
    }

    async fn close(self) {
        self.desktop.mcp_server.stop().await;
        self.runtime.shutdown().await;
        self.monitor.await.unwrap();
        self.runtime.database.close().await;
        self.runtime.signal_database.close().await;
        self.database.property_dao.0.close().await;
        self.stop_http.cancel();
        self.http.await.unwrap();
    }
}

#[tokio::test]
async fn failed_credential_deletion_keeps_account_and_mcp_running_until_retry_succeeds() {
    let fixture = Fixture::new().await;
    let (service, fail_delete, _fail_save, stored) =
        crate::runtime::screen_lock::tests::deletion_failure_fixture().await;
    fixture
        .runtime
        .screen_lock
        .set(service.clone())
        .ok()
        .unwrap();
    fixture
        .desktop
        .mcp_server
        .start(fixture.runtime.clone())
        .await
        .unwrap();
    let settings = fixture.desktop.mcp_server.settings().await.unwrap();
    fixture
        .desktop
        .mcp_server
        .update_settings(super::super::mcp::McpSettings {
            enabled: true,
            ..settings
        })
        .await
        .unwrap();
    fixture
        .desktop
        .mcp_server
        .start(fixture.runtime.clone())
        .await
        .unwrap();
    assert!(fixture.desktop.mcp_server.status().await.running);
    let original = stored().unwrap();
    fail_delete(true);
    assert!(fixture
        .desktop
        .sign_out_account(&fixture.runtime)
        .await
        .is_err());
    assert!(fixture.runtime.is_running());
    assert!(Arc::ptr_eq(
        fixture.desktop.account.lock().await.as_ref().unwrap(),
        &fixture.runtime
    ));
    assert!(fixture.desktop.mcp_server.status().await.running);
    assert!(service.state().enabled);
    assert_eq!(stored().unwrap(), original);
    assert!(fixture.desktop.auth_service.get_auth().is_some());
    assert_eq!(
        fixture
            .database
            .auth_dao
            .find_all_auth()
            .await
            .unwrap()
            .len(),
        1
    );
    fail_delete(false);
    fixture
        .desktop
        .sign_out_account(&fixture.runtime)
        .await
        .unwrap();
    assert!(fixture.desktop.account.lock().await.is_none());
    assert!(!fixture.desktop.mcp_server.status().await.running);
    assert!(!fixture.runtime.is_running());
    assert!(fixture.desktop.auth_service.get_auth().is_none());
    assert!(fixture
        .database
        .auth_dao
        .find_all_auth()
        .await
        .unwrap()
        .is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn real_rest_unauthorized_retires_runtime_clears_saved_auth_and_does_not_restore_it() {
    let fixture = Fixture::new().await;
    let (endpoint, token) = fixture.start_mcp().await;
    let http = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    let before = http
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(before.status(), reqwest::StatusCode::OK);
    drop(before);
    let cancellation = CancellationToken::new();
    fixture
        .runtime
        .attachment_downloads
        .lock()
        .unwrap()
        .insert("accepted-transfer".into(), cancellation.clone());
    fixture.unauthorized().await;
    fixture.wait_retired().await;
    assert!(cancellation.is_cancelled());
    assert!(!fixture.desktop.mcp_server.status().await.running);
    tokio::time::timeout(
        Duration::from_secs(3),
        fixture.desktop.mcp_server.wait_for_account_stop(),
    )
    .await
    .unwrap();
    let port = url::Url::parse(&endpoint).unwrap().port().unwrap();
    let reclaimed = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .unwrap();
    drop(reclaimed);
    match http.get(&endpoint).bearer_auth(&token).send().await {
        Ok(response) => assert_eq!(response.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE),
        Err(error) => assert!(
            !error.is_timeout(),
            "retired MCP must reject instead of hanging"
        ),
    }
    assert!(!fixture.runtime.is_running());
    assert!(fixture.desktop.auth_service.get_auth().is_none());
    assert!(fixture
        .database
        .auth_dao
        .find_all_auth()
        .await
        .unwrap()
        .is_empty());
    assert!(fixture.desktop.restore_account().await.unwrap().is_none());
    fixture.close().await;
}

#[tokio::test]
async fn rejected_session_loses_cached_authority_when_persistent_auth_removal_fails() {
    let fixture = Fixture::new().await;
    fixture.reject_auth_writes(true);
    fixture.unauthorized().await;
    fixture.wait_retired().await;
    assert!(!fixture.runtime.is_running());
    assert!(fixture.desktop.auth_service.get_auth().is_none());
    assert_eq!(
        fixture
            .database
            .auth_dao
            .find_all_auth()
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(fixture.desktop.restore_account().await.unwrap().is_none());
    fixture.reject_auth_writes(false);
    fixture.close().await;
}

#[tokio::test]
async fn late_rejection_of_old_session_cannot_clear_replacement_session_or_rotated_keys() {
    for same_session_id in [false, true] {
        let fixture = Fixture::new().await;
        let replacement = auth(
            if same_session_id {
                SESSION
            } else {
                "10000000-0000-4000-8000-000000000003"
            },
            2,
        );
        fixture
            .desktop
            .auth_service
            .save_auth(&replacement)
            .await
            .unwrap();
        fixture.unauthorized().await;
        fixture.wait_retired().await;
        let saved = fixture.desktop.auth_service.get_auth().unwrap();
        assert_eq!(saved.account.session_id, replacement.account.session_id);
        assert_eq!(saved.private_key, replacement.private_key);
        let persisted = fixture.database.auth_dao.find_all_auth().await.unwrap();
        assert_eq!(persisted.len(), 1);
        assert_eq!(persisted[0].private_key, replacement.private_key);
        fixture.close().await;
    }
}

#[tokio::test]
async fn failed_auth_persistence_restores_screen_credentials_or_keeps_content_covered_until_recovery(
) {
    for reject_restore in [false, true] {
        let fixture = Fixture::new().await;
        let (service, _fail_delete, fail_save, stored) =
            crate::runtime::screen_lock::tests::deletion_failure_fixture().await;
        fixture
            .runtime
            .screen_lock
            .set(service.clone())
            .ok()
            .unwrap();
        let original = stored().unwrap();
        fail_save(reject_restore);
        fixture.reject_auth_writes(true);
        assert!(fixture
            .desktop
            .sign_out_account(&fixture.runtime)
            .await
            .is_err());
        assert!(fixture.runtime.is_running());
        assert!(fixture.desktop.account.lock().await.is_some());
        assert!(fixture.desktop.auth_service.get_auth().is_some());
        assert_eq!(
            fixture
                .database
                .auth_dao
                .find_all_auth()
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(service.state().enabled);
        assert_eq!(service.state().locked, reject_restore);
        if reject_restore {
            assert!(stored().is_none());
        } else {
            assert_eq!(stored().unwrap(), original);
        }
        fail_save(false);
        fixture.reject_auth_writes(false);
        if reject_restore {
            service.unlock("024680".into()).await.unwrap();
        }
        fixture
            .desktop
            .sign_out_account(&fixture.runtime)
            .await
            .unwrap();
        assert!(fixture.desktop.auth_service.get_auth().is_none());
        assert!(fixture.desktop.account.lock().await.is_none());
        assert!(!service.state().enabled);
        fixture.close().await;
    }
}

#[tokio::test]
async fn shutting_down_old_account_does_not_stop_mcp_bound_to_its_replacement() {
    let old = Fixture::new().await;
    let replacement = Fixture::new().await;
    let (_, token) = old.start_mcp().await;
    let status = old
        .desktop
        .mcp_server
        .start(replacement.runtime.clone())
        .await
        .unwrap();
    let endpoint = status.endpoint.unwrap().replace("/mcp", "/events");
    old.runtime.shutdown().await;
    assert!(replacement.runtime.is_running());
    assert!(old.desktop.mcp_server.status().await.running);
    let response = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap()
        .get(endpoint)
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    drop(response);
    old.close().await;
    replacement.close().await;
}

#[test]
fn linux_device_id_matches_java_name_uuid_from_bytes() {
    // UUID.nameUUIDFromBytes("hello".getBytes()) in Java and the Flutter app.
    assert_eq!(
        name_uuid_from_bytes(b"hello"),
        "5d41402a-bc4b-3a76-b971-9d911017c592"
    );
}

#[tokio::test]
async fn saved_flutter_downloads_without_a_message_are_dropped_on_resume() {
    let fixture = Fixture::new().await;
    let downloads = crate::db::hive::account_box(
        &fixture._directory.path().join("account"),
        crate::db::hive::DOWNLOAD_BOX,
    )
    .unwrap();
    for key in ["missing-message", "missing-transcript|missing-message"] {
        downloads.put(key, &key.into()).unwrap();
    }

    fixture
        .runtime
        .attachment_access()
        .resume_saved_downloads()
        .await
        .unwrap();

    assert!(downloads.keys().unwrap().is_empty());
    fixture.close().await;
}
