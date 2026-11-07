//! Explicit local playback contract check. Plays a supplied Ogg/Opus fixture.
use futures::StreamExt as _;
use mixin_desktop_api::{AudioPlaybackEvent, AudioPlaybackItem, AudioPlaybackStatus, MediaClient};
use std::{error::Error, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("provide an Ogg/Opus fixture path")?;
    let original = std::fs::read(&path)?;
    let media = MediaClient::new();
    let mut events = Box::pin(media.subscribe_audio_playback());
    let result = async {
        media
            .play_audio(
                vec![AudioPlaybackItem {
                    id: "local-playback-check".into(),
                    path: path.clone(),
                    duration_millis: 0,
                }],
                0,
            )
            .await?;
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match events.next().await {
                    Some(AudioPlaybackEvent::Changed(snapshot)) if snapshot.position_millis > 0 => {
                        break Ok::<_, Box<dyn Error>>(())
                    }
                    Some(AudioPlaybackEvent::Failed { .. }) => {
                        break Err("native playback failed".into())
                    }
                    None => break Err("playback subscription ended".into()),
                    _ => {}
                }
            }
        })
        .await??;
        media.pause_audio().await?;
        assert_eq!(
            media.audio_playback_snapshot().status,
            AudioPlaybackStatus::Paused
        );
        media.set_audio_speed(1.5).await?;
        assert_eq!(media.audio_playback_snapshot().speed, 1.5);
        media.seek_audio(100);
        assert!(
            media
                .audio_playback_snapshot()
                .position_millis
                .abs_diff(100)
                <= 10
        );
        media.resume_audio().await?;
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match events.next().await {
                    Some(AudioPlaybackEvent::Finished(_)) => break Ok::<_, Box<dyn Error>>(()),
                    Some(AudioPlaybackEvent::Failed { .. }) => {
                        break Err("native playback failed".into())
                    }
                    None => break Err("playback subscription ended".into()),
                    _ => {}
                }
            }
        })
        .await??;
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    media.stop_audio().await?;
    assert_eq!(
        media.audio_playback_snapshot().status,
        AudioPlaybackStatus::Idle
    );
    assert!(media.audio_playback_snapshot().item.is_none());
    assert_eq!(std::fs::read(path)?, original);
    result?;
    println!("PASS: native playback clock, pause/resume, seek, speed, natural finish and stop; source bytes unchanged");
    Ok(())
}
