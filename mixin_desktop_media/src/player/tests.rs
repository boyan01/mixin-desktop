use super::*;

fn item(id: &str, path: &std::path::Path) -> AudioPlaybackItem {
    AudioPlaybackItem {
        id: id.into(),
        path: path.to_string_lossy().into_owned(),
        duration_millis: 100,
    }
}

fn cursor(ended: bool) -> Arc<PlaybackCursor> {
    Arc::new(PlaybackCursor {
        audio: DecodedAudio {
            samples: vec![0.0; 1600],
            sample_rate: 16_000,
            channels: 1,
        },
        buffer: Mutex::new(PlaybackBuffer {
            samples: vec![0.0; 1600],
            frame: 0.0,
        }),
        ended: AtomicBool::new(ended),
        advance_scheduled: AtomicBool::new(ended),
    })
}

fn playing_without_device(
    player: &AudioPlayer,
    playlist: Vec<AudioPlaybackItem>,
    cursor: Arc<PlaybackCursor>,
) {
    let mut state = player.inner.state.lock().unwrap();
    state.playlist = playlist;
    state.status = AudioPlaybackStatus::Playing;
    state.cursor = Some(cursor);
}

#[test]
fn natural_queue_end_emits_finished_once_while_stop_only_clears_playback() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let player = AudioPlayer::new();
    let current = cursor(true);
    playing_without_device(
        &player,
        vec![
            item("first", std::path::Path::new("first.ogg")),
            item("last", std::path::Path::new("last.ogg")),
        ],
        current.clone(),
    );
    player.inner.state.lock().unwrap().index = 1;
    let mut events = player.subscribe();
    advance(Arc::downgrade(&player.inner), Arc::downgrade(&current));
    assert_eq!(
        events.try_recv().unwrap(),
        AudioPlaybackEvent::Finished("last".into())
    );
    let AudioPlaybackEvent::Changed(snapshot) = events.try_recv().unwrap() else {
        panic!("expected idle snapshot");
    };
    assert_eq!(snapshot.status, AudioPlaybackStatus::Idle);
    assert_eq!(
        snapshot.item.as_ref().map(|item| item.id.as_str()),
        Some("last")
    );
    advance(Arc::downgrade(&player.inner), Arc::downgrade(&current));
    assert!(matches!(
        events.try_recv(),
        Err(broadcast::error::TryRecvError::Empty)
    ));
    player.stop();
    let AudioPlaybackEvent::Changed(snapshot) = events.try_recv().unwrap() else {
        panic!("stop must not emit Finished");
    };
    assert_eq!(snapshot.status, AudioPlaybackStatus::Idle);
    assert!(snapshot.item.is_none());
    assert!(matches!(
        events.try_recv(),
        Err(broadcast::error::TryRecvError::Empty)
    ));
    runtime.block_on(player.close()).unwrap();
}

#[test]
fn missing_or_corrupt_replacement_preserves_the_current_item_and_resources() {
    let directory = tempfile::tempdir().unwrap();
    let corrupt = directory.path().join("corrupt.ogg");
    fs::write(&corrupt, b"not an Ogg Opus file").unwrap();
    for path in [directory.path().join("missing.ogg"), corrupt] {
        let player = AudioPlayer::new();
        let current = cursor(false);
        playing_without_device(
            &player,
            vec![item("current", directory.path())],
            current.clone(),
        );
        let before = player.snapshot();
        let mut events = player.subscribe();
        assert!(player.play(vec![item("replacement", &path)], 0).is_err());
        assert_eq!(player.snapshot(), before);
        assert!(Arc::ptr_eq(
            player.inner.state.lock().unwrap().cursor.as_ref().unwrap(),
            &current
        ));
        assert!(matches!(
            events.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
    }
}

#[test]
fn stop_during_file_read_prevents_a_pending_play_from_restarting_audio() {
    let player = AudioPlayer::new();
    let worker = player.clone();
    let bytes = crate::codec::encode_voice(&vec![0; 1600]).unwrap();
    let (entered, waiting) = std::sync::mpsc::sync_channel(0);
    let (release, proceed) = std::sync::mpsc::sync_channel(0);
    let task = std::thread::spawn(move || {
        worker.play_with_read(
            vec![item("pending", std::path::Path::new("/pending.ogg"))],
            0,
            |path| {
                assert_eq!(path, "/pending.ogg");
                entered.send(()).unwrap();
                proceed
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
                Ok(bytes)
            },
        )
    });
    waiting
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    player.stop();
    let stopped = player.snapshot();
    release.send(()).unwrap();
    assert!(task.join().unwrap().is_err());
    assert_eq!(player.snapshot(), stopped);
    assert_eq!(stopped.status, AudioPlaybackStatus::Idle);
    assert!(stopped.item.is_none());
}

#[test]
fn a_speed_change_during_file_read_invalidates_the_old_speed_request() {
    let player = AudioPlayer::new();
    let worker = player.clone();
    let bytes = crate::codec::encode_voice(&vec![0; 1600]).unwrap();
    let (entered, waiting) = std::sync::mpsc::sync_channel(0);
    let (release, proceed) = std::sync::mpsc::sync_channel(0);
    let task = std::thread::spawn(move || {
        worker.play_with_read(
            vec![item("pending", std::path::Path::new("/pending.ogg"))],
            0,
            |_| {
                entered.send(()).unwrap();
                proceed
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
                Ok(bytes)
            },
        )
    });
    waiting
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    player.set_speed(1.5).unwrap();
    release.send(()).unwrap();
    assert!(task.join().unwrap().is_err());
    let snapshot = player.snapshot();
    assert_eq!(snapshot.speed, 1.5);
    assert_eq!(snapshot.status, AudioPlaybackStatus::Idle);
    assert!(snapshot.item.is_none());
}

#[test]
fn a_newer_play_request_invalidates_an_older_file_read() {
    let directory = tempfile::tempdir().unwrap();
    let player = AudioPlayer::new();
    let worker = player.clone();
    let bytes = crate::codec::encode_voice(&vec![0; 1600]).unwrap();
    let (entered, waiting) = std::sync::mpsc::sync_channel(0);
    let (release, proceed) = std::sync::mpsc::sync_channel(0);
    let task = std::thread::spawn(move || {
        worker.play_with_read(
            vec![item("older", std::path::Path::new("/pending.ogg"))],
            0,
            |_| {
                entered.send(()).unwrap();
                proceed
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
                Ok(bytes)
            },
        )
    });
    waiting
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    assert!(player
        .play(
            vec![item("newer", &directory.path().join("missing.ogg"))],
            0
        )
        .is_err());
    let latest = player.snapshot();
    release.send(()).unwrap();
    assert!(task.join().unwrap().is_err());
    assert_eq!(player.snapshot(), latest);
    assert!(latest.item.is_none());
}

#[test]
fn an_end_callback_during_a_user_play_does_not_replace_the_pending_request() {
    let player = AudioPlayer::new();
    let current = cursor(true);
    playing_without_device(
        &player,
        vec![item("current", std::path::Path::new("/unused.ogg"))],
        current.clone(),
    );
    let worker = player.clone();
    let (entered, waiting) = std::sync::mpsc::sync_channel(0);
    let (release, proceed) = std::sync::mpsc::sync_channel(0);
    let task = std::thread::spawn(move || {
        worker.play_with_read(
            vec![item("pending", std::path::Path::new("/pending.ogg"))],
            0,
            |_| {
                entered.send(()).unwrap();
                proceed
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "controlled read failure",
                ))
            },
        )
    });
    waiting
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    let before = player.snapshot();
    let mut events = player.subscribe();
    advance(Arc::downgrade(&player.inner), Arc::downgrade(&current));
    assert_eq!(player.snapshot(), before);
    assert!(matches!(
        events.try_recv(),
        Err(broadcast::error::TryRecvError::Empty)
    ));
    release.send(()).unwrap();
    assert!(task.join().unwrap().is_err());
    assert!(!current.advance_scheduled.load(Ordering::Acquire));
}

#[test]
fn an_old_end_callback_cannot_advance_a_replay_of_the_same_message() {
    let player = AudioPlayer::new();
    let old = cursor(true);
    let current = cursor(true);
    playing_without_device(
        &player,
        vec![item("same-message", std::path::Path::new("/unused.ogg"))],
        current.clone(),
    );
    let before = player.snapshot();
    let mut events = player.subscribe();
    advance(Arc::downgrade(&player.inner), Arc::downgrade(&old));
    assert_eq!(player.snapshot(), before);
    assert!(Arc::ptr_eq(
        player.inner.state.lock().unwrap().cursor.as_ref().unwrap(),
        &current
    ));
    assert!(matches!(
        events.try_recv(),
        Err(broadcast::error::TryRecvError::Empty)
    ));
}

#[test]
fn a_missing_next_item_stops_the_finished_queue_and_reports_that_item() {
    let directory = tempfile::tempdir().unwrap();
    let player = AudioPlayer::new();
    let current = cursor(true);
    playing_without_device(
        &player,
        vec![
            item("current", directory.path()),
            item("missing-next", &directory.path().join("missing.ogg")),
        ],
        current.clone(),
    );
    let mut events = player.subscribe();
    advance(Arc::downgrade(&player.inner), Arc::downgrade(&current));
    assert_eq!(player.snapshot().status, AudioPlaybackStatus::Idle);
    assert!(player.snapshot().item.is_none());
    assert!(
        matches!(events.try_recv().unwrap(), AudioPlaybackEvent::Failed { id: Some(id), .. } if id == "missing-next")
    );
    assert!(
        matches!(events.try_recv().unwrap(), AudioPlaybackEvent::Changed(snapshot) if snapshot.status == AudioPlaybackStatus::Idle && snapshot.item.is_none())
    );
}

#[test]
fn time_stretch_changes_duration_without_shifting_pitch() {
    let sample_rate = 48_000;
    let frequency = 440.0;
    let samples = (0..sample_rate * 2)
        .map(|index| (index as f32 * frequency * std::f32::consts::TAU / sample_rate as f32).sin())
        .collect::<Vec<_>>();
    let audio = DecodedAudio {
        samples,
        sample_rate,
        channels: 1,
    };

    let stretched = stretch_audio(&audio, 2.0).expect("time stretch should succeed");

    let expected_length = audio.samples.len() / 2;
    assert!(
        stretched.len().abs_diff(expected_length) < sample_rate as usize / 10,
        "2x output length {} should be close to {expected_length}",
        stretched.len()
    );
    let frequency = dominant_frequency(&stretched, sample_rate);
    assert!(
        (frequency - 440.0).abs() < 22.0,
        "2x playback shifted pitch to {frequency} Hz"
    );
}

fn dominant_frequency(samples: &[f32], sample_rate: u32) -> f32 {
    let start = samples.len() / 8;
    let end = samples.len() - start;
    let samples = &samples[start..end];
    let crossings = samples
        .windows(2)
        .filter(|pair| (pair[0] <= 0.0 && pair[1] > 0.0) || (pair[0] >= 0.0 && pair[1] < 0.0))
        .count();
    crossings as f32 * sample_rate as f32 / (2.0 * samples.len() as f32)
}

#[test]
fn stopping_a_play_queued_in_the_blocking_pool_rejects_it_before_reading_the_file() {
    use std::task::{Context, Poll, Waker};
    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let (entered, ready) = std::sync::mpsc::sync_channel(0);
    let (release, proceed) = std::sync::mpsc::sync_channel(0);
    let blocker = runtime.spawn_blocking(move || {
        entered.send(()).unwrap();
        proceed
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
    });
    ready
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    let player = AudioPlayer::new();
    let mut playback = Box::pin(player.play_async(
        vec![item(
            "queued",
            std::path::Path::new("/must-not-be-read.ogg"),
        )],
        0,
    ));
    {
        let _entered = runtime.enter();
        assert!(matches!(
            playback
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
    }
    let stopping = player.stop_async();
    release.send(()).unwrap();
    runtime.block_on(stopping).unwrap();
    let error = runtime.block_on(playback).unwrap_err();
    runtime.block_on(blocker).unwrap();
    assert!(matches!(error, MediaError::Stream(message) if message.contains("cancelled")));
    assert_eq!(player.snapshot().status, AudioPlaybackStatus::Idle);
    assert!(player.snapshot().item.is_none());
}

#[test]
fn dropping_a_pending_request_cancels_its_running_worker_and_preserves_previous_playback() {
    let player = AudioPlayer::new();
    playing_without_device(
        &player,
        vec![item("current", std::path::Path::new("/current.ogg"))],
        cursor(false),
    );
    let before = player.snapshot();
    let request = player.reserve().unwrap();
    let inner = request.inner.clone();
    let playlist = vec![item("pending", std::path::Path::new("/pending.ogg"))];
    let index = 0;
    let revision = request.revision;
    let bytes = crate::codec::encode_voice(&vec![0; 1600]).unwrap();
    let (entered, ready) = std::sync::mpsc::sync_channel(0);
    let (release, proceed) = std::sync::mpsc::sync_channel(0);
    let worker = std::thread::spawn(move || {
        play_reserved(&inner, playlist, index, revision, |_| {
            entered.send(()).unwrap();
            proceed
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            Ok(bytes)
        })
    });
    ready
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    drop(request);
    release.send(()).unwrap();
    assert!(matches!(worker.join().unwrap(), Err(MediaError::Stream(_))));
    assert_eq!(player.snapshot(), before);
}

#[test]
fn closing_a_session_invalidates_queued_and_running_play_without_affecting_a_new_session() {
    for running in [false, true] {
        let player = AudioPlayer::new();
        let next_session = AudioPlayer::new();
        playing_without_device(
            &next_session,
            vec![item("new-session", std::path::Path::new("/new.ogg"))],
            cursor(false),
        );
        let next_before = next_session.snapshot();
        let mut request = player.reserve().unwrap();
        let inner = request.inner.clone();
        let playlist = vec![item("pending", std::path::Path::new("/pending.ogg"))];
        let revision = request.revision;
        let bytes = crate::codec::encode_voice(&vec![0; 1600]).unwrap();
        let (entered, ready) = std::sync::mpsc::sync_channel(0);
        let (release, proceed) = std::sync::mpsc::sync_channel(0);
        let worker = std::thread::spawn(move || {
            if !running {
                entered.send(()).unwrap();
                proceed
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
            }
            play_reserved(&inner, playlist, 0, revision, |_| {
                assert!(
                    running,
                    "closed queued requests must not read or open a device"
                );
                entered.send(()).unwrap();
                proceed
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
                Ok(bytes)
            })
        });
        ready
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        // Invalidation must not wait for the resource mutex or device disposal.
        let held = player.inner.state.lock().unwrap();
        let closing = player.close();
        drop(held);
        release.send(()).unwrap();
        assert!(matches!(worker.join().unwrap(), Err(MediaError::Stream(_))));
        request.completed = true;
        drop(request);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(closing).unwrap();
        assert_eq!(player.snapshot().status, AudioPlaybackStatus::Idle);
        assert!(player
            .play(vec![item("late", std::path::Path::new("/late.ogg"))], 0)
            .is_err());
        assert!(player.resume().is_err());
        assert_eq!(next_session.snapshot(), next_before);
    }
}

#[test]
fn delayed_stop_cleanup_does_not_cancel_a_newer_pending_play() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let player = AudioPlayer::new();
    playing_without_device(
        &player,
        vec![item("previous", std::path::Path::new("/previous.ogg"))],
        cursor(false),
    );
    let stopping = player.stop_async();
    let directory = tempfile::tempdir().unwrap();
    let newer = player.play_async(
        vec![item("newer", &directory.path().join("missing.ogg"))],
        0,
    );
    runtime.block_on(stopping).unwrap();
    assert_eq!(player.snapshot().status, AudioPlaybackStatus::Idle);
    assert!(matches!(runtime.block_on(newer), Err(MediaError::Io(_))));
    assert!(player.snapshot().item.is_none());
}

#[test]
fn delayed_stop_cleanup_preserves_a_playback_published_by_a_newer_request() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let player = AudioPlayer::new();
    let stopping = player.stop_async();
    let mut newer = player.reserve().unwrap();
    // Install the same committed state as start_playlist, without an audio device.
    playing_without_device(
        &player,
        vec![item("newer", std::path::Path::new("/newer.ogg"))],
        cursor(false),
    );
    player.inner.state.lock().unwrap().revision = newer.revision;
    player
        .inner
        .revision
        .store(newer.revision & !1, Ordering::Release);
    newer.completed = true;
    drop(newer);
    let before = player.snapshot();
    runtime.block_on(stopping).unwrap();
    assert_eq!(player.snapshot(), before);
}

#[test]
fn cancelling_after_worker_publication_stops_that_stream_before_the_awaiter_accepts_it() {
    let player = AudioPlayer::new();
    let request = player.reserve().unwrap();
    playing_without_device(
        &player,
        vec![item("unaccepted", std::path::Path::new("/unaccepted.ogg"))],
        cursor(false),
    );
    player.inner.state.lock().unwrap().revision = request.revision;
    player
        .inner
        .revision
        .store(request.revision & !1, Ordering::Release);
    let mut events = player.subscribe();
    drop(request);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let event = runtime.block_on(async move {
        tokio::time::timeout(std::time::Duration::from_secs(5), events.recv())
            .await
            .unwrap()
            .unwrap()
    });
    assert!(
        matches!(event, AudioPlaybackEvent::Changed(snapshot) if snapshot.status == AudioPlaybackStatus::Idle && snapshot.item.is_none())
    );
    assert_eq!(player.snapshot().status, AudioPlaybackStatus::Idle);
}
