use mixin_desktop_core::runtime::AttachmentAccess as CoreAttachmentAccess;

use crate::ClientError;

pub struct AttachmentAccess {
    inner: CoreAttachmentAccess,
}

impl From<CoreAttachmentAccess> for AttachmentAccess {
    fn from(inner: CoreAttachmentAccess) -> Self {
        Self { inner }
    }
}

impl AttachmentAccess {
    /// Reads a downloaded Markdown file within the current account and optional transcript.
    pub async fn markdown_content(
        &self,
        message_id: String,
        transcript_id: Option<String>,
    ) -> crate::ClientResult<String> {
        Ok(self
            .inner
            .markdown_content(message_id, transcript_id)
            .await?)
    }

    /// Resumes upload or download using the persisted parent message state.
    pub async fn resume_attachment(
        &self,
        message_id: String,
        transcript_id: Option<String>,
    ) -> crate::ClientResult<()> {
        Ok(self
            .inner
            .resume_attachment(message_id, transcript_id)
            .await?)
    }

    pub async fn save_attachment_as(
        &self,
        message_id: String,
        destination: String,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .save_attachment_as(message_id, destination)
            .await?)
    }

    pub async fn retry_attachment(&self, message_id: String) -> Result<(), ClientError> {
        Ok(self.inner.retry_attachment(message_id).await?)
    }

    /// Saves a downloaded child selected within its persisted transcript root.
    pub async fn save_transcript_attachment_as(
        &self,
        transcript_id: String,
        message_id: String,
        destination: String,
    ) -> crate::ClientResult<()> {
        Ok(self
            .inner
            .save_transcript_attachment_as(transcript_id, message_id, destination)
            .await?)
    }

    /// Reads the current account image as a bounded, normalized PNG data URI.
    pub async fn attachment_image_data_uri(
        &self,
        message_id: String,
        transcript_id: Option<String>,
    ) -> crate::ClientResult<String> {
        Ok(self
            .inner
            .attachment_image_data_uri(message_id, transcript_id)
            .await?)
    }

    pub async fn download_transcript_attachment(
        &self,
        transcript_id: String,
        message_id: String,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .download_transcript_attachment(transcript_id, message_id)
            .await?)
    }

    pub async fn retry_transcript_attachment(
        &self,
        transcript_id: String,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .retry_transcript_attachment(transcript_id)
            .await?)
    }

    pub async fn cancel_transcript_attachment(
        &self,
        transcript_id: String,
        message_id: String,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .cancel_transcript_attachment(transcript_id, message_id)
            .await?)
    }

    pub async fn mark_transcript_audio_read(
        &self,
        transcript_id: String,
        message_id: String,
    ) -> Result<(), ClientError> {
        Ok(self
            .inner
            .mark_transcript_audio_read(transcript_id, message_id)
            .await?)
    }

    pub async fn download_attachment(&self, message_id: String) -> Result<(), ClientError> {
        Ok(self.inner.download_attachment(message_id).await?)
    }

    pub async fn cancel_attachment(&self, message_id: String) -> Result<(), ClientError> {
        Ok(self.inner.cancel_attachment(message_id).await?)
    }

    pub async fn mark_audio_read(&self, message_id: String) -> Result<(), ClientError> {
        Ok(self.inner.mark_audio_read(message_id).await?)
    }
}
