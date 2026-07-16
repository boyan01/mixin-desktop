use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{anyhow, Result};
use log::{error, warn};
use tokio::sync::{oneshot, watch};

use sdk::{
    Account, CircleConversationRequest, Client, ConversationCategory, ConversationRequest,
    Credential, KeyStore, ParticipantRequest,
};

use crate::core::attachment::AttachmentService;
use crate::core::constants::SCP;
use crate::core::crypto::signal_protocol::SignalProtocol;
use crate::core::message::blaze::Blaze;
use crate::core::message::decrypt::ServiceDecryptMessage;
use crate::core::message::sender::MessageSender;
use crate::core::model::signal::SignalService;
use crate::core::model::{AppService, ConversationService};
use crate::db::app::Auth;
use crate::db::mixin::circle::Circle;
use crate::db::mixin::conversation::ConversationListItem;
use crate::db::path::account_data_directory;
use crate::db::{MixinDatabase, SignalDatabase};

pub struct AccountRuntime {
    account_id: String,
    account: Account,
    client: Arc<Client>,
    database: Arc<MixinDatabase>,
    conversation_changes: watch::Sender<u64>,
    shutdown: watch::Sender<bool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl AccountRuntime {
    pub async fn start(auth: Auth) -> Result<Self> {
        if auth
            .primary_session_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .is_none()
        {
            return Err(anyhow!("authorization has no primary session"));
        }
        let account_id = auth.account.user_id.clone();
        let account = auth.account.clone();
        let client = Arc::new(Client::new(credential(&auth)));
        let (shutdown, shutdown_receiver) = watch::channel(false);
        let (conversation_changes, _) = watch::channel(0);
        let account_conversation_changes = conversation_changes.clone();
        let (ready_sender, ready_receiver) = oneshot::channel();
        let thread = std::thread::Builder::new()
            .name(format!("mixin-account-{account_id}"))
            .spawn({
                let client = client.clone();
                move || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    match runtime {
                        Ok(runtime) => runtime.block_on(run_account(
                            auth,
                            client,
                            shutdown_receiver,
                            account_conversation_changes,
                            ready_sender,
                        )),
                        Err(error) => {
                            let _ = ready_sender.send(Err(error.to_string()));
                        }
                    }
                }
            })?;
        let database = ready_receiver
            .await
            .map_err(|_| anyhow!("account runtime stopped during startup"))?
            .map_err(|error| anyhow!(error))?;

        Ok(Self {
            account_id,
            account,
            client,
            database,
            conversation_changes,
            shutdown,
            thread: Mutex::new(Some(thread)),
        })
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn account(&self) -> &Account {
        &self.account
    }

    pub async fn conversation_count(
        &self,
        category: &str,
        circle_id: Option<&str>,
        keyword: &str,
        unseen_only: bool,
    ) -> Result<i64> {
        Ok(self
            .database
            .conversation_dao
            .count_items(category, circle_id, keyword, unseen_only)
            .await?)
    }

    pub async fn conversations(
        &self,
        category: &str,
        circle_id: Option<&str>,
        keyword: &str,
        unseen_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ConversationListItem>> {
        Ok(self
            .database
            .conversation_dao
            .list_items(category, circle_id, keyword, unseen_only, limit, offset)
            .await?)
    }

    pub async fn circles(&self) -> Result<Vec<Circle>> {
        Ok(self.database.circle_dao.list().await?)
    }

    pub fn subscribe_conversation_changes(&self) -> watch::Receiver<u64> {
        self.conversation_changes.subscribe()
    }

    fn notify_conversation_changed(&self) {
        self.conversation_changes
            .send_modify(|revision| *revision = revision.wrapping_add(1));
    }

    pub async fn set_pinned(&self, conversation_id: &str, pinned: bool) -> Result<()> {
        self.database
            .conversation_dao
            .set_pinned(conversation_id, pinned)
            .await?;
        self.notify_conversation_changed();
        Ok(())
    }

    pub async fn set_muted(
        &self,
        conversation_id: &str,
        owner_id: &str,
        category: &str,
        duration_seconds: i64,
    ) -> Result<()> {
        let category = if category == "GROUP" {
            ConversationCategory::Group
        } else {
            ConversationCategory::Contact
        };
        let response = self
            .client
            .conversation_api
            .mute(&ConversationRequest {
                conversation_id: conversation_id.to_string(),
                category: Some(category.clone()),
                name: None,
                icon_base64: None,
                announcement: None,
                participants: (category == ConversationCategory::Contact).then(|| {
                    vec![ParticipantRequest {
                        user_id: self.account_id.clone(),
                    }]
                }),
                duration: Some(duration_seconds),
            })
            .await?;
        self.database
            .conversation_dao
            .set_mute_until(
                conversation_id,
                owner_id,
                if category == ConversationCategory::Group {
                    "GROUP"
                } else {
                    "CONTACT"
                },
                response.mute_until,
            )
            .await?;
        self.notify_conversation_changed();
        Ok(())
    }

    pub async fn delete_conversation(&self, conversation_id: &str) -> Result<()> {
        self.database
            .conversation_dao
            .delete_local(conversation_id)
            .await?;
        self.notify_conversation_changed();
        Ok(())
    }

    pub async fn edit_circle_conversation(
        &self,
        circle_id: &str,
        conversation_id: &str,
        owner_id: &str,
        is_group: bool,
        add: bool,
    ) -> Result<()> {
        let user_id = (!is_group).then(|| owner_id.to_string());
        let request = if add {
            CircleConversationRequest::Add {
                conversation_id: conversation_id.to_string(),
                user_id,
            }
        } else {
            CircleConversationRequest::Remove {
                conversation_id: conversation_id.to_string(),
                user_id,
            }
        };
        let result = self
            .client
            .circle_api
            .update_circle_conversation(circle_id, &request)
            .await?;
        if add {
            self.database
                .circle_conversation_dao
                .insert(&[result])
                .await?;
        } else {
            self.database
                .circle_conversation_dao
                .delete(circle_id, conversation_id)
                .await?;
        }
        self.notify_conversation_changed();
        Ok(())
    }

    pub async fn shutdown(&self) {
        let _ = self.shutdown.send(true);
        let thread = self.thread.lock().unwrap().take();
        if let Some(thread) = thread {
            let _ = tokio::task::spawn_blocking(move || thread.join()).await;
        }
    }
}

impl Drop for AccountRuntime {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
    }
}

async fn run_account(
    auth: Auth,
    client: Arc<Client>,
    mut shutdown_receiver: watch::Receiver<bool>,
    conversation_changes: watch::Sender<u64>,
    ready_sender: oneshot::Sender<std::result::Result<Arc<MixinDatabase>, String>>,
) {
    let result = prepare_account(&auth, client, conversation_changes).await;
    let (database, blaze, decrypt_message, sender, app_service) = match result {
        Ok(services) => services,
        Err(error) => {
            let _ = ready_sender.send(Err(error.to_string()));
            return;
        }
    };
    if ready_sender.send(Ok(database)).is_err() {
        return;
    }

    tokio::select! {
        result = blaze.connect() => {
            if let Err(error) = result {
                error!("Blaze stopped: {error:?}");
            }
        }
        _ = decrypt_message.start() => {
            warn!("message decrypt service stopped");
        }
        _ = sender.maintain_signal_keys() => {
            warn!("signal key service stopped");
        }
        result = app_service.job.start() => {
            if let Err(error) = result {
                error!("job service stopped: {error:?}");
            }
        }
        _ = shutdown_receiver.changed() => {}
    }
}

type AccountServices = (
    Arc<MixinDatabase>,
    Arc<Blaze>,
    Arc<ServiceDecryptMessage>,
    Arc<MessageSender>,
    Arc<AppService>,
);

async fn prepare_account(
    auth: &Auth,
    client: Arc<Client>,
    conversation_changes: watch::Sender<u64>,
) -> Result<AccountServices> {
    let account = &auth.account;
    let account_id = account.user_id.clone();
    let credential = credential(auth);
    client.account_api.get_me().await?;

    let database = Arc::new(
        MixinDatabase::new(account.identity_number.clone())
            .await
            .map_err(|error| anyhow!(error.to_string()))?,
    );
    let signal_database = Arc::new(
        SignalDatabase::connect(account.identity_number.clone())
            .await
            .map_err(|error| anyhow!(error.to_string()))?,
    );
    let blaze = Arc::new(Blaze::new(
        database.clone(),
        client.clone(),
        credential,
        account_id.clone(),
    ));
    let signal_protocol = Arc::new(SignalProtocol::new(
        signal_database.clone(),
        account_id.clone(),
    ));
    let conversation =
        ConversationService::new(database.clone(), client.clone(), account_id.clone());
    let signal_service = SignalService::new(signal_protocol.clone(), signal_database);
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
        client.clone(),
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(150))
            .build()?,
        account_data_directory(&account.identity_number)?,
    ));
    let app_service = Arc::new(AppService::new(
        database.clone(),
        client.clone(),
        auth,
        sender.clone(),
        attachment,
    ));
    let decrypt_message = Arc::new(
        ServiceDecryptMessage::new(
            database.clone(),
            app_service.clone(),
            signal_protocol,
            sender.clone(),
            blaze.pending_message_statuses(),
            auth,
        )
        .with_conversation_changes(conversation_changes),
    );
    Ok((database, blaze, decrypt_message, sender, app_service))
}

fn credential(auth: &Auth) -> Credential {
    Credential::KeyStore(KeyStore {
        app_id: auth.account.user_id.clone(),
        session_id: auth.account.session_id.clone(),
        server_public_key: String::new(),
        session_private_key: base16ct::lower::encode_string(&auth.private_key),
        scp: SCP.to_string(),
    })
}
