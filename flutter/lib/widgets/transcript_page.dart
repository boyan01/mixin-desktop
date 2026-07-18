import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/utils/system_clipboard.dart';
import 'package:mixin_desktop_ui/utils/app_logger.dart';
import 'package:mixin_desktop_ui/utils/web_view.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/action_button.dart';
import 'package:mixin_desktop_ui/widgets/message_action_policy.dart';
import 'package:mixin_desktop_ui/widgets/message_actions_menu.dart';
import 'package:mixin_desktop_ui/widgets/interactive_decorated_box.dart';
import 'package:mixin_desktop_ui/widgets/message_bubble.dart';
import 'package:mixin_desktop_ui/widgets/message_content.dart';
import 'package:mixin_desktop_ui/widgets/message_items/special_message_items.dart';
import 'package:mixin_desktop_ui/widgets/message_datetime_and_status.dart';
import 'package:mixin_desktop_ui/widgets/message_day_time.dart';
import 'package:mixin_desktop_ui/widgets/message_media_preview_pages.dart';
import 'package:mixin_desktop_ui/widgets/message_name.dart';
import 'package:mixin_desktop_ui/widgets/message_presentation.dart';
import 'package:mixin_desktop_ui/widgets/custom_context_menu.dart';
import 'package:mixin_desktop_ui/widgets/message_qr_dialog.dart';
import 'package:mixin_desktop_ui/widgets/message_rows.dart';
import 'package:mixin_desktop_ui/widgets/message_selectable_text.dart';
import 'package:mixin_desktop_ui/widgets/mixin_dialog.dart';
import 'package:mixin_desktop_ui/widgets/show_message_user_dialog.dart';
import 'package:provider/provider.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';
import 'package:super_context_menu/super_context_menu.dart';

Future<void> showTranscriptDialog(
  BuildContext context, {
  required rust.AccountHandle account,
  required String transcriptId,
  required bool sentByCurrentUser,
  required String currentUserId,
  required ValueChanged<ConversationListEntry> onSelectConversation,
  required ValueChanged<ConversationListEntry> onSelectConversationInfo,
  required ValueChanged<Uri> onOpenUri,
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
    onOpenUri: onOpenUri,
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
    required this.onOpenUri,
    super.key,
  });

  final rust.AccountHandle account;
  final String transcriptId;
  final bool sentByCurrentUser;
  final String currentUserId;
  final ValueChanged<ConversationListEntry> onSelectConversation;
  final ValueChanged<ConversationListEntry> onSelectConversationInfo;
  final ValueChanged<Uri> onOpenUri;

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
        writeAppLog('watch transcript messages failed: $error');
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
        writeAppLog('refresh transcript messages failed: $error');
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

  Future<void> _download(MessageListEntry message) async {
    try {
      if (widget.sentByCurrentUser) {
        await widget.account.attachment().retryTranscriptAttachment(
          transcriptId: widget.transcriptId,
        );
      } else {
        await widget.account.attachment().downloadTranscriptAttachment(
          transcriptId: widget.transcriptId,
          messageId: message.id,
        );
      }
      await _refresh();
    } on Object catch (error) {
      writeAppLog('download transcript attachment failed: $error');
    }
  }

  Future<void> _cancel(MessageListEntry message) async {
    try {
      await widget.account.attachment().cancelTranscriptAttachment(
        transcriptId: widget.transcriptId,
        messageId: message.id,
      );
      await _refresh();
    } on Object catch (error) {
      writeAppLog('cancel transcript attachment failed: $error');
    }
  }

  Future<void> _markAudioRead(MessageListEntry message) async {
    if (message.mediaStatus.toUpperCase() != 'DONE') return;
    try {
      await widget.account.attachment().markTranscriptAudioRead(
        transcriptId: widget.transcriptId,
        messageId: message.id,
      );
      await _refresh();
    } on Object catch (error) {
      writeAppLog('mark transcript audio read failed: $error');
    }
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
    _highlightController
      ..reset()
      ..forward();
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
        Expanded(child: _buildBody()),
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
                  onOpenMessage:
                      _messages[index].quoteMessageId != null &&
                          _messages.any(
                            (message) =>
                                message.id == _messages[index].quoteMessageId,
                          )
                      ? _locateMessage
                      : null,
                  onDownload: _download,
                  onCancel: _cancel,
                  onMarkAudioRead: _markAudioRead,
                  onSelectConversation: widget.onSelectConversation,
                  onSelectConversationInfo: widget.onSelectConversationInfo,
                  onOpenUri: widget.onOpenUri,
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
    required this.onOpenMessage,
    required this.onDownload,
    required this.onCancel,
    required this.onMarkAudioRead,
    required this.onSelectConversation,
    required this.onSelectConversationInfo,
    required this.onOpenUri,
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
  final ValueChanged<String>? onOpenMessage;
  final MessageEntryCallback onDownload;
  final MessageEntryCallback onCancel;
  final MessageEntryCallback onMarkAudioRead;
  final ValueChanged<ConversationListEntry> onSelectConversation;
  final ValueChanged<ConversationListEntry> onSelectConversationInfo;
  final ValueChanged<Uri> onOpenUri;

  @override
  Widget build(BuildContext context) {
    final row = MessageRowModel(
      message: message,
      previous: previous,
      next: next,
    );
    final presentation = MessagePresentation.fromRow(
      row: row,
      currentUserId: currentUserId,
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
      attachmentSentByCurrentUser: transcriptSentByCurrentUser,
      mentionNames: mentionNames,
      highlightOpacity: highlightOpacity,
      onOpenUri: onOpenUri,
      onOpenMessage: onOpenMessage,
      onAppAction: (action, {title}) => openMessageAction(
        context: context,
        account: account,
        conversationId: message.conversationId,
        action: action,
        title: title,
        onOpenUri: onOpenUri,
      ),
      onOpenImage: (message) => _openImage(context, message),
      onOpenVideo: (message) => _openVideo(context, message),
      onOpenPost: (message) => _openPost(context, message),
      onOpenFile: (message) => unawaited(_openFile(context, message)),
      onOpenUser: (userId) => _showUser(context, userId: userId),
      onOpenIdentityNumber: (identityNumber) =>
          _showUser(context, identityNumber: identityNumber),
      onOpenTranscript: (id) => unawaited(
        showTranscriptDialog(
          context,
          account: account,
          transcriptId: id,
          sentByCurrentUser: message.senderRelationship.toUpperCase() == 'ME',
          currentUserId: currentUserId,
          onSelectConversation: onSelectConversation,
          onSelectConversationInfo: onSelectConversationInfo,
          onOpenUri: onOpenUri,
        ),
      ),
      onMarkAudioRead: onMarkAudioRead,
      onDownloadAttachment: onDownload,
      onCancelAttachment: onCancel,
    );
    final bypassBubble =
        !message.isUnresolvedMessage &&
        !message.isInvalidSpecialMessage &&
        const {'APP_BUTTON_GROUP', 'APP_CARD'}.contains(message.category);
    final quote = buildMessageQuotePreview(
      message,
      onOpenMessage: onOpenMessage,
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
    final localImage = message.isImage
        ? existingLocalFile(message.mediaUrl)
        : null;
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
        callbacks: MessageActionCallbacks(
          onCopyText: (text) => Clipboard.setData(ClipboardData(text: text)),
          onCopyImage: localImage == null
              ? null
              : () => unawaited(copyLocalFileToClipboard(localImage)),
          onGenerateQr: (text) => showMessageQrDialog(context, content: text),
          onSaveAs: () => unawaited(_save(message)),
        ),
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
                  onTap: () => _showUser(context, userId: message.senderId),
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
                onTap: () => _showUser(context, userId: message.senderId),
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

  void _openImage(BuildContext context, MessageListEntry selected) {
    final images = messages
        .where((item) => item.isImage && (item.mediaUrl?.isNotEmpty ?? false))
        .map(
          (item) => ImagePreviewEntry(
            id: item.id,
            source: item.mediaUrl!,
            name: item.mediaName,
            thumbImage: item.thumbImage,
            userId: item.senderId,
            userFullName: item.senderName,
            userIdentityNumber: item.senderIdentityNumber,
            avatarUrl: item.senderAvatarUrl,
          ),
        )
        .toList(growable: false);
    final index = images.indexWhere((item) => item.id == selected.id);
    if (index < 0) return;
    unawaited(
      ImagePreviewPage.show(
        context,
        ImagePreviewPage(
          images: images,
          initialIndex: index,
          onCopy: (image) async {
            final file = existingLocalFile(image.source);
            if (file != null) await copyLocalFileToClipboard(file);
          },
          onSave: (image) =>
              saveMessageFileAs(image.source, suggestedName: image.name),
        ),
      ),
    );
  }

  void _openVideo(BuildContext context, MessageListEntry message) {
    final source = message.mediaUrl?.trim() ?? '';
    if (source.isEmpty) return;
    unawaited(
      VideoPreviewPage.show(
        context,
        VideoPreviewPage(
          source: source,
          title: message.mediaName,
          userId: message.senderId,
          userFullName: message.senderName,
          userIdentityNumber: message.senderIdentityNumber,
          avatarUrl: message.senderAvatarUrl,
          isTranscriptPage: true,
        ),
      ),
    );
  }

  void _openPost(BuildContext context, MessageListEntry message) {
    unawaited(
      PostPreviewPage.show(
        context,
        PostPreviewPage(content: message.content, title: message.senderName),
      ),
    );
  }

  Future<void> _openFile(BuildContext context, MessageListEntry message) async {
    final source = message.mediaUrl?.trim() ?? '';
    if (source.isEmpty) return;
    await openOrSaveMessageFile(context, source, mediaName: message.mediaName);
  }

  Future<void> _save(MessageListEntry message) async {
    final source = message.mediaUrl?.trim() ?? '';
    if (source.isEmpty) return;
    await saveMessageFileAs(source, suggestedName: message.mediaName);
  }

  void _showUser(
    BuildContext context, {
    String? userId,
    String? identityNumber,
  }) {
    unawaited(_showUserAndHandle(context, userId, identityNumber));
  }

  Future<void> _showUserAndHandle(
    BuildContext context,
    String? userId,
    String? identityNumber,
  ) async {
    final result = await showMessageUserDialog(
      context,
      account: account,
      userId: userId,
      identityNumber: identityNumber,
    );
    if (!context.mounted || result == null) return;
    await handleMessageUserDialogResult(
      context,
      account: account,
      result: result,
      onSelectConversation: onSelectConversation,
      onSelectConversationInfo: onSelectConversationInfo,
    );
  }
}
