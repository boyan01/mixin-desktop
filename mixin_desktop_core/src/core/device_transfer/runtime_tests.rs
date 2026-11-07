use super::*;
use crate::core::{
    crypto::signal_protocol::SignalProtocol,
    message::blaze::Blaze,
    model::{signal::SignalService, ConversationService},
};
use crate::db::SignalDatabase;

const USER: &str = "10000000-0000-4000-8000-000000000001";
const CONVERSATION: &str = "10000000-0000-4000-8000-000000000002";
const TEXT: &str = "10000000-0000-4000-8000-000000000003";
const IMAGE: &str = "10000000-0000-4000-8000-000000000004";
const ROOT: &str = "10000000-0000-4000-8000-000000000005";
const CHILD: &str = "10000000-0000-4000-8000-000000000006";

struct Fixture {
    directory: tempfile::TempDir,
    service: Arc<DeviceTransferService>,
    signal: Arc<SignalDatabase>,
}

impl Fixture {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let database = Arc::new(
            MixinDatabase::connect_at(directory.path().join("mixin.db"))
                .await
                .unwrap(),
        );
        let signal = Arc::new(
            SignalDatabase::connect_at(directory.path().join("signal.db"))
                .await
                .unwrap(),
        );
        // The control-plane services are real, but are never connected or run.
        // This fixture exercises only the production TCP backup/restore methods.
        let client = Arc::new(sdk::Client::new(sdk::Credential::None));
        let protocol = Arc::new(SignalProtocol::new(signal.clone(), USER.into()));
        let blaze = Arc::new(Blaze::new(
            database.clone(),
            client.clone(),
            sdk::Credential::None,
            USER.into(),
            None,
        ));
        let sender = Arc::new(MessageSender::new(
            blaze,
            ConversationService::new(database.clone(), client.clone(), USER.into()),
            database.clone(),
            USER.into(),
            Uuid::new_v4().to_string(),
            protocol.clone(),
            SignalService::new(protocol, signal.clone(), client),
        ));
        let service = DeviceTransferService::new(
            database,
            sender,
            USER.into(),
            Uuid::new_v4().to_string(),
            None,
            directory.path().into(),
            ConversationChangeNotifier::new(),
        );
        Self {
            directory,
            service,
            signal,
        }
    }

    async fn close(self) {
        self.service.shutdown().await;
        self.service.database.close().await;
        self.signal.close().await;
    }
}

async fn transfer_result(sender: &Fixture, receiver: &Fixture) -> (Result<()>, Result<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let key = generate_transfer_key().unwrap();
    let backup = Arc::new(CancellationToken::new());
    let restore = Arc::new(CancellationToken::new());
    sender
        .service
        .events
        .begin(Flow::Backup, Some(backup.clone()));
    receiver
        .service
        .events
        .begin(Flow::Restore, Some(restore.clone()));
    let remote = RemotePushData {
        ip: "127.0.0.1".into(),
        port,
        code: 1234,
        key,
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            sender.service.run_sender(listener, 1234, key, backup),
            receiver.service.run_receiver(remote, restore),
        )
    })
    .await
    .expect("local transfer must complete and release both sockets")
}

async fn transfer(sender: &Fixture, receiver: &Fixture) {
    let (sent, received) = transfer_result(sender, receiver).await;
    sent.unwrap();
    received.unwrap();
}

#[tokio::test]
async fn shared_attachment_uuid_across_messages_and_transcript_roots_requires_identical_bytes() {
    for conflict in [false, true] {
        let sender = Fixture::new().await;
        let receiver = Fixture::new().await;
        let now = chrono::Utc::now();
        crate::db::query("INSERT INTO conversations (conversation_id, category, created_at, status) VALUES (?, 'CONTACT', ?, 2)")
            .bind(CONVERSATION).bind(now).execute(sender.service.pool()).await.unwrap();
        let second_root = "10000000-0000-4000-8000-000000000007";
        let image = Message {
            message_id: IMAGE.into(),
            conversation_id: CONVERSATION.into(),
            user_id: USER.into(),
            category: "SIGNAL_IMAGE".into(),
            media_status: MediaStatus::Done,
            media_mime_type: Some("image/png".into()),
            created_at: now.naive_utc(),
            ..Message::default()
        };
        sender
            .service
            .database
            .message_dao
            .insert_message(&image)
            .await
            .unwrap();
        let ordinary = attachment_path(sender.directory.path(), &image).unwrap();
        std::fs::create_dir_all(ordinary.parent().unwrap()).unwrap();
        let bytes = vec![b'a'; 64 * 1024 + 7];
        std::fs::write(ordinary, &bytes).unwrap();
        for root in [ROOT, second_root] {
            sender
                .service
                .database
                .message_dao
                .insert_message(&Message {
                    message_id: root.into(),
                    conversation_id: CONVERSATION.into(),
                    user_id: USER.into(),
                    category: "SIGNAL_TRANSCRIPT".into(),
                    created_at: now.naive_utc(),
                    ..Message::default()
                })
                .await
                .unwrap();
            let path = sender
                .directory
                .path()
                .join("Media/Transcripts")
                .join(root)
                .join(format!("{IMAGE}.png"));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut copied = bytes.clone();
            if conflict && root == second_root {
                *copied.last_mut().unwrap() = b'b';
            }
            std::fs::write(&path, &copied).unwrap();
            let child = serde_json::from_value(serde_json::json!({
                "transcript_id":root, "message_id":IMAGE, "user_id":USER, "category":"SIGNAL_IMAGE",
                "created_at":now, "media_status":"DONE", "media_mime_type":"image/png", "media_url":path
            })).unwrap();
            sender
                .service
                .database
                .transcript_message_dao
                .insert_all(&[child])
                .await
                .unwrap();
        }
        let (sent, received) = transfer_result(&sender, &receiver).await;
        if conflict {
            assert!(sent
                .unwrap_err()
                .to_string()
                .contains("conflicting attachment bytes"));
            assert!(
                received.is_err(),
                "a conflicting file identity must not finish successfully"
            );
            let state = receiver.service.states().borrow().clone();
            assert_ne!(
                state.restore.phase,
                Some(DeviceTransferEvent::RestoreSucceed)
            );
            assert!(
                state.restore.progress < 100.0,
                "a received finish command sets progress to 100"
            );
        } else {
            sent.unwrap();
            received.unwrap();
            for root in [ROOT, second_root] {
                let row = receiver
                    .service
                    .database
                    .transcript_message_dao
                    .find(root, IMAGE)
                    .await
                    .unwrap()
                    .unwrap();
                let local = receiver
                    .directory
                    .path()
                    .join("Media/Transcripts")
                    .join(row.media_url.unwrap());
                assert_eq!(std::fs::read(local).unwrap(), bytes);
            }
            assert_eq!(
                std::fs::read(attachment_path(receiver.directory.path(), &image).unwrap()).unwrap(),
                bytes
            );
        }
        sender.close().await;
        receiver.close().await;
    }
}

#[tokio::test]
async fn local_tcp_backup_restores_messages_search_and_attachment_bytes() {
    let sender = Fixture::new().await;
    let receiver = Fixture::new().await;
    let now = chrono::DateTime::from_timestamp_millis(1_700_000_000_123)
        .unwrap()
        .naive_utc();
    crate::db::query("INSERT INTO conversations (conversation_id, category, created_at, status) VALUES (?, 'CONTACT', ?, 2)")
        .bind(CONVERSATION).bind(now).execute(sender.service.pool()).await.unwrap();
    let mut image = Message {
        message_id: IMAGE.into(),
        conversation_id: CONVERSATION.into(),
        user_id: USER.into(),
        category: "SIGNAL_IMAGE".into(),
        media_status: MediaStatus::Done,
        media_mime_type: Some("image/png".into()),
        media_key: Some(vec![1, 2, 3, 4]),
        media_digest: Some(vec![5, 6, 7, 8]),
        caption: Some("Fixture caption".into()),
        created_at: now,
        ..Message::default()
    };
    let sender_path = attachment_path(sender.directory.path(), &image).unwrap();
    image.media_url = Some(sender_path.to_string_lossy().into_owned());
    for message in [
        Message {
            message_id: TEXT.into(),
            conversation_id: CONVERSATION.into(),
            user_id: USER.into(),
            category: "SIGNAL_TEXT".into(),
            content: Some("transferneedle 漢字 🙂".into()),
            created_at: now,
            ..Message::default()
        },
        image.clone(),
    ] {
        sender
            .service
            .database
            .message_dao
            .insert_message(&message)
            .await
            .unwrap();
    }
    let path = attachment_path(sender.directory.path(), &image).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let bytes: Vec<u8> = (0..protocol::CHUNK_SIZE * 3 + 7)
        .map(|i| (i % 251) as u8)
        .collect();
    std::fs::write(path, &bytes).unwrap();

    transfer(&sender, &receiver).await;

    let text = receiver
        .service
        .database
        .message_dao
        .find_message_by_id(&TEXT.to_owned())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(text.content.as_deref(), Some("transferneedle 漢字 🙂"));
    assert_eq!(text.created_at, now);
    let received = receiver
        .service
        .database
        .message_dao
        .find_message_by_id(&IMAGE.to_owned())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(received.media_key, image.media_key);
    assert_eq!(received.media_digest, image.media_digest);
    assert_eq!(received.caption, image.caption);
    assert_eq!(received.media_url, Some(format!("{IMAGE}.png")));
    assert_eq!(
        std::fs::read(attachment_path(receiver.directory.path(), &received).unwrap()).unwrap(),
        bytes
    );
    let matches = crate::db::query_scalar::<String>("SELECT message_id FROM fts.messages_metas WHERE doc_id IN (SELECT rowid FROM fts.messages_fts WHERE messages_fts MATCH 'transferneedle')")
        .fetch_all(receiver.service.pool()).await.unwrap();
    assert_eq!(matches, vec![TEXT]);
    assert_eq!(receiver.service.states().borrow().restore.progress, 100.0);
    sender.close().await;
    receiver.close().await;
}

#[tokio::test]
async fn local_tcp_backup_restores_scoped_transcript_bytes_with_receiver_local_paths() {
    let sender = Fixture::new().await;
    let receiver = Fixture::new().await;
    let now = chrono::Utc::now();
    crate::db::query("INSERT INTO conversations (conversation_id, category, created_at, status) VALUES (?, 'CONTACT', ?, 2)")
        .bind(CONVERSATION).bind(now).execute(sender.service.pool()).await.unwrap();
    sender
        .service
        .database
        .message_dao
        .insert_message(&Message {
            message_id: ROOT.into(),
            conversation_id: CONVERSATION.into(),
            user_id: USER.into(),
            category: "SIGNAL_TRANSCRIPT".into(),
            created_at: now.naive_utc(),
            ..Message::default()
        })
        .await
        .unwrap();
    let path = sender
        .directory
        .path()
        .join("Media/Transcripts")
        .join(ROOT)
        .join(format!("{CHILD}.png"));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let bytes = b"root-scoped transcript fixture\0\xff\x01";
    std::fs::write(&path, bytes).unwrap();
    let child: crate::db::mixin::transcript_message::TranscriptMessage =
        serde_json::from_value(serde_json::json!({
            "transcript_id": ROOT, "message_id": CHILD, "user_id": USER,
            "category": "SIGNAL_IMAGE", "created_at": now, "media_status": "DONE",
            "media_mime_type": "image/png", "media_url": path,
        }))
        .unwrap();
    sender
        .service
        .database
        .transcript_message_dao
        .insert_all(&[child])
        .await
        .unwrap();

    transfer(&sender, &receiver).await;

    let received = receiver
        .service
        .database
        .transcript_message_dao
        .find(ROOT, CHILD)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        received.media_url,
        Some(format!("{CHILD}.png")),
        "a transferred row must not retain the sender's absolute path"
    );
    let local = receiver
        .directory
        .path()
        .join("Media/Transcripts")
        .join(format!("{CHILD}.png"));
    assert_eq!(std::fs::read(local).unwrap(), bytes);
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    sender.close().await;
    receiver.close().await;
}

#[tokio::test]
async fn incoming_transcript_metadata_replaces_sender_absolute_path_with_local_file_name() {
    let receiver = Fixture::new().await;
    let record = serde_json::json!({
        "transcript_id": ROOT, "message_id": CHILD, "user_id": USER,
        "category": "SIGNAL_IMAGE", "created_at": "2024-01-02T03:04:05.000Z",
        "media_status": "DONE", "media_mime_type": "image/png",
        "media_url": "/a-different-device/Media/Transcripts/private-copy.png",
    });
    receiver
        .service
        .insert_record("transcript_message", record.as_object().unwrap())
        .await
        .unwrap();
    let incoming = receiver.directory.path().join("incoming-fixture");
    std::fs::write(&incoming, b"incoming fixture").unwrap();
    receiver
        .service
        .insert_attachment(CHILD, &incoming)
        .await
        .unwrap();
    let row = receiver
        .service
        .database
        .transcript_message_dao
        .find(ROOT, CHILD)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.media_url, Some(format!("{CHILD}.png")));
    let file = receiver
        .directory
        .path()
        .join("Media/Transcripts")
        .join(row.media_url.unwrap());
    assert_eq!(std::fs::read(file).unwrap(), b"incoming fixture");
    receiver.close().await;
}

#[tokio::test]
async fn incoming_attachment_with_parent_traversal_conversation_creates_no_external_directory() {
    let mut receiver = Fixture::new().await;
    let account = receiver.directory.path().join("account");
    std::fs::create_dir(&account).unwrap();
    Arc::get_mut(&mut receiver.service)
        .unwrap()
        .account_data_dir = account;
    crate::db::query("INSERT INTO conversations (conversation_id, category, created_at, status) VALUES (?, 'CONTACT', ?, 2)")
        .bind("../../../escape").bind(chrono::Utc::now().naive_utc())
        .execute(receiver.service.pool()).await.unwrap();
    receiver
        .service
        .database
        .message_dao
        .insert_message(&Message {
            message_id: IMAGE.into(),
            conversation_id: "../../../escape".into(),
            category: "SIGNAL_IMAGE".into(),
            media_mime_type: Some("image/png".into()),
            ..Message::default()
        })
        .await
        .unwrap();
    let incoming = receiver.directory.path().join("authenticated-file");
    std::fs::write(&incoming, b"authenticated fixture").unwrap();
    assert!(receiver
        .service
        .insert_attachment(IMAGE, &incoming)
        .await
        .is_err());
    assert!(!receiver.directory.path().join("escape").exists());
    assert!(!receiver.directory.path().join("account/Media").exists());
    receiver.close().await;
}

#[tokio::test]
async fn local_tcp_restore_preserves_existing_attachment_and_fails_on_conflicting_bytes() {
    for conflict in [false, true] {
        let sender = Fixture::new().await;
        let receiver = Fixture::new().await;
        let image = Message {
            message_id: IMAGE.into(),
            conversation_id: CONVERSATION.into(),
            user_id: USER.into(),
            category: "SIGNAL_IMAGE".into(),
            media_status: MediaStatus::Done,
            media_mime_type: Some("image/png".into()),
            media_url: Some(format!("{IMAGE}.png")),
            ..Message::default()
        };
        for fixture in [&sender, &receiver] {
            crate::db::query("INSERT INTO conversations (conversation_id, category, created_at, status) VALUES (?, 'CONTACT', ?, 2)")
                .bind(CONVERSATION).bind(chrono::Utc::now().naive_utc())
                .execute(fixture.service.pool()).await.unwrap();
            fixture
                .service
                .database
                .message_dao
                .insert_message(&image)
                .await
                .unwrap();
        }
        let source = attachment_path(sender.directory.path(), &image).unwrap();
        let destination = attachment_path(receiver.directory.path(), &image).unwrap();
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        let incoming = b"new authenticated attachment";
        let original: &[u8] = if conflict {
            b"existing receiver attachment"
        } else {
            incoming
        };
        std::fs::write(source, incoming).unwrap();
        std::fs::write(&destination, original).unwrap();
        let (sent, restored) = transfer_result(&sender, &receiver).await;
        if conflict {
            assert!(sent.is_err());
            assert!(restored
                .unwrap_err()
                .to_string()
                .contains("existing attachment conflicts"));
            assert_ne!(
                receiver.service.states().borrow().restore.phase,
                Some(DeviceTransferEvent::RestoreSucceed)
            );
        } else {
            sent.unwrap();
            restored.unwrap();
            assert_eq!(receiver.service.states().borrow().restore.progress, 100.0);
        }
        assert_eq!(std::fs::read(&destination).unwrap(), original);
        assert_eq!(
            std::fs::read_dir(destination.parent().unwrap())
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            std::fs::read_dir(receiver.directory.path().join(".device-transfer"))
                .unwrap()
                .count(),
            0
        );
        sender.close().await;
        receiver.close().await;
    }
}

#[tokio::test]
async fn local_tcp_restore_rejects_known_record_database_failure_without_reporting_progress() {
    let receiver = Fixture::new().await;
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let key = generate_transfer_key().unwrap();
    let remote = RemotePushData {
        ip: "127.0.0.1".into(),
        port: listener.local_addr().unwrap().port(),
        code: 1234,
        key,
    };
    let cancel = Arc::new(CancellationToken::new());
    receiver
        .service
        .events
        .begin(Flow::Restore, Some(cancel.clone()));
    let peer = async {
        let (stream, _) = listener.accept().await.unwrap();
        let (mut read, mut write) = stream.into_split();
        assert!(
            matches!(read_packet(&mut read, &key, receiver.directory.path()).await.unwrap(), Packet::Command(command) if command.action == "connect")
        );
        write_command(&mut write, &key, &TransferCommand::start("peer", 1))
            .await
            .unwrap();
        // This message references a conversation that the backup never supplied.
        let record = serde_json::json!({
            "message_id": TEXT, "conversation_id": CONVERSATION, "user_id": USER,
            "category": "SIGNAL_TEXT", "content": "must not be reported as restored",
            "created_at": "2024-01-02T03:04:05.000Z", "status": "DELIVERED",
        });
        write_json_record(&mut write, &key, "message", record.as_object().unwrap())
            .await
            .unwrap();
        // An erroneous receiver used to swallow the database error and send 100%.
        assert!(read_packet(&mut read, &key, receiver.directory.path())
            .await
            .is_err());
    };
    let ((), restored) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(peer, receiver.service.run_receiver(remote, cancel))
    })
    .await
    .unwrap();
    assert!(restored
        .unwrap_err()
        .to_string()
        .contains("insert transferred message record"));
    assert_eq!(receiver.service.states().borrow().restore.progress, 0.0);
    assert!(receiver
        .service
        .database
        .message_dao
        .find_message_by_id(&TEXT.into())
        .await
        .unwrap()
        .is_none());
    receiver.close().await;
}

#[tokio::test]
async fn incoming_ordinary_media_keeps_unfinished_sources_and_normalizes_completed_local_paths() {
    for status in ["CANCELED", "DONE", "READ"] {
        let receiver = Fixture::new().await;
        crate::db::query("INSERT INTO conversations (conversation_id, category, created_at, status) VALUES (?, 'CONTACT', ?, 2)")
            .bind(CONVERSATION).bind(chrono::Utc::now().naive_utc())
            .execute(receiver.service.pool()).await.unwrap();
        let source = "https://example.invalid/unprepared-image.png";
        let record = serde_json::json!({
            "message_id": IMAGE, "conversation_id": CONVERSATION, "user_id": USER,
            "category": "SIGNAL_IMAGE", "created_at": "2024-01-02T03:04:05.000Z",
            "status": "SENDING", "media_status": status, "media_mime_type": "image/png", "media_url": source,
        });
        receiver
            .service
            .insert_record("message", record.as_object().unwrap())
            .await
            .unwrap();
        let message = receiver
            .service
            .database
            .message_dao
            .find_message_by_id(&IMAGE.into())
            .await
            .unwrap()
            .unwrap();
        let expected = if status == "CANCELED" {
            source.to_owned()
        } else {
            format!("{IMAGE}.png")
        };
        assert_eq!(message.media_url, Some(expected));
        receiver.close().await;
    }
}

#[tokio::test]
async fn cancelling_a_backpressured_control_write_releases_the_stream_and_staging_file() {
    use tokio::io::AsyncReadExt as _;
    let account = tempfile::tempdir().unwrap();
    let (staged, file) = protocol::StagedFile::create(account.path()).unwrap();
    drop(file);
    let path = staged.path().to_owned();
    std::fs::write(&path, b"unpublished partial plaintext").unwrap();
    let cancel = Arc::new(CancellationToken::new());
    let worker_cancel = cancel.clone();
    let (mut writer, mut reader) = tokio::io::duplex(1);
    let mut worker = tokio::spawn(async move {
        run_cancellable(&worker_cancel, async move {
            let _staged = staged;
            protocol::write_command(
                &mut writer,
                &[0x42_u8; KEY_SIZE],
                &TransferCommand::start("peer", 1),
            )
            .await?;
            Ok(())
        })
        .await
    });
    // The real codec has written the type byte and is now blocked on the
    // one-byte transport. Nothing reads the remainder before cancellation.
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), reader.read_u8())
            .await
            .unwrap()
            .unwrap(),
        PACKET_COMMAND
    );
    assert!(path.is_file());
    cancel.cancel();
    let completed = tokio::time::timeout(Duration::from_secs(3), &mut worker).await;
    if completed.is_err() {
        worker.abort();
        let _ = worker.await;
        panic!("cancelled control writer did not release its operation");
    }
    completed.unwrap().unwrap().unwrap();
    let mut tail = Vec::new();
    tokio::time::timeout(Duration::from_secs(3), reader.read_to_end(&mut tail))
        .await
        .unwrap()
        .unwrap();
    assert!(!path.exists());
    assert_eq!(std::fs::read_dir(account.path()).unwrap().count(), 0);
}
