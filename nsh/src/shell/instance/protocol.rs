//! Versioned launch intents, validated before any request enters the UI queue.
use std::io;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use super::super::{Event, NotificationTarget, accepted_url};

pub(super) const MAX_PAYLOAD: usize = 1024 * 1024;
const MAX_INTENTS: usize = 16;

pub(super) fn encode(events: &[Event]) -> io::Result<Vec<u8>> {
    let events = if events.is_empty() {
        vec![Event::Activate]
    } else {
        events.to_vec()
    };
    validate(&events)?;
    let values = events
        .iter()
        .map(|event| match event {
            Event::Activate => json!({"kind":"activate"}),
            Event::Open(url) => json!({"kind":"open", "url":url}),
            Event::Notification(target) => json!({
                "kind":"notification", "account_id":target.account_id,
                "conversation_id":target.conversation_id, "message_id":target.message_id,
            }),
        })
        .collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&json!({"version":1, "events":values}))?;
    if bytes.len() > MAX_PAYLOAD {
        return Err(invalid("The launch request is too large"));
    }
    Ok(bytes)
}

pub(super) fn decode(bytes: &[u8]) -> io::Result<Vec<Event>> {
    if bytes.len() > MAX_PAYLOAD {
        return Err(invalid("The launch request is too large"));
    }
    let value: Value = serde_json::from_slice(bytes)?;
    let object = value
        .as_object()
        .ok_or_else(|| invalid("Invalid launch request"))?;
    if object.len() != 2 || value["version"].as_u64() != Some(1) {
        return Err(invalid("Unsupported launch protocol"));
    }
    let rows = value["events"]
        .as_array()
        .filter(|rows| !rows.is_empty() && rows.len() <= MAX_INTENTS)
        .ok_or_else(|| invalid("Invalid launch intent count"))?;
    let mut events = Vec::with_capacity(rows.len());
    for row in rows {
        let object = row
            .as_object()
            .ok_or_else(|| invalid("Invalid launch intent"))?;
        let string = |key: &str| {
            row[key]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| invalid("Invalid launch intent"))
        };
        events.push(match row["kind"].as_str() {
            Some("activate") if object.len() == 1 => Event::Activate,
            Some("open") if object.len() == 2 => Event::Open(string("url")?),
            Some("notification") if object.len() == 4 => Event::Notification(NotificationTarget {
                account_id: string("account_id")?,
                conversation_id: string("conversation_id")?,
                message_id: string("message_id")?,
            }),
            _ => return Err(invalid("Unknown launch intent")),
        });
    }
    validate(&events)?;
    Ok(events)
}

fn validate(events: &[Event]) -> io::Result<()> {
    if events.is_empty() || events.len() > MAX_INTENTS {
        return Err(invalid("Invalid launch intent count"));
    }
    if events.iter().any(|event| match event {
        Event::Activate => false,
        Event::Open(url) => !accepted_url(url),
        Event::Notification(target) => !target.valid(),
    }) {
        return Err(invalid("Invalid launch target"));
    }
    Ok(())
}

pub(super) fn enqueue(events: Vec<Event>, sender: &mpsc::Sender<Event>) -> io::Result<()> {
    validate(&events)?;
    // One request either reserves every target or delivers none of them.
    let permits = sender
        .try_reserve_many(events.len())
        .map_err(|_| io::Error::other("The pending launch queue is full"))?;
    for (permit, event) in permits.zip(events) {
        permit.send(event);
    }
    Ok(())
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
