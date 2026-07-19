use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::error::BoxDynError;
use sqlx::sqlite::{SqliteTypeInfo, SqliteValueRef};
use sqlx::{Decode, Sqlite, Type, TypeInfo, ValueRef};

#[derive(Debug)]
pub(crate) struct DatabaseDateTime(DateTime<Utc>);

#[derive(Debug)]
pub(crate) struct OptionalDatabaseDateTime(Option<DateTime<Utc>>);

impl Type<Sqlite> for DatabaseDateTime {
    fn type_info() -> SqliteTypeInfo {
        <NaiveDateTime as Type<Sqlite>>::type_info()
    }

    fn compatible(ty: &SqliteTypeInfo) -> bool {
        <NaiveDateTime as Type<Sqlite>>::compatible(ty)
    }
}

impl<'r> Decode<'r, Sqlite> for DatabaseDateTime {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        if value.type_info().name() == "INTEGER" {
            let millis = <i64 as Decode<Sqlite>>::decode(value)?;
            let value = DateTime::from_timestamp_millis(millis)
                .ok_or_else(|| format!("invalid database timestamp: {millis}"))?;
            return Ok(Self(value));
        }

        <DateTime<Utc> as Decode<Sqlite>>::decode(value).map(Self)
    }
}

impl From<DatabaseDateTime> for DateTime<Utc> {
    fn from(value: DatabaseDateTime) -> Self {
        value.0
    }
}

impl From<DatabaseDateTime> for NaiveDateTime {
    fn from(value: DatabaseDateTime) -> Self {
        value.0.naive_utc()
    }
}

impl Type<Sqlite> for OptionalDatabaseDateTime {
    fn type_info() -> SqliteTypeInfo {
        DatabaseDateTime::type_info()
    }

    fn compatible(ty: &SqliteTypeInfo) -> bool {
        ty.is_null() || DatabaseDateTime::compatible(ty)
    }
}

impl<'r> Decode<'r, Sqlite> for OptionalDatabaseDateTime {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        if value.is_null() {
            return Ok(Self(None));
        }
        DatabaseDateTime::decode(value).map(|value| Self(Some(value.0)))
    }
}

impl From<OptionalDatabaseDateTime> for Option<DateTime<Utc>> {
    fn from(value: OptionalDatabaseDateTime) -> Self {
        value.0
    }
}
