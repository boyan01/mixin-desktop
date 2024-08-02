use std::error::Error;
use std::fs;
use std::sync::Arc;

use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};

use db::mixin::MixinDatabase;
use db::SignalDatabase;
use mixin_dekstop_core::core::crypto::signal_protocol::SignalProtocol;
use mixin_dekstop_core::core::message::blaze::Blaze;
use mixin_dekstop_core::core::message::decrypt::ServiceDecryptMessage;
use mixin_dekstop_core::core::message::sender::MessageSender;
use mixin_dekstop_core::core::model::AppService;
use mixin_dekstop_core::db;
use sdk::Credential;
use sdk::KeyStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])?;

    let file = fs::read("./keystore.json")?;
    let keystore: KeyStore = serde_json::from_slice(&file)?;

    let user_id = keystore.app_id.clone();
    let identity_number = "0".to_string();
    let client = Arc::new(sdk::Client::new(Credential::KeyStore(keystore.clone())));
    // let result = a.get_me().await;
    let database = Arc::new(MixinDatabase::new(identity_number.clone()).await?);
    let signal_database = Arc::new(SignalDatabase::connect(identity_number.to_string()).await?);
    let blaze = Arc::new(Blaze::new(
        database.clone(),
        Credential::KeyStore(keystore.clone()),
        keystore.app_id,
    ));
    let app_service = Arc::new(AppService::new(
        database.clone(),
        client.clone(),
        user_id.to_string(),
    ));

    let signal_protocol = Arc::new(SignalProtocol::new(
        signal_database.clone(),
        identity_number.to_string(),
    ));
    let sender = Arc::new(MessageSender::new(
        blaze.clone(),
        app_service.conversation.clone(),
        database.clone(),
        user_id.to_string(),
        signal_protocol.clone(),
    ));
    let decrypt_message = Arc::new(ServiceDecryptMessage::new(
        database.clone(),
        app_service.clone(),
        signal_protocol.clone(),
        sender.clone(),
        user_id.to_string(),
        identity_number.to_string(),
    ));
    let connection = blaze.connect();
    let decrypt = decrypt_message.start();
    let results = futures::join!(connection, decrypt);
    println!("{:?}", results);
    Ok(())
}
