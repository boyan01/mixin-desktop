//! The SQL operations used by the DAOs. Platform backends own connections;
//! statements, bound values and row mapping are shared.
pub(crate) mod files;
use sqlx::{Error, SqlSafeStr};
use std::marker::PhantomData;

mod value;
pub use value::{FromValue, IntoValue, Value};
#[cfg(target_arch = "wasm32")]
pub mod browser;
#[cfg(not(target_arch = "wasm32"))]
mod native;

#[derive(Debug, Default)]
pub struct ResultSet {
    pub rows: Vec<Row>,
    pub changes: u64,
    pub rowid: i64,
}
impl ResultSet {
    pub fn rows_affected(&self) -> u64 {
        self.changes
    }
    pub fn last_insert_rowid(&self) -> i64 {
        self.rowid
    }
}
#[derive(Debug)]
pub struct Row {
    pub columns: Vec<String>,
    pub values: Vec<Value>,
}
pub trait ColumnIndex {
    fn index(&self, row: &Row) -> Result<usize, Error>;
}
impl ColumnIndex for usize {
    fn index(&self, _: &Row) -> Result<usize, Error> {
        Ok(*self)
    }
}
impl ColumnIndex for &str {
    fn index(&self, row: &Row) -> Result<usize, Error> {
        row.columns
            .iter()
            .position(|name| name == self)
            .ok_or_else(|| Error::ColumnNotFound(self.to_string()))
    }
}
impl Row {
    pub fn try_get<T: FromValue, I: ColumnIndex>(&self, index: I) -> Result<T, Error> {
        let index = index.index(self)?;
        T::from_value(
            self.values
                .get(index)
                .ok_or(Error::ColumnIndexOutOfBounds {
                    index,
                    len: self.values.len(),
                })?,
        )
        .map_err(|error| Error::ColumnDecode {
            index: self.columns[index].clone(),
            source: Box::new(error),
        })
    }
    pub fn get<T: FromValue, I: ColumnIndex>(&self, index: I) -> T {
        self.try_get(index).expect("database column type")
    }
}
pub trait FromRow: Sized {
    fn from_row(row: &Row) -> Result<Self, Error>;
}
impl FromRow for Row {
    fn from_row(row: &Row) -> Result<Self, Error> {
        Ok(Self {
            columns: row.columns.clone(),
            values: row.values.clone(),
        })
    }
}
macro_rules! tuples {
    ($($t:ident:$i:tt),+) => { impl<$($t: FromValue),+> FromRow for ($($t,)+) {
        fn from_row(row: &Row) -> Result<Self, Error> { Ok(($(row.try_get::<$t,_>($i)?,)+)) }
    } };
}
tuples!(A:0);
tuples!(A:0,B:1);
tuples!(A:0,B:1,C:2);
tuples!(A:0,B:1,C:2,D:3);
tuples!(A:0,B:1,C:2,D:3,E:4);

// Explicit mappings live beside their DAO types, including SQL aliases and
// legacy timestamp conversions. This macro only removes repetitive field code.
macro_rules! row_mapping {
    ($ty:ty { $($field:ident : $column:literal $(=> $via:ty)?),* $(,)? }) => {
        impl $crate::db::storage::FromRow for $ty {
            fn from_row(row: &$crate::db::storage::Row) -> Result<Self, sqlx::Error> {
                Ok(Self { $($field: $crate::db::storage::row_mapping!(@get row, $column $(, $via)?)),* })
            }
        }
    };
    (@get $row:ident, $column:literal, $via:ty) => { $row.try_get::<$via,_>($column)?.into() };
    (@get $row:ident, $column:literal) => { $row.try_get($column)? };
}
pub(crate) use row_mapping;

pub trait Executor {
    fn run(
        self,
        sql: String,
        values: Vec<Value>,
        script: bool,
    ) -> impl std::future::Future<Output = Result<ResultSet, Error>> + Send;
}
#[derive(Debug)]
pub struct Query<O = Row> {
    sql: String,
    values: Vec<Value>,
    script: bool,
    output: PhantomData<O>,
}
pub fn query(sql: impl SqlSafeStr) -> Query {
    Query::new(sql, false)
}
pub fn raw_sql(sql: impl SqlSafeStr) -> Query {
    Query::new(sql, true)
}
pub fn query_as<O: FromRow>(sql: impl SqlSafeStr) -> Query<O> {
    Query::new(sql, false)
}
pub struct Scalar<T>(Query<(T,)>);
pub fn query_scalar<T: FromValue>(sql: impl SqlSafeStr) -> Scalar<T> {
    Scalar(Query::new(sql, false))
}
impl<O> Query<O> {
    fn new(sql: impl SqlSafeStr, script: bool) -> Self {
        Self {
            sql: sql.into_sql_str().as_str().to_owned(),
            values: Vec::new(),
            script,
            output: PhantomData,
        }
    }
    pub fn bind(mut self, value: impl IntoValue) -> Self {
        self.values.push(value.into_value());
        self
    }
    pub async fn execute(self, executor: impl Executor) -> Result<ResultSet, Error> {
        executor.run(self.sql, self.values, self.script).await
    }
}
impl<O: FromRow> Query<O> {
    pub async fn fetch_all(self, executor: impl Executor) -> Result<Vec<O>, Error> {
        self.execute(executor)
            .await?
            .rows
            .iter()
            .map(O::from_row)
            .collect()
    }
    pub async fn fetch_optional(self, executor: impl Executor) -> Result<Option<O>, Error> {
        self.execute(executor)
            .await?
            .rows
            .first()
            .map(O::from_row)
            .transpose()
    }
    pub async fn fetch_one(self, executor: impl Executor) -> Result<O, Error> {
        self.fetch_optional(executor)
            .await?
            .ok_or(Error::RowNotFound)
    }
}
impl<T: FromValue> Scalar<T> {
    pub fn bind(self, value: impl IntoValue) -> Self {
        Self(self.0.bind(value))
    }
    pub async fn fetch_all(self, executor: impl Executor) -> Result<Vec<T>, Error> {
        Ok(self
            .0
            .fetch_all(executor)
            .await?
            .into_iter()
            .map(|v| v.0)
            .collect())
    }
    pub async fn fetch_optional(self, executor: impl Executor) -> Result<Option<T>, Error> {
        Ok(self.0.fetch_optional(executor).await?.map(|v| v.0))
    }
    pub async fn fetch_one(self, executor: impl Executor) -> Result<T, Error> {
        Ok(self.0.fetch_one(executor).await?.0)
    }
}

pub struct QueryBuilder {
    sql: String,
    values: Vec<Value>,
}
impl QueryBuilder {
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            values: Vec::new(),
        }
    }
    pub fn push(&mut self, sql: impl std::fmt::Display) -> &mut Self {
        use std::fmt::Write;
        write!(&mut self.sql, "{sql}").unwrap();
        self
    }
    pub fn push_bind(&mut self, value: impl IntoValue) -> &mut Self {
        self.values.push(value.into_value());
        self.sql.push('?');
        self
    }
    pub fn separated(&mut self, separator: &str) -> Separated<'_> {
        Separated {
            builder: self,
            separator: separator.to_owned(),
            first: true,
        }
    }
    pub fn push_values<I, T>(&mut self, rows: I, mut f: impl FnMut(Separated<'_>, T)) -> &mut Self
    where
        I: IntoIterator<Item = T>,
    {
        self.push(" VALUES ");
        for (i, row) in rows.into_iter().enumerate() {
            if i > 0 {
                self.push(", ");
            }
            self.push("(");
            f(self.separated(", "), row);
            self.push(")");
        }
        self
    }
    pub fn build(&mut self) -> Query {
        self.build_query_as()
    }
    pub fn build_query_as<O: FromRow>(&mut self) -> Query<O> {
        Query {
            sql: self.sql.clone(),
            values: self.values.clone(),
            script: false,
            output: PhantomData,
        }
    }
    pub fn build_query_scalar<T: FromValue>(&mut self) -> Scalar<T> {
        Scalar(self.build_query_as())
    }
}
pub struct Separated<'a> {
    builder: &'a mut QueryBuilder,
    separator: String,
    first: bool,
}
impl Separated<'_> {
    fn prefix(&mut self) {
        if !self.first {
            self.builder.push(&self.separator);
        }
        self.first = false;
    }
    pub fn push(&mut self, sql: impl std::fmt::Display) -> &mut Self {
        self.prefix();
        self.builder.push(sql);
        self
    }
    pub fn push_bind(&mut self, value: impl IntoValue) -> &mut Self {
        self.prefix();
        self.builder.push_bind(value);
        self
    }
    pub fn push_unseparated(&mut self, sql: impl std::fmt::Display) -> &mut Self {
        self.builder.push(sql);
        self
    }
}

mod sdk_rows;

tuples!(A:0,B:1,C:2,D:3,E:4,F:5,G:6,H:7,I:8,J:9);

tuples!(A:0,B:1,C:2,D:3,E:4,F:5,G:6,H:7,I:8);
