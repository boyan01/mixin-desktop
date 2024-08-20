pub mod core;
pub mod db;

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;

    use log::LevelFilter;
    use simplelog::{Config, TestLogger};
    use tokio::fs;

    use sdk::{Client, Credential, KeyStore};

    use crate::db::{MixinDatabase, SignalDatabase};

    pub async fn new_test_client() -> Arc<Client> {
        let _ = TestLogger::init(LevelFilter::Trace, Config::default());
        let file = fs::read("../keystore.json")
            .await
            .expect("no keystore file");
        let keystore: KeyStore = serde_json::from_slice(&file).expect("failed to read keystore");
        Arc::new(Client::new(Credential::KeyStore(keystore)))
    }

    pub async fn new_test_mixin_db() -> Arc<MixinDatabase> {
        Arc::new(
            MixinDatabase::new("".to_string())
                .await
                .expect("failed to create db"),
        )
    }

    pub async fn new_test_signal_db() -> Arc<SignalDatabase> {
        Arc::new(
            SignalDatabase::connect("".to_string())
                .await
                .expect("failed to create db"),
        )
    }
}
