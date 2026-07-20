import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:mixin_desktop_ui/controllers/message_controller.dart';
import 'package:mixin_desktop_ui/controllers/voice_recorder_controller.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/utils/app_logger.dart';
import 'package:mixin_desktop_ui/widgets/toast.dart';

final _botGroupCache = <String, ({bool value, DateTime expiresAt})>{};
final _botGroupLoads = <String, Future<bool>>{};

class MessageActionController extends ChangeNotifier {
  MessageActionController({
    required this.account,
    required this.conversation,
    required this.messageController,
  }) : isBotGroup = conversation.isBotGroup {
    i(
      'Open conversation: conversation_id=${conversation.id}, '
      'category=${conversation.category}',
    );
    messageController.addListener(_onMessagesChanged);
    unawaited(_loadMetadata());
    unawaited(_loadBotGroup());
    _changeSubscription = account.conversationChanges().listen(
      (event) {
        if (event.reloadAll ||
            event.conversationIds.contains(conversation.id)) {
          changeRevision++;
          _scheduleMetadataRefresh();
        }
      },
      onError: (Object exception, StackTrace stackTrace) {
        e(
          'Message change stream failed: conversation_id=${conversation.id}',
          exception,
          stackTrace,
        );
      },
    );
  }

  final rust.AccountHandle account;
  final ConversationListEntry conversation;
  final MessageController messageController;

  List<MessageListEntry> get messages => messageController.state.list;
  List<String> pinnedMessageIds = const [];
  rust.PinMessagePreviewItem? pinMessagePreview;
  List<String> unreadMentionMessageIds = const [];
  bool sending = false;
  bool forwarding = false;
  String? currentUserRole;
  bool isBotGroup;
  int changeRevision = 0;

  StreamSubscription<rust.ConversationChangeEvent>? _changeSubscription;
  bool _refreshingMetadata = false;
  bool _metadataRefreshPending = false;
  bool _disposed = false;
  final Set<String> _markingMentionRead = {};
  final Set<String> _markedMentionRead = {};
  final Map<String, String> _recalledText = {};
  final Map<String, Timer> _recalledTextTimers = {};

  String? recalledText(String messageId) => _recalledText[messageId];

  Future<void> _loadBotGroup() async {
    if (!conversation.isBot) return;
    final cached = _botGroupCache[conversation.id];
    if (cached != null && cached.expiresAt.isAfter(DateTime.now())) {
      isBotGroup = cached.value;
      return;
    }
    try {
      final value = await _botGroupLoads.putIfAbsent(
        conversation.id,
        () =>
            account.conversation().isBotGroup(conversationId: conversation.id),
      );
      _botGroupCache[conversation.id] = (
        value: value,
        expiresAt: DateTime.now().add(const Duration(minutes: 10)),
      );
      if (_disposed || isBotGroup == value) return;
      isBotGroup = value;
      notifyListeners();
    } catch (exception, stackTrace) {
      e(
        'Load bot group failed: conversation_id=${conversation.id}',
        exception,
        stackTrace,
      );
    } finally {
      _botGroupLoads.remove(conversation.id);
    }
  }

  void _onMessagesChanged() {
    notifyListeners();
  }

  Future<bool> sendText(
    String content, {
    String? quoteMessageId,
    bool silent = false,
  }) async {
    final text = content.trim();
    if (text.isEmpty) return false;
    try {
      final messageId = await account.message().sendText(
        conversationId: conversation.id,
        content: text,
        quoteMessageId: quoteMessageId,
        silent: silent,
      );
      await messageController.refreshMessages([messageId]);
      return true;
    } catch (exception, stackTrace) {
      e(
        'Send text failed: conversation_id=${conversation.id}',
        exception,
        stackTrace,
      );
      rethrow;
    }
  }

  Future<bool> sendPost(String content) async {
    final text = content.trim();
    if (text.isEmpty) return false;
    try {
      final messageId = await account.message().sendPost(
        conversationId: conversation.id,
        content: text,
      );
      await messageController.refreshMessages([messageId]);
      return true;
    } catch (exception, stackTrace) {
      e(
        'Send post failed: conversation_id=${conversation.id}',
        exception,
        stackTrace,
      );
      rethrow;
    }
  }

  Future<bool> sendAudio({
    required String path,
    required Duration duration,
    required List<int> waveform,
    String? quoteMessageId,
  }) async {
    if (sending ||
        duration <= Duration.zero ||
        duration > maxVoiceRecordingDuration ||
        waveform.isEmpty) {
      return false;
    }
    sending = true;
    notifyListeners();
    try {
      final messageId = await account.message().sendAudio(
        conversationId: conversation.id,
        path: path,
        durationMillis: duration.inMilliseconds,
        waveform: waveform,
        quoteMessageId: quoteMessageId,
      );
      await messageController.refreshMessages([messageId]);
      return true;
    } catch (exception, stackTrace) {
      e(
        'Send audio failed: conversation_id=${conversation.id}',
        exception,
        stackTrace,
      );
      showToastFailed(exception);
      return false;
    } finally {
      if (!_disposed) {
        sending = false;
        notifyListeners();
      }
    }
  }

  Future<bool> sendAttachment({
    required String path,
    required String kind,
    required String mimeType,
    String? name,
    int? width,
    int? height,
    int? durationMillis,
    String? thumbnail,
    String? caption,
    String? quoteMessageId,
    bool silent = false,
  }) async {
    if (sending || path.trim().isEmpty || mimeType.trim().isEmpty) {
      return false;
    }
    sending = true;
    notifyListeners();
    try {
      final messageId = await account.message().sendAttachment(
        conversationId: conversation.id,
        path: path,
        kind: kind,
        mimeType: mimeType,
        name: name,
        width: width,
        height: height,
        durationMillis: durationMillis,
        thumbnail: thumbnail,
        caption: caption,
        quoteMessageId: quoteMessageId,
        silent: silent,
      );
      await messageController.refreshMessages([messageId]);
      return true;
    } catch (exception, stackTrace) {
      e(
        'Send attachment failed: conversation_id=${conversation.id}',
        exception,
        stackTrace,
      );
      showToastFailed(exception);
      return false;
    } finally {
      if (!_disposed) {
        sending = false;
        notifyListeners();
      }
    }
  }

  Future<bool> sendSticker({required String stickerId}) async {
    if (sending || stickerId.trim().isEmpty) return false;
    sending = true;
    notifyListeners();
    try {
      final messageId = await account.message().sendSticker(
        conversationId: conversation.id,
        stickerId: stickerId,
      );
      await messageController.refreshMessages([messageId]);
      return true;
    } catch (exception, stackTrace) {
      e(
        'Send sticker failed: conversation_id=${conversation.id}',
        exception,
        stackTrace,
      );
      return false;
    } finally {
      if (!_disposed) {
        sending = false;
        notifyListeners();
      }
    }
  }

  Future<bool> forwardMessages(
    Iterable<MessageListEntry> messages,
    String targetConversationId,
  ) async {
    final selected = messages.toList(growable: false);
    if (selected.isEmpty || forwarding) return false;
    forwarding = true;
    notifyListeners();
    try {
      await account.message().forwardMessages(
        targetConversationId: targetConversationId,
        sourceMessageIds: selected.map((message) => message.id).toList(),
      );
      return true;
    } catch (exception, stackTrace) {
      e(
        'Forward messages failed: target_conversation_id=$targetConversationId',
        exception,
        stackTrace,
      );
      return false;
    } finally {
      if (!_disposed) {
        forwarding = false;
        notifyListeners();
      }
    }
  }

  Future<bool> combineForwardMessages(
    Iterable<MessageListEntry> messages,
    String targetConversationId,
  ) async {
    final selected = messages.toList(growable: false);
    if (selected.length < 2 || selected.length >= 100 || forwarding) {
      return false;
    }
    forwarding = true;
    notifyListeners();
    try {
      await account.message().combineForwardMessages(
        targetConversationId: targetConversationId,
        sourceMessageIds: selected.map((message) => message.id).toList(),
      );
      return true;
    } catch (exception, stackTrace) {
      e(
        'Combine forward messages failed: '
        'target_conversation_id=$targetConversationId',
        exception,
        stackTrace,
      );
      return false;
    } finally {
      if (!_disposed) {
        forwarding = false;
        notifyListeners();
      }
    }
  }

  Future<void> setMessagePinned(MessageListEntry message, bool pinned) async {
    try {
      await account.message().setMessagePinned(
        conversationId: message.conversationId,
        messageId: message.id,
        pinned: pinned,
      );
      await messageController.refreshMessages([message.id]);
      await _refreshMetadata();
    } catch (exception, stackTrace) {
      e(
        'Set message pinned failed: conversation_id=${conversation.id}, '
        'message_id=${message.id}',
        exception,
        stackTrace,
      );
      rethrow;
    }
  }

  Future<void> recallMessages(Iterable<MessageListEntry> messages) async {
    final selected = messages.toList(growable: false);
    if (selected.isEmpty) return;
    try {
      await account.message().recallMessages(
        conversationId: conversation.id,
        messageIds: selected.map((message) => message.id).toList(),
      );
      for (final message in selected.where((message) => message.isText)) {
        _recalledText[message.id] = message.content;
        _recalledTextTimers.remove(message.id)?.cancel();
        _recalledTextTimers[message.id] = Timer(const Duration(minutes: 6), () {
          _recalledText.remove(message.id);
          _recalledTextTimers.remove(message.id);
          if (!_disposed) notifyListeners();
        });
      }
      await messageController.refreshMessages(
        selected.map((message) => message.id),
      );
    } catch (exception, stackTrace) {
      e(
        'Recall messages failed: conversation_id=${conversation.id}',
        exception,
        stackTrace,
      );
      rethrow;
    }
  }

  Future<void> deleteMessages(Iterable<MessageListEntry> messages) async {
    final selected = messages.toList(growable: false);
    if (selected.isEmpty) return;
    try {
      await account.message().deleteMessages(
        conversationId: conversation.id,
        messageIds: selected.map((message) => message.id).toList(),
      );
      messageController.removeMessages(selected.map((message) => message.id));
      await _refreshMetadata();
    } catch (exception, stackTrace) {
      e(
        'Delete messages failed: conversation_id=${conversation.id}',
        exception,
        stackTrace,
      );
      rethrow;
    }
  }

  Future<void> markMentionRead(MessageListEntry message) async {
    if (message.mentionRead != false) return;
    await markMentionReadById(message.id);
  }

  Future<void> markMentionReadById(String messageId) async {
    if (_markingMentionRead.contains(messageId) ||
        _markedMentionRead.contains(messageId)) {
      return;
    }
    _markingMentionRead.add(messageId);
    try {
      await account.message().markMentionRead(
        conversationId: conversation.id,
        messageId: messageId,
      );
      _markedMentionRead.add(messageId);
      unreadMentionMessageIds = unreadMentionMessageIds
          .where((id) => id != messageId)
          .toList(growable: false);
      if (!_disposed) notifyListeners();
    } catch (exception, stackTrace) {
      e(
        'Mark mention read failed: conversation_id=${conversation.id}, '
        'message_id=$messageId',
        exception,
        stackTrace,
      );
    } finally {
      _markingMentionRead.remove(messageId);
    }
  }

  Future<void> downloadAttachment(MessageListEntry message) async {
    try {
      if (message.senderRelationship.toUpperCase() == 'ME') {
        await account.attachment().retryAttachment(messageId: message.id);
      } else {
        await account.attachment().downloadAttachment(messageId: message.id);
      }
      await messageController.refreshMessages([message.id]);
    } catch (exception, stackTrace) {
      e(
        'Download attachment failed: conversation_id=${conversation.id}, '
        'message_id=${message.id}',
        exception,
        stackTrace,
      );
      rethrow;
    }
  }

  Future<void> cancelAttachment(MessageListEntry message) async {
    try {
      await account.attachment().cancelAttachment(messageId: message.id);
      await messageController.refreshMessages([message.id]);
    } catch (exception, stackTrace) {
      e(
        'Cancel attachment failed: conversation_id=${conversation.id}, '
        'message_id=${message.id}',
        exception,
        stackTrace,
      );
      rethrow;
    }
  }

  Future<void> markAudioRead(MessageListEntry message) async {
    if (message.mediaStatus.toUpperCase() != 'DONE') return;
    try {
      await account.attachment().markAudioRead(messageId: message.id);
      await messageController.refreshMessages([message.id]);
    } catch (exception, stackTrace) {
      e(
        'Mark audio read failed: conversation_id=${conversation.id}, '
        'message_id=${message.id}',
        exception,
        stackTrace,
      );
    }
  }

  Future<void> addSticker(MessageListEntry message) async {
    final stickerId = message.stickerId?.trim();
    if (stickerId == null || stickerId.isEmpty) {
      throw ArgumentError('Sticker message has no sticker id');
    }
    try {
      await account.sticker().addSticker(stickerId: stickerId);
    } catch (exception, stackTrace) {
      e(
        'Add sticker failed: conversation_id=${conversation.id}, '
        'message_id=${message.id}',
        exception,
        stackTrace,
      );
      rethrow;
    }
  }

  Future<void> addImageAsSticker(MessageListEntry message) async {
    try {
      await account.sticker().addStickerFromFile(messageId: message.id);
    } catch (exception, stackTrace) {
      e(
        'Add image as sticker failed: conversation_id=${conversation.id}, '
        'message_id=${message.id}',
        exception,
        stackTrace,
      );
      rethrow;
    }
  }

  Future<String?> handleStrangerAction(
    MessageListEntry message,
    String action,
  ) async {
    try {
      switch (action) {
        case 'block':
          await account.user().blockUser(userId: message.senderId);
          await _refreshLatest();
          break;
        case 'add_contact':
          await account.user().addContact(
            userId: message.senderId,
            fullName: message.senderName,
          );
          await _refreshLatest();
          break;
        case 'say_hi':
          final messageId = await account.message().sendText(
            conversationId: conversation.id,
            content: 'Hi',
            quoteMessageId: null,
            silent: false,
          );
          await messageController.refreshMessages([messageId]);
          break;
        case 'open_home':
          final appId = message.senderAppId?.trim();
          if (appId == null || appId.isEmpty) return null;
          return account.user().botHomeUri(appId: appId);
        default:
          return null;
      }
      return null;
    } catch (exception, stackTrace) {
      e(
        'Handle stranger action failed: conversation_id=${conversation.id}, '
        'action=$action',
        exception,
        stackTrace,
      );
      rethrow;
    }
  }

  Future<void> _loadMetadata() async {
    try {
      final roleFuture = account.conversation().currentUserRole(
        conversationId: conversation.id,
      );
      final pinnedFuture = _loadPinnedMessages();
      final pinPreviewFuture = _loadPinMessagePreview();
      final unreadMentionsFuture = _loadUnreadMentionMessageIds();
      currentUserRole = await roleFuture;
      pinnedMessageIds = await pinnedFuture;
      pinMessagePreview = await pinPreviewFuture;
      unreadMentionMessageIds = await unreadMentionsFuture;
      if (_disposed) return;
      notifyListeners();
    } catch (exception, stackTrace) {
      e(
        'Load conversation metadata failed: '
        'conversation_id=${conversation.id}, category=${conversation.category}',
        exception,
        stackTrace,
      );
    }
  }

  Future<List<String>> _loadPinnedMessages() =>
      account.message().pinnedMessageIds(conversationId: conversation.id);

  Future<rust.PinMessagePreviewItem?> _loadPinMessagePreview() =>
      account.message().pinMessagePreview(conversationId: conversation.id);

  Future<List<String>> _loadUnreadMentionMessageIds() => account
      .message()
      .unreadMentionMessageIds(conversationId: conversation.id);

  void _scheduleMetadataRefresh() {
    if (_refreshingMetadata) {
      _metadataRefreshPending = true;
      return;
    }
    unawaited(_refreshMetadata());
  }

  Future<void> _refreshLatest() async {
    messageController.loadLatestWindow();
    await _refreshMetadata();
  }

  Future<void> _refreshMetadata() async {
    if (_refreshingMetadata) {
      _metadataRefreshPending = true;
      return;
    }
    _refreshingMetadata = true;
    do {
      _metadataRefreshPending = false;
      try {
        await _loadMetadata();
      } catch (exception, stackTrace) {
        e(
          'Refresh conversation metadata failed: '
          'conversation_id=${conversation.id}',
          exception,
          stackTrace,
        );
      }
    } while (_metadataRefreshPending && !_disposed);
    _refreshingMetadata = false;
  }

  @override
  void dispose() {
    _disposed = true;
    messageController.removeListener(_onMessagesChanged);
    unawaited(_changeSubscription?.cancel());
    for (final timer in _recalledTextTimers.values) {
      timer.cancel();
    }
    super.dispose();
  }
}
