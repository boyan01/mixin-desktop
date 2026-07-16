import 'dart:async';
import 'dart:io';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:intl/intl.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/controllers/message_list_controller.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/controllers/sticker_controller.dart';
import 'package:mixin_desktop_ui/controllers/voice_recorder_controller.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/utils/system_clipboard.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/message_bubble.dart';
import 'package:mixin_desktop_ui/widgets/message_action_policy.dart';
import 'package:mixin_desktop_ui/widgets/message_actions_menu.dart';
import 'package:mixin_desktop_ui/widgets/message_audio.dart';
import 'package:mixin_desktop_ui/widgets/message_content.dart';
import 'package:mixin_desktop_ui/widgets/message_day_time.dart';
import 'package:mixin_desktop_ui/widgets/message_datetime_and_status.dart';
import 'package:mixin_desktop_ui/widgets/message_media_preview_pages.dart';
import 'package:mixin_desktop_ui/widgets/message_name.dart';
import 'package:mixin_desktop_ui/widgets/message_presentation.dart';
import 'package:mixin_desktop_ui/widgets/message_qr_dialog.dart';
import 'package:mixin_desktop_ui/widgets/message_rows.dart';
import 'package:mixin_desktop_ui/widgets/pinned_messages_page.dart';
import 'package:mixin_desktop_ui/widgets/show_message_user_dialog.dart';
import 'package:mixin_desktop_ui/widgets/show_snapshot_detail_dialog.dart';
import 'package:mixin_desktop_ui/widgets/show_forward_conversation_selector.dart';
import 'package:mixin_desktop_ui/widgets/show_add_image_sticker_dialog.dart';
import 'package:mixin_desktop_ui/widgets/transcript_page.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_button.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_detail_page.dart';
import 'package:provider/provider.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';
import 'package:super_context_menu/super_context_menu.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:visibility_detector/visibility_detector.dart';

class ChatView extends StatefulWidget {
  const ChatView({
    required this.account,
    required this.conversation,
    required this.draft,
    required this.onDraftChanged,
    super.key,
    this.onBack,
    this.onSearch,
    this.onInfo,
    this.onPinned,
    this.locateMessageId,
    this.locateRequest = 0,
  });

  final rust.AccountHandle account;
  final ConversationListEntry conversation;
  final String draft;
  final ValueChanged<String> onDraftChanged;
  final VoidCallback? onBack;
  final VoidCallback? onSearch;
  final VoidCallback? onInfo;
  final VoidCallback? onPinned;
  final String? locateMessageId;
  final int locateRequest;

  @override
  State<ChatView> createState() => _ChatViewState();
}

class _ChatViewState extends State<ChatView> {
  late MessageListController _messageController;
  late String _currentUserId;
  late final TextEditingController _inputController;
  late final FocusNode _inputFocusNode;
  late final VoiceRecorderController _voiceRecorderController;
  late StickerController _stickerController;
  final ItemScrollController _messageScrollController = ItemScrollController();
  MessageListEntry? _quoteMessage;
  String? _highlightedMessageId;
  int _highlightGeneration = 0;
  bool _initialUnreadScrollScheduled = false;
  final Set<String> _selectedMessageIds = {};
  final Map<String, GlobalKey> _messageKeys = {};

  @override
  void initState() {
    super.initState();
    _createMessageController();
    _inputController = TextEditingController(text: widget.draft);
    _inputController.addListener(_onInputChanged);
    _inputFocusNode = FocusNode(debugLabel: 'chat_input');
    _voiceRecorderController = VoiceRecorderController()
      ..addListener(_onVoiceRecorderChanged);
    _stickerController = StickerController(account: widget.account);
    if (widget.locateMessageId != null) {
      WidgetsBinding.instance.addPostFrameCallback(
        (_) => unawaited(_locateMessage(widget.locateMessageId!)),
      );
    }
  }

  @override
  void didUpdateWidget(covariant ChatView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.account, widget.account) ||
        oldWidget.conversation.id != widget.conversation.id) {
      _messageController.removeListener(_onMessageControllerChanged);
      _messageController.dispose();
      _createMessageController();
      _quoteMessage = null;
      _highlightedMessageId = null;
      _highlightGeneration++;
      _selectedMessageIds.clear();
      _messageKeys.clear();
      unawaited(_voiceRecorderController.cancel());
      if (!identical(oldWidget.account, widget.account)) {
        _stickerController.dispose();
        _stickerController = StickerController(account: widget.account);
      }
    }
    if (_inputController.text != widget.draft) {
      _inputController.value = TextEditingValue(
        text: widget.draft,
        selection: TextSelection.collapsed(offset: widget.draft.length),
      );
    }
    if (widget.locateMessageId != null &&
        widget.locateRequest != oldWidget.locateRequest) {
      WidgetsBinding.instance.addPostFrameCallback(
        (_) => unawaited(_locateMessage(widget.locateMessageId!)),
      );
    }
  }

  void _createMessageController() {
    _currentUserId = widget.account.accountId();
    _initialUnreadScrollScheduled = false;
    _messageController = MessageListController(
      account: widget.account,
      conversation: widget.conversation,
    );
    _messageController.addListener(_onMessageControllerChanged);
  }

  void _onMessageControllerChanged() {
    final controller = _messageController;
    if (_initialUnreadScrollScheduled ||
        !controller.initialUnreadAnchorPending ||
        controller.initialUnreadMessageIndex == null) {
      return;
    }
    _initialUnreadScrollScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback(
      (_) => _jumpToInitialUnread(controller, 0),
    );
  }

  void _jumpToInitialUnread(
    MessageListController controller,
    int attachmentAttempt,
  ) {
    if (!mounted || !identical(controller, _messageController)) return;
    final messageIndex = controller.initialUnreadMessageIndex;
    if (!controller.initialUnreadAnchorPending || messageIndex == null) return;
    if (!_messageScrollController.isAttached) {
      if (attachmentAttempt < 4) {
        WidgetsBinding.instance.addPostFrameCallback(
          (_) => _jumpToInitialUnread(controller, attachmentAttempt + 1),
        );
      }
      return;
    }
    if (messageIndex < 0 || messageIndex >= controller.messages.length) {
      controller.consumeInitialUnreadAnchor();
      return;
    }
    final reverseIndex = controller.messages.length - messageIndex - 1;
    _messageScrollController.jumpTo(index: reverseIndex, alignment: 0.12);
    controller.consumeInitialUnreadAnchor();
  }

  void _onInputChanged() => setState(() {});

  void _onVoiceRecorderChanged() {
    if (mounted) setState(() {});
  }

  Future<void> _startVoiceRecording() async {
    _inputFocusNode.unfocus();
    await _voiceRecorderController.start();
  }

  Future<void> _sendVoiceRecording() async {
    final messageController = _messageController;
    final quoteMessage = _quoteMessage;
    final sent = await _voiceRecorderController.send((recording) async {
      final success = await messageController.sendAudio(
        path: recording.path,
        duration: recording.duration,
        waveform: recording.waveform,
        quoteMessageId: quoteMessage?.id,
      );
      if (!success) throw StateError('Failed to send voice message');
    });
    if (!mounted ||
        !identical(messageController, _messageController) ||
        !sent) {
      return;
    }
    setState(() => _quoteMessage = null);
  }

  Future<bool> _sendSticker(String stickerId) async {
    final sent = await _messageController.sendSticker(stickerId: stickerId);
    if (!mounted || !sent) return false;
    setState(() => _quoteMessage = null);
    unawaited(_stickerController.refreshLocal());
    return true;
  }

  Future<void> _sendText() async {
    final messageController = _messageController;
    final original = _inputController.text;
    if (original.trim().isEmpty || messageController.sending) return;
    final quoteMessage = _quoteMessage;
    final sent = await messageController.sendText(
      original,
      quoteMessageId: quoteMessage?.id,
    );
    if (!mounted ||
        !identical(_messageController, messageController) ||
        !sent ||
        _inputController.text != original) {
      return;
    }
    _inputController.clear();
    setState(() => _quoteMessage = null);
    widget.onDraftChanged('');
  }

  void _replyTo(MessageListEntry message) {
    setState(() => _quoteMessage = message);
    _inputFocusNode.requestFocus();
  }

  void _selectMessage(MessageListEntry message) {
    setState(() => _selectedMessageIds.add(message.id));
  }

  void _toggleMessageSelection(MessageListEntry message) {
    setState(() {
      if (!_selectedMessageIds.remove(message.id)) {
        _selectedMessageIds.add(message.id);
      }
    });
  }

  Future<void> _forwardMessages(List<MessageListEntry> messages) async {
    final targetConversationId = await showForwardConversationSelector(
      context,
      account: widget.account,
    );
    if (targetConversationId == null || !mounted) return;
    final controller = _messageController;
    final forwarded = await controller.forwardMessages(
      messages,
      targetConversationId,
    );
    if (!mounted || !identical(controller, _messageController) || !forwarded) {
      return;
    }
    setState(_selectedMessageIds.clear);
  }

  Future<void> _combineForwardMessages(List<MessageListEntry> messages) async {
    final targetConversationId = await showForwardConversationSelector(
      context,
      account: widget.account,
    );
    if (targetConversationId == null || !mounted) return;
    final controller = _messageController;
    final forwarded = await controller.combineForwardMessages(
      messages,
      targetConversationId,
    );
    if (!mounted || !identical(controller, _messageController) || !forwarded) {
      return;
    }
    setState(_selectedMessageIds.clear);
  }

  void _openPinnedMessages() {
    unawaited(
      showPinnedMessagesDialog(
        context,
        account: widget.account,
        conversationId: widget.conversation.id,
        currentUserId: _currentUserId,
        currentUserRole: _messageController.currentUserRole,
        onLocate: (messageId) => unawaited(_locateMessage(messageId)),
      ),
    );
  }

  GlobalKey _messageKey(String messageId) => _messageKeys.putIfAbsent(
    messageId,
    () => GlobalKey(debugLabel: 'message_$messageId'),
  );

  Future<void> _locateMessage(String messageId) async {
    final controller = _messageController;
    final found = await controller.locateMessage(messageId);
    if (!found || !mounted || !identical(controller, _messageController)) {
      return;
    }
    await WidgetsBinding.instance.endOfFrame;
    var targetContext = _messageKey(messageId).currentContext;
    if (targetContext == null && _messageScrollController.isAttached) {
      final messageIndex = controller.messages.indexWhere(
        (message) => message.id == messageId,
      );
      if (messageIndex >= 0) {
        final reverseIndex = controller.messages.length - messageIndex - 1;
        await _messageScrollController.scrollTo(
          index: reverseIndex,
          alignment: 0.5,
          duration: const Duration(milliseconds: 250),
          curve: Curves.easeOut,
        );
        await WidgetsBinding.instance.endOfFrame;
        targetContext = _messageKey(messageId).currentContext;
      }
    }
    if (targetContext == null || !targetContext.mounted || !mounted) return;
    await Scrollable.ensureVisible(
      targetContext,
      alignment: 0.5,
      duration: const Duration(milliseconds: 250),
      curve: Curves.easeOut,
    );
    if (!mounted) return;
    final generation = ++_highlightGeneration;
    for (var index = 0; index < 6; index++) {
      if (!mounted || generation != _highlightGeneration) return;
      setState(() {
        _highlightedMessageId = index.isEven ? messageId : null;
      });
      await Future<void>.delayed(const Duration(milliseconds: 165));
    }
    if (mounted && generation == _highlightGeneration) {
      setState(() => _highlightedMessageId = null);
    }
  }

  Future<void> _copySelectedMessages() async {
    final selected = _messageController.messages
        .where((message) => _selectedMessageIds.contains(message.id))
        .toList(growable: false);
    final formatter = DateFormat.yMd().add_Hms();
    final text = selected
        .map(
          (message) =>
              '${message.senderName}, '
              '(${formatter.format(message.createdAt.toLocal())}):\n'
              '${message.isText ? message.content : _messagePreview(context, message)}',
        )
        .join('\n\n');
    await Clipboard.setData(ClipboardData(text: text));
    if (mounted) setState(_selectedMessageIds.clear);
  }

  @override
  void dispose() {
    _messageController.removeListener(_onMessageControllerChanged);
    _messageController.dispose();
    _inputController
      ..removeListener(_onInputChanged)
      ..dispose();
    _inputFocusNode.dispose();
    _voiceRecorderController
      ..removeListener(_onVoiceRecorderChanged)
      ..dispose();
    _stickerController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
    animation: _messageController,
    builder: (context, child) => Material(
      color: context.theme.primary,
      child: Column(
        children: [
          _ChatHeader(
            conversation: widget.conversation,
            onBack: widget.onBack,
            onSearch: widget.onSearch,
            onInfo: widget.onInfo,
            onPinned: _messageController.pinnedMessages.isEmpty
                ? null
                : widget.onPinned ?? _openPinnedMessages,
          ),
          Expanded(
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: context.theme.chatBackground,
                image: DecorationImage(
                  image: const ExactAssetImage(MixinAssets.chatBackground),
                  fit: BoxFit.cover,
                  colorFilter: ColorFilter.mode(
                    Theme.of(context).brightness == Brightness.dark
                        ? Colors.white.withValues(alpha: 0.02)
                        : Colors.black.withValues(alpha: 0.03),
                    BlendMode.srcIn,
                  ),
                ),
              ),
              child: Column(
                children: [
                  Expanded(
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        border: Border(
                          bottom: BorderSide(color: context.theme.divider),
                        ),
                      ),
                      child: _MessageList(
                        account: widget.account,
                        controller: _messageController,
                        currentUserId: _currentUserId,
                        conversation: widget.conversation,
                        scrollController: _messageScrollController,
                        selectedMessageIds: _selectedMessageIds,
                        highlightedMessageId: _highlightedMessageId,
                        messageKey: _messageKey,
                        onReply: _replyTo,
                        onForward: (messages) =>
                            unawaited(_forwardMessages(messages)),
                        onSelect: _selectMessage,
                        onToggleSelection: _toggleMessageSelection,
                        onOpenMessage: _locateMessage,
                        onStickerAlbumChanged: _stickerController.refreshLocal,
                      ),
                    ),
                  ),
                  if (_selectedMessageIds.isNotEmpty)
                    _SelectionBottomBar(
                      selectedMessages: _messageController.messages
                          .where(
                            (message) =>
                                _selectedMessageIds.contains(message.id),
                          )
                          .toList(growable: false),
                      currentUserId: _currentUserId,
                      onCombineForward: _combineForwardMessages,
                      onForward: _forwardMessages,
                      onCopy: _copySelectedMessages,
                      onDelete: (messages) async {
                        try {
                          await _messageController.deleteMessages(messages);
                        } on Object {
                          return;
                        }
                        if (mounted) setState(_selectedMessageIds.clear);
                      },
                      onRecall: (messages) async {
                        try {
                          await _messageController.recallMessages(messages);
                        } on Object {
                          return;
                        }
                        if (mounted) setState(_selectedMessageIds.clear);
                      },
                    )
                  else if (_voiceRecorderController.value.status ==
                      VoiceRecorderStatus.idle)
                    ChatInputBar(
                      controller: _inputController,
                      focusNode: _inputFocusNode,
                      quoteMessage: _quoteMessage,
                      sending: _messageController.sending,
                      onChanged: widget.onDraftChanged,
                      onCancelQuote: () => setState(() => _quoteMessage = null),
                      onSend: _sendText,
                      onVoicePressed: () => unawaited(_startVoiceRecording()),
                      stickerAction: StickerButton(
                        textEditingController: _inputController,
                        controller: _stickerController,
                        onStickerSelected: _sendSticker,
                        onStickerSent: () {},
                        onEmojiUsed: (_) {
                          widget.onDraftChanged(_inputController.text);
                          _inputFocusNode.requestFocus();
                        },
                        child: const _ChatInputAction(
                          actionKey: Key('chat-sticker'),
                          asset: MixinAssets.sticker,
                        ),
                      ),
                    )
                  else
                    VoiceRecorderBar(
                      state: _voiceRecorderController.value,
                      onCancel: _voiceRecorderController.cancel,
                      onStop: () async {
                        await _voiceRecorderController.stop();
                      },
                      onRetry: () async {
                        await _voiceRecorderController.start();
                      },
                      onSend: _sendVoiceRecording,
                    ),
                ],
              ),
            ),
          ),
        ],
      ),
    ),
  );
}

class _ChatHeader extends StatelessWidget {
  const _ChatHeader({
    required this.conversation,
    required this.onBack,
    required this.onSearch,
    required this.onInfo,
    required this.onPinned,
  });

  final ConversationListEntry conversation;
  final VoidCallback? onBack;
  final VoidCallback? onSearch;
  final VoidCallback? onInfo;
  final VoidCallback? onPinned;

  @override
  Widget build(BuildContext context) {
    final subtitle = conversation.isGroup
        ? AppLocalizations.of(
            context,
          ).participantsCount(conversation.participantCount)
        : conversation.identityNumber;
    return Container(
      key: const Key('chat-header'),
      height: 64,
      decoration: BoxDecoration(
        color: context.theme.primary,
        border: Border(bottom: BorderSide(color: context.theme.divider)),
      ),
      child: Row(
        children: [
          if (onBack == null)
            const SizedBox(width: 16)
          else
            Padding(
              padding: const EdgeInsets.only(right: 8),
              child: _HeaderAction(
                key: const Key('chat-back'),
                asset: MixinAssets.back,
                onPressed: onBack,
              ),
            ),
          ConversationAvatarView(conversation: conversation, size: 36),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Text(
                  conversation.name,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: context.theme.text,
                    fontSize: 16,
                    height: 1,
                  ),
                ),
                if (subtitle.isNotEmpty) ...[
                  const SizedBox(height: 4),
                  Text(
                    subtitle,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      color: context.theme.secondaryText,
                      fontSize: 14,
                      height: 1,
                    ),
                  ),
                ],
              ],
            ),
          ),
          if (onPinned != null)
            _HeaderAction(asset: MixinAssets.messagePin, onPressed: onPinned),
          _HeaderAction(asset: MixinAssets.chatSearch, onPressed: onSearch),
          _HeaderAction(
            key: const Key('chat-info'),
            asset: MixinAssets.chatInfo,
            onPressed: onInfo,
          ),
          const SizedBox(width: 16),
        ],
      ),
    );
  }
}

class _HeaderAction extends StatelessWidget {
  const _HeaderAction({
    required this.asset,
    required this.onPressed,
    super.key,
  });

  final String asset;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    final color = onPressed == null
        ? context.theme.secondaryText.withValues(alpha: 0.5)
        : context.theme.icon;
    final icon = Center(
      child: SvgPicture.asset(
        asset,
        width: 20,
        height: 20,
        colorFilter: ColorFilter.mode(color, BlendMode.srcIn),
      ),
    );
    return SizedBox.square(
      dimension: 40,
      child: onPressed == null
          ? ExcludeSemantics(child: icon)
          : InkResponse(onTap: onPressed, radius: 20, child: icon),
    );
  }
}

class _MessageList extends StatelessWidget {
  const _MessageList({
    required this.account,
    required this.controller,
    required this.currentUserId,
    required this.conversation,
    required this.scrollController,
    required this.selectedMessageIds,
    required this.highlightedMessageId,
    required this.messageKey,
    required this.onReply,
    required this.onForward,
    required this.onSelect,
    required this.onToggleSelection,
    required this.onOpenMessage,
    required this.onStickerAlbumChanged,
  });

  final rust.AccountHandle account;
  final MessageListController controller;
  final String currentUserId;
  final ConversationListEntry conversation;
  final ItemScrollController scrollController;
  final Set<String> selectedMessageIds;
  final String? highlightedMessageId;
  final GlobalKey Function(String messageId) messageKey;
  final ValueChanged<MessageListEntry> onReply;
  final ValueChanged<List<MessageListEntry>> onForward;
  final ValueChanged<MessageListEntry> onSelect;
  final ValueChanged<MessageListEntry> onToggleSelection;
  final ValueChanged<String> onOpenMessage;
  final Future<void> Function() onStickerAlbumChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    if (controller.loading && controller.messages.isEmpty) {
      return const Center(child: CircularProgressIndicator(strokeWidth: 2));
    }
    if (controller.error != null && controller.messages.isEmpty) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                controller.error!,
                textAlign: TextAlign.center,
                style: TextStyle(color: context.theme.secondaryText),
              ),
              const SizedBox(height: 12),
              TextButton(
                onPressed: controller.retry,
                child: Text(l10n?.retry ?? 'Retry'),
              ),
            ],
          ),
        ),
      );
    }

    final messages = controller.messages;
    final itemCount = messages.length + (controller.loadingOlder ? 1 : 0);
    final messageList = NotificationListener<ScrollNotification>(
      onNotification: (notification) {
        final metrics = notification.metrics;
        if (metrics.pixels >= metrics.maxScrollExtent - 80) {
          controller.loadOlder();
        }
        return false;
      },
      child: ScrollablePositionedList.builder(
        itemScrollController: scrollController,
        reverse: true,
        minCacheExtent: 800,
        padding: const EdgeInsets.symmetric(vertical: 8),
        itemCount: itemCount,
        itemBuilder: (context, index) {
          if (index == messages.length) {
            return const Padding(
              padding: EdgeInsets.symmetric(vertical: 12),
              child: Center(
                child: SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              ),
            );
          }
          final messageIndex = messages.length - index - 1;
          final message = messages[messageIndex];
          final previous = messageIndex == 0
              ? null
              : messages[messageIndex - 1];
          final next = messageIndex + 1 >= messages.length
              ? null
              : messages[messageIndex + 1];
          final chatMessage = _ChatMessage(
            key: messageKey(message.id),
            account: account,
            message: message,
            previous: previous,
            next: next,
            currentUserId: currentUserId,
            isGroup: conversation.isGroup,
            isBot: conversation.isBot,
            conversationOwnerId: conversation.ownerId,
            currentUserRole: controller.currentUserRole,
            messages: messages,
            mentionNames: controller.mentionNames,
            selected: selectedMessageIds.contains(message.id),
            highlighted: highlightedMessageId == message.id,
            inSelectionMode: selectedMessageIds.isNotEmpty,
            onReply: () => onReply(message),
            onForward: () => onForward([message]),
            onSelect: () => onSelect(message),
            onToggleSelection: () => onToggleSelection(message),
            onTogglePin: () => unawaited(
              _ignoreMutation(
                controller.setMessagePinned(message, !message.pinned),
              ),
            ),
            onRecall: () => unawaited(
              _ignoreMutation(controller.recallMessages([message])),
            ),
            onDelete: () => unawaited(
              _ignoreMutation(controller.deleteMessages([message])),
            ),
            onOpenMessage: onOpenMessage,
            onMarkMentionRead: () =>
                unawaited(controller.markMentionRead(message)),
            onMarkAudioRead: () => unawaited(controller.markAudioRead(message)),
            onDownloadAttachment: () => unawaited(
              _ignoreMutation(controller.downloadAttachment(message)),
            ),
            onCancelAttachment: () => unawaited(
              _ignoreMutation(controller.cancelAttachment(message)),
            ),
            onAddSticker: () => controller.addSticker(message),
            onAddImageAsSticker: () => controller.addImageAsSticker(message),
            onStrangerAction: (action) =>
                controller.handleStrangerAction(message, action),
            onStickerAlbumChanged: onStickerAlbumChanged,
          );
          if (message.id != controller.unreadBoundaryMessageId ||
              next == null) {
            return chatMessage;
          }
          return Column(
            mainAxisSize: MainAxisSize.min,
            children: [chatMessage, const _UnreadMessageBar()],
          );
        },
      ),
    );
    if (controller.error == null) return messageList;
    return Column(
      children: [
        Container(
          key: const Key('chat-message-error'),
          width: double.infinity,
          color: context.theme.primary,
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: Text(
            controller.error!,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(color: context.theme.secondaryText, fontSize: 12),
          ),
        ),
        Expanded(child: messageList),
      ],
    );
  }
}

class _UnreadMessageBar extends StatelessWidget {
  const _UnreadMessageBar();

  @override
  Widget build(BuildContext context) => Container(
    key: const Key('unread-message-bar'),
    color: context.theme.background,
    padding: const EdgeInsets.symmetric(vertical: 4),
    margin: const EdgeInsets.symmetric(vertical: 6),
    alignment: Alignment.center,
    child: Text(
      context.l10n.unreadMessages,
      style: TextStyle(color: context.theme.secondaryText, fontSize: 14),
    ),
  );
}

class _ChatMessage extends StatefulWidget {
  const _ChatMessage({
    required this.account,
    required this.message,
    required this.previous,
    required this.next,
    required this.currentUserId,
    required this.isGroup,
    required this.isBot,
    required this.conversationOwnerId,
    required this.currentUserRole,
    required this.messages,
    required this.mentionNames,
    required this.selected,
    required this.highlighted,
    required this.inSelectionMode,
    required this.onReply,
    required this.onForward,
    required this.onSelect,
    required this.onToggleSelection,
    required this.onTogglePin,
    required this.onRecall,
    required this.onDelete,
    required this.onOpenMessage,
    required this.onMarkMentionRead,
    required this.onMarkAudioRead,
    required this.onDownloadAttachment,
    required this.onCancelAttachment,
    required this.onAddSticker,
    required this.onAddImageAsSticker,
    required this.onStrangerAction,
    required this.onStickerAlbumChanged,
    super.key,
  });

  final rust.AccountHandle account;
  final MessageListEntry message;
  final MessageListEntry? previous;
  final MessageListEntry? next;
  final String currentUserId;
  final bool isGroup;
  final bool isBot;
  final String conversationOwnerId;
  final String? currentUserRole;
  final List<MessageListEntry> messages;
  final Map<String, String> mentionNames;
  final bool selected;
  final bool highlighted;
  final bool inSelectionMode;
  final VoidCallback onReply;
  final VoidCallback onForward;
  final VoidCallback onSelect;
  final VoidCallback onToggleSelection;
  final VoidCallback onTogglePin;
  final VoidCallback onRecall;
  final VoidCallback onDelete;
  final ValueChanged<String> onOpenMessage;
  final VoidCallback onMarkMentionRead;
  final VoidCallback onMarkAudioRead;
  final VoidCallback onDownloadAttachment;
  final VoidCallback onCancelAttachment;
  final Future<void> Function() onAddSticker;
  final Future<void> Function() onAddImageAsSticker;
  final Future<String?> Function(String action) onStrangerAction;
  final Future<void> Function() onStickerAlbumChanged;

  @override
  State<_ChatMessage> createState() => _ChatMessageState();
}

class _ChatMessageState extends State<_ChatMessage> {
  bool _menuHighlighted = false;
  bool _strangerResolved = false;

  @override
  Widget build(BuildContext context) {
    if (_strangerResolved && widget.message.category == 'STRANGER') {
      return const SizedBox.shrink();
    }
    final settings = context.watch<SettingsController>();
    final row = MessageRowModel(
      message: widget.message,
      previous: widget.previous,
      next: widget.next,
    );
    final presentation = MessagePresentation.fromRow(
      row: row,
      currentUserId: widget.currentUserId,
      isGroupOrBotGroupConversation: resolveGroupOrBotGroupConversation(
        message: widget.message,
        isGroup: widget.isGroup,
        isBot: widget.isBot,
        conversationOwnerId: widget.conversationOwnerId,
      ),
      enableShowAvatar: settings.messageShowAvatar,
    );
    final showIdentityNumber =
        settings.messageShowIdentityNumber &&
        widget.message.senderIdentityNumber.isNotEmpty &&
        widget.message.senderIdentityNumber != '0';
    final messageContent = MessageContent(
      message: widget.message,
      isCurrentUser: presentation.isCurrentUser,
      currentUserId: widget.currentUserId,
      onOpenUri: (uri) => unawaited(launchUrl(uri)),
      onOpenMessage: widget.onOpenMessage,
      onOpenSnapshot: (_) =>
          showSnapshotDetailDialog(context, message: widget.message),
      onAction: widget.message.category == 'STRANGER'
          ? _handleStrangerAction
          : _openAction,
      onOpenImage: _openImage,
      onOpenVideo: _openVideo,
      onOpenPost: _openPost,
      onOpenFile: _openFile,
      onOpenSticker: _openSticker,
      onOpenTranscript: _openTranscript,
      onOpenUser: (userId) => _showUser(userId: userId),
      onOpenIdentityNumber: (identityNumber) =>
          _showUser(identityNumber: identityNumber),
      mentionNames: widget.mentionNames,
      showNip: presentation.showNip,
      highlighted: _menuHighlighted || widget.highlighted,
      onMarkAudioRead: (_) => widget.onMarkAudioRead(),
      onDownloadAttachment: (_) => widget.onDownloadAttachment(),
      onCancelAttachment: (_) => widget.onCancelAttachment(),
      dateAndStatus: MessageDatetimeAndStatus(
        message: widget.message,
        isCurrentUser: presentation.isCurrentUser,
        isRepresentative:
            widget.isBot &&
            widget.message.senderId != widget.conversationOwnerId &&
            !presentation.isCurrentUser,
      ),
      overlayDateAndStatus: MessageDatetimeAndStatus(
        message: widget.message,
        isCurrentUser: presentation.isCurrentUser,
        color: Colors.white,
        isRepresentative:
            widget.isBot &&
            widget.message.senderId != widget.conversationOwnerId &&
            !presentation.isCurrentUser,
      ),
    );
    final bypassActions = const {
      'SYSTEM_CONVERSATION',
      'MESSAGE_PIN',
      'SECRET',
      'STRANGER',
    }.contains(widget.message.category);
    final bypassBubble =
        !widget.message.isUnresolvedMessage &&
        (bypassActions ||
            widget.message.category == 'APP_BUTTON_GROUP' ||
            widget.message.category == 'APP_CARD');
    final renderedMessage = bypassBubble
        ? Align(
            key: Key('message-bubble-${widget.message.id}'),
            alignment: presentation.isCurrentUser
                ? Alignment.centerRight
                : Alignment.centerLeft,
            child: AnimatedContainer(
              duration: const Duration(milliseconds: 120),
              decoration: BoxDecoration(
                color: _menuHighlighted || widget.highlighted
                    ? context.theme.accent.withValues(alpha: 0.16)
                    : Colors.transparent,
                borderRadius: const BorderRadius.all(Radius.circular(8)),
              ),
              child: messageContent,
            ),
          )
        : MessageBubble(
            key: Key('message-bubble-${widget.message.id}'),
            isCurrentUser: presentation.isCurrentUser,
            showNip: presentation.showNip,
            showBubble: widget.message.showMessageBubble,
            includeNip: widget.message.includeMessageBubbleNip,
            clip: widget.message.clipMessageBubble,
            highlighted: _menuHighlighted || widget.highlighted,
            padding: widget.message.messageBubblePadding,
            forceIsCurrentUserColor:
                widget.message.forceCurrentMessageBubbleColor,
            outerTimeAndStatusWidget:
                widget.message.useOuterMessageDateAndStatus
                ? MessageDatetimeAndStatus(
                    message: widget.message,
                    isCurrentUser: presentation.isCurrentUser,
                    hideStatus: widget.message.hideOuterMessageStatus,
                    isRepresentative:
                        widget.isBot &&
                        widget.message.senderId != widget.conversationOwnerId &&
                        !presentation.isCurrentUser,
                  )
                : null,
            child: messageContent,
          );
    final policy = MessageActionPolicy(
      message: widget.message,
      currentUserId: widget.currentUserId,
      currentUserRole: widget.currentUserRole,
      now: DateTime.now(),
    );
    final copyableImage =
        widget.message.isImage &&
            const {
              'DONE',
              'READ',
            }.contains(widget.message.mediaStatus.toUpperCase())
        ? existingLocalFile(widget.message.mediaUrl)
        : null;
    final contextMenuMessage = ContextMenuWidget(
      menuProvider: (request) {
        request.onShowMenu.addListener(() {
          if (mounted) setState(() => _menuHighlighted = true);
        });
        request.onHideMenu.addListener(() {
          if (mounted) setState(() => _menuHighlighted = false);
        });
        return buildMessageActionsMenu(
          context: context,
          message: widget.message,
          policy: policy,
          callbacks: MessageActionCallbacks(
            onReply: widget.onReply,
            onForward: widget.onForward,
            onCopyText: (content) =>
                Clipboard.setData(ClipboardData(text: content)),
            onCopyImage: copyableImage == null
                ? null
                : () => unawaited(_copyImage(copyableImage)),
            onGenerateQr: (content) =>
                showMessageQrDialog(context, content: content),
            onAddSticker: widget.message.stickerId?.trim().isNotEmpty == true
                ? () => unawaited(_addSticker())
                : copyableImage == null
                ? null
                : () => unawaited(
                    showAddImageStickerDialog(
                      context,
                      file: copyableImage,
                      onConfirm: widget.onAddImageAsSticker,
                    ),
                  ),
            onSelect: widget.onSelect,
            onTogglePin: widget.onTogglePin,
            onSaveAs: () => _saveMessage(widget.message),
            onRecall: widget.onRecall,
            onDelete: widget.onDelete,
          ),
          selectedText: _findSelectedText(context),
          hasSelectedMessages: widget.inSelectionMode,
        );
      },
      child: _MessageQuickReplyDetector(
        onReply: policy.canReply ? widget.onReply : null,
        child: renderedMessage,
      ),
    );
    final interactiveMessage = bypassActions
        ? renderedMessage
        : contextMenuMessage;
    final messageColumn = Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (presentation.showSender)
          GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: () => _showUser(userId: widget.message.senderId),
            child: MouseRegion(
              cursor: SystemMouseCursors.click,
              child: MessageName(
                userName: widget.message.senderName,
                userId: widget.message.senderId,
                userIdentityNumber: widget.message.senderIdentityNumber,
                verified: widget.message.senderIsVerified,
                isBot: widget.message.senderIsBot,
                showIdentityNumber: showIdentityNumber,
              ),
            ),
          ),
        interactiveMessage,
      ],
    );

    Widget child;
    if (presentation.showSender && presentation.showAvatar) {
      child = Row(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const SizedBox(width: 8),
          InkWell(
            customBorder: const CircleBorder(),
            onTap: () => _showUser(userId: widget.message.senderId),
            child: AvatarView(
              userId: widget.message.senderId,
              name: widget.message.senderName,
              avatarUrl: widget.message.senderAvatarUrl,
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
      );
    } else {
      child = Padding(
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
    }

    final selectionWrapped = bypassActions
        ? child
        : GestureDetector(
            key: Key('message-selection-${widget.message.id}'),
            behavior: HitTestBehavior.translucent,
            onTap: widget.inSelectionMode ? widget.onToggleSelection : null,
            onLongPress: widget.inSelectionMode || !policy.canSelect
                ? null
                : widget.onSelect,
            child: Row(
              children: [
                AnimatedContainer(
                  duration: const Duration(milliseconds: 300),
                  curve: Curves.easeInOut,
                  width: widget.inSelectionMode ? 48 : 0,
                  height: 24,
                  alignment: Alignment.center,
                  child: widget.inSelectionMode
                      ? _SelectionIndicator(selected: widget.selected)
                      : null,
                ),
                Expanded(
                  child: IgnorePointer(
                    ignoring: widget.inSelectionMode,
                    child: child,
                  ),
                ),
              ],
            ),
          );

    return VisibilityDetector(
      key: ValueKey('message_visibility_${widget.message.id}'),
      onVisibilityChanged: (info) {
        if (widget.message.mentionRead == false && info.visibleFraction >= 1) {
          widget.onMarkMentionRead();
        }
      },
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (row.dateTime != null) MessageDayTimeChip(dateTime: row.dateTime!),
          Padding(
            padding: row.sameUserPrevious
                ? EdgeInsets.zero
                : const EdgeInsets.only(top: 8),
            child: selectionWrapped,
          ),
        ],
      ),
    );
  }

  void _showUser({String? userId, String? identityNumber}) {
    unawaited(
      showMessageUserDialog(
        context,
        account: widget.account,
        userId: userId,
        identityNumber: identityNumber,
      ),
    );
  }

  bool _canForwardPreview(MessageListEntry message) => MessageActionPolicy(
    message: message,
    currentUserId: widget.currentUserId,
    currentUserRole: widget.currentUserRole,
    now: DateTime.now(),
  ).canForward;

  Future<bool> _forwardPreviewMessage(MessageListEntry message) async {
    if (!_canForwardPreview(message)) return false;
    final targetConversationId = await showForwardConversationSelector(
      context,
      account: widget.account,
    );
    if (targetConversationId == null || !mounted) return false;
    await widget.account.forwardMessages(
      targetConversationId: targetConversationId,
      sourceMessageIds: [message.id],
    );
    return true;
  }

  void _openImage(MessageListEntry message) {
    unawaited(_openConversationImage(message));
  }

  Future<void> _openConversationImage(MessageListEntry message) async {
    List<ImagePreviewEntry> images;
    try {
      images = await _loadConversationImages(message.id, before: 30, after: 30);
    } on Object {
      final source = _messageMediaSource(message);
      images = source.isEmpty
          ? const []
          : [
              ImagePreviewEntry(
                id: message.id,
                source: source,
                name: message.mediaName,
                canForward: _canForwardPreview(message),
              ),
            ];
    }
    if (!mounted) return;
    final index = images.indexWhere((item) => item.id == message.id);
    if (index < 0) return;
    Navigator.of(context, rootNavigator: true).push(
      MaterialPageRoute<void>(
        builder: (_) => ImagePreviewPage(
          images: images,
          initialIndex: index,
          onCopy: (image) {
            final file = existingLocalFile(image.source);
            if (file != null) return _copyImage(file);
          },
          onSave: (image) => _saveSource(image.source, image.name),
          onForward: (image) => _forwardPreviewImage(image.id),
          loadOlder: (boundaryId) =>
              _loadConversationImages(boundaryId, before: 40, after: 0),
          loadNewer: (boundaryId) =>
              _loadConversationImages(boundaryId, before: 0, after: 40),
        ),
      ),
    );
  }

  Future<List<ImagePreviewEntry>> _loadConversationImages(
    String targetMessageId, {
    required int before,
    required int after,
  }) async {
    final items = await widget.account.imageMessagesAround(
      conversationId: widget.message.conversationId,
      targetMessageId: targetMessageId,
      before: before,
      after: after,
    );
    return items
        .map(
          (item) => ImagePreviewEntry(
            id: item.messageId,
            source: item.mediaUrl,
            name: item.mediaName,
            canForward: item.canForward,
          ),
        )
        .toList(growable: false);
  }

  Future<bool> _forwardPreviewImage(String messageId) async {
    final targetConversationId = await showForwardConversationSelector(
      context,
      account: widget.account,
    );
    if (targetConversationId == null || !mounted) return false;
    await widget.account.forwardMessages(
      targetConversationId: targetConversationId,
      sourceMessageIds: [messageId],
    );
    return true;
  }

  void _openVideo(MessageListEntry message) {
    final source = _messageMediaSource(message);
    if (source.isEmpty) return;
    Navigator.of(context, rootNavigator: true).push(
      MaterialPageRoute<void>(
        builder: (_) => VideoPreviewPage(
          source: source,
          title: message.mediaName,
          onForward: _canForwardPreview(message)
              ? () => _forwardPreviewMessage(message)
              : null,
        ),
      ),
    );
  }

  void _openPost(MessageListEntry message) {
    Navigator.of(context, rootNavigator: true).push(
      MaterialPageRoute<void>(
        builder: (_) => PostPreviewPage(
          content: message.content,
          title: message.senderName,
        ),
      ),
    );
  }

  void _openTranscript(String transcriptId) {
    unawaited(
      showTranscriptDialog(
        context,
        account: widget.account,
        transcriptId: transcriptId,
        currentUserId: widget.currentUserId,
      ),
    );
  }

  void _openSticker(MessageListEntry message) {
    final stickerId = message.stickerId;
    if (stickerId == null || stickerId.isEmpty) return;
    unawaited(
      showStickerDetailPage(
        context,
        account: widget.account,
        stickerId: stickerId,
        onAlbumChanged: widget.onStickerAlbumChanged,
      ),
    );
  }

  Future<void> _openFile(MessageListEntry message) async {
    final source = _messageMediaSource(message);
    if (source.isEmpty) return;
    try {
      final result = await openMessageFile(source);
      if (result.type.name == 'done' || !mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(result.message)));
    } on Object catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(error.toString())));
    }
  }

  Future<void> _saveMessage(MessageListEntry message) =>
      _saveSource(_messageMediaSource(message), message.mediaName);

  Future<void> _copyImage(File file) async {
    try {
      final copied = await copyLocalFileToClipboard(file);
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(copied ? context.l10n.copy : context.l10n.failed),
        ),
      );
    } on Object catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(error.toString())));
    }
  }

  void _openAction(String action) {
    final value = action.trim();
    if (value.startsWith('input:')) {
      final content = value.substring(6).trim();
      if (content.isNotEmpty) {
        unawaited(
          widget.account.sendText(
            conversationId: widget.message.conversationId,
            content: content,
            quoteMessageId: null,
          ),
        );
      }
      return;
    }
    final uri = Uri.tryParse(value);
    if (uri == null) return;
    unawaited(launchUrl(uri));
  }

  Future<void> _addSticker() async {
    try {
      await widget.onAddSticker();
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(context.l10n.successful)));
    } on Object {
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(context.l10n.addStickerFailed)));
    }
  }

  void _handleStrangerAction(String action) {
    unawaited(() async {
      try {
        final uri = await widget.onStrangerAction(action);
        if (mounted && (action == 'block' || action == 'add_contact')) {
          setState(() => _strangerResolved = true);
        }
        if (uri != null) _openAction(uri);
      } on Object catch (error) {
        if (!mounted) return;
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(error.toString())));
      }
    }());
  }

  Future<void> _saveSource(String source, String? name) async {
    if (source.isEmpty) return;
    try {
      await saveMessageFileAs(source, suggestedName: name);
    } on Object catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(error.toString())));
    }
  }
}

class _SelectionIndicator extends StatelessWidget {
  const _SelectionIndicator({required this.selected});

  final bool selected;

  @override
  Widget build(BuildContext context) => AnimatedContainer(
    duration: const Duration(milliseconds: 120),
    width: 18,
    height: 18,
    decoration: BoxDecoration(
      shape: BoxShape.circle,
      color: selected ? context.theme.accent : Colors.transparent,
      border: Border.all(
        color: selected ? context.theme.accent : context.theme.secondaryText,
      ),
    ),
    child: selected
        ? const Icon(Icons.check, size: 12, color: Colors.white)
        : null,
  );
}

class _MessageQuickReplyDetector extends StatefulWidget {
  const _MessageQuickReplyDetector({
    required this.child,
    required this.onReply,
  });

  final Widget child;
  final VoidCallback? onReply;

  @override
  State<_MessageQuickReplyDetector> createState() =>
      _MessageQuickReplyDetectorState();
}

class _MessageQuickReplyDetectorState
    extends State<_MessageQuickReplyDetector> {
  Duration? _lastPrimaryDownTime;
  Offset? _lastPrimaryDownPosition;

  @override
  Widget build(BuildContext context) => Listener(
    behavior: HitTestBehavior.translucent,
    onPointerDown: (event) {
      if ((event.buttons & kPrimaryButton) == 0 ||
          _hitsSelectableText(context, event.position)) {
        _reset();
        return;
      }
      final previousTime = _lastPrimaryDownTime;
      final previousPosition = _lastPrimaryDownPosition;
      _lastPrimaryDownTime = event.timeStamp;
      _lastPrimaryDownPosition = event.position;
      if (previousTime == null || previousPosition == null) return;
      if (event.timeStamp - previousTime > kDoubleTapTimeout ||
          (event.position - previousPosition).distance > kDoubleTapSlop) {
        return;
      }
      _reset();
      widget.onReply?.call();
    },
    child: widget.child,
  );

  void _reset() {
    _lastPrimaryDownTime = null;
    _lastPrimaryDownPosition = null;
  }
}

bool _hitsSelectableText(BuildContext context, Offset globalPosition) {
  var hit = false;
  void visit(Element element) {
    if (hit) return;
    final renderObject = element.renderObject;
    if (renderObject is RenderParagraph &&
        renderObject.registrar != null &&
        renderObject.attached &&
        _paragraphHitsText(renderObject, globalPosition)) {
      hit = true;
      return;
    }
    if (renderObject is RenderEditable && renderObject.attached) {
      final localPosition = renderObject.globalToLocal(globalPosition);
      if (renderObject.paintBounds.contains(localPosition)) {
        hit = true;
        return;
      }
    }
    element.visitChildren(visit);
  }

  context.visitChildElements(visit);
  return hit;
}

String? _findSelectedText(BuildContext context) {
  String? selectedText;
  void visit(Element element) {
    if (selectedText != null) return;
    if (element is StatefulElement && element.state is EditableTextState) {
      final state = element.state as EditableTextState;
      final value = state.textEditingValue;
      final selection = value.selection;
      if (selection.isValid && !selection.isCollapsed) {
        selectedText = selection.textInside(value.text);
        return;
      }
    }
    element.visitChildren(visit);
  }

  if (context is Element) visit(context);
  return selectedText;
}

bool _paragraphHitsText(RenderParagraph paragraph, Offset globalPosition) {
  final localPosition = paragraph.globalToLocal(globalPosition);
  if (!paragraph.paintBounds.contains(localPosition)) return false;
  final text = paragraph.text.toPlainText(includeSemanticsLabels: false);
  if (text.isEmpty) return false;
  final offset = paragraph.getPositionForOffset(localPosition).offset;
  for (final index in {offset, offset - 1}) {
    if (index < 0 || index >= text.length) continue;
    if (text.substring(index, index + 1).trim().isEmpty) continue;
    final boxes = paragraph.getBoxesForSelection(
      TextSelection(baseOffset: index, extentOffset: index + 1),
    );
    if (boxes.any((box) => box.toRect().contains(localPosition))) return true;
  }
  return false;
}

class VoiceRecorderBar extends StatelessWidget {
  const VoiceRecorderBar({
    required this.state,
    required this.onCancel,
    required this.onStop,
    required this.onRetry,
    required this.onSend,
    super.key,
  });

  final VoiceRecorderState state;
  final Future<void> Function() onCancel;
  final Future<void> Function() onStop;
  final Future<void> Function() onRetry;
  final Future<void> Function() onSend;

  @override
  Widget build(BuildContext context) {
    final recording = state.recording;
    final isRecording = state.status == VoiceRecorderStatus.recording;
    final isSending = state.status == VoiceRecorderStatus.sending;
    return Container(
      key: const Key('voice-recorder-bar'),
      height: 56,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      color: context.theme.primary,
      child: Row(
        children: [
          _RecorderAction(
            actionKey: const Key('voice-cancel'),
            icon: Icons.cancel_outlined,
            tooltip: 'Cancel recording',
            onPressed: isSending ? null : onCancel,
          ),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: recording == null
                  ? _RecordingDuration(duration: state.elapsed)
                  : _VoiceRecordingPreview(recording: recording),
            ),
          ),
          if (isSending)
            const SizedBox.square(
              dimension: 40,
              child: Center(
                child: SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              ),
            )
          else if (isRecording)
            _RecorderAction(
              actionKey: const Key('voice-stop'),
              icon: Icons.stop_circle_outlined,
              color: context.theme.accent,
              tooltip: 'Stop recording',
              onPressed: onStop,
            )
          else
            _RecorderAction(
              actionKey: const Key('voice-retry'),
              icon: Icons.replay,
              tooltip: 'Record again',
              onPressed: onRetry,
            ),
          _RecorderAction(
            actionKey: const Key('voice-send'),
            asset: MixinAssets.send,
            tooltip: 'Send voice message',
            onPressed: isSending ? null : onSend,
          ),
        ],
      ),
    );
  }
}

class _RecorderAction extends StatelessWidget {
  const _RecorderAction({
    required this.actionKey,
    required this.tooltip,
    required this.onPressed,
    this.icon,
    this.asset,
    this.color,
  });

  final Key actionKey;
  final String tooltip;
  final Future<void> Function()? onPressed;
  final IconData? icon;
  final String? asset;
  final Color? color;

  @override
  Widget build(BuildContext context) => SizedBox.square(
    dimension: 40,
    child: IconButton(
      key: actionKey,
      tooltip: tooltip,
      onPressed: onPressed == null ? null : () => unawaited(onPressed!()),
      icon: asset == null
          ? Icon(icon, size: 24, color: color ?? context.theme.icon)
          : SvgPicture.asset(
              asset!,
              width: 24,
              height: 24,
              colorFilter: ColorFilter.mode(
                color ?? context.theme.icon,
                BlendMode.srcIn,
              ),
            ),
    ),
  );
}

class _RecordingDuration extends StatelessWidget {
  const _RecordingDuration({required this.duration});

  final Duration duration;

  @override
  Widget build(BuildContext context) => Row(
    mainAxisAlignment: MainAxisAlignment.center,
    children: [
      const SizedBox.square(
        dimension: 8,
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: Color(0xFFE57874),
            shape: BoxShape.circle,
          ),
        ),
      ),
      const SizedBox(width: 4),
      Text(
        _formatVoiceDuration(duration),
        style: TextStyle(color: context.theme.text, fontSize: 14),
      ),
    ],
  );
}

class _VoiceRecordingPreview extends StatefulWidget {
  const _VoiceRecordingPreview({required this.recording});

  final VoiceRecording recording;

  @override
  State<_VoiceRecordingPreview> createState() => _VoiceRecordingPreviewState();
}

class _VoiceRecordingPreviewState extends State<_VoiceRecordingPreview> {
  final _coordinator = AudioMessagePlaybackCoordinator.instance;
  late String _previewId;

  @override
  void initState() {
    super.initState();
    _previewId = 'voice-preview-${widget.recording.path.hashCode}';
    _coordinator
      ..attach()
      ..addListener(_onPlaybackChanged);
  }

  @override
  void didUpdateWidget(covariant _VoiceRecordingPreview oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.recording.path == widget.recording.path) return;
    if (_coordinator.currentMessageId == _previewId) _coordinator.stop();
    _previewId = 'voice-preview-${widget.recording.path.hashCode}';
  }

  @override
  void dispose() {
    if (_coordinator.currentMessageId == _previewId) _coordinator.stop();
    _coordinator
      ..removeListener(_onPlaybackChanged)
      ..detach();
    super.dispose();
  }

  void _onPlaybackChanged() {
    if (mounted) setState(() {});
  }

  Future<void> _togglePlayback() async {
    if (_coordinator.currentMessageId == _previewId && _coordinator.isPlaying) {
      _coordinator.stop();
      return;
    }
    await _coordinator.play(_previewId, widget.recording.path);
  }

  @override
  Widget build(BuildContext context) {
    final playing =
        _coordinator.currentMessageId == _previewId && _coordinator.isPlaying;
    final duration = widget.recording.duration;
    final progress = playing && duration.inMilliseconds > 0
        ? (_coordinator.position.inMilliseconds / duration.inMilliseconds)
              .clamp(0.0, 1.0)
        : 0.0;
    return Container(
      height: 32,
      decoration: BoxDecoration(
        color: context.theme.listSelected,
        borderRadius: const BorderRadius.all(Radius.circular(15)),
      ),
      child: Row(
        children: [
          IconButton(
            key: const Key('voice-preview-play'),
            onPressed: () => unawaited(_togglePlayback()),
            padding: const EdgeInsets.all(6),
            icon: Icon(
              playing ? Icons.stop : Icons.play_arrow,
              size: 20,
              color: context.theme.icon,
            ),
          ),
          Expanded(
            child: SizedBox(
              height: 20,
              child: CustomPaint(
                painter: AudioWaveformPainter(
                  waveform: widget.recording.waveform,
                  progress: progress,
                  backgroundColor: context.theme.waveformBackground,
                  foregroundColor: context.theme.waveformForeground,
                ),
              ),
            ),
          ),
          const SizedBox(width: 10),
          Text(
            _formatVoiceDuration(duration),
            style: TextStyle(color: context.theme.text, fontSize: 14),
          ),
          const SizedBox(width: 12),
        ],
      ),
    );
  }
}

String _formatVoiceDuration(Duration duration) {
  final minutes = duration.inMinutes;
  final seconds = duration.inSeconds.remainder(60);
  return '$minutes:${seconds.toString().padLeft(2, '0')}';
}

class ChatInputBar extends StatefulWidget {
  const ChatInputBar({
    required this.controller,
    required this.focusNode,
    required this.quoteMessage,
    required this.sending,
    required this.onChanged,
    required this.onCancelQuote,
    required this.onSend,
    this.onAddPressed,
    this.onStickerPressed,
    this.stickerAction,
    this.onVoicePressed,
    this.voiceButtonActive = false,
    super.key,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final MessageListEntry? quoteMessage;
  final bool sending;
  final ValueChanged<String> onChanged;
  final VoidCallback onCancelQuote;
  final Future<void> Function() onSend;
  final VoidCallback? onAddPressed;
  final VoidCallback? onStickerPressed;
  final Widget? stickerAction;
  final VoidCallback? onVoicePressed;
  final bool voiceButtonActive;

  @override
  State<ChatInputBar> createState() => _ChatInputBarState();
}

class _ChatInputBarState extends State<ChatInputBar> {
  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_handleTextChanged);
  }

  @override
  void didUpdateWidget(covariant ChatInputBar oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller == widget.controller) return;
    oldWidget.controller.removeListener(_handleTextChanged);
    widget.controller.addListener(_handleTextChanged);
  }

  @override
  void dispose() {
    widget.controller.removeListener(_handleTextChanged);
    super.dispose();
  }

  void _handleTextChanged() => setState(() {});

  @override
  Widget build(BuildContext context) {
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    final sendable =
        widget.controller.text.trim().isNotEmpty && !widget.sending;
    return Container(
      key: const Key('chat-input-bar'),
      color: context.theme.primary,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (widget.quoteMessage != null)
            _QuoteInputPreview(
              message: widget.quoteMessage!,
              onCancel: widget.onCancelQuote,
            ),
          Container(
            constraints: const BoxConstraints(minHeight: 56),
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                _ChatInputAction(
                  actionKey: const Key('chat-add'),
                  asset: MixinAssets.add,
                  tooltip: 'Add attachment',
                  onPressed: widget.onAddPressed,
                ),
                const SizedBox(width: 6),
                widget.stickerAction ??
                    _ChatInputAction(
                      actionKey: const Key('chat-sticker'),
                      asset: MixinAssets.sticker,
                      onPressed: widget.onStickerPressed,
                    ),
                const SizedBox(width: 16),
                Expanded(
                  child: Container(
                    key: const Key('chat-input-surface'),
                    constraints: const BoxConstraints(minHeight: 40),
                    decoration: BoxDecoration(
                      color: Theme.of(context).brightness == Brightness.dark
                          ? const Color.fromRGBO(255, 255, 255, 0.08)
                          : const Color.fromRGBO(245, 247, 250, 1),
                      borderRadius: const BorderRadius.all(Radius.circular(4)),
                    ),
                    alignment: Alignment.center,
                    child: TextField(
                      key: const Key('chat-input'),
                      controller: widget.controller,
                      focusNode: widget.focusNode,
                      onChanged: widget.onChanged,
                      minLines: 1,
                      maxLines: 7,
                      textInputAction: TextInputAction.newline,
                      onSubmitted: (_) {
                        if (sendable) widget.onSend();
                      },
                      cursorColor: context.theme.accent,
                      style: TextStyle(fontSize: 14, color: context.theme.text),
                      decoration: InputDecoration(
                        isDense: true,
                        border: InputBorder.none,
                        enabledBorder: InputBorder.none,
                        focusedBorder: InputBorder.none,
                        hintText: l10n?.typeMessage ?? 'Type message',
                        hintStyle: TextStyle(
                          color: context.theme.secondaryText,
                          fontSize: 14,
                        ),
                        contentPadding: const EdgeInsets.fromLTRB(8, 9, 8, 8),
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 16),
                SizedBox.square(
                  dimension: 40,
                  child: widget.sending
                      ? const Center(
                          child: SizedBox.square(
                            dimension: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          ),
                        )
                      : AnimatedSwitcher(
                          duration: const Duration(milliseconds: 120),
                          transitionBuilder: (child, animation) =>
                              ScaleTransition(scale: animation, child: child),
                          child: sendable
                              ? _ChatInputAction(
                                  actionKey: const Key('chat-send'),
                                  asset: MixinAssets.send,
                                  tooltip: 'Send',
                                  onPressed: widget.onSend,
                                )
                              : _ChatInputAction(
                                  actionKey: const Key('chat-voice'),
                                  asset: MixinAssets.microphone,
                                  tooltip: 'Voice message',
                                  active: widget.voiceButtonActive,
                                  onPressed: widget.onVoicePressed,
                                ),
                        ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _ChatInputAction extends StatelessWidget {
  const _ChatInputAction({
    required this.asset,
    required this.actionKey,
    this.tooltip,
    this.onPressed,
    this.active = false,
  });

  final String asset;
  final String? tooltip;
  final Key actionKey;
  final VoidCallback? onPressed;
  final bool active;

  @override
  Widget build(BuildContext context) => SizedBox.square(
    dimension: 40,
    child: IconButton(
      key: actionKey,
      onPressed: onPressed,
      tooltip: tooltip,
      padding: const EdgeInsets.all(8),
      icon: SvgPicture.asset(
        asset,
        width: 24,
        height: 24,
        colorFilter: ColorFilter.mode(
          active ? context.theme.accent : context.theme.icon,
          BlendMode.srcIn,
        ),
      ),
    ),
  );
}

class _QuoteInputPreview extends StatelessWidget {
  const _QuoteInputPreview({required this.message, required this.onCancel});

  final MessageListEntry message;
  final VoidCallback onCancel;

  @override
  Widget build(BuildContext context) => Container(
    key: const Key('chat-quote-preview'),
    constraints: const BoxConstraints(minHeight: 50),
    color: context.theme.popUp,
    child: Row(
      children: [
        const SizedBox(width: 16),
        Container(
          key: const Key('chat-quote-accent'),
          width: 3,
          height: 36,
          decoration: BoxDecoration(
            color: context.theme.accent,
            borderRadius: const BorderRadius.all(Radius.circular(2)),
          ),
        ),
        const SizedBox(width: 10),
        Expanded(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                message.senderName,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(color: context.theme.accent, fontSize: 13),
              ),
              const SizedBox(height: 2),
              Text(
                _messagePreview(context, message),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  color: context.theme.secondaryText,
                  fontSize: 13,
                ),
              ),
            ],
          ),
        ),
        IconButton(
          key: const Key('chat-quote-cancel'),
          onPressed: onCancel,
          icon: const Icon(Icons.close, size: 18),
          color: context.theme.secondaryText,
        ),
        const SizedBox(width: 4),
      ],
    ),
  );
}

class _SelectionBottomBar extends StatelessWidget {
  const _SelectionBottomBar({
    required this.selectedMessages,
    required this.currentUserId,
    required this.onCombineForward,
    required this.onForward,
    required this.onCopy,
    required this.onDelete,
    required this.onRecall,
  });

  final List<MessageListEntry> selectedMessages;
  final String currentUserId;
  final Future<void> Function(List<MessageListEntry>) onCombineForward;
  final Future<void> Function(List<MessageListEntry>) onForward;
  final Future<void> Function() onCopy;
  final Future<void> Function(List<MessageListEntry>) onDelete;
  final Future<void> Function(List<MessageListEntry>) onRecall;

  @override
  Widget build(BuildContext context) {
    final policies = selectedMessages
        .map(
          (message) => MessageActionPolicy(
            message: message,
            currentUserId: currentUserId,
            currentUserRole: null,
            now: DateTime.now(),
          ),
        )
        .toList(growable: false);
    final canForward =
        selectedMessages.length < 100 &&
        policies.every((item) => item.canForward);
    final canCombineForward =
        selectedMessages.length >= 2 &&
        selectedMessages.length < 100 &&
        policies.every((item) => item.canCombineForward);
    final canRecall =
        selectedMessages.length < 100 &&
        policies.every((item) => item.canRecall);

    return Container(
      key: const Key('chat-selection-bar'),
      height: 80,
      color: context.theme.primary,
      child: Row(
        children: [
          _SelectionAction(
            label: context.l10n.combineAndForward,
            icon: Icons.topic_outlined,
            enabled: canCombineForward,
            onTap: canCombineForward
                ? () => onCombineForward(selectedMessages)
                : null,
          ),
          _SelectionAction(
            label: context.l10n.oneByOneForward,
            icon: Icons.forward,
            enabled: canForward,
            onTap: canForward ? () => onForward(selectedMessages) : null,
          ),
          _SelectionAction(
            label: context.l10n.copy,
            icon: Icons.copy,
            onTap: onCopy,
          ),
          _SelectionAction(
            label: context.l10n.delete,
            icon: Icons.delete_outline,
            onTap: () => _showSelectionDeleteDialog(
              context,
              selectedMessages: selectedMessages,
              canRecall: canRecall,
              onDelete: onDelete,
              onRecall: onRecall,
            ),
          ),
        ],
      ),
    );
  }
}

class _SelectionAction extends StatelessWidget {
  const _SelectionAction({
    required this.label,
    required this.icon,
    this.enabled = true,
    this.onTap,
  });

  final String label;
  final IconData icon;
  final bool enabled;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final active = enabled && onTap != null;
    return Expanded(
      child: InkWell(
        onTap: active ? onTap : null,
        child: Opacity(
          opacity: active ? 1 : 0.5,
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(icon, size: 24, color: context.theme.icon),
              const SizedBox(height: 8),
              Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(color: context.theme.text, fontSize: 14),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

Future<void> _showSelectionDeleteDialog(
  BuildContext context, {
  required List<MessageListEntry> selectedMessages,
  required bool canRecall,
  required Future<void> Function(List<MessageListEntry>) onDelete,
  required Future<void> Function(List<MessageListEntry>) onRecall,
}) async {
  final action = await showDialog<_SelectionDeleteAction>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(context.l10n.delete),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(context.l10n.cancel),
        ),
        TextButton(
          onPressed: () =>
              Navigator.pop(context, _SelectionDeleteAction.deleteForMe),
          child: Text(context.l10n.deleteForMe),
        ),
        if (canRecall)
          TextButton(
            onPressed: () => Navigator.pop(
              context,
              _SelectionDeleteAction.deleteForEveryone,
            ),
            child: Text(context.l10n.deleteForEveryone),
          ),
      ],
    ),
  );
  switch (action) {
    case _SelectionDeleteAction.deleteForMe:
      await onDelete(selectedMessages);
    case _SelectionDeleteAction.deleteForEveryone:
      await onRecall(selectedMessages);
    case null:
      return;
  }
}

enum _SelectionDeleteAction { deleteForMe, deleteForEveryone }

Future<void> _ignoreMutation(Future<void> mutation) async {
  try {
    await mutation;
  } on Object {
    // MessageListController already exposes the error through its state.
  }
}

String _messageMediaSource(MessageListEntry message) =>
    message.mediaUrl?.trim() ?? '';

String _messagePreview(BuildContext context, MessageListEntry message) {
  if (message.isText || message.isPost) return message.content;
  if (message.isImage) {
    return message.caption?.trim().isNotEmpty == true
        ? message.caption!
        : context.l10n.image;
  }
  if (message.isVideo) return context.l10n.video;
  if (message.isAudio) return context.l10n.audio;
  if (message.isSticker) return context.l10n.sticker;
  if (message.category.endsWith('_DATA')) {
    return message.mediaName ?? context.l10n.file;
  }
  return message.content.isEmpty
      ? context.l10n.messageNotSupport
      : message.content;
}
