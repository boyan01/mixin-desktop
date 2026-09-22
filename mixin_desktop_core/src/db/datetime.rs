use crate::db::storage::{FromValue, Value};
use chrono::{DateTime, NaiveDateTime, Utc};
#[derive(Debug)]
pub(crate) struct DatabaseDateTime(DateTime<Utc>);
#[derive(Debug)]
pub(crate) struct OptionalDatabaseDateTime(Option<DateTime<Utc>>);
impl FromValue for DatabaseDateTime {
    fn from_value(v: &Value) -> Result<Self, sqlx::Error> {
        if let Value::Integer(millis) = v {
            return DateTime::from_timestamp_millis(*millis)
                .map(Self)
                .ok_or_else(|| sqlx::Error::Decode("invalid database timestamp".into()));
        }
        Ok(Self(DateTime::<Utc>::from_value(v)?))
    }
}
impl FromValue for OptionalDatabaseDateTime {
    fn from_value(v: &Value) -> Result<Self, sqlx::Error> {
        if matches!(v, Value::Null) {
            return Ok(Self(None));
        }
        Ok(Self(Some(DatabaseDateTime::from_value(v)?.0)))
    }
}
impl From<DatabaseDateTime> for DateTime<Utc> {
    fn from(v: DatabaseDateTime) -> Self {
        v.0
    }
}
impl From<DatabaseDateTime> for NaiveDateTime {
    fn from(v: DatabaseDateTime) -> Self {
        v.0.naive_utc()
    }
}
impl From<OptionalDatabaseDateTime> for Option<DateTime<Utc>> {
    fn from(v: OptionalDatabaseDateTime) -> Self {
        v.0
    }
}
