use std::error::Error;
use std::fs;

use mixin_dekstop_core::{db, sdk};
use sdk::Credential;
use sdk::KeyStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let file = fs::read("./keystore.json")?;
    let keystore: KeyStore = serde_json::from_slice(&file)?;
    let _ = sdk::Client::new(Credential::KeyStore(keystore));
    // let result = a.get_me().await;
    let database = db::MixinDatabase::new("".to_string()).await?;
    let result = database.query_friends().await?;
    Ok(())
}
