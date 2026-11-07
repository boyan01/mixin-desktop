//! Localized system-message copy shared by messages and notifications.
use crate::i18n::I18n;

pub(crate) fn actor(i18n: I18n, id: &str, name: &str, current_user_id: &str) -> String {
    if !id.is_empty() && current_user_id == id {
        i18n.tr("You", "你").into()
    } else if name.is_empty() {
        i18n.tr("A group member", "群成员").into()
    } else {
        name.into()
    }
}

pub(crate) fn text(
    i18n: I18n,
    sender: &str,
    participant: &str,
    action: Option<&str>,
    content: &str,
) -> String {
    let (en, zh) = match action {
        Some("JOIN") => ("{participant} joined the group", "{participant}加入了群组"),
        Some("EXIT") => ("{participant} left the group", "{participant}退出了群组"),
        Some("ADD") => (
            "{sender} added {participant}",
            "{sender}添加了{participant}",
        ),
        Some("REMOVE") => (
            "{sender} removed {participant}",
            "{sender}移除了{participant}",
        ),
        Some("CREATE") => ("{sender} created this group", "{sender}创建了群组"),
        Some("ROLE") => ("{participant} became an admin", "{participant}成为了管理员"),
        Some("EXPIRE") => {
            return match content.parse::<i64>() {
                Ok(seconds) if seconds > 0 => {
                    let duration = expire_duration(i18n, seconds);
                    i18n.format(
                        "{sender} set disappearing messages to {duration}",
                        "{sender}将消息自动销毁时间设置为{duration}",
                        &[("sender", sender), ("duration", &duration)],
                    )
                }
                Ok(_) => i18n.format(
                    "{sender} disabled disappearing messages",
                    "{sender}关闭了消息自动销毁",
                    &[("sender", sender)],
                ),
                Err(_) => i18n.format(
                    "{sender} changed disappearing message settings",
                    "{sender}修改了消息自动销毁设置",
                    &[("sender", sender)],
                ),
            };
        }
        _ => {
            return i18n
                .tr("This message type is not supported.", "暂不支持此类消息。")
                .into();
        }
    };
    i18n.format(en, zh, &[("sender", sender), ("participant", participant)])
}

pub(crate) fn expire_duration(i18n: I18n, seconds: i64) -> String {
    let (count, en, zh) = if seconds < 60 {
        (seconds, "{count} seconds", "{count}秒")
    } else if seconds < 3600 {
        (seconds / 60, "{count} minutes", "{count}分钟")
    } else if seconds < 86400 {
        (seconds / 3600, "{count} hours", "{count}小时")
    } else if seconds < 604800 {
        (seconds / 86400, "{count} days", "{count}天")
    } else {
        (seconds / 604800, "{count} weeks", "{count}周")
    };
    let en = if count == 1 {
        match en {
            "{count} seconds" => "{count} second",
            "{count} minutes" => "{count} minute",
            "{count} hours" => "{count} hour",
            "{count} days" => "{count} day",
            _ => "{count} week",
        }
    } else {
        en
    };
    i18n.format(en, zh, &[("count", &count.to_string())])
}
