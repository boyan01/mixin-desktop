//! Playback state shared by every target.

#[derive(Clone, Debug, PartialEq)]
pub struct AudioPlaybackItem {
    pub id: String,
    pub path: String,
    pub duration_millis: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioPlaybackStatus {
    Idle,
    Playing,
    Paused,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioPlaybackSnapshot {
    pub status: AudioPlaybackStatus,
    pub item: Option<AudioPlaybackItem>,
    pub position_millis: u64,
    pub duration_millis: u64,
    pub speed: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AudioPlaybackEvent {
    Changed(AudioPlaybackSnapshot),
    Finished(String),
    Failed { id: Option<String>, message: String },
}
