import '../models/message_list_entry.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../utils/app_logger.dart';

class MessageAttachmentController {
  const MessageAttachmentController.conversation({
    required this.account,
    required this.onChanged,
  }) : transcriptId = null,
       transcriptSentByCurrentUser = false;

  const MessageAttachmentController.transcript({
    required this.account,
    required this.transcriptId,
    required bool sentByCurrentUser,
    required this.onChanged,
  }) : transcriptSentByCurrentUser = sentByCurrentUser;

  final rust.AccountHandle account;
  final String? transcriptId;
  final bool transcriptSentByCurrentUser;
  final Future<void> Function(MessageListEntry message) onChanged;

  Future<void> download(MessageListEntry message) async {
    try {
      final transcriptId = this.transcriptId;
      if (transcriptId != null) {
        if (transcriptSentByCurrentUser) {
          await account.attachment().retryTranscriptAttachment(
            transcriptId: transcriptId,
          );
        } else {
          await account.attachment().downloadTranscriptAttachment(
            transcriptId: transcriptId,
            messageId: message.id,
          );
        }
      } else if (message.senderRelationship.toUpperCase() == 'ME') {
        await account.attachment().retryAttachment(messageId: message.id);
      } else {
        await account.attachment().downloadAttachment(messageId: message.id);
      }
      await onChanged(message);
    } on Object catch (error, stackTrace) {
      e('Download message attachment failed: ${message.id}', error, stackTrace);
    }
  }

  Future<void> cancel(MessageListEntry message) async {
    try {
      final transcriptId = this.transcriptId;
      if (transcriptId == null) {
        await account.attachment().cancelAttachment(messageId: message.id);
      } else {
        await account.attachment().cancelTranscriptAttachment(
          transcriptId: transcriptId,
          messageId: message.id,
        );
      }
      await onChanged(message);
    } on Object catch (error, stackTrace) {
      e('Cancel message attachment failed: ${message.id}', error, stackTrace);
    }
  }

  Future<void> markAudioRead(MessageListEntry message) async {
    if (message.mediaStatus.toUpperCase() != 'DONE') return;
    try {
      final transcriptId = this.transcriptId;
      if (transcriptId == null) {
        await account.attachment().markAudioRead(messageId: message.id);
      } else {
        await account.attachment().markTranscriptAudioRead(
          transcriptId: transcriptId,
          messageId: message.id,
        );
      }
      await onChanged(message);
    } on Object catch (error, stackTrace) {
      e('Mark message audio read failed: ${message.id}', error, stackTrace);
    }
  }
}
