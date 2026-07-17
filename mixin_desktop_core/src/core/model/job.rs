mod ack;
mod sending;
mod session_ack;
mod sync_inscription;
mod update_asset;
mod update_sticker;
mod update_token;

use ack::AckJobRunner;
use sending::SendingJobRunner;
use session_ack::SessionAckJobRunner;
use sync_inscription::SyncInscriptionJobRunner;
use update_asset::UpdateAssetJobRunner;
use update_sticker::UpdateStickerJobRunner;
use update_token::UpdateTokenJobRunner;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use futures::{future::pending, StreamExt};
use log::{error, info};
use strum_macros::Display;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::{watch, Mutex, Notify};
use tokio::time::{interval, sleep};
use tokio_stream::wrappers::ReceiverStream;

use sdk::err::error_code::{BAD_DATA, FORBIDDEN};
use sdk::message_category::MessageCategory;
use sdk::{
    Client, ACKNOWLEDGE_MESSAGE_RECEIPTS, CREATE_MESSAGE, PIN_MESSAGE, RECALL_MESSAGE,
    SENDING_MESSAGE,
};

use crate::core::message::sender::{MessageResult, MessageSender};
use crate::db::app::Auth;
use crate::db::mixin::job::{
    Job, JobDao, SYNC_INSCRIPTION_MESSAGE, UPDATE_ASSET, UPDATE_STICKER, UPDATE_TOKEN,
};
use crate::db::MixinDatabase;

pub struct JobService {
    job_dao: JobDao,
    ack_job_signer: Sender<()>,
    session_ack_job_signer: Sender<()>,
    sending_job_signer: Sender<()>,
    update_asset_job_signer: Sender<()>,
    update_token_job_signer: Sender<()>,
    update_sticker_job_signer: Sender<()>,
    sync_inscription_job_signer: Sender<()>,
    params: Mutex<Option<JobParams>>,
}

impl JobService {
    pub fn new(
        database: Arc<MixinDatabase>,
        message_sender: Arc<MessageSender>,
        client: Arc<Client>,
        auth: &Auth,
        changes: Option<watch::Sender<u64>>,
        expired_message_notify: Arc<Notify>,
    ) -> Self {
        let (ack_sender, ack_receiver) = channel(1);
        let (session_ack_sender, session_ack_receiver) = channel(1);
        let (sending_sender, sending_receiver) = channel(1);
        let (update_asset_sender, update_asset_receiver) = channel(1);
        let (update_token_sender, update_token_receiver) = channel(1);
        let (update_sticker_sender, update_sticker_receiver) = channel(1);
        let (sync_inscription_sender, sync_inscription_receiver) = channel(1);

        let params = JobParams {
            database: database.clone(),
            client: client.clone(),
            receiver: HashMap::from([
                (JobCategory::Ack, ack_receiver),
                (JobCategory::SessionAck, session_ack_receiver),
                (JobCategory::Sending, sending_receiver),
                (JobCategory::UpdateAsset, update_asset_receiver),
                (JobCategory::UpdateToken, update_token_receiver),
                (JobCategory::UpdateSticker, update_sticker_receiver),
                (JobCategory::SyncInscription, sync_inscription_receiver),
            ]),
            user_id: auth.account.user_id.clone(),
            primary_session_id: auth.primary_session_id.clone(),
            message_sender,
            private_key: auth.private_key.clone(),
            session_id: auth.account.session_id.clone(),
            changes,
            expired_message_notify,
        };
        JobService {
            job_dao: database.job_dao.clone(),
            ack_job_signer: ack_sender,
            session_ack_job_signer: session_ack_sender,
            sending_job_signer: sending_sender,
            update_asset_job_signer: update_asset_sender,
            update_token_job_signer: update_token_sender,
            update_sticker_job_signer: update_sticker_sender,
            sync_inscription_job_signer: sync_inscription_sender,
            params: Mutex::new(Some(params)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let params = self
            .params
            .lock()
            .await
            .take()
            .ok_or_else(|| anyhow!("job service is already running"))?;
        start_all_jobs(params).await;
        Ok(())
    }

    pub async fn add(&self, job: &Job) -> Result<()> {
        let signaler = self.signaler(&job.action)?;
        self.job_dao.insert_job(job).await?;
        let _ = signaler.try_send(());
        Ok(())
    }

    pub async fn add_all(&self, jobs: &[Job]) -> Result<()> {
        let Some(first) = jobs.first() else {
            return Ok(());
        };
        if jobs.iter().any(|job| job.action != first.action) {
            bail!("batch contains multiple job actions");
        }
        let signaler = self.signaler(&first.action)?;
        self.job_dao.insert_all(jobs).await?;
        let _ = signaler.try_send(());
        Ok(())
    }

    pub fn wake(&self, action: &str) -> Result<()> {
        let _ = self.signaler(action)?.try_send(());
        Ok(())
    }

    fn signaler(&self, action: &str) -> Result<&Sender<()>> {
        let signaler = match action {
            ACKNOWLEDGE_MESSAGE_RECEIPTS => &self.ack_job_signer,
            CREATE_MESSAGE => &self.session_ack_job_signer,
            SENDING_MESSAGE | PIN_MESSAGE | RECALL_MESSAGE => &self.sending_job_signer,
            UPDATE_ASSET => &self.update_asset_job_signer,
            UPDATE_TOKEN => &self.update_token_job_signer,
            UPDATE_STICKER => &self.update_sticker_job_signer,
            SYNC_INSCRIPTION_MESSAGE => &self.sync_inscription_job_signer,
            _ => bail!("unknown job action: {action}"),
        };
        Ok(signaler)
    }
}

#[derive(Debug, Display, Eq, PartialEq, Hash)]
enum JobCategory {
    Ack,
    SessionAck,
    Sending,
    UpdateAsset,
    UpdateToken,
    UpdateSticker,
    SyncInscription,
}

struct JobParams {
    database: Arc<MixinDatabase>,
    client: Arc<Client>,
    receiver: HashMap<JobCategory, Receiver<()>>,
    user_id: String,
    primary_session_id: Option<String>,
    message_sender: Arc<MessageSender>,
    private_key: Vec<u8>,
    session_id: String,
    changes: Option<watch::Sender<u64>>,
    expired_message_notify: Arc<Notify>,
}

async fn start_all_jobs(mut params: JobParams) {
    tokio::join!(
        run_job(
            params.receiver.remove(&JobCategory::Ack),
            AckJobRunner {
                database: params.database.clone(),
                client: params.client.clone(),
            },
        ),
        run_job(
            params.receiver.remove(&JobCategory::SessionAck),
            SessionAckJobRunner {
                database: params.database.clone(),
                user_id: params.user_id.clone(),
                sender: params.message_sender.clone(),
                primary_session_id: params.primary_session_id,
            },
        ),
        run_job(
            params.receiver.remove(&JobCategory::Sending),
            SendingJobRunner {
                database: params.database.clone(),
                client: params.client.clone(),
                user_id: params.user_id.clone(),
                session_id: params.session_id.clone(),
                private_key: params.private_key,
                sender: params.message_sender,
                changes: params.changes,
                expired_message_notify: params.expired_message_notify,
            },
        ),
        run_job(
            params.receiver.remove(&JobCategory::UpdateAsset),
            UpdateAssetJobRunner {
                database: params.database.clone(),
                client: params.client.clone(),
            },
        ),
        run_job(
            params.receiver.remove(&JobCategory::UpdateToken),
            UpdateTokenJobRunner {
                database: params.database.clone(),
                client: params.client.clone(),
            },
        ),
        run_job(
            params.receiver.remove(&JobCategory::UpdateSticker),
            UpdateStickerJobRunner {
                database: params.database.clone(),
                client: params.client.clone(),
            },
        ),
        run_job(
            params.receiver.remove(&JobCategory::SyncInscription),
            SyncInscriptionJobRunner {
                database: params.database,
                client: params.client,
            },
        )
    );
}

async fn run_job(receiver: Option<Receiver<()>>, trigger: impl JobTrigger) {
    let receiver = match receiver {
        Some(r) => r,
        None => {
            error!("{} job receiver is None, exit", trigger.category());
            return;
        }
    };
    let mut stream = ReceiverStream::new(receiver);
    let mut interval = interval(Duration::from_secs(42));
    let mut retry_attempt = 0;
    let mut retry_delay = None;
    loop {
        tokio::select! {
           msg = stream.next() => {
                if msg.is_none() {
                    error!("job receiver closed");
                    break;
                }
            }
            _ = interval.tick() => {}
            _ = async {
                match retry_delay {
                    Some(delay) => sleep(delay).await,
                    None => pending().await,
                }
            } => {}
        }
        let retry = match trigger.trigger().await {
            Ok(retry) => retry,
            Err(err) => {
                error!("failed to trigger job {}: {:?}", trigger.category(), err);
                true
            }
        };
        if retry {
            let delay = retry_backoff(retry_attempt);
            retry_attempt = retry_attempt.saturating_add(1);
            retry_delay = Some(delay);
            info!("retry {} job in {:?}", trigger.category(), delay);
        } else {
            retry_attempt = 0;
            retry_delay = None;
        }
    }
}

trait JobTrigger {
    async fn trigger(&self) -> Result<bool>;
    fn category(&self) -> JobCategory;
}

fn retry_backoff(attempt: u32) -> Duration {
    Duration::from_secs(1 << attempt.min(5))
}

fn is_terminal_result(result: &MessageResult) -> bool {
    result.success || matches!(result.error_code, Some(BAD_DATA | FORBIDDEN))
}

pub(crate) fn sanitize_transcript_app_card(
    transcript: &mut crate::db::mixin::transcript_message::TranscriptMessage,
) {
    if !transcript.category.is_app_card() {
        return;
    }
    let Some(content) = transcript.content.as_mut() else {
        return;
    };
    let Ok(mut card) = serde_json::from_str::<serde_json::Value>(content) else {
        return;
    };
    let action_is_empty = card["action"].as_str().unwrap_or_default().is_empty();
    if action_is_empty
        && card["actions"].as_array().is_some_and(|actions| {
            actions
                .iter()
                .any(|action| !is_shareable_card_action(action))
        })
    {
        card.as_object_mut().map(|card| card.remove("actions"));
    }
    if action_is_empty {
        card.as_object_mut().map(|card| card.remove("action"));
    }
    if let Ok(sanitized) = serde_json::to_string(&card) {
        *content = sanitized;
    }
}

fn is_shareable_card_action(action: &serde_json::Value) -> bool {
    let Some(action) = action.get("action").and_then(serde_json::Value::as_str) else {
        return false;
    };
    let Ok(uri) = url::Url::parse(action) else {
        return false;
    };
    let mixin_host = matches!(uri.host_str(), Some("mixin.one" | "www.mixin.one"));
    let web_send =
        mixin_host && uri.path_segments().and_then(|mut path| path.next()) == Some("send");
    let mixin_send = uri.scheme() == "mixin" && uri.host_str() == Some("send");
    let has_user = uri
        .query_pairs()
        .any(|(key, value)| key == "user" && !value.trim().is_empty());
    if (web_send || mixin_send) && has_user {
        return true;
    }
    matches!(uri.scheme(), "http" | "https") && !web_send
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex as StdMutex};
    use std::time::Duration;

    use anyhow::anyhow;
    use chrono::Utc;
    use tokio::sync::mpsc;
    use tokio::time::Instant;

    use crate::db::mixin::transcript_message::TranscriptMessage;

    use super::{
        is_terminal_result, run_job, sanitize_transcript_app_card, JobCategory, JobTrigger,
    };
    use crate::core::message::sender::MessageResult;

    struct TestJobTrigger {
        invocations: mpsc::UnboundedSender<Instant>,
        results: Arc<StdMutex<VecDeque<Option<bool>>>>,
    }

    impl JobTrigger for TestJobTrigger {
        async fn trigger(&self) -> anyhow::Result<bool> {
            self.invocations.send(Instant::now()).unwrap();
            match self.results.lock().unwrap().pop_front() {
                Some(Some(retry)) => Ok(retry),
                Some(None) => Err(anyhow!("job failed")),
                None => Ok(false),
            }
        }

        fn category(&self) -> JobCategory {
            JobCategory::Sending
        }
    }

    fn test_trigger(
        results: impl IntoIterator<Item = Option<bool>>,
    ) -> (TestJobTrigger, mpsc::UnboundedReceiver<Instant>) {
        let (invocations, receiver) = mpsc::unbounded_channel();
        (
            TestJobTrigger {
                invocations,
                results: Arc::new(StdMutex::new(results.into_iter().collect())),
            },
            receiver,
        )
    }

    #[tokio::test(start_paused = true)]
    async fn runs_job_immediately_when_signaled() {
        let (sender, receiver) = mpsc::channel(1);
        let (trigger, mut invocations) = test_trigger([]);
        let task = tokio::spawn(run_job(Some(receiver), trigger));

        let started_at = invocations.recv().await.unwrap();
        sender.send(()).await.unwrap();
        let signaled_at = invocations.recv().await.unwrap();

        assert_eq!(signaled_at, started_at);
        task.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn retries_failed_jobs_with_exponential_backoff() {
        let (_sender, receiver) = mpsc::channel(1);
        let (trigger, mut invocations) = test_trigger([None, Some(true), Some(false)]);
        let task = tokio::spawn(run_job(Some(receiver), trigger));

        let first = invocations.recv().await.unwrap();
        let second = invocations.recv().await.unwrap();
        let third = invocations.recv().await.unwrap();

        assert_eq!(second.duration_since(first), Duration::from_secs(1));
        assert_eq!(third.duration_since(second), Duration::from_secs(2));
        task.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn runs_job_on_fallback_interval() {
        let (_sender, receiver) = mpsc::channel(1);
        let (trigger, mut invocations) = test_trigger([]);
        let task = tokio::spawn(run_job(Some(receiver), trigger));

        let first = invocations.recv().await.unwrap();
        let second = invocations.recv().await.unwrap();

        assert_eq!(second.duration_since(first), Duration::from_secs(42));
        task.abort();
    }

    #[test]
    fn treats_forbidden_and_bad_data_as_terminal_delivery_results() {
        for code in [
            sdk::err::error_code::FORBIDDEN,
            sdk::err::error_code::BAD_DATA,
        ] {
            assert!(is_terminal_result(&MessageResult {
                success: false,
                retry: false,
                error_code: Some(code),
            }));
        }
    }

    #[test]
    fn removes_unsafe_actions_from_forwarded_transcript_card() {
        let mut transcript = TranscriptMessage {
            transcript_id: "root".into(),
            message_id: "child".into(),
            user_id: None,
            user_full_name: None,
            category: sdk::message_category::APP_CARD.into(),
            created_at: Utc::now(),
            content: Some(
                r#"{"action":"","actions":[{"action":"https://mixin.one/send?conversation=id"}]}"#
                    .into(),
            ),
            media_url: None,
            media_name: None,
            media_size: None,
            media_width: None,
            media_height: None,
            media_mime_type: None,
            media_duration: None,
            media_status: None,
            media_waveform: None,
            thumb_image: None,
            thumb_url: None,
            media_key: None,
            media_digest: None,
            media_created_at: None,
            sticker_id: None,
            shared_user_id: None,
            mentions: None,
            quote_id: None,
            quote_content: None,
            caption: None,
        };

        sanitize_transcript_app_card(&mut transcript);

        let card: serde_json::Value = serde_json::from_str(&transcript.content.unwrap()).unwrap();
        assert!(card.get("actions").is_none());
    }
}
