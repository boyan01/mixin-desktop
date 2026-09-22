//! One SQLite connection per pool. The JS module initializes OPFS asynchronously;
//! statements then execute synchronously on the owning Worker.
use super::{Executor, ResultSet, Row, Value};
use sqlx::Error;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::sync::{Mutex, MutexGuard};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(module = "/web/platform.js")]
extern "C" {
    #[wasm_bindgen(js_name=databaseReserve)]
    fn reserve() -> u32;
    #[wasm_bindgen(catch,js_name=databaseOpen)]
    async fn open(handle: u32, path: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch,js_name=databaseExecute)]
    fn execute(handle: u32, sql: &str, params: &str) -> Result<String, JsValue>;
    #[wasm_bindgen(js_name=databaseClose)]
    fn close(handle: u32);
}
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
struct BrowserDatabaseError {
    message: String,
    code: Option<String>,
    result_code: i32,
}
impl sqlx::error::DatabaseError for BrowserDatabaseError {
    fn message(&self) -> &str {
        &self.message
    }
    fn code(&self) -> Option<std::borrow::Cow<'_, str>> {
        self.code.as_deref().map(Into::into)
    }
    fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        self
    }
    fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
        self
    }
    fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
        self
    }
    fn kind(&self) -> sqlx::error::ErrorKind {
        use sqlx::error::ErrorKind;
        match self.result_code {
            1555 | 2067 => ErrorKind::UniqueViolation,
            787 => ErrorKind::ForeignKeyViolation,
            1299 => ErrorKind::NotNullViolation,
            275 => ErrorKind::CheckViolation,
            _ => ErrorKind::Other,
        }
    }
}
fn error(value: JsValue) -> Error {
    let code = js_sys::Reflect::get(&value, &"resultCode".into())
        .ok()
        .and_then(|v| v.as_f64());
    let message = js_sys::Reflect::get(&value, &"message".into())
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| format!("{value:?}"));
    Error::Database(Box::new(BrowserDatabaseError {
        message,
        code: code.map(|v| (v as i32).to_string()),
        result_code: code.unwrap_or(0.0) as i32,
    }))
}
#[derive(Debug)]
pub struct Conn {
    handle: Option<u32>,
}
impl Conn {
    fn execute_statement(&mut self, sql: &str, values: &[Value]) -> Result<ResultSet, Error> {
        let handle = self.handle.ok_or(Error::PoolClosed)?;
        let json = serde_json::to_string(values).map_err(|e| Error::Encode(e.into()))?;
        let response = execute(handle, sql, &json).map_err(error)?;
        #[derive(serde::Deserialize)]
        struct Response {
            columns: Vec<String>,
            rows: Vec<Vec<Value>>,
            changes: u64,
            rowid: String,
        }
        let response: Response =
            serde_json::from_str(&response).map_err(|e| Error::Decode(e.into()))?;
        Ok(ResultSet {
            rows: response
                .rows
                .into_iter()
                .map(|values| Row {
                    columns: response.columns.clone(),
                    values,
                })
                .collect(),
            changes: response.changes,
            rowid: response
                .rowid
                .parse()
                .map_err(|e| Error::Decode(Box::new(e)))?,
        })
    }
    fn close(&mut self) {
        if let Some(handle) = self.handle.take() {
            close(handle);
        }
    }
}
impl Drop for Conn {
    fn drop(&mut self) {
        self.close();
    }
}
#[derive(Clone, Debug)]
pub struct Pool(Arc<Mutex<Conn>>);
impl Pool {
    pub async fn open(path: &str) -> Result<Self, Error> {
        let handle = reserve();
        let mut conn = Conn {
            handle: Some(handle),
        };
        open(handle, path).await.map_err(error)?;
        conn.execute_statement("PRAGMA foreign_keys = ON", &[])?;
        Ok(Self(Arc::new(Mutex::new(conn))))
    }
    pub async fn acquire(&self) -> Result<MutexGuard<'_, Conn>, Error> {
        let guard = self.0.lock().await;
        if guard.handle.is_none() {
            return Err(Error::PoolClosed);
        }
        Ok(guard)
    }
    pub async fn begin_with(&self, sql: impl sqlx::SqlSafeStr) -> Result<Transaction<'_>, Error> {
        let mut guard = self.acquire().await?;
        guard.execute_statement(sql.into_sql_str().as_str(), &[])?;
        Ok(Transaction {
            guard,
            finished: false,
        })
    }
    pub async fn close(&self) {
        self.0.lock().await.close();
    }
}
pub struct Transaction<'a> {
    guard: MutexGuard<'a, Conn>,
    finished: bool,
}
impl Transaction<'_> {
    pub async fn commit(mut self) -> Result<(), Error> {
        self.guard.execute_statement("COMMIT", &[])?;
        self.finished = true;
        Ok(())
    }
    pub async fn rollback(&mut self) -> Result<(), Error> {
        self.guard.execute_statement("ROLLBACK", &[])?;
        self.finished = true;
        Ok(())
    }
}
impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        if !self.finished && self.guard.execute_statement("ROLLBACK", &[]).is_err() {
            self.guard.close();
        }
    }
}
impl Deref for Transaction<'_> {
    type Target = Conn;
    fn deref(&self) -> &Conn {
        &self.guard
    }
}
impl DerefMut for Transaction<'_> {
    fn deref_mut(&mut self) -> &mut Conn {
        &mut self.guard
    }
}
impl Executor for &Pool {
    async fn run(self, sql: String, values: Vec<Value>, _: bool) -> Result<ResultSet, Error> {
        self.acquire().await?.execute_statement(&sql, &values)
    }
}
impl Executor for &mut Conn {
    async fn run(self, sql: String, values: Vec<Value>, _: bool) -> Result<ResultSet, Error> {
        Conn::execute_statement(self, &sql, &values)
    }
}
impl Executor for &mut MutexGuard<'_, Conn> {
    async fn run(self, sql: String, values: Vec<Value>, _: bool) -> Result<ResultSet, Error> {
        Conn::execute_statement(self, &sql, &values)
    }
}
