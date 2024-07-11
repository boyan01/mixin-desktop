use std::error::Error;
use std::fs;
use std::sync::Arc;
use chrono::Duration;
use tokio::time::sleep;
use mixin_dekstop_core::{db, sdk, core};
use core::Blaze;
use sdk::Credential;
use sdk::KeyStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let file = fs::read("./keystore.json")?;
    let keystore: KeyStore = serde_json::from_slice(&file)?;
    let client = sdk::Client::new(Credential::KeyStore(keystore.clone()));
    // let result = a.get_me().await;
    let database = Arc::new(db::MixinDatabase::new("".to_string()).await?);
    let result = database.query_friends().await?;
    let mut blaze = Blaze::new(database, client, Credential::KeyStore(keystore));
    blaze.connect().await.expect("TODO: panic message");
    sleep(Duration::minutes(100).to_std()?).await;
    Ok(())
}
