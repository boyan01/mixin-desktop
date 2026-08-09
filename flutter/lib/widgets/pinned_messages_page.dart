import 'dart:async';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:super_context_menu/super_context_menu.dart';

import '../controllers/message_attachment_controller.dart';
import '../controllers/settings_controller.dart';
import '../l10n/l10n.dart';
import '../models/conversation_list_entry.dart';
import '../models/message_list_entry.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../theme.dart';
import '../utils/app_logger.dart';
import 'avatar_view.dart';
import 'custom_context_menu.dart';
import 'interactive_decorated_box.dart';
import 'message_action_policy.dart';
import 'message_actions_menu.dart';
import 'message_bubble.dart';
import 'message_content.dart';
import 'message_datetime_and_status.dart';
import 'message_day_time.dart';
import 'message_items/quote_message_preview.dart';
import 'message_name.dart';
import 'message_navigation_scope.dart';
import 'message_presentation.dart';
import 'message_rows.dart';
import 'message_selectable_text.dart';
import 'mixin_dialog.dart';
import 'show_message_user_dialog.dart';

class PinnedMessagesPage extends StatefulWidget {
  const PinnedMessagesPage({
    required this.account,
    required this.conversationId,
    required this.currentUserId,
    required this.currentUserRole,
    required this.onLocate,
    required this.onSelectConversation,
    required this.onSelectConversationInfo,
    super.key,
    this.onEmpty,
    this.onCountChanged,
  });

  final rust.AccountHandle account;
  final String conversationId;
  final String currentUserId;
  final String? currentUserRole;
  final ValueChanged<String> onLocate;
  final ValueChanged<ConversationListEntry> onSelectConversation;
  final ValueChanged<ConversationListEntry> onSelectConversationInfo;
  final VoidCallback? onEmpty;
  final ValueChanged<int>? onCountChanged;

  @override
  State<PinnedMessagesPage> createState() => _PinnedMessagesPageState();
}

class _PinnedMessagesPageState extends State<PinnedMessagesPage> {
  List<MessageListEntry> _messages = const [];
  final Map<String, GlobalKey> _messageKeys = {};
  final Map<String, GlobalKey> _dayTimeKeys = {};
  StreamSubscription<BigInt>? _changes;
  bool _loading = true;
  bool _refreshing = false;
  bool _refreshPending = false;
  Map<String, String> _mentionNames = const {};

  @override
  void initState() {
    super.initState();
    unawaited(_refresh());
    _changes = widget.account.messageChanges().listen(
      (_) => unawaited(_refresh()),
      onError: (Object error) {
        e('watch pinned messages failed', error);
      },
    );
  }

  @override
  void dispose() {
    unawaited(_changes?.cancel());
    super.dispose();
  }

  Future<void> _refresh() async {
    if (_refreshing) {
      _refreshPending = true;
      return;
    }
    _refreshing = true;
    do {
      _refreshPending = false;
      try {
        final result = await widget.account.message().pinnedMessages(
          conversationId: widget.conversationId,
        );
        final messages = result.map(MessageListEntry.fromRust).toList();
        final mentionNames = await _resolveMentionNames(messages);
        if (!mounted) return;
        setState(() {
          _messages = messages;
          _mentionNames = mentionNames;
          _loading = false;
        });
        widget.onCountChanged?.call(_messages.length);
        if (_messages.isEmpty) widget.onEmpty?.call();
      } on Object catch (error, stackTrace) {
        if (!mounted) return;
        e('Refresh pinned messages failed', error, stackTrace);
        setState(() => _loading = false);
      }
    } while (_refreshPending && mounted);
    _refreshing = false;
  }

  Future<Map<String, String>> _resolveMentionNames(
    Iterable<MessageListEntry> messages,
  ) async {
    final identityNumbers = messageMentionIdentityNumbers(
      messages.expand((message) => [message.content, message.caption]),
    );
    if (identityNumbers.isEmpty) return const {};
    final users = await widget.account.user().usersByIdentityNumbers(
      identityNumbers: identityNumbers.toList(growable: false),
    );
    return Map.unmodifiable({
      for (final user in users)
        if (user.fullName.trim().isNotEmpty)
          user.identityNumber: user.fullName.trim(),
    });
  }

  void _locate(String messageId) {
    widget.onLocate(messageId);
  }

  Future<void> _unpin(MessageListEntry message) async {
    try {
      await widget.account.message().setMessagePinned(
        conversationId: widget.conversationId,
        messageId: message.id,
        pinned: false,
      );
      await _refresh();
      if (mounted && _messages.isEmpty) widget.onEmpty?.call();
    } on Object catch (error, stackTrace) {
      e('Unpin message failed', error, stackTrace);
    }
  }

  Future<void> _unpinAll() async {
    final confirmed = await showConfirmMixinDialog(
      context,
      context.l10n.unpinAllMessagesConfirmation,
    );
    if (confirmed != DialogEvent.positive) return;
    try {
      for (final message in _messages) {
        await widget.account.message().setMessagePinned(
          conversationId: widget.conversationId,
          messageId: message.id,
          pinned: false,
        );
      }
      await _refresh();
    } on Object catch (error, stackTrace) {
      e('Unpin all messages failed', error, stackTrace);
    }
  }

  @override
  Widget build(BuildContext context) => Material(
    color: context.theme.popUp,
    child: Column(
      children: [
        Expanded(
          child: Provider<rust.AccountHandle>.value(
            value: widget.account,
            child: MessageNavigationScope(
              currentUserId: widget.currentUserId,
              onLocateMessage: _locate,
              onSelectConversation: widget.onSelectConversation,
              onSelectConversationInfo: widget.onSelectConversationInfo,
              child: Provider<MessageAttachmentController>(
                create: (_) => MessageAttachmentController.conversation(
                  account: widget.account,
                  onChanged: (_) => _refresh(),
                ),
                child: _body(),
              ),
            ),
          ),
        ),
        InteractiveDecoratedBox(
          cursor: SystemMouseCursors.click,
          onTap: _unpinAll,
          child: SizedBox(
            height: 56,
            child: Center(
              child: Text(
                context.l10n.unpinAllMessages,
                style: TextStyle(color: context.theme.accent, fontSize: 16),
              ),
            ),
          ),
        ),
      ],
    ),
  );

  Widget _body() {
    if (_loading || _messages.isEmpty) return const SizedBox();
    return MessageDayTimeViewportWidget(
      entries: [
        for (var index = _messages.length - 1; index >= 0; index--)
          MessageDayTimeViewportEntry(
            dateTime: _messages[index].createdAt,
            messageKey: _messageKey(_messages[index].id),
            dayTimeKey:
                MessageRowModel(
                      message: _messages[index],
                      previous: index + 1 == _messages.length
                          ? null
                          : _messages[index + 1],
                      next: index == 0 ? null : _messages[index - 1],
                    ).dateTime ==
                    null
                ? null
                : _dayTimeKey(_messages[index].id),
          ),
      ],
      reTraversalKey: Object.hashAll(_messages.map((message) => message.id)),
      child: ListView.builder(
        reverse: true,
        padding: const EdgeInsets.only(bottom: 16),
        itemCount: _messages.length,
        itemBuilder: (context, index) {
          final previous = index + 1 == _messages.length
              ? null
              : _messages[index + 1];
          final next = index == 0 ? null : _messages[index - 1];
          final row = MessageRowModel(
            message: _messages[index],
            previous: previous,
            next: next,
          );
          return _PinnedMessage(
            key: _messageKey(_messages[index].id),
            dayTimeKey: row.dateTime == null
                ? null
                : _dayTimeKey(_messages[index].id),
            account: widget.account,
            message: _messages[index],
            previous: previous,
            next: next,
            messages: _messages,
            currentUserId: widget.currentUserId,
            currentUserRole: widget.currentUserRole,
            mentionNames: _mentionNames,
            onUnpin: _unpin,
          );
        },
      ),
    );
  }

  GlobalKey _messageKey(String messageId) => _messageKeys.putIfAbsent(
    messageId,
    () => GlobalKey(debugLabel: 'pinned_message_$messageId'),
  );

  GlobalKey _dayTimeKey(String messageId) => _dayTimeKeys.putIfAbsent(
    messageId,
    () => GlobalKey(debugLabel: 'pinned_message_day_time_$messageId'),
  );
}

class _PinnedMessage extends StatelessWidget {
  const _PinnedMessage({
    required this.dayTimeKey,
    required this.account,
    required this.message,
    required this.previous,
    required this.next,
    required this.messages,
    required this.currentUserId,
    required this.currentUserRole,
    required this.mentionNames,
    required this.onUnpin,
    super.key,
  });

  final GlobalKey? dayTimeKey;
  final rust.AccountHandle account;
  final MessageListEntry message;
  final MessageListEntry? previous;
  final MessageListEntry? next;
  final List<MessageListEntry> messages;
  final String currentUserId;
  final String? currentUserRole;
  final Map<String, String> mentionNames;
  final ValueChanged<MessageListEntry> onUnpin;

  @override
  Widget build(BuildContext context) {
    final row = MessageRowModel(
      message: message,
      previous: previous,
      next: next,
    );
    final presentation = MessagePresentation.fromRow(
      row: row,
      isGroupOrBotGroupConversation: true,
      enableShowAvatar: context.watch<SettingsController>().messageShowAvatar,
    );
    final status = MessageDatetimeAndStatus(
      message: message,
      isCurrentUser: presentation.isCurrentUser,
      hideStatus: true,
    );
    final content = MessageContent(
      message: message,
      isCurrentUser: presentation.isCurrentUser,
      currentUserId: currentUserId,
      dateAndStatus: status,
      overlayDateAndStatus: MessageDatetimeAndStatus(
        message: message,
        isCurrentUser: presentation.isCurrentUser,
        hideStatus: true,
        color: Colors.white,
      ),
      showNip: presentation.showNip,
      isPinnedPage: true,
      audioPlaylist: messages,
      mentionNames: mentionNames,
    );
    final bypassBubble =
        !message.isUnresolvedMessage &&
        !message.isInvalidSpecialMessage &&
        const {'APP_BUTTON_GROUP', 'APP_CARD'}.contains(message.category);
    final rendered = bypassBubble
        ? content
        : MessageBubble(
            isCurrentUser: presentation.isCurrentUser,
            showNip: presentation.showNip,
            showBubble: message.showMessageBubble,
            includeNip: message.includeMessageBubbleNip,
            clip: message.clipMessageBubble,
            padding: message.messageBubblePadding,
            forceIsCurrentUserColor: message.forceCurrentMessageBubbleColor,
            isDisappearingMessage: (message.expireIn ?? 0) > 0,
            isPinnedPage: true,
            quote: buildMessageQuotePreview(
              message,
              mentionNames: mentionNames,
            ),
            constrainQuoteWidth:
                message.isImage || message.isVideo || message.isLive,
            pinnedMessageId: message.id,
            outerTimeAndStatusWidget: message.useOuterMessageDateAndStatus
                ? status
                : null,
            child: content,
          );
    final policy = MessageActionPolicy(
      message: message,
      currentUserId: currentUserId,
      currentUserRole: currentUserRole,
      now: DateTime.now(),
      isPinnedPage: true,
    );
    final interactive = ContextMenuWidget(
      desktopMenuWidgetBuilder: CustomDesktopMenuWidgetBuilder(),
      menuProvider: (_) => buildMessageActionsMenu(
        context: context,
        message: message,
        policy: policy,
        onLocateToChat: () => MessageNavigationScope.maybeOf(
          context,
        )?.onLocateMessage(message.id),
        onTogglePin: () => onUnpin(message),
      ),
      child: rendered,
    );
    final column = Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (presentation.showSender)
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Flexible(
                child: MessageName(
                  userName: message.senderName,
                  userId: message.senderId,
                  userIdentityNumber: message.senderIdentityNumber,
                  verified: message.senderIsVerified,
                  isBot: message.senderIsBot,
                  membership: message.senderMembership,
                  showIdentityNumber: false,
                  onTap: () => openMessageUserDialog(
                    context,
                    userId: message.senderId,
                  ),
                ),
              ),
            ],
          ),
        interactive,
      ],
    );
    final child = presentation.showSender && presentation.showAvatar
        ? Row(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const SizedBox(width: 8),
              InteractiveDecoratedBox(
                cursor: SystemMouseCursors.click,
                onTap: () => openMessageUserDialog(
                  context,
                  userId: message.senderId,
                ),
                child: AvatarView(
                  userId: message.senderId,
                  name: message.senderName,
                  avatarUrl: message.senderAvatarUrl,
                  size: 32,
                ),
              ),
              Flexible(
                child: Padding(
                  padding: const EdgeInsets.only(top: 2, bottom: 2),
                  child: column,
                ),
              ),
              const SizedBox(width: 33),
            ],
          )
        : Padding(
            padding: EdgeInsets.only(
              left: presentation.isCurrentUser
                  ? 33
                  : (presentation.showAvatar ? 40 : 16),
              right: presentation.isCurrentUser ? 16 : 33,
              top: 2,
              bottom: 2,
            ),
            child: column,
          );
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (row.dateTime != null)
          MessageDayTime(key: dayTimeKey, dateTime: row.dateTime!),
        Padding(
          padding: row.sameUserPrevious
              ? EdgeInsets.zero
              : const EdgeInsets.only(top: 8),
          child: child,
        ),
      ],
    );
  }
}
