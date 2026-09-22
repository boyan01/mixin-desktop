//! Browser-only contract checks used by the non-UI Worker test harness.
use crate::db::{self, MixinDatabase, SignalDatabase};
use anyhow::{ensure, Result};
use std::{sync::Arc, time::Duration};

use wasm_bindgen::prelude::*;
#[wasm_bindgen(module = "/web/platform.js")]
extern "C" {
    #[wasm_bindgen(js_name=fileReserve)]
    fn reserve_file() -> u32;
    #[wasm_bindgen(js_name=fileClose)]
    fn close_file(handle: u32);
    #[wasm_bindgen(catch, js_name=fileOpen)]
    async fn open_file(handle: u32, path: &str, mode: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name=databaseReserve)]
    fn reserve_database() -> u32;
    #[wasm_bindgen(js_name=databaseClose)]
    fn close_database(handle: u32);
    #[wasm_bindgen(catch, js_name=databaseOpen)]
    async fn open_database(handle: u32, path: &str) -> Result<JsValue, JsValue>;
}

pub async fn run(restore: bool) -> Result<()> {
    let desktop = crate::runtime::desktop::DesktopRuntime::open().await?;
    ensure!(desktop.restore_account().await?.is_none());
    desktop.settings.set_photo_auto_download(false).await?;
    ensure!(!desktop.settings.photo_auto_download().await?);
    let token = sdk::credential::Credential::KeyStore(sdk::credential::KeyStore {
        app_id: "browser-test".into(),
        session_id: "browser-session".into(),
        server_public_key: String::new(),
        session_private_key: "11".repeat(32),
        scp: "FULL".into(),
    })
    .sign_authentication_token(&reqwest::Method::GET, &"/".into(), b"")
    .map_err(anyhow::Error::msg)?;
    ensure!(token.split('.').count() == 3);
    let pool = db::Pool::open("/contract.db").await?;
    db::raw_sql("CREATE TABLE IF NOT EXISTS items (id INTEGER PRIMARY KEY, nullable TEXT, tail INTEGER, bytes BLOB);").execute(&pool).await?;
    if !restore {
        db::query("DELETE FROM items").execute(&pool).await?;
        db::query("INSERT INTO items VALUES (?, ?, ?, ?)")
            .bind(1i64)
            .bind(None::<String>)
            .bind(i64::MAX)
            .bind(vec![0u8, 255, 7])
            .execute(&pool)
            .await?;
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
        db::query("INSERT INTO items VALUES (2, 'rollback', 0, NULL)")
            .execute(&mut *tx)
            .await?;
        let mut waiting = Box::pin(pool.acquire());
        ensure!(
            futures::poll!(waiting.as_mut()).is_pending(),
            "transaction does not own its connection"
        );
        drop(waiting);
        drop(tx);
        ensure!(
            db::query_scalar::<i64>("SELECT COUNT(*) FROM items")
                .fetch_one(&pool)
                .await?
                == 1
        );
        // Failed COMMIT must roll back before the connection becomes reusable.
        db::raw_sql("CREATE TABLE IF NOT EXISTS parents(id INTEGER PRIMARY KEY); CREATE TABLE IF NOT EXISTS children(id INTEGER REFERENCES parents(id) DEFERRABLE INITIALLY DEFERRED);").execute(&pool).await?;
        let mut tx = pool.begin_with("BEGIN").await?;
        db::query("INSERT INTO children VALUES (999)")
            .execute(&mut *tx)
            .await?;
        let error = tx.commit().await.unwrap_err();
        ensure!(error
            .as_database_error()
            .is_some_and(|e| e.is_foreign_key_violation()));
        ensure!(
            db::query_scalar::<i64>("SELECT COUNT(*) FROM children")
                .fetch_one(&pool)
                .await?
                == 0
        );
    }
    let row = db::query_as::<(i64, Option<String>, i64, Vec<u8>)>(
        "SELECT id,nullable,tail,bytes FROM items",
    )
    .fetch_one(&pool)
    .await?;
    ensure!(
        row == (1, None, i64::MAX, vec![0, 255, 7]),
        "NULL/blob/i64 persistence changed: {row:?}"
    );
    pool.close().await;
    ensure!(db::query("SELECT 1").fetch_one(&pool).await.is_err());

    let database = MixinDatabase::connect_at("/contract/mixin.db")
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let message_id = "browser-message".to_owned();
    if !restore {
        let pool = db::Pool::open("/contract/mixin.db").await?;
        db::query("INSERT OR IGNORE INTO conversations (conversation_id, created_at, status) VALUES ('browser-chat', '2023-11-14 00:00:00', 0)").execute(&pool).await?;
        pool.close().await;
        database
            .message_dao
            .insert_message(&crate::db::mixin::message::Message {
                message_id: message_id.clone(),
                conversation_id: "browser-chat".into(),
                user_id: "browser-user".into(),
                category: "PLAIN_TEXT".into(),
                content: Some("browser contract".into()),
                status: sdk::MessageStatus::Sent,
                created_at: chrono::DateTime::from_timestamp_millis(1700000000123)
                    .unwrap()
                    .naive_utc(),
                ..Default::default()
            })
            .await?;
    }
    let message = database
        .message_dao
        .find_message_by_id(&message_id)
        .await?
        .unwrap();
    ensure!(message.content.as_deref() == Some("browser contract"));
    ensure!(message.media_key.is_none() && message.status == sdk::MessageStatus::Sent);
    ensure!(message.created_at.and_utc().timestamp_millis() == 1700000000123);
    database.close().await;

    if !restore {
        use crate::core::crypto::{
            compose_message::ComposeMessageData, signal_protocol::SignalProtocol,
        };
        use libsignal_protocol::{
            create_sender_key_distribution_message, ProtocolAddress, SenderKeyName,
        };
        let sender_db = Arc::new(
            SignalDatabase::connect_at("/contract/sender.db")
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?,
        );
        let receiver_db = Arc::new(
            SignalDatabase::connect_at("/contract/receiver.db")
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?,
        );
        sender_db.init(123, None).await?;
        ensure!(sender_db
            .identity_dao
            .get_local_identity()
            .await?
            .unwrap()
            .private_key
            .is_some());
        let sender = SignalProtocol::new(sender_db.clone(), "sender-id".into());
        let receiver = SignalProtocol::new(receiver_db.clone(), "receiver-id".into());
        let address = ProtocolAddress::new("sender-id".into(), 1);
        let name = SenderKeyName::new("group-id".into(), address.clone())
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let mut store = sender.protocol_store.clone();
        let distribution = create_sender_key_distribution_message(
            &name,
            &mut store.sender_key_store,
            &mut rand::rand_core::UnwrapErr(rand::rngs::SysRng),
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        receiver
            .process_group_session("group-id", address, &distribution)
            .await?;
        let encoded = sender
            .encrypt_group_message("group-id", b"browser signal")
            .await?;
        let composed = ComposeMessageData::decode(&encoded)?;
        let plaintext = receiver
            .decrypt(
                "group-id",
                "sender-id",
                composed.key_type,
                composed.cipher,
                sdk::message_category::SIGNAL_TEXT,
                None,
            )
            .await?;
        ensure!(plaintext == b"browser signal");
        sender_db.close().await;
        receiver_db.close().await;

        crate::fs::create_dir_all("/contract").await?;
        crate::fs::write("/contract/input", vec![42u8; 128 * 1024 + 7]).await?;
        let (key, digest) = crate::core::attachment::encrypt_attachment_file(
            "/contract/input".as_ref(),
            "/contract/encrypted".as_ref(),
        )
        .await?;
        crate::core::attachment::decrypt_attachment_file(
            "/contract/encrypted".as_ref(),
            "/contract/output".as_ref(),
            &key,
            &digest,
        )
        .await?;
        crate::fs::rename("/contract/output", "/contract/persisted").await?;
        crate::fs::remove_file("/contract/encrypted").await?;
    }
    ensure!(crate::fs::read("/contract/persisted").await? == vec![42u8; 128 * 1024 + 7]);
    // Cancel while acquisition is pending, then wait for the underlying JS
    // operation to finish. A cancelled open must not keep an OPFS lock alive.
    let handle = reserve_file();
    let mut opening = Box::pin(open_file(handle, "/contract/persisted", "read"));
    ensure!(futures::poll!(opening.as_mut()).is_pending());
    close_file(handle);
    ensure!(opening.await.is_err());
    ensure!(crate::fs::read("/contract/persisted").await?.len() == 128 * 1024 + 7);
    let handle = reserve_database();
    let mut opening = Box::pin(open_database(handle, "/contract.db"));
    ensure!(futures::poll!(opening.as_mut()).is_pending());
    close_database(handle);
    ensure!(opening.await.is_err());
    crate::network::browser::upload_file("/contract/persisted".as_ref(), "/upload").await?;
    let pool = db::Pool::open("/removed/account.db").await?;
    db::raw_sql("CREATE TABLE marker(id INTEGER); INSERT INTO marker VALUES (7)")
        .execute(&pool)
        .await?;
    pool.close().await;
    db::storage::files::rename(
        "/removed/account.db".as_ref(),
        "/removed/backup.db".as_ref(),
    )
    .await?;
    let backup = db::Pool::open("/removed/backup.db").await?;
    ensure!(
        db::query_scalar::<i64>("SELECT id FROM marker")
            .fetch_one(&backup)
            .await?
            == 7
    );
    ensure!(db::storage::files::remove_directory("/removed".as_ref())
        .await
        .is_err());
    backup.close().await;
    db::storage::files::remove_directory("/removed".as_ref()).await?;
    let empty = db::Pool::open("/removed/backup.db").await?;
    ensure!(db::query("SELECT * FROM marker")
        .fetch_all(&empty)
        .await
        .is_err());
    empty.close().await;
    db::storage::files::remove_directory("/removed".as_ref()).await?;
    let task = crate::task::spawn(async {
        crate::time::sleep(Duration::from_millis(5)).await;
        42
    });
    ensure!(crate::time::timeout(Duration::from_secs(5), task).await?? == 42);
    let task = crate::task::spawn(std::future::pending::<()>());
    task.abort();
    ensure!(task.await.is_err());
    ensure!(
        crate::time::timeout(Duration::from_millis(5), std::future::pending::<()>())
            .await
            .is_err()
    );
    Ok(())
}
