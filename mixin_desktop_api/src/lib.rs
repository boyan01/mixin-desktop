pub mod access;
mod account;
mod desktop;
pub mod dto;
pub mod error;
mod logging;
mod login;
pub mod model;

pub use access::{AttachmentAccess, ConversationAccess, MessageAccess, StickerAccess, UserAccess};
pub use account::AccountClient;
pub use desktop::{DesktopClient, SettingsClient};
pub use dto::{
    CircleItem, CodeResult, ConversationDetailItem, ConversationListData,
    ConversationParticipantItem, ConversationStorageUsage, ConversationUnseenCount, GroupAvatar,
    GroupConversationItem, ImageMessageView, MessageListView, MessageOrderInfoView,
    NotificationEvent, PinMessagePreviewItem, SharedAppItem, SnapshotDetailItem, StickerAlbumItem,
    StickerDetailItem, StickerItem, StorageCategoryUsage, UserProfileItem,
};
pub use error::{ClientError, ClientResult};
pub use logging::{init_logging, log_directory};
pub use login::LoginClient;
pub use model::{
    AccountProfile, ConnectionFailedReason, ConversationChangeEvent, ConversationListItem,
    DeviceTransferCommand, DeviceTransferEvent, HttpResponseItem, McpServerStatusItem,
    McpSettingsItem, ProxyItem, ProxySettingsItem,
};
