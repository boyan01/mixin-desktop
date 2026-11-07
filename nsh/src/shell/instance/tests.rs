use super::*;

#[test]
fn a_second_process_forwards_without_acquiring_database_ownership() {
    // Exercise actual filesystem locks and the Unix socket, without AppKit.
    let directory = std::env::temp_dir().join(format!(
        "nsh-{:x}-{:x}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let (sender, mut events) = mpsc::channel(4);
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(directory.join("nsh-instance.lock"))
        .unwrap();
    lock.try_lock().unwrap();
    let listener = std::os::unix::net::UnixListener::bind(
        socket_path(&std::fs::canonicalize(&directory).unwrap()).unwrap(),
    )
    .unwrap();
    let worker = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        receive(&mut stream, &sender).unwrap();
        stream.write_all(&[0]).unwrap();
    });
    let (unused, _) = mpsc::channel(1);
    let launch = acquire(
        &directory,
        vec![Event::Open(
            "mixin://users/11111111-1111-4111-8111-111111111111".into(),
        )],
        unused,
    )
    .unwrap();
    assert!(matches!(launch, Launch::Forwarded));
    assert!(matches!(events.blocking_recv(), Some(Event::Open(url)) if url.contains("/users/")));
    worker.join().unwrap();
    drop(lock);
    std::fs::remove_file(socket_path(&std::fs::canonicalize(&directory).unwrap()).unwrap())
        .unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn an_invalid_or_oversized_forwarded_request_delivers_no_events() {
    let (sender, mut events) = mpsc::channel(4);
    for bytes in [
        serde_json::to_vec(&["file:///etc/passwd"]).unwrap(),
        vec![0; MAX_PAYLOAD + 1],
    ] {
        let mut wire = (bytes.len() as u32).to_be_bytes().to_vec();
        wire.extend(bytes);
        assert!(receive(&mut wire.as_slice(), &sender).is_err());
        assert!(events.try_recv().is_err());
    }
}
#[test]
fn a_full_pending_queue_rejects_the_whole_request_without_partial_delivery() {
    let (sender, mut events) = mpsc::channel(2);
    sender.try_send(Event::Activate).unwrap();
    let bytes = protocol::encode(&[
        Event::Open("mixin://users/one".into()),
        Event::Open("mixin://users/two".into()),
    ])
    .unwrap();
    let mut wire = (bytes.len() as u32).to_be_bytes().to_vec();
    wire.extend(bytes);
    assert!(receive(&mut wire.as_slice(), &sender).is_err());
    assert!(matches!(events.try_recv(), Ok(Event::Activate)));
    assert!(events.try_recv().is_err());
}

#[test]
fn forwarded_notification_preserves_account_target_and_intent_order() {
    let target = super::super::NotificationTarget {
        account_id: "11111111-1111-4111-8111-111111111111".into(),
        conversation_id: "22222222-2222-4222-8222-222222222222".into(),
        message_id: "33333333-3333-4333-8333-333333333333".into(),
    };
    let intents = vec![
        Event::Open("mixin://users/11111111-1111-4111-8111-111111111111".into()),
        Event::Notification(target),
    ];
    let bytes = protocol::encode(&intents).unwrap();
    let (sender, mut received) = mpsc::channel(2);
    let (mut client, mut server) = std::os::unix::net::UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    server
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let worker = std::thread::spawn(move || {
        receive(&mut server, &sender).unwrap();
        server.write_all(&[0]).unwrap();
    });
    client
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .unwrap();
    client.write_all(&bytes).unwrap();
    let mut acknowledgement = [1];
    client.read_exact(&mut acknowledgement).unwrap();
    assert_eq!(acknowledgement, [0]);
    for expected in intents {
        assert_eq!(received.blocking_recv(), Some(expected));
    }
    worker.join().unwrap();
    assert!(received.blocking_recv().is_none());
}

#[test]
fn invalid_version_or_notification_rejects_every_intent_in_the_request() {
    let (sender, mut events) = mpsc::channel(4);
    for value in [
        serde_json::json!({"version":2,"events":[{"kind":"activate"}]}),
        serde_json::json!({"version":1,"events":[{"kind":"activate"},{"kind":"notification","account_id":"invalid","conversation_id":"22222222-2222-4222-8222-222222222222","message_id":"33333333-3333-4333-8333-333333333333"}]}),
        serde_json::json!({"version":1,"events":vec![serde_json::json!({"kind":"activate"}); 17]}),
    ] {
        let bytes = serde_json::to_vec(&value).unwrap();
        let mut wire = (bytes.len() as u32).to_be_bytes().to_vec();
        wire.extend(bytes);
        assert!(receive(&mut wire.as_slice(), &sender).is_err());
        assert!(events.try_recv().is_err());
    }
}

#[test]
fn dropping_owner_interrupts_an_accepted_incomplete_request() {
    let directory = std::env::temp_dir().join(format!(
        "nsh-drop-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let socket = directory.join("instance.sock");
    let (sender, mut events) = mpsc::channel(2);
    let (accepted, observed) = std::sync::mpsc::sync_channel(1);
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let server_barrier = barrier.clone();
    let server = UnixServer::start_observed(socket.clone(), sender, move || {
        accepted.send(()).unwrap();
        server_barrier.wait();
    })
    .unwrap();
    let mut client = std::os::unix::net::UnixStream::connect(socket).unwrap();
    // Keep the peer alive with a declared but incomplete body. The owner
    // must cancel active I/O instead of waiting for the request to complete.
    client
        .write_all(&(MAX_PAYLOAD as u32).to_be_bytes())
        .unwrap();
    client.write_all(b"{").unwrap();
    observed.recv_timeout(Duration::from_secs(2)).unwrap();
    let (finished, done) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        drop(server);
        let _ = finished.send(());
    });
    barrier.wait();
    let stopped = done.recv_timeout(Duration::from_millis(750)).is_ok();
    drop(client);
    worker.join().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    assert!(
        stopped,
        "owner disposal waited for an unfinished client request"
    );
    assert!(events.try_recv().is_err());
}

#[test]
fn repeated_shutdown_preserves_the_successor_socket_and_handoff() {
    let directory = std::env::temp_dir().join(format!(
        "nsh-next-{:x}-{:x}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let socket = directory.join("instance.sock");
    let (old_sender, _) = mpsc::channel(1);
    let mut old = UnixServer::start(socket.clone(), old_sender).unwrap();
    old.shutdown();
    let (sender, mut received) = mpsc::channel(1);
    let successor = UnixServer::start(socket.clone(), sender).unwrap();
    // Instance explicitly shuts down before its lock is released. The
    // transport field's later Drop must not touch a newly started owner.
    drop(old);
    forward(&socket, &[Event::Activate]).unwrap();
    assert_eq!(received.blocking_recv(), Some(Event::Activate));
    drop(successor);
    std::fs::remove_dir_all(directory).unwrap();
}
