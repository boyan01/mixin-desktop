//! Fixed data for preview scenes. Nothing here reads an account or a database.

use mixin_desktop_api::{AccountProfile, ConversationUnseenCount};

/// A fixed user id in the UUID form the avatar palette hashes.
pub(super) const USER_ID: &str = "3a8f2c61-7b4d-4e19-9c05-d2e7f1a64b38";

pub(super) const NAME: &str = "Alex Chen";
pub(super) const IDENTITY_NUMBER: &str = "70001001";

pub(super) const POST_PROSE: &str = r#"# Markdown 阅读体验

这是一篇包含中文、English 和 emoji 🦀 的文章。正文支持 **重点内容**、*斜体*、~~删除线~~ 和 `inline code`。

## 清晰的内容层级

文章应当留出舒适的边距。窗口变窄时，正文自然换行；调整消息字号后，行高和标题也应保持协调。

### 列表与引用

1. 标题、段落和列表保持一致的节奏。
2. 长文章可以垂直滚动，代码和表格可以横向滚动。

> 引用用于补充上下文。中文与 English 应当在同一个段落中自然换行。

- [x] 支持标题、强调和引用
- [ ] 检查深色模式与较大的字号

访问 [Mixin](https://mixin.one) 查看更多信息。
"#;

pub(super) const POST_BLOCKS: &str = r#"# 代码与表格

正文中的 `inline code` 应当清晰可辨。

```rust
fn main() {
    let message = "A deliberately long code line: 中文与 English stay on one line while the code block scrolls horizontally.";
    println!("{message}");
}
```

## 支持情况

| 类型 | Markdown 写法 | 展示方式 |
| :--- | :--- | :--- |
| 标题 | # Heading | 区分文章层级 |
| 代码 | Fenced code block | 保留缩进、语法高亮、横向滚动 |
| 表格 | GitHub Flavored Markdown | 较窄的窗口中仍可以阅读每一列的内容 |

---

最后一段用于检查滚动区域底部的留白。
"#;

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

/// Message rows for the message scenes. Each list reads oldest first and is
/// returned newest first, as a timeline holds it.
pub(super) mod messages {
    use std::collections::HashMap;

    use mixin_desktop_api::MessageListView;
    use serde_json::json;

    use super::{IDENTITY_NUMBER, NAME, PEOPLE, USER_ID};

    const CONVERSATION_ID: &str = "5d0c3b8e-91a4-4f27-b6e2-8c1f4a7d2e90";
    /// 2026-11-02 09:00 UTC: a fixed day, so captures do not move with the clock.
    const START_MICROS: i64 = 1_793_610_000_000_000;
    const PHOTO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/preview/fixtures/photo.jpg");
    const STICKER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/logo.png");
    /// A small file that exists, for attachments shown as downloaded.
    const FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/preview/nsh.json");
    /// BlurHash placeholders, as a message carries before its media downloads.
    const BLUR_SKY: &str = "LEHV6nWB2yk8pyo0adR*.7kCMdnj";
    const BLUR_DUSK: &str = "LGF5]+Yk^6#M@-5c,1J5@[or[Q6.";
    const WAVEFORM: &str =
        "Hmux5P320JJFRo3C3Nq/kFcfUXWHiHphRSkpOD9APDYuJR4mLzg/Q0E5KCxJZ3+Ni3ZPIV2Xxd/ewYpB";

    fn me() -> (&'static str, &'static str) {
        (USER_ID, NAME)
    }

    fn person(index: usize) -> (&'static str, &'static str) {
        PEOPLE[index]
    }

    /// Names the text rows mention, by identity number.
    pub(in crate::preview) fn mention_names() -> HashMap<String, String> {
        HashMap::from([
            (IDENTITY_NUMBER.to_owned(), NAME.to_owned()),
            ("70002002".to_owned(), PEOPLE[0].1.to_owned()),
        ])
    }

    /// A delivered row of `category` from `sender`, with no media or quote.
    fn row(sender: (&str, &str), category: &str, content: &str) -> MessageListView {
        let (sender_id, sender_name) = sender;
        MessageListView {
            message_id: String::new(),
            conversation_id: CONVERSATION_ID.into(),
            sender_id: sender_id.into(),
            sender_name: sender_name.into(),
            sender_identity_number: None,
            sender_avatar_url: String::new(),
            sender_is_verified: false,
            sender_membership: None,
            sender_relationship: if sender_id == USER_ID {
                "ME".into()
            } else {
                "FRIEND".into()
            },
            sender_app_id: None,
            sender_is_scam: false,
            sender_is_bot: false,
            sender_participant_id: None,
            sender_role: None,
            category: category.into(),
            content: content.into(),
            status: if sender_id == USER_ID {
                "READ"
            } else {
                "DELIVERED"
            }
            .into(),
            created_at_micros: 0,
            media_url: None,
            media_mime_type: None,
            media_size: None,
            media_duration: String::new(),
            media_width: None,
            media_height: None,
            thumb_image: None,
            media_status: String::new(),
            quote_message_id: None,
            quote_content: None,
            quote_user_membership: None,
            caption: None,
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
            media_name: None,
            album_id: None,
            sticker_id: None,
            shared_user_id: None,
            media_waveform: None,
            thumb_url: None,
            conversation_owner_id: None,
            conversation_category: Some("GROUP".into()),
            shared_user_full_name: None,
            shared_user_identity_number: None,
            shared_user_avatar_url: None,
            shared_user_is_verified: false,
            shared_user_membership: None,
            shared_user_app_id: None,
            sticker_asset_url: None,
            sticker_asset_width: None,
            sticker_asset_height: None,
            sticker_asset_name: None,
            sticker_asset_type: None,
            mention_read: None,
            pinned: false,
            expire_in: None,
        }
    }

    fn text(sender: (&str, &str), content: &str) -> MessageListView {
        row(sender, "SIGNAL_TEXT", content)
    }

    fn status(mut row: MessageListView, status: &str) -> MessageListView {
        row.status = status.into();
        row
    }

    /// `row` replying to `quoted`, which must come earlier in the same list.
    fn reply(mut row: MessageListView, quoted: &MessageListView) -> MessageListView {
        row.quote_message_id = Some(quoted.message_id.clone());
        row.quote_content = Some(
            json!({
                "message_id": quoted.message_id,
                "type": quoted.category,
                "status": quoted.status,
                "content": quoted.content,
                "user_id": quoted.sender_id,
                "user_full_name": quoted.sender_name,
                "media_url": quoted.media_url,
                "media_name": quoted.media_name,
                "thumb_image": quoted.thumb_image,
                "asset_url": quoted.sticker_asset_url,
                "asset_type": quoted.sticker_asset_type,
                "sticker_id": quoted.sticker_id,
                "shared_user_id": quoted.shared_user_id,
                "shared_user_full_name": quoted.shared_user_full_name,
                "shared_user_identity_number": quoted.shared_user_identity_number,
                "shared_user_avatar_url": quoted.shared_user_avatar_url,
            })
            .to_string(),
        );
        row
    }

    fn image(sender: (&str, &str), downloaded: bool) -> MessageListView {
        let mut row = row(sender, "SIGNAL_IMAGE", "");
        row.media_mime_type = Some("image/jpeg".into());
        row.media_width = Some(1200);
        row.media_height = Some(800);
        row.media_size = Some(18_143);
        row.thumb_image = Some(BLUR_SKY.into());
        if downloaded {
            row.media_url = Some(PHOTO.into());
            row.media_status = "DONE".into();
        } else {
            row.media_status = "CANCELED".into();
        }
        row
    }

    fn attachment(
        sender: (&str, &str),
        category: &str,
        name: &str,
        size: i64,
        downloaded: bool,
    ) -> MessageListView {
        let mut row = row(sender, category, "");
        row.media_name = Some(name.into());
        row.media_size = Some(size);
        if downloaded {
            row.media_url = Some(FILE.into());
            row.media_status = "DONE".into();
        } else {
            row.media_status = "CANCELED".into();
        }
        row
    }

    fn audio(sender: (&str, &str), downloaded: bool, heard: bool) -> MessageListView {
        let mut row = attachment(sender, "SIGNAL_AUDIO", "voice.ogg", 24_576, downloaded);
        row.media_mime_type = Some("audio/ogg".into());
        row.media_duration = "14000".into();
        row.media_waveform = Some(WAVEFORM.into());
        if heard {
            row.media_status = "READ".into();
        }
        row
    }

    /// Ids and times in list order, a minute apart, then newest first.
    fn timeline(rows: Vec<MessageListView>) -> Vec<MessageListView> {
        let mut rows = numbered(rows);
        rows.reverse();
        rows
    }

    fn numbered(rows: Vec<MessageListView>) -> Vec<MessageListView> {
        rows.into_iter()
            .enumerate()
            .map(|(index, mut row)| {
                if row.message_id.is_empty() {
                    row.message_id = id(index);
                }
                row.created_at_micros = START_MICROS + index as i64 * 60_000_000;
                row
            })
            .collect()
    }

    fn id(index: usize) -> String {
        format!("00000000-0000-4000-8000-{index:012}")
    }

    fn sticker(sender: (&str, &str)) -> MessageListView {
        let mut row = row(sender, "SIGNAL_STICKER", "");
        row.sticker_id = Some("2c9e6a4f-8b1d-4e37-9a05-71d3c8f0b6e2".into());
        row.sticker_asset_url = Some(STICKER.into());
        row.sticker_asset_type = Some("PNG".into());
        row.sticker_asset_width = Some(192);
        row.sticker_asset_height = Some(192);
        row
    }

    fn contact(sender: (&str, &str)) -> MessageListView {
        let mut row = row(sender, "SIGNAL_CONTACT", "");
        row.shared_user_id = Some(PEOPLE[3].0.into());
        row.shared_user_full_name = Some(PEOPLE[3].1.into());
        row.shared_user_identity_number = Some("70004004".into());
        row.shared_user_is_verified = true;
        row
    }

    fn bot(category: &str, content: serde_json::Value) -> MessageListView {
        let mut row = row(person(3), category, &content.to_string());
        row.sender_app_id = Some(PEOPLE[3].0.into());
        row.sender_is_bot = true;
        row
    }

    /// The same bot message from another, verified bot: its badge replaces the bot's.
    fn verified(mut row: MessageListView) -> MessageListView {
        (row.sender_id, row.sender_name) = (PEOPLE[2].0.into(), PEOPLE[2].1.into());
        row.sender_app_id = Some(PEOPLE[2].0.into());
        row.sender_is_verified = true;
        row
    }

    /// Text in both directions, a reply, and each delivery state of one's own.
    pub(in crate::preview) fn text_rows() -> Vec<MessageListView> {
        let mut question = text(person(0), "下午的设计评审改到 3 点，可以吗？");
        question.message_id = id(100);
        timeline(vec![
            question.clone(),
            text(
                person(0),
                "今天我们一起看看消息的显示效果。较长的文字应该有足够的行间距，让每一行都容易阅读。\n\nThis paragraph mixes English text, punctuation, and several lines, to check the rhythm of a long message.",
            ),
            reply(text(me(), "可以，3 点见 👍"), &question),
            text(
                person(1),
                "项目地址 https://github.com/MixinNetwork ，邮箱 hello@example.com\n有问题也可以找 @70001001 或 @70002002。",
            ),
            status(
                text(me(), "这是发出的消息，末尾带一个链接 https://mixin.one"),
                "DELIVERED",
            ),
            status(text(me(), "已发送，对方还没收到"), "SENT"),
            status(text(me(), "正在发送…"), "SENDING"),
        ])
    }

    /// Replies to each kind of message a quote shows its own way.
    pub(in crate::preview) fn reply_rows() -> Vec<MessageListView> {
        let quoted = |mut row: MessageListView, index: usize| {
            row.message_id = id(100 + index);
            row
        };
        let photo = quoted(image(person(1), true), 0);
        let sticker = quoted(sticker(person(0)), 1);
        let file = quoted(
            attachment(me(), "SIGNAL_DATA", "设计评审.pdf", 2_457_600, true),
            2,
        );
        let contact = quoted(contact(person(1)), 3);
        let card = quoted(
            bot(
                "APP_CARD",
                json!({"title": "每周报告已生成", "description": "", "action": ""}),
            ),
            4,
        );
        timeline(vec![
            reply(
                text(person(2), "这张图的配色不错，就用这个方向吧。"),
                &photo,
            ),
            reply(text(me(), "哈哈"), &sticker),
            reply(text(person(0), "第 3 页的流程图需要再改一下。"), &file),
            reply(text(me(), "已经加好友了"), &contact),
            reply(text(person(2), "报告里的数据对不上"), &card),
        ])
    }

    /// Pictures: downloaded, with a caption, and waiting to download.
    pub(in crate::preview) fn photo_rows() -> Vec<MessageListView> {
        let mut captioned = image(me(), true);
        captioned.caption = Some("周末拍的天空，颜色很特别。".into());
        let mut panorama = image(person(1), false);
        panorama.media_width = Some(1800);
        panorama.media_height = Some(600);
        timeline(vec![image(person(0), true), captioned, panorama])
    }

    /// A video waiting to download, a sticker and a location.
    pub(in crate::preview) fn media_rows() -> Vec<MessageListView> {
        let mut video = row(person(0), "SIGNAL_VIDEO", "");
        video.media_mime_type = Some("video/mp4".into());
        video.media_width = Some(1280);
        video.media_height = Some(720);
        video.media_duration = "83000".into();
        video.media_size = Some(4_718_592);
        video.thumb_image = Some(BLUR_DUSK.into());
        video.media_status = "CANCELED".into();
        let location = row(
            person(2),
            "SIGNAL_LOCATION",
            &json!({
                "latitude": 31.2304,
                "longitude": 121.4737,
                "name": "人民广场",
                "address": "上海市黄浦区人民大道",
            })
            .to_string(),
        );
        let mut animated = sticker(person(1));
        animated.sticker_id = Some("00000000-0000-4000-8000-000000000901".into());
        animated.sticker_asset_url = Some(super::animated_sticker());
        animated.sticker_asset_type = Some("JSON".into());
        animated.sticker_asset_width = Some(96);
        animated.sticker_asset_height = Some(96);
        timeline(vec![video, sticker(me()), animated, location])
    }

    /// Voice messages and files, downloaded or not.
    pub(in crate::preview) fn file_rows() -> Vec<MessageListView> {
        timeline(vec![
            audio(person(2), true, false),
            audio(person(2), false, false),
            audio(me(), true, true),
            attachment(me(), "SIGNAL_DATA", "release-notes.md", 3_072, true),
            attachment(person(0), "SIGNAL_DATA", "设计评审.pdf", 2_457_600, true),
            attachment(person(0), "SIGNAL_DATA", "assets.zip", 52_428_800, false),
        ])
    }

    /// A contact, a post, a transcript and a live stream.
    pub(in crate::preview) fn card_rows() -> Vec<MessageListView> {
        let post = row(
            me(),
            "SIGNAL_POST",
            "# 发布说明\n\n- 消息行高更宽松\n- 链接不再带下划线\n\n```rust\nfn main() {}\n```",
        );
        let transcript = row(
            person(1),
            "SIGNAL_TRANSCRIPT",
            &json!([
                {"name": PEOPLE[0].1, "category": "SIGNAL_TEXT", "content": "评审改到 3 点"},
                {"name": NAME, "category": "SIGNAL_TEXT", "content": "可以，3 点见"},
                {"name": PEOPLE[1].1, "category": "SIGNAL_IMAGE", "content": ""},
                {"name": PEOPLE[2].1, "category": "SIGNAL_DATA", "content": ""},
            ])
            .to_string(),
        );
        let mut live = row(person(2), "SIGNAL_LIVE", "");
        live.media_width = Some(1280);
        live.media_height = Some(720);
        timeline(vec![contact(person(0)), post, transcript, live])
    }

    pub(in crate::preview) fn retry_transcript_rows() -> Vec<MessageListView> {
        let mut transcript = card_rows()
            .into_iter()
            .find(|row| row.category == "SIGNAL_TRANSCRIPT")
            .expect("fixture transcript");
        transcript.sender_id = USER_ID.into();
        transcript.sender_name = NAME.into();
        transcript.status = "SENDING".into();
        transcript.media_status = "CANCELED".into();
        vec![transcript]
    }

    /// A bot's cards: one with its own action, actions cards with and without a
    /// cover, and buttons.
    pub(in crate::preview) fn app_rows() -> Vec<MessageListView> {
        timeline(vec![
            bot(
                "APP_CARD",
                json!({
                    "app_id": PEOPLE[3].0,
                    "icon_url": STICKER,
                    "title": "Mixin Wallet",
                    "description": "Send, receive and swap assets",
                    "action": "https://mixin.one",
                }),
            ),
            bot(
                "APP_CARD",
                json!({
                    "app_id": PEOPLE[3].0,
                    "title": "每周报告已生成",
                    "description": "本周共有 12 条新消息，3 个待办事项需要处理。",
                    "cover": {"url": PHOTO, "width": 1800, "height": 600},
                    "actions": [
                        {"label": "查看", "color": "#3D75E3", "action": "https://mixin.one"},
                        {"label": "稍后", "color": "#7B7B7B", "action": "input:later"},
                    ],
                }),
            ),
            verified(bot(
                "APP_CARD",
                json!({
                    "app_id": PEOPLE[2].0,
                    "title": "⚠️ Overdue payments",
                    "description": "💵 Interest $0.01\n🧾 Overdue Principal $2.11\n🛒 Overdue Transactions\n• Normal Test $1.11\n• Credit Limit Test $1.00",
                    "actions": [
                        {"label": "View Details", "color": "#3D75E3", "action": "https://example.com/loans"},
                    ],
                }),
            )),
            bot(
                "APP_BUTTON_GROUP",
                json!([
                    {"label": "Help", "color": "#3D75E3", "action": "input:help"},
                    {"label": "Balance", "color": "#5FA7A4", "action": "input:balance"},
                    {"label": "Website", "color": "#E0794F", "action": "https://mixin.one"},
                ]),
            ),
        ])
    }

    /// A transfer, a wallet snapshot with a memo, and an inscription.
    pub(in crate::preview) fn wallet_rows() -> Vec<MessageListView> {
        let mut transfer = row(person(0), "SYSTEM_ACCOUNT_SNAPSHOT", "");
        transfer.snapshot_id = Some("7a1e4c92-3b6d-4f80-a5c7-e29d10b8f436".into());
        transfer.snapshot_amount = Some("0.05".into());
        transfer.snapshot_asset_id = Some("c6d0c728-2624-429b-8e0d-d9d19b6592fa".into());
        transfer.snapshot_asset_symbol = Some("BTC".into());
        let mut safe = row(me(), "SYSTEM_SAFE_SNAPSHOT", "");
        safe.snapshot_id = Some("e3b9d7a1-58c2-4f6e-9d04-6a1c7b2f8e53".into());
        safe.snapshot_amount = Some("-128.5".into());
        safe.snapshot_memo = Some("午餐 AA".into());
        safe.snapshot_asset_id = Some("4d8c508b-91c5-375b-92b0-ee702ed2dac5".into());
        safe.snapshot_asset_symbol = Some("USDT".into());
        let mut inscription = row(person(1), "SYSTEM_SAFE_INSCRIPTION", "");
        inscription.snapshot_id = Some("b2f6a9c3-7e14-4d58-8a3b-0c5e9f1d7a26".into());
        inscription.inscription_name = Some("Mixin Pass".into());
        inscription.inscription_sequence = Some(1024);
        inscription.inscription_content_type = Some("image/jpeg".into());
        inscription.inscription_content_url = Some(PHOTO.into());
        timeline(vec![transfer, safe, inscription])
    }

    /// Notices, recalls, and rows the client cannot show.
    pub(in crate::preview) fn system_rows() -> Vec<MessageListView> {
        let mut added = row(person(0), "SYSTEM_CONVERSATION", "");
        added.action = Some("ADD".into());
        added.participant_id = Some(USER_ID.into());
        added.participant_full_name = Some(NAME.into());
        let pin = row(
            person(1),
            "MESSAGE_PIN",
            &json!({"message_id": id(100), "category": "SIGNAL_TEXT", "content": "明天的发布清单在这里"})
                .to_string(),
        );
        let mut waiting = text(person(3), "");
        waiting.status = "FAILED".into();
        timeline(vec![
            row(person(0), "SECRET", ""),
            added,
            pin,
            row(person(1), "MESSAGE_RECALL", ""),
            row(me(), "MESSAGE_RECALL", ""),
            row(person(2), "SIGNAL_UNKNOWN_KIND", ""),
            waiting,
            row(person(2), "STRANGER", ""),
        ])
    }
    /// Rows in the stress scene's history.
    pub(in crate::preview) const STRESS_ROWS: usize = 10_000;
    /// How far back, in rows, the newest message quotes.
    pub(in crate::preview) const STRESS_FAR_QUOTE: usize = 9_000;

    /// A long history, newest first: every sixth row a long text of several
    /// paragraphs, every fortieth a reply to the row twenty earlier, and the
    /// newest a reply to the row `STRESS_FAR_QUOTE` back.
    pub(in crate::preview) fn stress_rows() -> Vec<MessageListView> {
        let mut rows: Vec<MessageListView> = Vec::with_capacity(STRESS_ROWS);
        for index in 0..STRESS_ROWS {
            let sender = if index % 5 == 0 {
                me()
            } else {
                person(index % 4)
            };
            let content = if index % 6 == 0 {
                long_text(index)
            } else {
                format!("第 {index} 条消息 · message {index}")
            };
            let mut row = text(sender, &content);
            row.message_id = id(index);
            row.created_at_micros = START_MICROS + index as i64 * 60_000_000;
            let quoted = if index == STRESS_ROWS - 1 {
                Some(index - STRESS_FAR_QUOTE)
            } else {
                (index % 40 == 39).then(|| index - 20)
            };
            if let Some(quoted) = quoted {
                row = reply(row, &rows[quoted]);
            }
            rows.push(row);
        }
        rows.reverse();
        rows
    }

    fn long_text(index: usize) -> String {
        let paragraph = "这是一段很长的消息，用来检查长文本在列表里的测量、换行和滚动。\
             It mixes Chinese and English, a link https://mixin.one/messages and \
             enough words to wrap across many lines in a narrow window.";
        let paragraphs = 3 + index % 5;
        let mut text = format!("#{index} ");
        for at in 0..paragraphs {
            if at > 0 {
                text.push_str("\n\n");
            }
            text.push_str(paragraph);
        }
        text
    }
}

/// Conversation rows for the conversation list stress scene.
pub(super) mod conversations {
    use mixin_desktop_api::ConversationListData;

    use super::{PEOPLE, USER_ID};

    pub(in crate::preview) const STRESS_ROWS: usize = 10_000;

    /// Newest first: groups and contacts, every fifth unread, every seventh
    /// with a long last message, every ninety-seventh pinned.
    pub(in crate::preview) fn stress_rows() -> Vec<ConversationListData> {
        (0..STRESS_ROWS)
            .map(|index| {
                let group = index % 3 == 0;
                let (person_id, person_name) = PEOPLE[index % PEOPLE.len()];
                let last_message = if index % 7 == 0 {
                    format!(
                        "第 {index} 个会话的最后一条消息很长，需要在一行里截断显示 \
                         and keep its row height steady while scrolling"
                    )
                } else {
                    format!("message {index}")
                };
                ConversationListData {
                    conversation_id: format!("00000000-0000-4000-9000-{index:012}"),
                    owner_id: person_id.into(),
                    name: if group {
                        format!("讨论组 {index}")
                    } else {
                        format!("{person_name} {index}")
                    },
                    avatar_url: String::new(),
                    category: if group { "GROUP" } else { "CONTACT" }.into(),
                    draft: String::new(),
                    status: 2,
                    last_read_message_id: None,
                    last_message,
                    last_message_category: Some("SIGNAL_TEXT".into()),
                    last_message_status: Some("READ".into()),
                    last_message_sender_id: Some(if index % 4 == 0 {
                        USER_ID.into()
                    } else {
                        person_id.into()
                    }),
                    last_message_sender_name: Some(person_name.into()),
                    last_message_action: None,
                    last_message_participant_id: None,
                    last_message_participant_name: None,
                    // 2026-11-02 09:00 UTC, a minute apart going back.
                    updated_at_millis: 1_793_610_000_000 - index as i64 * 60_000,
                    unseen_count: if index % 5 == 0 {
                        (index % 120) as i64 + 1
                    } else {
                        0
                    },
                    mention_count: i64::from(index % 50 == 0),
                    is_muted: index % 11 == 0,
                    is_verified: false,
                    is_scam: false,
                    is_bot: false,
                    is_bot_group: false,
                    membership: None,
                    is_pinned: index % 97 == 0,
                    pin_time_millis: 0,
                    relationship: if group {
                        String::new()
                    } else {
                        "FRIEND".into()
                    },
                    identity_number: String::new(),
                    circle_ids: Vec::new(),
                    participant_count: if group { 12 } else { 0 },
                    group_avatars: Vec::new(),
                    expire_in: 0,
                }
            })
            .collect()
    }
}

pub(super) fn sticker_albums() -> Vec<mixin_desktop_api::StickerAlbumItem> {
    (0..12)
        .map(|index| mixin_desktop_api::StickerAlbumItem {
            album_id: format!("9d508cd6-1405-4bb7-a529-{index:012}"),
            name: format!("Sticker album {}", index + 1),
            icon_url: sticker_image(index),
            category: "SYSTEM".into(),
            description: String::new(),
            banner: None,
            added: true,
            is_verified: false,
        })
        .collect()
}

pub(super) fn sticker_image(index: usize) -> String {
    format!(
        "{}/assets/{}",
        env!("CARGO_MANIFEST_DIR"),
        [
            "logo.png",
            "badges/plan_basic.png",
            "badges/plan_standard.png"
        ][index % 3]
    )
}

pub(super) fn sticker_store_albums() -> Vec<mixin_desktop_api::StickerAlbumItem> {
    const ALBUMS: [(&str, &str); 6] = [
        ("猫咪的日常", "打招呼、撒娇和发呆，让猫咪替你表达心情。"),
        (
            "今天也要开心：工作与生活中的每一个小表情",
            "从周一早安到周末快乐，一套可以陪你聊天、工作和放松的日常贴纸。",
        ),
        ("摸鱼俱乐部", "忙里偷闲，快乐加倍。"),
        ("小小心意", "把感谢与关心送给朋友。"),
        ("周末计划", "出发、吃饭、看电影，还有好好休息。"),
        (
            "Hello, friends!",
            "A small collection for everyday conversations.",
        ),
    ];
    sticker_albums()
        .into_iter()
        .zip(ALBUMS)
        .enumerate()
        .map(|(index, (mut album, (name, description)))| {
            album.added = index < 3;
            album.name = name.into();
            album.description = description.into();
            album
        })
        .collect()
}

/// Contacts shared by creation and recipient review scenes.
pub(super) fn users() -> Vec<mixin_desktop_api::UserProfileItem> {
    PEOPLE
        .iter()
        .enumerate()
        .map(|(index, &(id, name))| mixin_desktop_api::UserProfileItem {
            user_id: id.into(),
            identity_number: format!("7000200{index}"),
            full_name: name.into(),
            avatar_url: String::new(),
            biography: "Design and engineering".into(),
            is_verified: false,
            is_deactivated: false,
            is_bot: false,
            relationship: "FRIEND".into(),
            code_url: String::new(),
            membership: None,
        })
        .collect()
}

pub(super) fn participants() -> Vec<mixin_desktop_api::ConversationParticipantItem> {
    let mut users = users();
    users.insert(
        0,
        mixin_desktop_api::UserProfileItem {
            user_id: USER_ID.into(),
            identity_number: IDENTITY_NUMBER.into(),
            full_name: NAME.into(),
            avatar_url: String::new(),
            biography: String::new(),
            is_verified: false,
            is_deactivated: false,
            is_bot: false,
            relationship: "ME".into(),
            code_url: String::new(),
            membership: None,
        },
    );
    users
        .into_iter()
        .enumerate()
        .map(
            |(index, user)| mixin_desktop_api::ConversationParticipantItem {
                user_id: user.user_id,
                role: Some(if index == 0 { "OWNER" } else { "MEMBER" }.into()),
                created_at_millis: 1_793_610_000_000,
                identity_number: user.identity_number,
                full_name: user.full_name,
                avatar_url: user.avatar_url,
                biography: user.biography,
                is_verified: user.is_verified,
                is_bot: user.is_bot,
                relationship: user.relationship,
                membership: None,
            },
        )
        .collect()
}

pub(super) fn photo_attachment() -> mixin_desktop_api::PreparedAttachment {
    let mut photo = mixin_desktop_api::PreparedAttachment::default();
    photo.path = concat!(env!("CARGO_MANIFEST_DIR"), "/preview/fixtures/photo.jpg").into();
    photo.name = "photo.jpg".into();
    photo.mime_type = "image/jpeg".into();
    photo.kind = "IMAGE".into();
    photo.width = Some(1200);
    photo.height = Some(800);
    photo
}

pub(super) fn snapshot() -> mixin_desktop_api::SnapshotDetailItem {
    mixin_desktop_api::SnapshotDetailItem {
        snapshot_id: "ba0ce05a-607f-498e-bfe5-fbbf9583ac35".into(),
        trace_id: None,
        snapshot_type: "transfer".into(),
        asset_id: USER_ID.into(),
        amount: "1".into(),
        created_at_millis: 1_793_610_000_000,
        opponent_id: Some(PEOPLE[0].0.into()),
        opponent_name: Some(PEOPLE[0].1.into()),
        transaction_hash: None,
        sender: Some(NAME.into()),
        receiver: Some(PEOPLE[0].1.into()),
        memo: Some("Design review".into()),
        confirmations: Some(12),
        snapshot_hash: None,
        opening_balance: None,
        closing_balance: None,
        symbol: "MIX".into(),
        asset_name: "Mixin".into(),
        asset_icon_url: String::new(),
        chain_icon_url: String::new(),
        asset_confirmations: 1,
        asset_tag: None,
        current_user_name: NAME.into(),
        is_safe: true,
        price_usd: None,
        fiat_rate: None,
        ticker_price_usd: None,
        deposit_hash: None,
        withdrawal_hash: None,
        withdrawal_receiver: None,
        inscription_hash: Some("preview-inscription".into()),
        inscription_collection_hash: None,
        inscription_sequence: Some(12),
        inscription_content_type: Some("application/pdf".into()),
        inscription_content_url: Some("https://example.com/design-review.pdf".into()),
        inscription_name: Some("Design review".into()),
        inscription_icon_url: None,
    }
}

pub(super) fn picker_users() -> Vec<mixin_desktop_api::UserProfileItem> {
    let mut rows = users();
    for (index, name) in [
        "Eva Lin",
        "周亦然",
        "Daniel Park",
        "Sofia Chen",
        "林子涵",
        "Chris Wang",
        "Mia Johnson",
        "Alex Morgan",
    ]
    .into_iter()
    .enumerate()
    {
        let mut user = rows[0].clone();
        user.user_id = format!("9b73a424-4e30-4896-8ac0-{:012x}", index + 1);
        user.full_name = name.into();
        user.identity_number = format!("700030{index:02}");
        rows.push(user);
    }
    rows
}

pub(super) fn animated_sticker() -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    format!(
        "data:application/json;base64,{}",
        STANDARD.encode(include_bytes!("../../preview/fixtures/moving-sticker.json"))
    )
}

pub(super) fn sticker_detail() -> mixin_desktop_api::StickerDetailItem {
    let album = sticker_store_albums().remove(0);
    let items = (0..12)
        .map(|index| mixin_desktop_api::StickerItem {
            sticker_id: format!("00000000-0000-4000-8000-{:012}", index + 901),
            album_id: Some(album.album_id.clone()),
            name: format!("Sticker {}", index + 1),
            asset_url: if index == 0 {
                animated_sticker()
            } else {
                sticker_image(index)
            },
            asset_width: 256,
            asset_height: 256,
            asset_type: if index == 0 {
                "json".into()
            } else {
                "png".into()
            },
            created_at_millis: 0,
            last_use_at_millis: None,
        })
        .collect::<Vec<_>>();
    mixin_desktop_api::StickerDetailItem {
        sticker: items[1].clone(),
        album: Some(album),
        album_stickers: items,
        is_personal: false,
    }
}
