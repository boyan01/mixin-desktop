use super::{Executor, ResultSet, Row, Value};
use futures::TryStreamExt;
use sqlx::{Column as _, Error, Row as _, TypeInfo as _, ValueRef as _};

async fn run<'e>(
    executor: impl sqlx::Executor<'e, Database = sqlx::Sqlite>,
    sql: String,
    values: Vec<Value>,
    script: bool,
) -> Result<ResultSet, Error> {
    let mut result = ResultSet::default();
    if script {
        let done = sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
            .execute(executor)
            .await?;
        result.changes = done.rows_affected();
        result.rowid = done.last_insert_rowid();
        return Ok(result);
    }
    let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
    for value in values {
        query = match value {
            Value::Null => query.bind(Option::<String>::None),
            Value::Integer(v) => query.bind(v),
            Value::Real(v) => query.bind(v),
            Value::Text(v) => query.bind(v),
            Value::Blob(v) => query.bind(v),
        };
    }
    let mut stream = sqlx::Executor::fetch_many(executor, query);
    while let Some(step) = stream.try_next().await? {
        match step {
            sqlx::Either::Left(done) => {
                result.changes += done.rows_affected();
                result.rowid = done.last_insert_rowid();
            }
            sqlx::Either::Right(row) => {
                let mut values = Vec::with_capacity(row.len());
                for index in 0..row.len() {
                    let raw = row.try_get_raw(index)?;
                    values.push(if raw.is_null() {
                        Value::Null
                    } else {
                        match raw.type_info().name() {
                            "INTEGER" | "BOOLEAN" => Value::Integer(row.try_get(index)?),
                            "REAL" => Value::Real(row.try_get(index)?),
                            "BLOB" => Value::Blob(row.try_get(index)?),
                            _ => Value::Text(row.try_get(index)?),
                        }
                    });
                }
                result.rows.push(Row {
                    columns: row.columns().iter().map(|c| c.name().to_owned()).collect(),
                    values,
                });
            }
        }
    }
    Ok(result)
}
impl Executor for &sqlx::SqlitePool {
    async fn run(self, sql: String, values: Vec<Value>, script: bool) -> Result<ResultSet, Error> {
        run(self, sql, values, script).await
    }
}
impl Executor for &mut sqlx::SqliteConnection {
    async fn run(self, sql: String, values: Vec<Value>, script: bool) -> Result<ResultSet, Error> {
        run(self, sql, values, script).await
    }
}
impl Executor for &mut sqlx::pool::PoolConnection<sqlx::Sqlite> {
    async fn run(self, sql: String, values: Vec<Value>, script: bool) -> Result<ResultSet, Error> {
        run(&mut **self, sql, values, script).await
    }
}
