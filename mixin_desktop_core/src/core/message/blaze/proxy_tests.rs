use super::*;
use crate::{
    db::app::AppDatabase,
    network::{NetworkService, ProxyConfig, ProxySettings, ProxyType},
};
use std::collections::HashSet;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    task::JoinSet,
};

async fn header(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        assert!(bytes.len() < 8192);
        bytes.push(stream.read_u8().await.unwrap());
    }
    String::from_utf8(bytes).unwrap()
}

// Tungstenite's callback fixes its unboxed HTTP error response type.
#[allow(clippy::result_large_err)]
async fn session(stream: TcpStream, id: usize, events: mpsc::UnboundedSender<(usize, bool)>) {
    let mut socket = tokio_tungstenite::accept_hdr_async(
        stream,
        |_: &tokio_tungstenite::tungstenite::handshake::server::Request,
         mut response: tokio_tungstenite::tungstenite::handshake::server::Response| {
            response
                .headers_mut()
                .insert("Sec-WebSocket-Protocol", "Mixin-Blaze-1".parse().unwrap());
            Ok(response)
        },
    )
    .await
    .unwrap();
    let first = socket.next().await.unwrap().unwrap();
    assert!(first.is_binary());
    events.send((id, true)).unwrap();
    let _ = socket.next().await;
    events.send((id, false)).unwrap();
}

async fn proxy(
    listener: TcpListener,
    id: usize,
    events: mpsc::UnboundedSender<(usize, bool)>,
    mut stop: watch::Receiver<bool>,
) {
    let mut tasks = JoinSet::new();
    loop {
        tokio::select! {
            changed=stop.changed()=> {assert!(changed.is_ok());break;},
            accepted=listener.accept()=> {
                let(mut stream,_)=accepted.unwrap();let events=events.clone();
                tasks.spawn(async move {
                    let request=header(&mut stream).await;
                    if request.starts_with("CONNECT 127.0.0.1:") {
                        stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await.unwrap();
                        session(stream,id,events).await;
                    } else {
                        assert!(request.starts_with("CONNECT api.mixin.one:443"));
                        stream.write_all(b"HTTP/1.1 502 Fixture does not forward API traffic\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
                    }
                });
            },
            finished=tasks.join_next(),if !tasks.is_empty()=> {finished.unwrap().unwrap();},
        }
    }
    tasks.shutdown().await;
}

async fn event(
    receiver: &mut mpsc::UnboundedReceiver<(usize, bool)>,
    observed: &mut HashSet<(usize, bool)>,
    expected: (usize, bool),
) {
    while !observed.contains(&expected) {
        observed.insert(receiver.recv().await.unwrap());
    }
}

#[tokio::test]
async fn changing_and_disabling_proxy_reconnects_live_blaze_and_cancellation_releases_the_socket() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let directory = tempfile::tempdir().unwrap();
        let app = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        let database = Arc::new(
            MixinDatabase::connect_at(directory.path().join("mixin.db"))
                .await
                .unwrap(),
        );
        let first = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let second = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let direct = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let configurations = vec![
            ProxyConfig {
                id: "first".into(),
                proxy_type: ProxyType::Http,
                host: "127.0.0.1".into(),
                port: first.local_addr().unwrap().port(),
                username: None,
                password: None,
            },
            ProxyConfig {
                id: "second".into(),
                proxy_type: ProxyType::Http,
                host: "127.0.0.1".into(),
                port: second.local_addr().unwrap().port(),
                username: None,
                password: None,
            },
        ];
        app.setting_dao
            .set_proxy_settings(ProxySettings {
                enabled: true,
                selected_proxy_id: Some("first".into()),
                proxies: configurations.clone(),
            })
            .await
            .unwrap();
        let network = Arc::new(NetworkService::new(app.setting_dao.clone()).await.unwrap());
        let mut applied = network.proxy_changes();
        // Reject status-API signing locally; disabling the proxy must never
        // make this socket-only fixture contact a real Mixin endpoint.
        let invalid = Credential::KeyStore(sdk::KeyStore {
            app_id: "fixture".into(),
            session_id: "fixture".into(),
            server_public_key: String::new(),
            session_private_key: "invalid".into(),
            scp: String::new(),
        });
        let blaze = Arc::new(
            Blaze::new(
                database.clone(),
                Arc::new(network.sdk_client(invalid)),
                Credential::None,
                "fixture".into(),
                None,
            )
            .with_network(network),
        );
        let (send, mut events) = mpsc::unbounded_channel();
        let (stop, stop_rx) = watch::channel(false);
        let first_server = tokio::spawn(proxy(first, 1, send.clone(), stop_rx.clone()));
        let second_server = tokio::spawn(proxy(second, 2, send.clone(), stop_rx));
        let target = format!("ws://{}", direct.local_addr().unwrap());
        let direct_server = tokio::spawn(async move {
            let (stream, _) = direct.accept().await.unwrap();
            session(stream, 3, send).await;
        });
        let client = blaze.clone();
        let run = tokio::spawn(async move { client.connect_hosts(&[&target]).await });
        let mut observed = HashSet::new();
        event(&mut events, &mut observed, (1, true)).await;
        assert!(*blaze.connection_status.borrow());
        app.setting_dao
            .set_proxy_settings(ProxySettings {
                enabled: true,
                selected_proxy_id: Some("second".into()),
                proxies: configurations.clone(),
            })
            .await
            .unwrap();
        applied.changed().await.unwrap();
        event(&mut events, &mut observed, (1, false)).await;
        event(&mut events, &mut observed, (2, true)).await;
        app.setting_dao
            .set_proxy_settings(ProxySettings {
                enabled: false,
                selected_proxy_id: Some("second".into()),
                proxies: configurations,
            })
            .await
            .unwrap();
        applied.changed().await.unwrap();
        assert!(applied.borrow().is_none());
        event(&mut events, &mut observed, (2, false)).await;
        event(&mut events, &mut observed, (3, true)).await;
        run.abort();
        assert!(run.await.unwrap_err().is_cancelled());
        event(&mut events, &mut observed, (3, false)).await;
        assert!(!*blaze.connection_status.borrow());
        assert!(!blaze.connect_running.load(Ordering::Acquire));
        assert!(blaze.connection.lock().unwrap().sink.is_none());
        stop.send_replace(true);
        first_server.await.unwrap();
        second_server.await.unwrap();
        direct_server.await.unwrap();
        database.close().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn disabling_proxy_cancels_a_pending_connect_tunnel_before_its_timeout() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let directory = tempfile::tempdir().unwrap();
        let app = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        let database = Arc::new(
            MixinDatabase::connect_at(directory.path().join("mixin.db"))
                .await
                .unwrap(),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let direct = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config = ProxyConfig {
            id: "stalled".into(),
            proxy_type: ProxyType::Http,
            host: "127.0.0.1".into(),
            port: listener.local_addr().unwrap().port(),
            username: None,
            password: None,
        };
        app.setting_dao
            .set_proxy_settings(ProxySettings {
                enabled: true,
                selected_proxy_id: Some("stalled".into()),
                proxies: vec![config.clone()],
            })
            .await
            .unwrap();
        let network = Arc::new(NetworkService::new(app.setting_dao.clone()).await.unwrap());
        let mut applied = network.proxy_changes();
        let invalid = Credential::KeyStore(sdk::KeyStore {
            app_id: "fixture".into(),
            session_id: "fixture".into(),
            server_public_key: String::new(),
            session_private_key: "invalid".into(),
            scp: String::new(),
        });
        let blaze = Arc::new(
            Blaze::new(
                database.clone(),
                Arc::new(network.sdk_client(invalid)),
                Credential::None,
                "fixture".into(),
                None,
            )
            .with_network(network),
        );
        let (send, mut events) = mpsc::unbounded_channel();
        let pending = send.clone();
        let stalled_server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            assert!(header(&mut stream).await.starts_with("CONNECT 127.0.0.1:"));
            pending.send((1, true)).unwrap();
            let mut byte = [0];
            assert_eq!(stream.read(&mut byte).await.unwrap(), 0);
            pending.send((1, false)).unwrap();
        });
        let target = format!("ws://{}", direct.local_addr().unwrap());
        let direct_server = tokio::spawn(async move {
            let (stream, _) = direct.accept().await.unwrap();
            session(stream, 2, send).await;
        });
        let client = blaze.clone();
        let run = tokio::spawn(async move { client.connect_hosts(&[&target]).await });
        let mut observed = HashSet::new();
        event(&mut events, &mut observed, (1, true)).await;
        app.setting_dao
            .set_proxy_settings(ProxySettings {
                enabled: false,
                selected_proxy_id: Some("stalled".into()),
                proxies: vec![config],
            })
            .await
            .unwrap();
        applied.changed().await.unwrap();
        event(&mut events, &mut observed, (1, false)).await;
        event(&mut events, &mut observed, (2, true)).await;
        run.abort();
        assert!(run.await.unwrap_err().is_cancelled());
        event(&mut events, &mut observed, (2, false)).await;
        assert!(!*blaze.connection_status.borrow());
        assert!(!blaze.connect_running.load(Ordering::Acquire));
        stalled_server.await.unwrap();
        direct_server.await.unwrap();
        database.close().await;
    })
    .await
    .unwrap();
}
