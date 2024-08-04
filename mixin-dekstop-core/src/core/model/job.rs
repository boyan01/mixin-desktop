use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use base64ct::{Base64, Encoding};
use futures::StreamExt;
use log::{error, info, warn};
use strum_macros::Display;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio_stream::wrappers::ReceiverStream;

use sdk::{
    ACKNOWLEDGE_MESSAGE_RECEIPTS, BlazeAckMessage, BlazeMessage, Client, CREATE_MESSAGE,
    PlainJsonMessage,
};
use sdk::err::error_code::BAD_DATA;

use crate::core::constants::TEAM_MIXIN_USER_ID;
use crate::core::message::sender::MessageSender;
use crate::core::util::generate_conversation_id;
use crate::db::mixin::job::{Job, JobDao};
use crate::db::MixinDatabase;

pub struct JobService {
    job_dao: JobDao,
    ack_job_signer: Sender<()>,
    session_ack_job_signer: Sender<()>,
    handle: JoinHandle<()>,
}

impl Drop for JobService {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl JobService {
    pub fn new(
        database: Arc<MixinDatabase>,
        message_sender: Arc<MessageSender>,
        client: Arc<Client>,
        user_id: String,
        primary_session_id: Option<String>,
    ) -> Self {
        let (ack_sender, ack_receiver) = channel(1);
        let (session_ack_sender, session_ack_receiver) = channel(1);

        let params = JobParams {
            database: database.clone(),
            client: client.clone(),
            receiver: HashMap::from([
                (JobCategory::Ack, ack_receiver),
                (JobCategory::SessionAck, session_ack_receiver),
            ]),
            user_id,
            primary_session_id,
            message_sender,
        };
        let handle = tokio::spawn(async move {
            start_all_jobs(params).await;
        });

        JobService {
            job_dao: database.job_dao.clone(),
            ack_job_signer: ack_sender,
            session_ack_job_signer: session_ack_sender,
            handle,
        }
    }

    pub async fn add(&self, job: &Job) -> Result<()> {
        self.job_dao.insert_job(job).await?;

        let _ = match job.action.as_str() {
            ACKNOWLEDGE_MESSAGE_RECEIPTS => self.ack_job_signer.try_send(()),
            CREATE_MESSAGE => self.session_ack_job_signer.try_send(()),
            _ => {
                error!("unknown job action: {:?}", &job.action);
                Ok(())
            }
        };

        Ok(())
    }
}

#[derive(Debug, Display, Eq, PartialEq, Hash)]
enum JobCategory {
    Ack,
    SessionAck,
}

struct JobParams {
    database: Arc<MixinDatabase>,
    client: Arc<Client>,
    receiver: HashMap<JobCategory, Receiver<()>>,
    user_id: String,
    primary_session_id: Option<String>,
    message_sender: Arc<MessageSender>,
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
            SessionAckJob {
                database: params.database.clone(),
                user_id: params.user_id.clone(),
                sender: params.message_sender,
                primary_session_id: params.primary_session_id,
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
    loop {
        tokio::select! {
           msg = stream.next() => {
                if msg.is_none() {
                    error!("job receiver closed");
                    break;
                }
            }
            _ = interval.tick() => {
                info!("time out");
            }
        }
        if let Err(err) = trigger.trigger().await {
            error!("failed to trigger job {}: {:?}", trigger.category(), err);
        }
    }
}

trait JobTrigger {
    async fn trigger(&self) -> Result<()>;
    fn category(&self) -> JobCategory;
}

struct AckJobRunner {
    database: Arc<MixinDatabase>,
    client: Arc<Client>,
}

struct SessionAckJob {
    database: Arc<MixinDatabase>,
    user_id: String,
    sender: Arc<MessageSender>,
    primary_session_id: Option<String>,
}

impl JobTrigger for AckJobRunner {
    async fn trigger(&self) -> Result<()> {
        let jobs = self.database.job_dao.ack_jobs().await?;
        info!("trigger ack job runner: {:?}", jobs.len());
        let mut job_ids = vec![];
        let mut acks = vec![];
        for job in jobs {
            if let Some(m) = job
                .blaze_message
                .and_then(|m| serde_json::from_str::<BlazeAckMessage>(&m).ok())
            {
                acks.push(m);
            } else {
                error!("AckJobRunner: failed to parse message: {:?}", &job.job_id);
            }
            job_ids.push(job.job_id);
        }

        let result = self.client.message_api.acknowledgements(&acks).await;
        if let Err(err) = result {
            error!("failed to ack messages: {:?}", err);
        } else {
            info!("ack messages success");
            self.database.job_dao.delete_jobs(&job_ids).await?;
        }
        Ok(())
    }

    fn category(&self) -> JobCategory {
        JobCategory::Ack
    }
}

impl JobTrigger for SessionAckJob {
    async fn trigger(&self) -> Result<()> {
        if self.primary_session_id.is_none() {
            warn!("primary session id is None, skip session ack job");
            return Ok(());
        };

        let conversation_id = self
            .database
            .participant_dao
            .find_any_joined_conversation_id(&self.user_id)
            .await?
            .unwrap_or_else(|| {
                generate_conversation_id(&self.user_id, TEAM_MIXIN_USER_ID).to_string()
            });

        let jobs = self.database.job_dao.session_ack_jobs().await?;
        let mut job_ids = vec![];
        let mut acks = vec![];
        for job in jobs {
            if let Some(m) = job
                .blaze_message
                .and_then(|m| serde_json::from_str::<BlazeAckMessage>(&m).ok())
            {
                acks.push(m);
            } else {
                error!("failed to parse message: {:?}", &job.job_id);
            }
            job_ids.push(job.job_id);
        }

        let plain_text = PlainJsonMessage {
            action: ACKNOWLEDGE_MESSAGE_RECEIPTS.to_string(),
            messages: None,
            user_id: None,
            message_id: None,
            session_id: None,
            content: None,
            ack_messages: Some(acks),
        };

        let encoded = serde_json::to_vec(&plain_text)?;
        let encoded = Base64::encode_string(&encoded);

        let bm = BlazeMessage::new_plain_json(
            &conversation_id,
            self.sender.get_check_sum(&conversation_id).await?,
            &self.user_id,
            encoded,
            self.primary_session_id.clone(),
        );

        let result = self.sender.deliver(bm).await?;

        if result.success || result.error_code == Some(BAD_DATA) {
            self.database.job_dao.delete_jobs(&job_ids).await?;
        } else if let Some(err) = result.error_code {
            error!("failed to ack messages, code: {:?}", err);
        }
        Ok(())
    }

    fn category(&self) -> JobCategory {
        JobCategory::SessionAck
    }
}
