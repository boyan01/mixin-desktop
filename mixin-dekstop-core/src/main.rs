use std::error::Error;
use std::fs;
use std::io::Read;

use mixin_dekstop_core::sdk;
use sdk::Credential;
use sdk::KeyStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let file = fs::read("./keystore.json")?;
    let keystore: KeyStore = serde_json::from_slice(&file)?;
    let a = sdk::Client::new(Credential::KeyStore(keystore));
    let result = a.get_me().await;
    println!("a: {:?}", result);
    Ok(())
}
