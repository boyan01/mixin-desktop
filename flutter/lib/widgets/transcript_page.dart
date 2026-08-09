import 'dart:async';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';
import 'package:super_context_menu/super_context_menu.dart';

import '../constants/assets.dart';
import '../controllers/message_attachment_controller.dart';
import '../controllers/settings_controller.dart';
import '../l10n/l10n.dart';
import '../models/conversation_list_entry.dart';
import '../models/message_list_entry.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../theme.dart';
import '../utils/app_logger.dart';
import 'action_button.dart';
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

Future<void> showTranscriptDialog(
  BuildContext context, {
  required rust.AccountHandle account,
  required String transcriptId,
  required bool sentByCurrentUser,
  required String currentUserId,
  required ValueChanged<ConversationListEntry> onSelectConversation,
  required ValueChanged<ConversationListEntry> onSelectConversationInfo,
}) => showMixinDialog<void>(
  context: context,
  padding: const EdgeInsets.symmetric(vertical: 80),
  backgroundColor: context.theme.chatBackground,
  child: TranscriptPage(
    account: account,
    transcriptId: transcriptId,
    sentByCurrentUser: sentByCurrentUser,
    currentUserId: currentUserId,
    onSelectConversation: onSelectConversation,
    onSelectConversationInfo: onSelectConversationInfo,
  ),
);

class TranscriptPage extends StatefulWidget {
  const TranscriptPage({
    required this.account,
    required this.transcriptId,
    required this.sentByCurrentUser,
    required this.currentUserId,
    required this.onSelectConversation,
    required this.onSelectConversationInfo,
    super.key,
  });

  final rust.AccountHandle account;
  final String transcriptId;
  final bool sentByCurrentUser;
  final String currentUserId;
  final ValueChanged<ConversationListEntry> onSelectConversation;
  final ValueChanged<ConversationListEntry> onSelectConversationInfo;

  @override
  State<TranscriptPage> createState() => _TranscriptPageState();
}

class _TranscriptPageState extends State<TranscriptPage>
    with SingleTickerProviderStateMixin {
  List<MessageListEntry> _messages = const [];
  final ItemScrollController _scrollController = ItemScrollController();
  final Map<String, GlobalKey> _messageKeys = {};
  final Map<String, GlobalKey> _dayTimeKeys = {};
  StreamSubscription<BigInt>? _changes;
  bool _loading = true;
  bool _refreshing = false;
  bool _refreshPending = false;
  String? _highlightedMessageId;
  double _highlightOpacity = 0;
  late final AnimationController _highlightController;
  Map<String, String> _mentionNames = const {};

  @override
  void initState() {
    super.initState();
    _highlightController =
        AnimationController(
            duration: const Duration(milliseconds: 700),
            vsync: this,
          )
          ..addListener(_onHighlightAnimation)
          ..addStatusListener(_onHighlightStatus);
    unawaited(_refresh());
    _changes = widget.account.messageChanges().listen(
      (_) => unawaited(_refresh()),
      onError: (Object error) {
        e('watch transcript messages failed', error);
      },
    );
  }

  @override
  void dispose() {
    unawaited(_changes?.cancel());
    _highlightController
      ..removeListener(_onHighlightAnimation)
      ..removeStatusListener(_onHighlightStatus)
      ..dispose();
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
        final items = await widget.account.message().transcriptMessages(
          transcriptId: widget.transcriptId,
        );
        final messages = items.map(MessageListEntry.fromRust).toList();
        final mentionNames = await _resolveMentionNames(messages);
        if (!mounted) return;
        setState(() {
          _messages = messages;
          _mentionNames = mentionNames;
          _loading = false;
        });
      } on Object catch (error) {
        if (!mounted) return;
        e('refresh transcript messages failed', error);
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

  Future<void> _locateMessage(String messageId) async {
    final index = _messages.indexWhere((message) => message.id == messageId);
    if (index < 0 || !_scrollController.isAttached) return;
    await _scrollController.scrollTo(
      index: index,
      alignment: 0.5,
      duration: const Duration(milliseconds: 250),
      curve: Curves.easeOut,
    );
    if (!mounted) return;
    setState(() {
      _highlightedMessageId = messageId;
      _highlightOpacity = 1;
    });
    _highlightController.reset();
    unawaited(_highlightController.forward());
  }

  void _onHighlightAnimation() {
    if (!mounted || _highlightedMessageId == null) return;
    final value = _highlightController.value;
    const hold = 5 / 7;
    setState(() {
      _highlightOpacity = value <= hold
          ? 1
          : 1 - ((value - hold) / (1 - hold)).clamp(0.0, 1.0);
    });
  }

  void _onHighlightStatus(AnimationStatus status) {
    if (status != AnimationStatus.completed || !mounted) return;
    setState(() {
      _highlightedMessageId = null;
      _highlightOpacity = 0;
    });
  }

  @override
  Widget build(BuildContext context) => ConstrainedBox(
    constraints: const BoxConstraints(
      maxWidth: 600,
      minWidth: 600,
      minHeight: 800,
    ),
    child: Column(
      children: [
        Padding(
          padding: const EdgeInsets.only(right: 16, left: 16, top: 16),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Align(
                  alignment: Alignment.centerLeft,
                  child: ActionButton(
                    name: MixinAssets.close,
                    color: context.theme.icon,
                    onTap: () => Navigator.pop(context),
                  ),
                ),
              ),
              Expanded(
                child: Align(
                  child: Text(
                    context.l10n.transcript,
                    style: TextStyle(color: context.theme.text, fontSize: 16),
                  ),
                ),
              ),
              const Expanded(child: SizedBox()),
            ],
          ),
        ),
        Expanded(
          child: Provider<rust.AccountHandle>.value(
            value: widget.account,
            child: MessageNavigationScope(
              currentUserId: widget.currentUserId,
              onLocateMessage: _locateMessage,
              onSelectConversation: widget.onSelectConversation,
              onSelectConversationInfo: widget.onSelectConversationInfo,
              child: Provider<MessageAttachmentController>(
                create: (_) => MessageAttachmentController.transcript(
                  account: widget.account,
                  transcriptId: widget.transcriptId,
                  sentByCurrentUser: widget.sentByCurrentUser,
                  onChanged: (_) => _refresh(),
                ),
                child: _buildBody(),
              ),
            ),
          ),
        ),
      ],
    ),
  );

  Widget _buildBody() {
    if (_loading || _messages.isEmpty) return const SizedBox();
    return Column(
      children: [
        Expanded(
          child: MessageDayTimeViewportWidget(
            entries: [
              for (var index = 0; index < _messages.length; index++)
                MessageDayTimeViewportEntry(
                  dateTime: _messages[index].createdAt,
                  messageKey: _messageKey(_messages[index].id),
                  dayTimeKey:
                      MessageRowModel(
                            message: _messages[index],
                            previous: index == 0 ? null : _messages[index - 1],
                            next: index + 1 == _messages.length
                                ? null
                                : _messages[index + 1],
                          ).dateTime ==
                          null
                      ? null
                      : _dayTimeKey(_messages[index].id),
                ),
            ],
            reTraversalKey: Object.hashAll(
              _messages.map((message) => message.id),
            ),
            child: ScrollablePositionedList.builder(
              itemScrollController: _scrollController,
              padding: const EdgeInsets.only(bottom: 16),
              itemCount: _messages.length,
              itemBuilder: (context, index) {
                final previous = index == 0 ? null : _messages[index - 1];
                final next = index + 1 == _messages.length
                    ? null
                    : _messages[index + 1];
                final row = MessageRowModel(
                  message: _messages[index],
                  previous: previous,
                  next: next,
                );
                return _TranscriptMessage(
                  key: _messageKey(_messages[index].id),
                  dayTimeKey: row.dateTime == null
                      ? null
                      : _dayTimeKey(_messages[index].id),
                  account: widget.account,
                  transcriptId: widget.transcriptId,
                  transcriptSentByCurrentUser: widget.sentByCurrentUser,
                  mentionNames: _mentionNames,
                  currentUserId: widget.currentUserId,
                  messages: _messages,
                  message: _messages[index],
                  previous: previous,
                  next: next,
                  highlighted: _highlightedMessageId == _messages[index].id,
                  highlightOpacity: _highlightedMessageId == _messages[index].id
                      ? _highlightOpacity
                      : 0,
                );
              },
            ),
          ),
        ),
      ],
    );
  }

  GlobalKey _messageKey(String messageId) => _messageKeys.putIfAbsent(
    messageId,
    () => GlobalKey(debugLabel: 'transcript_message_$messageId'),
  );

  GlobalKey _dayTimeKey(String messageId) => _dayTimeKeys.putIfAbsent(
    messageId,
    () => GlobalKey(debugLabel: 'transcript_message_day_time_$messageId'),
  );
}

class _TranscriptMessage extends StatelessWidget {
  const _TranscriptMessage({
    required this.dayTimeKey,
    required this.account,
    required this.transcriptId,
    required this.transcriptSentByCurrentUser,
    required this.mentionNames,
    required this.currentUserId,
    required this.messages,
    required this.message,
    required this.previous,
    required this.next,
    required this.highlighted,
    required this.highlightOpacity,
    super.key,
  });

  final GlobalKey? dayTimeKey;
  final rust.AccountHandle account;
  final String transcriptId;
  final bool transcriptSentByCurrentUser;
  final Map<String, String> mentionNames;
  final String currentUserId;
  final List<MessageListEntry> messages;
  final MessageListEntry message;
  final MessageListEntry? previous;
  final MessageListEntry? next;
  final bool highlighted;
  final double highlightOpacity;

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
      audioPlaylist: messages,
      isTranscriptPage: true,
      attachmentSentByCurrentUser: transcriptSentByCurrentUser,
      mentionNames: mentionNames,
      highlightOpacity: highlightOpacity,
    );
    final bypassBubble =
        !message.isUnresolvedMessage &&
        !message.isInvalidSpecialMessage &&
        const {'APP_BUTTON_GROUP', 'APP_CARD'}.contains(message.category);
    final quote = buildMessageQuotePreview(
      message,
      mentionNames: mentionNames,
    );
    final rendered = bypassBubble
        ? content
        : MessageBubble(
            isCurrentUser: presentation.isCurrentUser,
            showNip: presentation.showNip,
            showBubble: message.showMessageBubble,
            includeNip: message.includeMessageBubbleNip,
            clip: message.clipMessageBubble,
            highlightOpacity: highlightOpacity,
            padding: message.messageBubblePadding,
            forceIsCurrentUserColor: message.forceCurrentMessageBubbleColor,
            isDisappearingMessage: (message.expireIn ?? 0) > 0,
            quote: quote,
            constrainQuoteWidth:
                message.isImage || message.isVideo || message.isLive,
            highlightMedia:
                !message.showMessageBubble &&
                quote == null &&
                (message.isImage ||
                    message.isVideo ||
                    message.isLive ||
                    message.isSticker),
            outerTimeAndStatusWidget: message.useOuterMessageDateAndStatus
                ? status
                : null,
            child: content,
          );
    final policy = MessageActionPolicy(
      message: message,
      currentUserId: currentUserId,
      currentUserRole: null,
      now: DateTime.now(),
      isTranscriptPage: true,
    );
    final interactive = ContextMenuWidget(
      desktopMenuWidgetBuilder: CustomDesktopMenuWidgetBuilder(),
      menuProvider: (_) => buildMessageActionsMenu(
        context: context,
        message: message,
        policy: policy,
      ),
      child: rendered,
    );
    final messageColumn = Column(
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
                  child: messageColumn,
                ),
              ),
              const SizedBox(width: 65),
            ],
          )
        : Padding(
            padding: EdgeInsets.only(
              left: presentation.isCurrentUser
                  ? 65
                  : (presentation.showAvatar ? 40 : 16),
              right: presentation.isCurrentUser ? 16 : 65,
              top: 2,
              bottom: 2,
            ),
            child: messageColumn,
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
