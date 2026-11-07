//! Bounded, account-scoped text inscription content and its disposable cache.

use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;

use crate::db::mixin::inscription::InscriptionDao;
use crate::network::NetworkService;

use super::AccountRuntime;

const MAX_CONTENT_BYTES: usize = 1024 * 1024;

impl AccountRuntime {
    /// Reads text inscriptions without executing embedded HTML or script content.
    /// Unknown metadata and non-text inscriptions have no text preview.
    pub async fn inscription_content(&self, hash: String) -> Result<Option<String>> {
        self.ensure_active()?;
        let directory = crate::db::path::account_data_directory(&self.account().identity_number)?
            .join("Cache")
            .join("InscriptionText");
        let mut shutdown = self.state.shutdown.clone();
        tokio::select! {
            biased;
            _ = shutdown.wait_for(|stopping| *stopping) => {
                Err(anyhow!("account runtime is shutting down"))
            }
            result = load_content(&self.database.inscription_dao, &self.network, &directory, &hash, Some(&self.state)) => result,
        }
    }
}

async fn load_content(
    inscriptions: &InscriptionDao,
    network: &NetworkService,
    directory: &Path,
    hash: &str,
    account: Option<&super::AccountState>,
) -> Result<Option<String>> {
    let Some(item) = inscriptions.find_item(hash).await? else {
        return Ok(None);
    };
    if !item.content_type.starts_with("text/") {
        return Ok(None);
    }
    let url = url::Url::parse(&item.content_url).context("invalid inscription content URL")?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(anyhow!("unsupported inscription content URL scheme"));
    }
    // Include the URL so a metadata refresh cannot reuse content from an old URL.
    let mut digest = Sha256::new();
    digest.update(hash.as_bytes());
    digest.update([0]);
    digest.update(url.as_str().as_bytes());
    let path = directory.join(hex::encode(digest.finalize()));
    if let Some(bytes) = read_cache(&path).await? {
        return Ok(Some(decode_content(&bytes)));
    }
    let response = network
        .request(
            "GET",
            url.as_str(),
            HashMap::new(),
            None,
            Some(30_000),
            Some(MAX_CONTENT_BYTES),
        )
        .await
        // Transport errors may contain signed content URLs; keep them private.
        .map_err(|_| anyhow!("inscription content could not be downloaded"))?;
    if response.status_code != 200 {
        return Err(anyhow!(
            "inscription content returned HTTP {}",
            response.status_code
        ));
    }
    // Only local publication participates in account teardown. Network work must
    // remain cancellable without holding up the account's exclusive shutdown lock.
    let _mutation = match account {
        Some(account) => {
            let guard = account.mutation_gate.read().await;
            account.ensure_active()?;
            Some(guard)
        }
        None => None,
    };
    write_cache(&path, &response.body).await?;
    Ok(Some(decode_content(&response.body)))
}

fn decode_content(bytes: &[u8]) -> String {
    // Match Flutter's utf8.decode(..., allowMalformed: true).
    String::from_utf8_lossy(bytes).into_owned()
}

async fn read_cache(path: &Path) -> Result<Option<Vec<u8>>> {
    let file = match crate::fs::File::open(path).await {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    // A bounded read also covers a corrupt cache that grows after metadata lookup.
    let mut bytes = Vec::new();
    file.take(MAX_CONTENT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .await?;
    if bytes.len() > MAX_CONTENT_BYTES {
        return Err(anyhow!("cached inscription content is too large"));
    }
    Ok(Some(bytes))
}

#[cfg(not(target_arch = "wasm32"))]
async fn write_cache(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;

    let directory = path.parent().context("inscription cache has no parent")?;
    std::fs::create_dir_all(directory)?;
    let temporary = directory.join(format!("{}.part", uuid::Uuid::new_v4()));
    // This bounded local write has no suspension point. Cancellation therefore
    // cannot leave a partial file or race an in-flight file creation.
    let result = (|| -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)?;
        Ok(())
    })();
    let _ = std::fs::remove_file(&temporary);
    result
}

#[cfg(target_arch = "wasm32")]
async fn write_cache(path: &Path, bytes: &[u8]) -> Result<()> {
    let directory = path.parent().context("inscription cache has no parent")?;
    crate::fs::create_dir_all(directory).await?;
    let temporary = directory.join(format!("{}.part", uuid::Uuid::new_v4()));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let path = self.0.clone();
            crate::task::spawn(async move {
                let _ = crate::fs::remove_file(path).await;
            });
        }
    }
    let _cleanup = Cleanup(temporary.clone());
    crate::fs::write(&temporary, bytes).await?;
    crate::fs::rename(&temporary, path).await?;
    Ok(())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

    use super::*;
    use crate::db::{app::AppDatabase, mixin::MixinDatabase};

    async fn fixture() -> (tempfile::TempDir, MixinDatabase, Arc<NetworkService>) {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let app = AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        let network = Arc::new(NetworkService::new(app.setting_dao).await.unwrap());
        (directory, database, network)
    }

    async fn insert(database: &MixinDatabase, hash: &str, kind: &str, url: &str) {
        let now = chrono::Utc::now();
        database
            .inscription_dao
            .insert_item(&sdk::InscriptionItem {
                inscription_hash: hash.to_owned(),
                collection_hash: "collection".into(),
                sequence: 1,
                content_type: kind.to_owned(),
                content_url: url.to_owned(),
                occupied_by: None,
                occupied_at: None,
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();
    }

    async fn server(response: Vec<u8>) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/content", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let count = stream.read(&mut request).await.unwrap();
            assert!(String::from_utf8_lossy(&request[..count]).starts_with("GET /content HTTP/1.1"));
            let _ = stream.write_all(&response).await;
        });
        (url, task)
    }

    #[tokio::test]
    async fn text_download_is_cached_by_safe_filename_and_remains_available_offline() {
        let (directory, database, network) = fixture().await;
        let cache = directory.path().join("cache");
        let (url, server) = server(
            b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nhello\xff!".to_vec(),
        )
        .await;
        insert(&database, "../../unsafe", "text/html", &url).await;
        let content = load_content(
            &database.inscription_dao,
            &network,
            &cache,
            "../../unsafe",
            None,
        )
        .await
        .unwrap();
        assert_eq!(content.as_deref(), Some("hello\u{fffd}!"));
        server.await.unwrap();
        assert_eq!(
            load_content(
                &database.inscription_dao,
                &network,
                &cache,
                "../../unsafe",
                None
            )
            .await
            .unwrap(),
            content
        );
        let paths = std::fs::read_dir(&cache)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].len(), 64);
        assert!(paths[0]
            .chars()
            .all(|character| character.is_ascii_hexdigit()));
        database.close().await;
    }

    #[tokio::test]
    async fn missing_or_nontext_metadata_returns_no_preview_without_reading_its_url() {
        let (directory, database, network) = fixture().await;
        let cache = directory.path().join("cache");
        insert(&database, "image", "image/png", "file:///private/content").await;
        for hash in ["missing", "image"] {
            assert!(
                load_content(&database.inscription_dao, &network, &cache, hash, None)
                    .await
                    .unwrap()
                    .is_none()
            );
        }
        assert!(!cache.exists());
        database.close().await;
    }

    #[tokio::test]
    async fn chunked_content_exceeding_limit_fails_without_writing_a_cache() {
        let (directory, database, network) = fixture().await;
        let cache = directory.path().join("cache");
        let bytes = vec![b'a'; MAX_CONTENT_BYTES + 1];
        let mut response =
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_vec();
        response.extend_from_slice(format!("{:x}\r\n", bytes.len()).as_bytes());
        response.extend_from_slice(&bytes);
        response.extend_from_slice(b"\r\n0\r\n\r\n");
        let (url, server) = server(response).await;
        insert(&database, "large", "text/plain", &url).await;
        assert!(
            load_content(&database.inscription_dao, &network, &cache, "large", None)
                .await
                .is_err()
        );
        server.await.unwrap();
        assert!(!cache.exists());
        database.close().await;
    }

    #[tokio::test]
    async fn oversized_cached_content_is_rejected_by_a_bounded_read() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cached");
        std::fs::write(&path, vec![b'a'; MAX_CONTENT_BYTES + 1]).unwrap();
        assert!(read_cache(&path)
            .await
            .unwrap_err()
            .to_string()
            .contains("too large"));
    }

    #[tokio::test]
    async fn failed_atomic_cache_publication_removes_its_temporary_file() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("existing-directory");
        std::fs::create_dir(&destination).unwrap();
        assert!(write_cache(&destination, b"text").await.is_err());
        let paths = std::fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(paths, [destination]);
    }

    #[tokio::test]
    async fn cancelling_incomplete_download_closes_connection_without_partial_cache() {
        let (directory, database, network) = fixture().await;
        let database = Arc::new(database);
        let cache = directory.path().join("cache");
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/content", listener.local_addr().unwrap());
        insert(&database, "cancel", "text/plain", &url).await;
        let (ready, started) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let count = stream.read(&mut request).await.unwrap();
            assert!(String::from_utf8_lossy(&request[..count]).starts_with("GET /content HTTP/1.1"));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nincomplete")
                .await
                .unwrap();
            ready.send(()).unwrap();
            let mut byte = [0];
            match stream.read(&mut byte).await {
                Ok(0) => {}
                Err(error) if error.kind() == ErrorKind::ConnectionReset => {}
                result => panic!("cancelled download kept the connection alive: {result:?}"),
            }
        });
        let task = {
            let database = database.clone();
            let cache = cache.clone();
            tokio::spawn(async move {
                load_content(&database.inscription_dao, &network, &cache, "cancel", None).await
            })
        };
        tokio::time::timeout(Duration::from_secs(3), started)
            .await
            .unwrap()
            .unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        tokio::time::timeout(Duration::from_secs(3), server)
            .await
            .unwrap()
            .unwrap();
        assert!(!cache.exists());
        database.close().await;
    }

    #[tokio::test]
    async fn non_success_http_response_and_local_url_never_become_display_content() {
        let (directory, database, network) = fixture().await;
        let cache = directory.path().join("cache");
        let (url, server) = server(
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 5\r\nConnection: close\r\n\r\nerror"
                .to_vec(),
        )
        .await;
        insert(&database, "missing-content", "text/plain", &url).await;
        assert!(load_content(
            &database.inscription_dao,
            &network,
            &cache,
            "missing-content",
            None
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("404"));
        server.await.unwrap();
        insert(&database, "local", "text/plain", "file:///private/content").await;
        assert!(
            load_content(&database.inscription_dao, &network, &cache, "local", None)
                .await
                .unwrap_err()
                .to_string()
                .contains("scheme")
        );
        assert!(!cache.exists());
        database.close().await;
    }
}
