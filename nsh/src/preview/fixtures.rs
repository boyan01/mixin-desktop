//! Fixed data for preview scenes. Nothing here reads an account or a database.

use mixin_desktop_api::{AccountProfile, ConversationUnseenCount};

/// A fixed user id in the UUID form the avatar palette hashes.
pub(super) const USER_ID: &str = "3a8f2c61-7b4d-4e19-9c05-d2e7f1a64b38";

pub(super) const NAME: &str = "Alex Chen";
pub(super) const IDENTITY_NUMBER: &str = "70001001";

pub(super) fn profile() -> AccountProfile {
    AccountProfile {
        // A UUID, as real ids are: the avatar placeholder color is picked from it.
        user_id: USER_ID.into(),
        full_name: NAME.into(),
        avatar_url: String::new(),
        identity_number: IDENTITY_NUMBER.into(),
        biography: String::new(),
        phone: String::new(),
        created_at: String::new(),
        is_verified: false,
        fiat_currency: String::new(),
        membership: None,
    }
}

/// Unread counts for the rail categories: one muted category, one empty.
pub(super) fn unseen_counts() -> Vec<ConversationUnseenCount> {
    [("chats", 12, 0), ("groups", 5, 5), ("bots", 2, 0)]
        .into_iter()
        .map(|(category, count, muted_count)| ConversationUnseenCount {
            category: category.into(),
            circle_id: None,
            count,
            muted_count,
        })
        .collect()
}

/// Names with stable UUIDs, so each avatar placeholder shows its own palette color.
pub(super) const PEOPLE: &[(&str, &str)] = &[
    ("6f1d9a27-3c58-4b0e-8e72-a94c5d1b0f63", "Bella Wu"),
    ("c2e84b19-0a6f-47d3-b5c1-7e39f2d8a540", "Carlos Diaz"),
    ("91b7e3d4-5f20-4c8a-a6e9-03d1c7b25f8e", "陈小明"),
    ("e4a0c6f8-2d93-41b7-9f5e-6b8a1c3d7e02", "Dana Okafor"),
];
