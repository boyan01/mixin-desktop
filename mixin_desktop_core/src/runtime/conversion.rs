use super::model::*;

impl From<crate::db::mixin::conversation::ConversationListItem> for ConversationListData {
    fn from(item: crate::db::mixin::conversation::ConversationListItem) -> Self {
        let updated_at_millis = item.updated_at_millis();
        Self {
            conversation_id: item.conversation_id,
            owner_id: item.owner_id,
            name: item.name,
            avatar_url: item.avatar_url,
            category: item.category,
            draft: item.draft,
            status: item.status,
            last_read_message_id: item.last_read_message_id,
            last_message: item.last_message,
            last_message_category: item.last_message_category,
            last_message_status: item.last_message_status,
            last_message_sender_id: item.last_message_sender_id,
            last_message_sender_name: item.last_message_sender_name,
            updated_at_millis,
            unseen_count: item.unseen_count,
            mention_count: item.mention_count,
            is_muted: item.is_muted,
            is_verified: item.is_verified,
            is_bot: item.is_bot,
            is_pinned: item.is_pinned,
            relationship: item.relationship,
            identity_number: item.identity_number,
            circle_ids: item
                .circle_ids
                .split(',')
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect(),
            participant_count: item.participant_count,
            group_avatars: item
                .group_avatar_data
                .split('\u{1e}')
                .filter_map(|value| {
                    let mut parts = value.split('\u{1f}');
                    Some(GroupAvatar {
                        user_id: parts.next()?.to_string(),
                        name: parts.next()?.to_string(),
                        avatar_url: parts.next()?.to_string(),
                    })
                })
                .collect(),
        }
    }
}

impl From<crate::db::mixin::conversation::Conversation> for ConversationDetailItem {
    fn from(conversation: crate::db::mixin::conversation::Conversation) -> Self {
        Self {
            conversation_id: conversation.conversation_id,
            name: conversation.name,
            announcement: conversation.announcement,
            code_url: conversation.code_url,
            created_at_millis: conversation.created_at.timestamp_millis(),
            mute_until_millis: conversation.mute_until.timestamp_millis(),
            expire_in: conversation.expire_in,
        }
    }
}

impl From<crate::db::mixin::conversation::ConversationListItem> for GroupConversationItem {
    fn from(conversation: crate::db::mixin::conversation::ConversationListItem) -> Self {
        Self {
            conversation_id: conversation.conversation_id,
            name: conversation.name,
            avatar_url: conversation.avatar_url,
            participant_count: conversation.participant_count,
        }
    }
}

impl From<crate::db::mixin::participant::ParticipantListItem> for ConversationParticipantItem {
    fn from(participant: crate::db::mixin::participant::ParticipantListItem) -> Self {
        Self {
            user_id: participant.user_id,
            role: participant.role,
            created_at_millis: participant.created_at.timestamp_millis(),
            identity_number: participant.identity_number,
            full_name: participant.full_name,
            avatar_url: participant.avatar_url,
            biography: participant.biography,
            is_verified: participant.is_verified,
            is_bot: participant.is_bot,
            relationship: participant.relationship,
        }
    }
}

impl From<crate::db::mixin::user::User> for ConversationParticipantItem {
    fn from(user: crate::db::mixin::user::User) -> Self {
        Self {
            user_id: user.user_id,
            role: None,
            created_at_millis: user.created_at.timestamp_millis(),
            identity_number: user.identity_number,
            full_name: user.full_name,
            avatar_url: user.avatar_url,
            biography: user.biography,
            is_verified: user.is_verified,
            is_bot: user.app_id.is_some_and(|app_id| !app_id.is_empty()),
            relationship: format!("{:?}", user.relationship).to_uppercase(),
        }
    }
}

impl From<crate::db::mixin::user::User> for UserProfileItem {
    fn from(user: crate::db::mixin::user::User) -> Self {
        Self {
            user_id: user.user_id,
            identity_number: user.identity_number,
            full_name: user.full_name,
            avatar_url: user.avatar_url,
            biography: user.biography,
            is_verified: user.is_verified,
            is_bot: user.app_id.is_some_and(|app_id| !app_id.is_empty()),
            relationship: format!("{:?}", user.relationship).to_uppercase(),
            code_url: user.code_url,
        }
    }
}

impl From<sdk::App> for SharedAppItem {
    fn from(app: sdk::App) -> Self {
        Self {
            app_id: app.app_id,
            name: app.name,
            icon_url: app.icon_url,
            description: app.description,
            home_uri: app.home_uri,
        }
    }
}

impl From<crate::db::mixin::app::App> for SharedAppItem {
    fn from(app: crate::db::mixin::app::App) -> Self {
        Self {
            app_id: app.app_id,
            name: app.name,
            icon_url: app.icon_url,
            description: app.description,
            home_uri: app.home_uri,
        }
    }
}

impl From<crate::db::mixin::circle::CircleSummary> for CircleItem {
    fn from(circle: crate::db::mixin::circle::CircleSummary) -> Self {
        Self {
            circle_id: circle.circle_id,
            name: circle.name,
            conversation_count: circle.conversation_count,
        }
    }
}

impl From<sdk::Circle> for CircleItem {
    fn from(circle: sdk::Circle) -> Self {
        Self {
            circle_id: circle.circle_id,
            name: circle.name,
            conversation_count: 0,
        }
    }
}

impl From<super::StickerDetail> for StickerDetailItem {
    fn from(detail: super::StickerDetail) -> Self {
        Self {
            sticker: detail.sticker.into(),
            album: detail.album.map(Into::into),
            album_stickers: detail.album_stickers.into_iter().map(Into::into).collect(),
            is_personal: detail.is_personal,
        }
    }
}

impl From<crate::db::mixin::sticker::Sticker> for StickerItem {
    fn from(sticker: crate::db::mixin::sticker::Sticker) -> Self {
        Self {
            sticker_id: sticker.sticker_id,
            album_id: sticker.album_id,
            name: sticker.name,
            asset_url: sticker.asset_url,
            asset_width: sticker.asset_width,
            asset_height: sticker.asset_height,
            asset_type: sticker.asset_type,
            created_at_millis: sticker.created_at.timestamp_millis(),
            last_use_at_millis: sticker.last_use_at.map(|value| value.timestamp_millis()),
        }
    }
}

impl From<crate::db::mixin::sticker::StickerAlbum> for StickerAlbumItem {
    fn from(album: crate::db::mixin::sticker::StickerAlbum) -> Self {
        Self {
            album_id: album.album_id,
            name: album.name,
            icon_url: album.icon_url,
            category: album.category,
            description: album.description,
            banner: album.banner,
            added: album.added,
            is_verified: album.is_verified,
        }
    }
}

impl From<crate::db::mixin::message::ImageMessageItem> for ImageMessageView {
    fn from(item: crate::db::mixin::message::ImageMessageItem) -> Self {
        Self {
            message_id: item.message_id,
            created_at_micros: item.created_at.and_utc().timestamp_micros(),
            media_url: item.media_url,
            media_name: item.media_name,
            can_forward: item.can_forward,
        }
    }
}

impl From<crate::db::mixin::message::MessageListItem> for MessageListView {
    fn from(item: crate::db::mixin::message::MessageListItem) -> Self {
        let created_at_micros = item.created_at_micros();
        Self {
            message_id: item.message_id,
            conversation_id: item.conversation_id,
            sender_id: item.user_id,
            sender_name: item.sender_name,
            sender_identity_number: Some(item.sender_identity_number),
            sender_avatar_url: item.sender_avatar_url,
            sender_is_verified: item.sender_is_verified,
            sender_relationship: item.sender_relationship,
            sender_app_id: item.sender_app_id,
            sender_is_scam: item.sender_is_scam,
            sender_is_bot: item.sender_is_bot,
            category: item.category,
            content: item.content.unwrap_or_default(),
            status: item.status.into(),
            created_at_micros,
            media_url: item.media_url,
            media_mime_type: item.media_mime_type,
            media_size: item.media_size,
            media_duration: item.media_duration,
            media_width: item.media_width,
            media_height: item.media_height,
            thumb_image: item.thumb_image,
            media_status: format!("{:?}", item.media_status).to_uppercase(),
            quote_message_id: item.quote_message_id,
            quote_content: item.quote_content,
            caption: item.caption,
            action: item.action,
            participant_id: item.participant_id,
            participant_full_name: item.participant_full_name,
            snapshot_id: item.snapshot_id,
            snapshot_type: item.snapshot_type,
            snapshot_amount: item.snapshot_amount,
            snapshot_memo: item.snapshot_memo,
            snapshot_asset_id: item.snapshot_asset_id,
            snapshot_asset_symbol: item.snapshot_asset_symbol,
            snapshot_asset_icon_url: item.snapshot_asset_icon_url,
            snapshot_chain_icon_url: item.snapshot_chain_icon_url,
            snapshot_opponent_id: item.snapshot_opponent_id,
            snapshot_transaction_hash: item.snapshot_transaction_hash,
            snapshot_created_at: item.snapshot_created_at,
            inscription_hash: item.inscription_hash,
            inscription_collection_hash: item.inscription_collection_hash,
            inscription_sequence: item.inscription_sequence,
            inscription_content_type: item.inscription_content_type,
            inscription_content_url: item.inscription_content_url,
            inscription_name: item.inscription_name,
            inscription_icon_url: item.inscription_icon_url,
            hyperlink: item.hyperlink,
            media_name: item.media_name,
            album_id: item.album_id,
            sticker_id: item.sticker_id,
            shared_user_id: item.shared_user_id,
            media_waveform: item.media_waveform,
            thumb_url: item.thumb_url,
            conversation_owner_id: item.conversation_owner_id,
            conversation_category: item.conversation_category,
            shared_user_full_name: item.shared_user_full_name,
            shared_user_identity_number: item.shared_user_identity_number,
            shared_user_avatar_url: item.shared_user_avatar_url,
            shared_user_is_verified: item.shared_user_is_verified,
            shared_user_app_id: item.shared_user_app_id,
            sticker_asset_url: item.sticker_asset_url,
            sticker_asset_width: item.sticker_asset_width,
            sticker_asset_height: item.sticker_asset_height,
            sticker_asset_name: item.sticker_asset_name,
            sticker_asset_type: item.sticker_asset_type,
            mention_read: item.mention_read,
            pinned: item.pinned,
            expire_in: item.expire_in,
        }
    }
}

impl From<crate::db::mixin::transcript_message::TranscriptMessageListItem> for MessageListView {
    fn from(item: crate::db::mixin::transcript_message::TranscriptMessageListItem) -> Self {
        Self {
            message_id: item.message_id,
            conversation_id: item.conversation_id,
            sender_id: item.user_id,
            sender_name: item.sender_name,
            sender_identity_number: Some(item.sender_identity_number),
            sender_avatar_url: item.sender_avatar_url,
            sender_is_verified: item.sender_is_verified,
            sender_relationship: item.sender_relationship,
            sender_app_id: item.sender_app_id,
            sender_is_scam: item.sender_is_scam,
            sender_is_bot: item.sender_is_bot,
            category: item.category,
            content: item.content.unwrap_or_default(),
            status: item.status.into(),
            created_at_micros: item.created_at.timestamp_micros(),
            media_url: item.media_url,
            media_mime_type: item.media_mime_type,
            media_size: item.media_size,
            media_duration: item.media_duration.unwrap_or_default(),
            media_width: item.media_width,
            media_height: item.media_height,
            thumb_image: item.thumb_image,
            media_status: format!("{:?}", item.media_status.unwrap_or_default()).to_uppercase(),
            quote_message_id: item.quote_message_id,
            quote_content: item.quote_content,
            caption: item.caption,
            action: None,
            participant_id: None,
            participant_full_name: None,
            snapshot_id: None,
            snapshot_type: None,
            snapshot_amount: None,
            snapshot_memo: None,
            snapshot_asset_id: None,
            snapshot_asset_symbol: None,
            snapshot_asset_icon_url: None,
            snapshot_chain_icon_url: None,
            snapshot_opponent_id: None,
            snapshot_transaction_hash: None,
            snapshot_created_at: None,
            inscription_hash: None,
            inscription_collection_hash: None,
            inscription_sequence: None,
            inscription_content_type: None,
            inscription_content_url: None,
            inscription_name: None,
            inscription_icon_url: None,
            hyperlink: None,
            media_name: item.media_name,
            album_id: None,
            sticker_id: item.sticker_id,
            shared_user_id: item.shared_user_id,
            media_waveform: item.media_waveform,
            thumb_url: item.thumb_url,
            conversation_owner_id: None,
            conversation_category: None,
            shared_user_full_name: item.shared_user_full_name,
            shared_user_identity_number: item.shared_user_identity_number,
            shared_user_avatar_url: item.shared_user_avatar_url,
            shared_user_is_verified: item.shared_user_is_verified,
            shared_user_app_id: item.shared_user_app_id,
            sticker_asset_url: item.sticker_asset_url,
            sticker_asset_width: item.sticker_asset_width,
            sticker_asset_height: item.sticker_asset_height,
            sticker_asset_name: item.sticker_asset_name,
            sticker_asset_type: item.sticker_asset_type,
            mention_read: None,
            pinned: false,
            expire_in: None,
        }
    }
}
