use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};
use sqlx::Error;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Value {
    Null,
    Integer(#[serde(with = "integer_text")] i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}
pub fn decode_error(message: impl Into<String>) -> Error {
    Error::Decode(message.into().into())
}
pub trait IntoValue {
    fn into_value(self) -> Value;
}
pub trait FromValue: Sized {
    fn from_value(value: &Value) -> Result<Self, Error>;
}
impl<T: IntoValue + Clone> IntoValue for &T {
    fn into_value(self) -> Value {
        self.clone().into_value()
    }
}
impl<T: IntoValue> IntoValue for Option<T> {
    fn into_value(self) -> Value {
        self.map(IntoValue::into_value).unwrap_or(Value::Null)
    }
}
impl<T: FromValue> FromValue for Option<T> {
    fn from_value(v: &Value) -> Result<Self, Error> {
        if matches!(v, Value::Null) {
            Ok(None)
        } else {
            T::from_value(v).map(Some)
        }
    }
}
impl IntoValue for &str {
    fn into_value(self) -> Value {
        Value::Text(self.to_owned())
    }
}
impl IntoValue for String {
    fn into_value(self) -> Value {
        Value::Text(self)
    }
}
impl FromValue for String {
    fn from_value(v: &Value) -> Result<Self, Error> {
        match v {
            Value::Text(v) => Ok(v.clone()),
            Value::Null => Ok(String::new()),
            _ => Err(decode_error(format!("expected TEXT, got {v:?}"))),
        }
    }
}
impl IntoValue for Vec<u8> {
    fn into_value(self) -> Value {
        Value::Blob(self)
    }
}
impl FromValue for Vec<u8> {
    fn from_value(v: &Value) -> Result<Self, Error> {
        match v {
            Value::Blob(v) => Ok(v.clone()),
            Value::Text(v) => Ok(v.as_bytes().to_vec()),
            Value::Null => Ok(Vec::new()),
            _ => Err(decode_error("expected BLOB")),
        }
    }
}
impl IntoValue for i64 {
    fn into_value(self) -> Value {
        Value::Integer(self)
    }
}
impl FromValue for i64 {
    fn from_value(v: &Value) -> Result<Self, Error> {
        match v {
            Value::Integer(v) => Ok(*v),
            Value::Null => Ok(0),
            _ => Err(decode_error("expected INTEGER")),
        }
    }
}
impl IntoValue for f64 {
    fn into_value(self) -> Value {
        Value::Real(self)
    }
}
impl FromValue for f64 {
    fn from_value(v: &Value) -> Result<Self, Error> {
        match v {
            Value::Real(v) => Ok(*v),
            Value::Null => Ok(0.0),
            Value::Integer(v) => Ok(*v as f64),
            _ => Err(decode_error("expected REAL")),
        }
    }
}
macro_rules! integer {
    ($ty:ty) => {
        impl IntoValue for $ty {
            fn into_value(self) -> Value {
                Value::Integer(i64::from(self))
            }
        }
        impl FromValue for $ty {
            fn from_value(v: &Value) -> Result<Self, Error> {
                Self::try_from(i64::from_value(v)?).map_err(|e| decode_error(e.to_string()))
            }
        }
    };
}
integer!(i32);
integer!(u32);
impl IntoValue for bool {
    fn into_value(self) -> Value {
        Value::Integer(i64::from(self))
    }
}
impl FromValue for bool {
    fn from_value(v: &Value) -> Result<Self, Error> {
        Ok(i64::from_value(v)? != 0)
    }
}
impl IntoValue for DateTime<Utc> {
    fn into_value(self) -> Value {
        Value::Text(self.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, false))
    }
}
impl IntoValue for NaiveDateTime {
    fn into_value(self) -> Value {
        Value::Text(self.format("%F %T%.f").to_string())
    }
}
fn datetime(v: &Value) -> Result<DateTime<FixedOffset>, Error> {
    let utc = |dt: DateTime<Utc>| dt.fixed_offset();
    match v {
        // Plain SQLx datetime columns use seconds. Legacy millisecond columns
        // are decoded explicitly by DatabaseDateTime.
        Value::Integer(seconds) => DateTime::from_timestamp(*seconds, 0)
            .map(utc)
            .ok_or_else(|| decode_error("timestamp out of range")),
        Value::Real(days) => {
            let timestamp = (days - 2_440_587.5) * 86400.0;
            if !timestamp.is_finite() {
                return Err(decode_error("invalid Julian timestamp"));
            }
            DateTime::from_timestamp(
                timestamp.trunc() as i64,
                (timestamp.fract().abs() * 1e9) as u32,
            )
            .map(utc)
            .ok_or_else(|| decode_error("timestamp out of range"))
        }
        Value::Text(s) => {
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Ok(dt);
            }
            for format in [
                "%F %T%.f",
                "%F %R",
                "%F %RZ",
                "%F %R%:z",
                "%F %T%.fZ",
                "%F %T%.f%:z",
                "%FT%R",
                "%FT%RZ",
                "%FT%R%:z",
                "%FT%T%.f",
                "%FT%T%.fZ",
                "%FT%T%.f%:z",
            ] {
                if let Ok(dt) = DateTime::parse_from_str(s, format) {
                    return Ok(dt);
                }
                if let Ok(dt) = NaiveDateTime::parse_from_str(s, format) {
                    return Ok(utc(dt.and_utc()));
                }
            }
            Err(decode_error(format!("invalid datetime: {s}")))
        }
        _ => Err(decode_error("expected timestamp")),
    }
}
impl FromValue for DateTime<Utc> {
    fn from_value(v: &Value) -> Result<Self, Error> {
        Ok(datetime(v)?.with_timezone(&Utc))
    }
}
impl FromValue for NaiveDateTime {
    fn from_value(v: &Value) -> Result<Self, Error> {
        Ok(datetime(v)?.naive_local())
    }
}

macro_rules! labels {($ty:ty,{$($variant:path=>$label:literal),+})=>{
    impl IntoValue for $ty {fn into_value(self)->Value {Value::Text(match self {$($variant=>$label),+}.to_owned())}}
    impl FromValue for $ty {fn from_value(v:&Value)->Result<Self,Error> {match String::from_value(v)?.as_str() {$($label=>Ok($variant)),+,other=>Err(decode_error(format!("invalid database enum: {other}")))}}}
};}
labels!(sdk::MessageStatus,{sdk::MessageStatus::Failed=>"FAILED",sdk::MessageStatus::Unknown=>"UNKNOWN",sdk::MessageStatus::Sending=>"SENDING",sdk::MessageStatus::Sent=>"SENT",sdk::MessageStatus::Delivered=>"DELIVERED",sdk::MessageStatus::Read=>"READ"});
labels!(sdk::ConversationCategory,{sdk::ConversationCategory::Group=>"GROUP",sdk::ConversationCategory::Contact=>"CONTACT"});
labels!(sdk::UserRelationship,{sdk::UserRelationship::Friend=>"FRIEND",sdk::UserRelationship::Me=>"ME",sdk::UserRelationship::Stranger=>"STRANGER",sdk::UserRelationship::Blocking=>"BLOCKING"});
use crate::db::mixin::{conversation::ConversationStatus, message::MediaStatus};
impl IntoValue for ConversationStatus {
    fn into_value(self) -> Value {
        self.0.into_value()
    }
}
impl FromValue for ConversationStatus {
    fn from_value(v: &Value) -> Result<Self, Error> {
        Ok(Self(i32::from_value(v)?))
    }
}
impl IntoValue for MediaStatus {
    fn into_value(self) -> Value {
        Value::Text(
            match self {
                Self::Pending => "PENDING",
                Self::Done => "DONE",
                Self::Canceled => "CANCELED",
                Self::Expired => "EXPIRED",
                Self::Read => "READ",
            }
            .to_owned(),
        )
    }
}
impl FromValue for MediaStatus {
    fn from_value(v: &Value) -> Result<Self, Error> {
        match Option::<String>::from_value(v)?.as_deref() {
            Some("PENDING") => Ok(Self::Pending),
            Some("DONE") => Ok(Self::Done),
            Some("EXPIRED") => Ok(Self::Expired),
            Some("READ") => Ok(Self::Read),
            None | Some("") | Some("CANCELED") => Ok(Self::Canceled),
            Some(s) => Err(decode_error(format!("invalid media status: {s}"))),
        }
    }
}
impl FromValue for sdk::api::token_api::SafeWithdrawal {
    fn from_value(v: &Value) -> Result<Self, Error> {
        serde_json::from_str(&String::from_value(v)?).map_err(|e| decode_error(e.to_string()))
    }
}
impl FromValue for sdk::api::token_api::SafeDeposit {
    fn from_value(v: &Value) -> Result<Self, Error> {
        serde_json::from_str(&String::from_value(v)?).map_err(|e| decode_error(e.to_string()))
    }
}

mod integer_text {
    pub fn serialize<S: serde::Serializer>(v: &i64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_string())
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
        use serde::Deserialize;
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::datetime::DatabaseDateTime;

    #[tokio::test]
    async fn null_columns_preserve_sqlx_defaults_and_optional_absence() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        let expected: (i64, f64, bool, String, Vec<u8>, Option<i64>) =
            sqlx::query_as("SELECT NULL, NULL, NULL, NULL, NULL, NULL")
                .fetch_one(&pool)
                .await
                .unwrap();
        let actual = (
            i64::from_value(&Value::Null).unwrap(),
            f64::from_value(&Value::Null).unwrap(),
            bool::from_value(&Value::Null).unwrap(),
            String::from_value(&Value::Null).unwrap(),
            Vec::<u8>::from_value(&Value::Null).unwrap(),
            Option::<i64>::from_value(&Value::Null).unwrap(),
        );
        assert_eq!(actual, expected);
        pool.close().await;
    }

    #[tokio::test]
    async fn shared_datetime_mapping_preserves_sqlx_formats_and_legacy_milliseconds() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        for text in [
            "2023-11-14 12:30",
            "2023-11-14T12:30Z",
            "2023-11-14 12:30:00.123+08:00",
            "2023-11-14T12:30:00.123Z",
        ] {
            let expected: DateTime<Utc> = sqlx::query_scalar("SELECT ?")
                .bind(text)
                .fetch_one(&pool)
                .await
                .unwrap();
            let actual = DateTime::<Utc>::from_value(&Value::Text(text.into())).unwrap();
            assert_eq!(actual, expected);
            let expected: NaiveDateTime = sqlx::query_scalar("SELECT ?")
                .bind(text)
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(
                NaiveDateTime::from_value(&Value::Text(text.into())).unwrap(),
                expected
            );
        }
        let expected = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        assert_eq!(
            DateTime::<Utc>::from_value(&Value::Integer(1_700_000_000)).unwrap(),
            expected
        );
        let legacy: DateTime<Utc> =
            DatabaseDateTime::from_value(&Value::Integer(1_700_000_000_000))
                .unwrap()
                .into();
        assert_eq!(legacy, expected);
        let encoded: String = sqlx::query_scalar("SELECT ?")
            .bind(expected)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(expected.into_value(), Value::Text(encoded));
        pool.close().await;
    }
}
