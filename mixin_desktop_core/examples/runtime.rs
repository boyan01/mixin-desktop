//! Live runtime smoke test for the desktop core.

use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use log::{info, warn, LevelFilter};
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};

use db::mixin::MixinDatabase;
use db::SignalDatabase;
use mixin_desktop_core::core::attachment::AttachmentService;
use mixin_desktop_core::core::constants::SCP;
use mixin_desktop_core::core::crypto::signal_protocol::SignalProtocol;
use mixin_desktop_core::core::message::blaze::{Blaze, BlazeAuthenticationError};
use mixin_desktop_core::core::message::decrypt::ServiceDecryptMessage;
use mixin_desktop_core::core::message::sender::MessageSender;
use mixin_desktop_core::core::model::auth::AuthService;
use mixin_desktop_core::core::model::signal::SignalService;
use mixin_desktop_core::core::model::{AppService, ConversationService};
use mixin_desktop_core::db;
use mixin_desktop_core::db::app::{AppDatabase, Auth};
use mixin_desktop_core::db::path::account_data_directory;
use sdk::{ApiError, Client, Credential, KeyStore};

struct AuthenticatedRuntime {
    auth: Auth,
    credential: Credential,
    client: Arc<Client>,
}

enum ServiceExit {
    AuthenticationFailed,
    Shutdown,
}

async fn wait_for_authentication_failure(
    receiver: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<(), tokio::sync::watch::error::RecvError> {
    if *receiver.borrow() {
        return Ok(());
    }
    loop {
        receiver.changed().await?;
        if *receiver.borrow_and_update() {
            return Ok(());
        }
    }
}

async fn authorize_and_return(auth_service: &AuthService) -> Result<Auth, Box<dyn Error>> {
    let auth = auth_service.authorize().await?;

    let identity_number = auth.auth.account.identity_number.clone();
    let signal_database = Arc::new(SignalDatabase::connect(identity_number).await?);
    signal_database
        .init(auth.registration_id, Some(&auth.identity_key_private))
        .await?;
    auth_service.save_auth(&auth.auth).await?;
    Ok(auth.auth)
}

async fn load_authenticated_runtime(
    auth_service: &AuthService,
) -> Result<AuthenticatedRuntime, Box<dyn Error>> {
    loop {
        let mut auth = match auth_service.get_auth() {
            Some(auth) => auth,
            None => authorize_and_return(auth_service).await?,
        };
        if auth
            .primary_session_id
            .as_deref()
            .is_none_or(|session_id| session_id.trim().is_empty())
        {
            warn!("stored authorization has no primary session, requesting a new authorization");
            auth_service.clear_auth(&auth.user_id).await?;
            continue;
        }
        let credential = Credential::KeyStore(KeyStore {
            app_id: auth.user_id.clone(),
            session_id: auth.account.session_id.clone(),
            server_public_key: String::new(),
            session_private_key: base16ct::lower::encode_string(&auth.private_key),
            scp: SCP.to_string(),
        });
        let client = Arc::new(Client::new(credential.clone()));

        match client.account_api.get_me().await {
            Ok(account) => {
                info!("account: {:?}", account);
                auth.account = account;
                auth_service.save_auth(&auth).await?;
                return Ok(AuthenticatedRuntime {
                    auth,
                    credential,
                    client,
                });
            }
            Err(ApiError::Server(sdk::Error { code: 401, .. })) => {
                warn!("stored session is unauthorized, requesting a new authorization");
                auth_service.clear_auth(&auth.user_id).await?;
            }
            Err(err) => return Err(Box::new(err)),
        }
    }
}

async fn run_authenticated_services(
    runtime: &AuthenticatedRuntime,
) -> Result<ServiceExit, Box<dyn Error>> {
    let mut authentication_errors = runtime.client.subscribe_authentication_errors();
    let account = &runtime.auth.account;
    let account_id = account.user_id.clone();
    let database = Arc::new(MixinDatabase::new(account.identity_number.clone()).await?);
    let signal_database = Arc::new(SignalDatabase::connect(account.identity_number.clone()).await?);
    let blaze = Arc::new(Blaze::new(
        database.clone(),
        runtime.client.clone(),
        runtime.credential.clone(),
        account_id.clone(),
        None,
    ));

    let signal_protocol = Arc::new(SignalProtocol::new(
        signal_database.clone(),
        account_id.clone(),
    ));
    let conversation =
        ConversationService::new(database.clone(), runtime.client.clone(), account_id.clone());
    let signal_service = SignalService::new(signal_protocol.clone(), signal_database.clone());
    let sender = Arc::new(MessageSender::new(
        blaze.clone(),
        conversation,
        database.clone(),
        account_id.clone(),
        account.session_id.clone(),
        signal_protocol.clone(),
        signal_service,
    ));
    let attachment = Arc::new(AttachmentService::new(
        runtime.client.clone(),
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(150))
            .build()?,
        account_data_directory(&account.identity_number)?,
    ));
    let app_service = Arc::new(AppService::new(
        database.clone(),
        runtime.client.clone(),
        &runtime.auth,
        sender.clone(),
        attachment,
        None,
    ));
    let decrypt_message = Arc::new(ServiceDecryptMessage::new(
        database,
        app_service.clone(),
        signal_protocol,
        sender.clone(),
        blaze.pending_message_statuses(),
        &runtime.auth,
    ));

    tokio::select! {
        result = blaze.connect() => {
            match result {
                Err(err) if err.downcast_ref::<BlazeAuthenticationError>().is_some() => {
                    Ok(ServiceExit::AuthenticationFailed)
                }
                Err(err) => Err(err.into()),
                Ok(()) => Err("blaze connection loop exited unexpectedly".into()),
            }
        }
        _ = decrypt_message.start() => {
            Err("message decrypt service exited unexpectedly".into())
        }
        _ = sender.maintain_signal_keys() => {
            Err("signal key maintenance service exited unexpectedly".into())
        }
        result = app_service.job.start() => {
            match result {
                Ok(()) => Err("job service exited unexpectedly".into()),
                Err(err) => Err(err.into()),
            }
        }
        result = wait_for_authentication_failure(&mut authentication_errors) => {
            result?;
            Ok(ServiceExit::AuthenticationFailed)
        }
        _ = tokio::signal::ctrl_c() => Ok(ServiceExit::Shutdown),
    }
}

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

    loop {
        let runtime = load_authenticated_runtime(&auth_service).await?;
        match run_authenticated_services(&runtime).await? {
            ServiceExit::AuthenticationFailed => {
                warn!("session authentication failed, requesting a new authorization");
                auth_service.clear_auth(&runtime.auth.user_id).await?;
            }
            ServiceExit::Shutdown => return Ok(()),
        }
    }
}
