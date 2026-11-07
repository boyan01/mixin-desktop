//! Shared membership validity used by native presentation clients.
use chrono::{DateTime, Utc};

pub fn active_membership_plan(raw: Option<&str>) -> Option<&'static str> {
    active_plan_at(raw?, Utc::now())
}

fn active_plan_at(raw: &str, now: DateTime<Utc>) -> Option<&'static str> {
    let membership: sdk::api::user_api::Membership = serde_json::from_str(raw).ok()?;
    if membership.expired_at <= now {
        return None;
    }
    match membership.plan.as_str() {
        "BASIC" => Some("BASIC"),
        "STANDARD" => Some("STANDARD"),
        "PREMIUM" => Some("PREMIUM"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_supported_unexpired_memberships_receive_a_plan_badge() {
        let now = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        for (plan, expiry, expected) in [
            ("BASIC", "2027-01-01T00:00:00Z", Some("BASIC")),
            ("STANDARD", "2027-01-01T00:00:00Z", Some("STANDARD")),
            ("PREMIUM", "2027-01-01T00:00:00Z", Some("PREMIUM")),
            ("NONE", "2027-01-01T00:00:00Z", None),
            ("FUTURE", "2027-01-01T00:00:00Z", None),
            ("PREMIUM", "2026-01-01T00:00:00Z", None),
            ("PREMIUM", "2025-01-01T00:00:00Z", None),
        ] {
            let raw = serde_json::json!({"plan":plan,"expired_at":expiry}).to_string();
            assert_eq!(active_plan_at(&raw, now), expected);
        }
        assert_eq!(active_plan_at("invalid", now), None);
    }
}
