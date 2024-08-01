use anyhow::Result;

use sdk::BlazeMessageData;

pub struct MessageSender {}

pub enum ProcessSignalKeyAction {
    AddParticipant,
    RemoveParticipant,
    ResendKey,
}

impl MessageSender {
    pub async fn send_process_signal_key(
        &self,
        data: &BlazeMessageData,
        action: ProcessSignalKeyAction,
        participant_id: Option<&str>,
    ) -> Result<()> {
        todo!()
    }

    pub async fn refresh_session(&self, conversation_id: &str, user_ids: &[String]) -> Result<()> {
        todo!()
    }
}
