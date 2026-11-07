use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD},
};
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

#[derive(Clone)]
pub(super) enum Payload {
    Text(String),
    Post(String),
    Image(String),
    Sticker(String),
    Contact(String),
    Card(Value),
}

#[derive(Clone)]
pub(super) enum Recipient {
    User(String),
    Conversation(String),
    Choose,
}

#[derive(Clone)]
pub(super) struct Send {
    pub payload: Payload,
    pub recipient: Recipient,
}

pub(super) enum Target {
    External(Url),
    User(String),
    App(String, Url),
    Code(String),
    Conversation {
        id: String,
        user: Option<String>,
        message: Option<String>,
    },
    Message(String),
    Snapshot(crate::snapshot::Reference),
    Send(Send),
    Unsupported(String),
}

pub(super) fn parse(raw: &str, source: Option<String>) -> Result<Target, ()> {
    let url = Url::parse(raw.trim()).map_err(|_| ())?;
    let mixin = url.scheme() == "mixin"
        || matches!(url.scheme(), "http" | "https")
            && matches!(url.host_str(), Some("mixin.one" | "www.mixin.one"));
    if !mixin {
        return if matches!(url.scheme(), "http" | "https" | "mailto") {
            Ok(Target::External(url))
        } else {
            Err(())
        };
    }
    let path = url
        .path_segments()
        .into_iter()
        .flatten()
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let (kind, values) = if url.scheme() == "mixin"
        && !matches!(url.host_str(), Some("mixin.one" | "www.mixin.one"))
    {
        (url.host_str().unwrap_or_default(), path.as_slice())
    } else {
        (
            path.first().copied().unwrap_or_default(),
            path.get(1..).unwrap_or_default(),
        )
    };
    let valid_shape = match kind {
        "users" | "apps" | "codes" | "messages" | "safe_snapshots" => values.len() == 1,
        "conversations" => (1..=2).contains(&values.len()),
        "snapshots" => values.len() <= 1,
        "send" => values.is_empty(),
        _ => true,
    };
    if !valid_shape {
        return Err(());
    }
    let query = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<HashMap<_, _>>();
    let value = || {
        values
            .first()
            .map(|value| (*value).to_owned())
            .filter(|value| !value.trim().is_empty())
            .ok_or(())
    };
    match kind {
        "users" => Ok(Target::User(value()?)),
        "apps" => {
            let id = value()?;
            if query.get("action").is_some_and(|action| action == "open") {
                Ok(Target::App(id, url))
            } else {
                Ok(Target::User(id))
            }
        }
        "codes" => Ok(Target::Code(value()?)),
        "conversations" => {
            let id = value()?;
            if let Some(start) = query.get("start").filter(|text| !text.trim().is_empty()) {
                return Ok(Target::Send(Send {
                    payload: Payload::Text(start.clone()),
                    recipient: Recipient::Conversation(id),
                }));
            }
            Ok(Target::Conversation {
                id,
                user: query.get("user").cloned(),
                message: query
                    .get("message")
                    .cloned()
                    .or_else(|| values.get(1).map(|value| (*value).to_owned())),
            })
        }
        "messages" => Ok(Target::Message(value()?)),
        "snapshots" => {
            if let Some(trace) = query.get("trace").filter(|value| !value.is_empty()) {
                Ok(Target::Snapshot(crate::snapshot::Reference::Trace(
                    trace.clone(),
                )))
            } else {
                Ok(Target::Snapshot(crate::snapshot::Reference::Snapshot(
                    value()?,
                )))
            }
        }
        "safe_snapshots" => Ok(Target::Snapshot(crate::snapshot::Reference::Safe(value()?))),
        "send" => {
            let category = query.get("category").ok_or(())?.to_lowercase();
            let data = query.get("data").ok_or(())?;
            let data = [STANDARD, URL_SAFE, STANDARD_NO_PAD, URL_SAFE_NO_PAD]
                .into_iter()
                .find_map(|engine| engine.decode(data.as_bytes()).ok())
                .ok_or(())?;
            let data = String::from_utf8(data).map_err(|_| ())?;
            let payload = match category.as_str() {
                "text" if !data.trim().is_empty() => Payload::Text(data),
                "post" if !data.trim().is_empty() => Payload::Post(data),
                "image" => {
                    let value: Value = serde_json::from_str(&data).map_err(|_| ())?;
                    let raw = value.get("url").and_then(Value::as_str).ok_or(())?;
                    let image = Url::parse(raw).map_err(|_| ())?;
                    if !matches!(image.scheme(), "http" | "https") {
                        return Err(());
                    }
                    Payload::Image(image.into())
                }
                "contact" => {
                    let value: Value = serde_json::from_str(&data).map_err(|_| ())?;
                    let id = value
                        .get("user_id")
                        .and_then(Value::as_str)
                        .filter(|id| valid_id(id))
                        .ok_or(())?;
                    Payload::Contact(id.into())
                }
                "sticker" if valid_id(&data) => Payload::Sticker(data),
                "app_card" => {
                    let value: Value = serde_json::from_str(&data).map_err(|_| ())?;
                    if !value.get("title").is_some_and(Value::is_string)
                        || !value.get("description").is_some_and(Value::is_string)
                    {
                        return Err(());
                    }
                    Payload::Card(value)
                }
                _ => return Err(()),
            };
            let recipient = if let Some(user) = query.get("user").filter(|value| !value.is_empty())
            {
                if !valid_id(user) {
                    return Err(());
                }
                Recipient::User(user.clone())
            } else if let Some(id) = query.get("conversation").filter(|value| !value.is_empty()) {
                Recipient::Conversation(id.clone())
            } else if matches!(payload, Payload::Text(_)) {
                source
                    .map(Recipient::Conversation)
                    .unwrap_or(Recipient::Choose)
            } else {
                Recipient::Choose
            };
            Ok(Target::Send(Send { payload, recipient }))
        }
        _ => Ok(Target::Unsupported(url.into())),
    }
}

fn valid_id(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "11111111-1111-1111-1111-111111111111";
    const USER: &str = "22222222-2222-2222-2222-222222222222";
    const DESTINATION: &str = "33333333-3333-3333-3333-333333333333";

    fn send_url(category: &str, data: &str) -> Url {
        let mut url = Url::parse("mixin://send").unwrap();
        url.query_pairs_mut()
            .append_pair("category", category)
            .append_pair("data", &STANDARD.encode(data));
        url
    }

    #[test]
    fn unknown_mixin_actions_require_phone_handoff_instead_of_external_launch() {
        for raw in [
            "mixin://pay?amount=1",
            "mixin://unknown/value",
            "https://mixin.one/multisigs/value",
            "https://www.mixin.one/markets",
        ] {
            assert!(
                matches!(parse(raw, None), Ok(Target::Unsupported(_))),
                "{raw}"
            );
        }
    }

    #[test]
    fn external_links_allow_web_and_mailto_but_reject_local_or_executable_schemes() {
        for raw in [
            "https://example.com/",
            "http://example.com/",
            "mailto:recipient@example.com",
        ] {
            assert!(matches!(parse(raw, None), Ok(Target::External(_))), "{raw}");
        }
        for raw in [
            "file:///tmp/private",
            "file://mixin.one/users/22222222-2222-2222-2222-222222222222",
            "javascript://mixin.one/users/22222222-2222-2222-2222-222222222222",
            "javascript:alert(1)",
            "data:text/plain,private",
            "custom://example",
        ] {
            assert!(parse(raw, None).is_err(), "{raw}");
        }
    }

    #[test]
    fn text_send_without_destination_retains_the_original_source_conversation() {
        let raw = send_url("text", "A message from the source chat");
        let Ok(Target::Send(plan)) = parse(raw.as_str(), Some(SOURCE.into())) else {
            panic!("expected send preview");
        };
        assert!(matches!(plan.recipient, Recipient::Conversation(id) if id == SOURCE));
        assert!(
            matches!(plan.payload, Payload::Text(text) if text == "A message from the source chat")
        );
    }

    #[test]
    fn non_text_send_without_destination_requires_an_explicit_recipient() {
        for (category, data) in [
            ("post", "Post body"),
            ("image", r#"{"url":"https://example.com/image.png"}"#),
            ("sticker", USER),
        ] {
            let raw = send_url(category, data);
            let Ok(Target::Send(plan)) = parse(raw.as_str(), Some(SOURCE.into())) else {
                panic!("expected send preview");
            };
            assert!(matches!(plan.recipient, Recipient::Choose), "{category}");
        }
    }

    #[test]
    fn send_user_destination_takes_precedence_over_conversation_and_source() {
        let mut raw = send_url("text", "Message");
        raw.query_pairs_mut()
            .append_pair("user", USER)
            .append_pair("conversation", DESTINATION);
        let Ok(Target::Send(plan)) = parse(raw.as_str(), Some(SOURCE.into())) else {
            panic!("expected send preview");
        };
        assert!(matches!(plan.recipient, Recipient::User(id) if id == USER));
    }

    #[test]
    fn invalid_send_data_is_rejected_before_a_preview_can_be_confirmed() {
        for (category, data) in [
            ("unknown", "value"),
            ("image", "not-json"),
            ("image", r#"{"url":"file:///tmp/private"}"#),
            ("contact", r#"{"user_id":"invalid"}"#),
            ("sticker", "not-a-sticker-id"),
            ("app_card", r#"{"title":"Missing description"}"#),
        ] {
            assert!(
                parse(send_url(category, data).as_str(), None).is_err(),
                "{category}"
            );
        }
        assert!(parse("mixin://send?category=text&data=%25%25%25", None).is_err());
    }

    #[test]
    fn malformed_resource_paths_are_rejected_instead_of_opening_a_different_object() {
        for raw in [
            "mixin://users",
            "mixin://users/one/two",
            "https://mixin.one/codes",
            "mixin://send/unexpected?category=text&data=dGV4dA==",
        ] {
            assert!(parse(raw, None).is_err(), "{raw}");
        }
    }

    #[test]
    fn conversation_user_hint_is_retained_for_business_identity_validation() {
        let raw = format!("mixin://conversations/{DESTINATION}?user={USER}");
        let Ok(Target::Conversation { id, user, .. }) = parse(&raw, None) else {
            panic!("expected conversation resolution");
        };
        assert_eq!(id, DESTINATION);
        assert_eq!(user.as_deref(), Some(USER));
    }
    #[test]
    fn valid_contact_and_card_links_preserve_their_preview_payloads() {
        let contact = format!(r#"{{"user_id":"{USER}"}}"#);
        let Ok(Target::Send(plan)) = parse(send_url("contact", &contact).as_str(), None) else {
            panic!("expected contact preview");
        };
        assert!(matches!(plan.payload, Payload::Contact(id) if id == USER));
        let card = r#"{"title":"A shared card","description":"Card description","action":"https://example.com/"}"#;
        let Ok(Target::Send(plan)) = parse(send_url("app_card", card).as_str(), None) else {
            panic!("expected card preview");
        };
        let Payload::Card(value) = plan.payload else {
            panic!("expected card payload");
        };
        assert_eq!(value["title"], "A shared card");
        assert_eq!(value["action"], "https://example.com/");
    }
}
