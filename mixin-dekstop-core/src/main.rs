use std::error::Error;
use std::sync::Arc;

use log::{info, LevelFilter};
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};

use db::mixin::MixinDatabase;
use db::SignalDatabase;
use mixin_dekstop_core::core::constants::SCP;
use mixin_dekstop_core::core::crypto::signal_protocol::SignalProtocol;
use mixin_dekstop_core::core::message::blaze::Blaze;
use mixin_dekstop_core::core::message::decrypt::ServiceDecryptMessage;
use mixin_dekstop_core::core::message::sender::MessageSender;
use mixin_dekstop_core::core::model::auth::AuthService;
use mixin_dekstop_core::core::model::signal::SignalService;
use mixin_dekstop_core::core::model::{AppService, ConversationService};
use mixin_dekstop_core::db;
use mixin_dekstop_core::db::app::AppDatabase;
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

    let app_db = Arc::new(AppDatabase::connect().await?);
    let auth_service = AuthService::new(app_db);
    auth_service.initialize().await?;

    let auth = match auth_service.get_auth() {
        Some(auth) => auth,
        None => {
            let auth = auth_service.authorize().await?;

            let identity_number = auth.auth.account.identity_number.clone();
            let signal_database = Arc::new(SignalDatabase::connect(identity_number).await?);
            signal_database
                .init(auth.registration_id, Some(&auth.identity_key_private))
                .await?;
            auth_service.save_auth(&auth.auth).await?;
            auth.auth
        }
    };

    let credential = Credential::KeyStore(KeyStore {
        app_id: auth.user_id.clone(),
        session_id: auth.account.session_id.clone(),
        server_public_key: "".to_string(),
        session_private_key: base16ct::lower::encode_string(&auth.private_key),
        scp: SCP.to_string(),
    });

    let account = auth.account;
    let client = Arc::new(sdk::Client::new(credential.clone()));
    let account_id = account.user_id;
    let result = client.account_api.get_me().await?;
    info!("account: {:?}", result);

    let database = Arc::new(MixinDatabase::new(account.identity_number.clone()).await?);
    let signal_database =
        Arc::new(SignalDatabase::connect(account.identity_number.to_string()).await?);
    let blaze = Arc::new(Blaze::new(database.clone(), credential, account_id.clone()));

    let signal_protocol = Arc::new(SignalProtocol::new(
        signal_database.clone(),
        account.identity_number.to_string(),
    ));

    let conversation =
        ConversationService::new(database.clone(), client.clone(), account_id.clone());

    let signal_service = SignalService::new(
        signal_protocol.clone(),
        signal_database.clone(),
        account_id.to_string(),
    );

    let sender = Arc::new(MessageSender::new(
        blaze.clone(),
        conversation,
        database.clone(),
        account_id.to_string(),
        signal_protocol.clone(),
        signal_service,
    ));

    let app_service = Arc::new(AppService::new(
        database.clone(),
        client.clone(),
        account_id.to_string(),
        None,
        sender.clone(),
    ));

    let decrypt_message = Arc::new(ServiceDecryptMessage::new(
        database.clone(),
        app_service.clone(),
        signal_protocol.clone(),
        sender.clone(),
        account_id.to_string(),
        account.identity_number.to_string(),
    ));
    let connection = blaze.connect();
    let decrypt = decrypt_message.start();
    let results = futures::join!(connection, decrypt);
    println!("{:?}", results);
    Ok(())
}
