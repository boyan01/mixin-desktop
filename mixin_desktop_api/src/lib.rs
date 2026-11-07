pub mod access;
mod account;
mod bot;
#[cfg(not(target_arch = "wasm32"))]
mod device_transfer;
pub use bot::BotClient;
mod desktop;
pub mod dto;
pub mod error;
mod logging;
mod login;
#[cfg(not(target_arch = "wasm32"))]
mod media;
#[cfg(not(target_arch = "wasm32"))]
mod screen_lock;
#[cfg(not(target_arch = "wasm32"))]
pub use screen_lock::ScreenLockClient;
pub mod model;

pub use access::{AttachmentAccess, ConversationAccess, MessageAccess, StickerAccess, UserAccess};
pub use account::AccountClient;
pub use desktop::{DesktopClient, SettingsClient};
pub use dto::{
    BotContext, BotSignature, CircleConversationEdit, CircleItem, CodeResult,
    ConversationDetailItem, ConversationListData, ConversationParticipantItem,
    ConversationStorageUsage, ConversationUnseenCount, GifItem, GroupAvatar, GroupConversationItem,
    ImageMessageView, MessageActions, MessageListView, MessageOrderInfoView, NotificationEvent,
    PinMessagePreviewItem, QuickSearchItem, ScreenLockState, SendReceipt, SharedAppItem,
    SnapshotDetailItem, StickerAlbumItem, StickerDetailItem, StickerItem, StorageCategoryUsage,
    TranscriptSummaryItem, UserProfileItem, WebShareContent,
};
pub use error::{ClientError, ClientResult};
pub use logging::{data_directory, init_logging, log_directory, write_log, LogLevel};
pub use login::LoginClient;
#[cfg(not(target_arch = "wasm32"))]
pub use media::{MediaClient, StickerAnimationClient};
#[cfg(not(target_arch = "wasm32"))]
pub use mixin_desktop_media::{
    AudioPlaybackEvent, AudioPlaybackItem, AudioPlaybackSnapshot, AudioPlaybackStatus, ImageEdit,
    PreparedAttachment, VoiceRecorderEvent, VoiceRecorderSnapshot, VoiceRecorderStatus,
    VoiceRecording,
};
pub use model::{
    AccountProfile, AttachmentTransferProgress, ConnectionFailedReason, ConversationChangeEvent,
    ConversationListItem, DeviceTransferCommand, DeviceTransferEvent, DeviceTransferOperation,
    DeviceTransferSnapshot, HttpResponseItem, McpServerStatusItem, McpSettingsItem, ProxyItem,
    ProxySettingsItem,
};

pub use mixin_desktop_core::runtime::active_membership_plan;
