import 'dart:async';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:intl/intl.dart';
import 'package:provider/provider.dart';

import '../constants/assets.dart';
import '../controllers/message_action_controller.dart';
import '../controllers/message_attachment_controller.dart';
import '../controllers/message_controller.dart';
import '../controllers/settings_controller.dart';
import '../controllers/settings_store.dart';
import '../controllers/sticker_controller.dart';
import '../controllers/voice_recorder_controller.dart';
import '../l10n/l10n.dart';
import '../models/conversation_list_entry.dart';
import '../models/message_list_entry.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../theme.dart';
import '../utils/app_logger.dart';
import '../utils/web_view.dart';
import 'chat/chat_header.dart';
import 'chat/chat_input_bar.dart';
import 'chat/chat_message_list.dart';
import 'chat/chat_message_preview.dart';
import 'chat/chat_scroll_coordinator.dart';
import 'chat/chat_selection_bar.dart';
import 'chat/chat_voice_recorder.dart';
import 'chat_drop_overlay.dart';
import 'message_navigation_scope.dart';
import 'show_attachment_preview_dialog.dart';
import 'show_forward_conversation_selector.dart';
import 'sticker_page/sticker_button.dart';
import 'toast.dart';

export 'chat/chat_input_bar.dart' show ChatInputBar;
export 'chat/chat_message_list.dart' show shouldMarkMentionRead;
export 'chat/chat_voice_recorder.dart'
    show VoiceRecorderBar, VoiceRecorderBarOverlayComposition;

const _maxTextLength = 64 * 1024;

class ChatView extends StatefulWidget {
  const ChatView({
    required this.account,
    required this.conversation,
    required this.draft,
    required this.onDraftChanged,
    this.media,
    super.key,
    this.onBack,
    this.onSearch,
    this.onInfo,
    this.onOpenPinnedMessages,
    this.onSelectConversation,
    this.onSelectConversationInfo,
    this.showInfoAction = true,
    this.locateMessageId,
    this.locateSourceMessageId,
    this.locateRequest = 0,
  });

  final rust.AccountHandle account;
  final rust.MediaHandle? media;
  final ConversationListEntry conversation;
  final String draft;
  final ValueChanged<String> onDraftChanged;
  final ValueChanged<ConversationListEntry>? onSelectConversation;
  final ValueChanged<ConversationListEntry>? onSelectConversationInfo;
  final VoidCallback? onBack;
  final VoidCallback? onSearch;
  final VoidCallback? onInfo;
  final VoidCallback? onOpenPinnedMessages;
  final bool showInfoAction;
  final String? locateMessageId;
  final String? locateSourceMessageId;
  final int locateRequest;

  @override
  State<ChatView> createState() => _ChatViewState();
}

class _ChatViewState extends State<ChatView>
    with SingleTickerProviderStateMixin {
  late MessageActionController _messageActions;
  late MessageController _messageController;
  late final ChatScrollCoordinator _scrollCoordinator;
  late String _currentUserId;
  late final MentionTextEditingController _inputController;
  late final FocusNode _inputFocusNode;
  late final VoiceRecorderController _voiceRecorderController;
  late StickerController _stickerController;
  late final SettingsStore _settings;
  MessageListEntry? _quoteMessage;
  String? _highlightedMessageId;
  double _highlightOpacity = 0;
  late final AnimationController _highlightController;
  final Set<String> _selectedMessageIds = {};
  bool _isEncryptConversation = true;
  bool _showPinnedMessage = false;
  bool _showScamWarning = false;

  @override
  void initState() {
    super.initState();
    _settings = context.read<SettingsController>().store;
    _highlightController =
        AnimationController(
            duration: const Duration(milliseconds: 700),
            vsync: this,
          )
          ..addListener(_onHighlightAnimation)
          ..addStatusListener(_onHighlightStatus);
    _scrollCoordinator = ChatScrollCoordinator();
    _createMessageController();
    unawaited(_loadOverlayPreferences());
    _inputController = MentionTextEditingController(text: widget.draft);
    _inputController.addListener(_onInputChanged);
    _inputFocusNode = FocusNode(debugLabel: 'chat_input');
    _voiceRecorderController = VoiceRecorderController(
      media: widget.media,
      backend: widget.media == null
          ? const _UnavailableVoiceRecorderBackend()
          : null,
    )..addListener(_onVoiceRecorderChanged);
    _stickerController = StickerController(
      account: widget.account,
      settings: _settings,
    );
    _scheduleInputFocus();
    if (widget.locateMessageId != null) {
      WidgetsBinding.instance.addPostFrameCallback(
        (_) => unawaited(
          _locateMessage(
            widget.locateMessageId!,
            sourceMessageId: widget.locateSourceMessageId,
          ),
        ),
      );
    }
  }

  @override
  void didUpdateWidget(covariant ChatView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.account, widget.account) ||
        oldWidget.conversation.id != widget.conversation.id) {
      _messageController.dispose();
      _messageActions.dispose();
      _createMessageController();
      unawaited(_loadOverlayPreferences());
      _quoteMessage = null;
      _highlightedMessageId = null;
      _highlightOpacity = 0;
      _highlightController.reset();
      _selectedMessageIds.clear();
      unawaited(_voiceRecorderController.cancel());
      if (!identical(oldWidget.account, widget.account)) {
        _stickerController.dispose();
        _stickerController = StickerController(
          account: widget.account,
          settings: _settings,
        );
      }
      _scheduleInputFocus();
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
        (_) => unawaited(
          _locateMessage(
            widget.locateMessageId!,
            sourceMessageId: widget.locateSourceMessageId,
          ),
        ),
      );
    }
  }

  void _createMessageController() {
    _currentUserId = widget.account.accountId();
    _isEncryptConversation = !widget.conversation.isBot;
    _messageController = MessageController(
      account: widget.account,
      conversation: widget.conversation,
      limit: 60,
      initialMessageId: widget.locateMessageId,
    );
    _messageActions = MessageActionController(
      account: widget.account,
      conversation: widget.conversation,
      messageController: _messageController,
    );
    unawaited(_refreshConversationEncryption());
  }

  String get _showPinnedMessageKey =>
      'show_pin_message_${widget.account.accountId()}_${widget.conversation.id}';

  String get _scamWarningKey =>
      'scam_warning_${widget.account.accountId()}_${widget.conversation.ownerId}';

  Future<void> _loadOverlayPreferences() async {
    final conversationId = widget.conversation.id;
    final scamDismissedUntil = (await _settings.get(_scamWarningKey) as num?)
        ?.toInt();
    final showPinnedMessage =
        await _settings.get(_showPinnedMessageKey) as bool? ?? true;
    if (!mounted || conversationId != widget.conversation.id) return;
    setState(() {
      _showPinnedMessage = showPinnedMessage;
      _showScamWarning =
          widget.conversation.isScam &&
          (scamDismissedUntil == null ||
              DateTime.now().millisecondsSinceEpoch > scamDismissedUntil);
    });
  }

  Future<void> _dismissPinnedMessage() async {
    setState(() => _showPinnedMessage = false);
    await _settings.set(_showPinnedMessageKey, false);
  }

  Future<void> _dismissScamWarning() async {
    setState(() => _showScamWarning = false);
    await _settings.set(
      _scamWarningKey,
      DateTime.now().add(const Duration(days: 1)).millisecondsSinceEpoch,
    );
  }

  Future<void> _jumpToLatest() async {
    if (_messageController.state.isLatest &&
        await _scrollCoordinator.scrollToBottomIfInLoadedWindow(
          animated: true,
        )) {
      return;
    }
    _scrollCoordinator.animateNextRestore(
      direction: ChatScrollRestoreDirection.towardNewer,
    );
    _messageController.loadLatestWindow();
  }

  Future<void> _refreshConversationEncryption() async {
    final account = widget.account;
    final conversationId = widget.conversation.id;
    try {
      final encrypted = await account.message().conversationIsEncrypted(
        conversationId: conversationId,
      );
      if (!mounted ||
          !identical(account, widget.account) ||
          conversationId != widget.conversation.id ||
          encrypted == _isEncryptConversation) {
        return;
      }
      setState(() => _isEncryptConversation = encrypted);
    } on Object catch (error, stackTrace) {
      e('Resolve conversation encryption failed', error, stackTrace);
    }
  }

  void _scheduleInputFocus() {
    if (widget.conversation.isGroup &&
        widget.conversation.participantCount == 0) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _inputFocusNode.requestFocus();
    });
  }

  void _onInputChanged() {
    setState(() {});
  }

  void _onVoiceRecorderChanged() {
    if (mounted) setState(() {});
  }

  Future<void> _startVoiceRecording() async {
    _inputFocusNode.unfocus();
    await _voiceRecorderController.start();
  }

  Future<void> _sendVoiceRecording() async {
    final messageController = _messageActions;
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
    if (!mounted || !identical(messageController, _messageActions) || !sent) {
      return;
    }
    setState(() => _quoteMessage = null);
  }

  void _reedit(String content) {
    _inputController.text = _inputController.text + content;
    _inputController.selection = TextSelection.collapsed(
      offset: _inputController.text.length,
    );
    _inputFocusNode.requestFocus();
  }

  Future<void> _pickAttachments() async {
    final files = await openFiles();
    if (!mounted || files.isEmpty) return;
    await _showAttachments(files);
  }

  Future<void> _sendContact() async {
    final contact = await showConversationSelector(
      context,
      account: widget.account,
      title: context.l10n.select,
      category: ConversationCategoryFilter.contacts,
    );
    if (contact == null || !mounted) return;
    final sent = await runWithToast(() async {
      await widget.account.message().sendContact(
        conversationId: widget.conversation.id,
        sharedUserId: contact.ownerId,
        quoteMessageId: _quoteMessage?.id,
        silent: false,
      );
    });
    if (sent && mounted) setState(() => _quoteMessage = null);
  }

  Future<void> _showAttachments(List<XFile> files) async {
    if (!mounted || files.isEmpty) return;
    final quoteMessageId = _quoteMessage?.id;
    final sent = await showAttachmentPreviewDialog(
      context: context,
      files: files,
      onSend:
          ({
            required path,
            required kind,
            required mimeType,
            required silent,
            name,
            width,
            height,
            durationMillis,
            thumbnail,
            caption,
          }) => _messageActions.sendAttachment(
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
          ),
    );
    if (sent && mounted) setState(() => _quoteMessage = null);
  }

  Future<bool> _sendSticker(String stickerId) async {
    final sent = await _messageActions.sendSticker(stickerId: stickerId);
    if (!mounted || !sent) return false;
    setState(() => _quoteMessage = null);
    unawaited(_stickerController.refreshLocal());
    return true;
  }

  Future<void> _sendText({bool silent = false}) async {
    final messageController = _messageActions;
    final original = _inputController.text;
    if (original.trim().isEmpty) return;
    if (original.length > _maxTextLength) {
      showToastFailed(ToastError(context.l10n.contentTooLong));
      return;
    }
    final quoteMessage = _quoteMessage;
    unawaited(
      messageController
          .sendText(original, quoteMessageId: quoteMessage?.id, silent: silent)
          .catchError((Object _) => false),
    );
    _inputController.clear();
    setState(() => _quoteMessage = null);
    widget.onDraftChanged('');
  }

  Future<void> _sendPost() async {
    final content = _inputController.text.trim();
    if (content.isEmpty) return;
    if (content.length > _maxTextLength) {
      showToastFailed(ToastError(context.l10n.contentTooLong));
      return;
    }
    unawaited(
      _messageActions.sendPost(content).catchError((Object _) => false),
    );
    _inputController.clear();
    widget.onDraftChanged('');
    setState(() => _quoteMessage = null);
    _inputFocusNode.requestFocus();
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
    final target = await showConversationSelector(
      context,
      account: widget.account,
      title: context.l10n.forward,
    );
    if (target == null || !mounted) return;
    final controller = _messageActions;
    await runWithLoading(() async {
      await controller.forwardMessages(messages, target.id);
    });
    if (!mounted || !identical(controller, _messageActions)) {
      return;
    }
    setState(_selectedMessageIds.clear);
  }

  Future<void> _combineForwardMessages(List<MessageListEntry> messages) async {
    final target = await showConversationSelector(
      context,
      account: widget.account,
      title: context.l10n.forward,
    );
    if (target == null || !mounted) return;
    final controller = _messageActions;
    await runWithLoading(() async {
      await controller.combineForwardMessages(messages, target.id);
    });
    if (!mounted || !identical(controller, _messageActions)) {
      return;
    }
    setState(_selectedMessageIds.clear);
  }

  Future<void> _openBotApp() async {
    final homeUri = await widget.account.user().botHomeUri(
      appId: widget.conversation.ownerId,
    );
    if (!mounted || homeUri == null || homeUri.trim().isEmpty) return;
    await openBotWebViewWindow(
      context: context,
      url: homeUri,
      title: widget.conversation.name,
      conversationId: widget.conversation.id,
      currency: widget.account.profile().fiatCurrency,
    );
  }

  Future<void> _locateMessage(
    String messageId, {
    String? sourceMessageId,
  }) async {
    final controller = _messageController;
    final conversationId = widget.conversation.id;
    if (await _scrollCoordinator.scrollToMessageIfInLoadedWindow(
      messageId,
      animated: true,
    )) {
      if (!mounted ||
          !identical(controller, _messageController) ||
          widget.conversation.id != conversationId) {
        return;
      }
      _highlightMessage(messageId);
      return;
    }
    final directionSourceMessageId =
        sourceMessageId ?? _currentWindowSourceMessageId(controller.state);
    final direction = await controller.restoreDirectionFromSource(
      sourceMessageId: directionSourceMessageId,
      targetMessageId: messageId,
    );
    if (!mounted ||
        !identical(controller, _messageController) ||
        widget.conversation.id != conversationId) {
      return;
    }
    _scrollCoordinator.animateNextMessageRestore(
      messageId,
      direction: switch (direction) {
        MessageWindowDirection.older => ChatScrollRestoreDirection.towardOlder,
        MessageWindowDirection.newer => ChatScrollRestoreDirection.towardNewer,
        null => null,
      },
      onComplete: () => _highlightMessage(messageId),
    );
    controller.loadAroundMessage(messageId);
  }

  String? _currentWindowSourceMessageId(MessageState state) {
    if (state.center != null) return state.center!.id;
    return state.bottomMessage?.id ?? state.topMessage?.id;
  }

  void _highlightMessage(String messageId) {
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

  Future<void> _copySelectedMessages() async {
    try {
      final selected = _messageActions.messages
          .where((message) => _selectedMessageIds.contains(message.id))
          .toList(growable: false);
      final formatter = DateFormat.yMd().add_Hms();
      final text = selected
          .map(
            (message) =>
                '${message.senderName}, '
                '(${formatter.format(message.createdAt.toLocal())}):\n'
                '${message.isText ? message.content : messagePreview(context, message)}',
          )
          .join('\n\n');
      await Clipboard.setData(ClipboardData(text: text));
      if (mounted) setState(_selectedMessageIds.clear);
    } catch (exception, stackTrace) {
      e('Copy selected messages failed', exception, stackTrace);
    }
  }

  @override
  void dispose() {
    _messageController.dispose();
    _messageActions.dispose();
    _scrollCoordinator.dispose();
    _inputController
      ..removeListener(_onInputChanged)
      ..dispose();
    _inputFocusNode.dispose();
    _voiceRecorderController
      ..removeListener(_onVoiceRecorderChanged)
      ..dispose();
    _stickerController.dispose();
    _highlightController
      ..removeListener(_onHighlightAnimation)
      ..removeStatusListener(_onHighlightStatus)
      ..dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Material(
    color: context.theme.primary,
    child: Column(
      children: [
        ChatHeader(
          conversation: widget.conversation,
          onBack: widget.onBack,
          onSearch: widget.onSearch,
          onInfo: widget.onInfo,
          showInfoAction: widget.showInfoAction,
          selectionMode: _selectedMessageIds.isNotEmpty,
          onCancelSelection: () => setState(_selectedMessageIds.clear),
          onBot: widget.conversation.isBot
              ? () => unawaited(_openBotApp())
              : null,
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
            child: ChatDropOverlay(
              enabled: _selectedMessageIds.isEmpty,
              onFilesDropped: _showAttachments,
              child: Column(
                children: [
                  Expanded(
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        border: Border(
                          bottom: BorderSide(color: context.theme.divider),
                        ),
                      ),
                      child: Stack(
                        children: [
                          Provider<rust.AccountHandle>.value(
                            value: widget.account,
                            child:
                                ChangeNotifierProvider<
                                  MessageActionController
                                >.value(
                                  value: _messageActions,
                                  child:
                                      ChangeNotifierProvider<
                                        StickerController
                                      >.value(
                                        value: _stickerController,
                                        child:
                                            Provider<
                                              MessageAttachmentController
                                            >(
                                              key: ObjectKey(widget.account),
                                              create: (_) =>
                                                  MessageAttachmentController.conversation(
                                                    account: widget.account,
                                                    onChanged: (message) =>
                                                        _messageController
                                                            .refreshMessages([
                                                              message.id,
                                                            ]),
                                                  ),
                                              child: MessageNavigationScope(
                                                currentUserId: _currentUserId,
                                                onLocateMessage: _locateMessage,
                                                onReedit: _reedit,
                                                onSelectConversation:
                                                    widget.onSelectConversation,
                                                onSelectConversationInfo: widget
                                                    .onSelectConversationInfo,
                                                child: RepaintBoundary(
                                                  child: ChatMessageList(
                                                    messageController:
                                                        _messageController,
                                                    scrollCoordinator:
                                                        _scrollCoordinator,
                                                    selectedMessageIds:
                                                        _selectedMessageIds,
                                                    highlightedMessageId:
                                                        _highlightedMessageId,
                                                    highlightOpacity:
                                                        _highlightOpacity,
                                                    onReply: _replyTo,
                                                    onSelect: _selectMessage,
                                                    onToggleSelection:
                                                        _toggleMessageSelection,
                                                  ),
                                                ),
                                              ),
                                            ),
                                      ),
                                ),
                          ),
                          Positioned(
                            left: 6,
                            right: 6,
                            bottom: 6,
                            child: ScamWarningBanner(
                              visible: _showScamWarning,
                              onDismiss: _dismissScamWarning,
                            ),
                          ),
                          Positioned(
                            right: 16,
                            bottom: 16,
                            child: ListenableBuilder(
                              listenable: _messageActions,
                              builder: (context, child) => Column(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  JumpMentionButton(
                                    messageIds:
                                        _messageActions.unreadMentionMessageIds,
                                    onJump: (messageId) async {
                                      await _locateMessage(messageId);
                                      await _messageActions.markMentionReadById(
                                        messageId,
                                      );
                                    },
                                    onClear: () async {
                                      for (final messageId
                                          in _messageActions
                                              .unreadMentionMessageIds) {
                                        await _messageActions
                                            .markMentionReadById(messageId);
                                      }
                                    },
                                  ),
                                  ValueListenableBuilder<bool>(
                                    valueListenable:
                                        _scrollCoordinator.showJumpToLatest,
                                    builder: (context, visible, child) =>
                                        JumpCurrentButton(
                                          visible: visible,
                                          onTap: _jumpToLatest,
                                        ),
                                  ),
                                ],
                              ),
                            ),
                          ),
                          ListenableBuilder(
                            listenable: _messageActions,
                            builder: (context, child) => PinMessagesBanner(
                              messageIds: _messageActions.pinnedMessageIds,
                              preview: _messageActions.pinMessagePreview,
                              visible: _showPinnedMessage,
                              onDismiss: _dismissPinnedMessage,
                              onOpenAll: widget.onOpenPinnedMessages,
                              onLocate: _locateMessage,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                  if (_selectedMessageIds.isNotEmpty)
                    ChatSelectionBottomBar(
                      selectedMessages: _messageController.state.list
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
                        await runWithLoading(() async {
                          await _messageActions.deleteMessages(messages);
                        });
                        if (mounted) setState(_selectedMessageIds.clear);
                      },
                      onRecall: (messages) async {
                        await runWithLoading(() async {
                          await _messageActions.recallMessages(messages);
                        });
                        if (mounted) setState(_selectedMessageIds.clear);
                      },
                    )
                  else if (widget.conversation.isGroup &&
                      widget.conversation.participantCount == 0)
                    const GroupCannotSendBar()
                  else
                    VoiceRecorderBarOverlayComposition(
                      controller: _voiceRecorderController,
                      onCancel: _voiceRecorderController.cancel,
                      onStop: _voiceRecorderController.stop,
                      onRetry: _voiceRecorderController.start,
                      onSend: _sendVoiceRecording,
                      child: ChatInputBar(
                        controller: _inputController,
                        focusNode: _inputFocusNode,
                        mentionAccount: widget.account,
                        mentionConversation: widget.conversation,
                        onMentionSelected: _inputController.markMention,
                        onMentionNamesChanged:
                            _inputController.cacheMentionNames,
                        isEncryptConversation: _isEncryptConversation,
                        quoteMessage: _quoteMessage,
                        onChanged: widget.onDraftChanged,
                        onCancelQuote: () =>
                            setState(() => _quoteMessage = null),
                        onSend: _sendText,
                        onSendSilent: () => _sendText(silent: true),
                        onSendPost: _sendPost,
                        onPasteFiles: _showAttachments,
                        onContactPressed: () => unawaited(_sendContact()),
                        onFilesPressed: () => unawaited(_pickAttachments()),
                        onPicturesPressed: () => unawaited(_pickAttachments()),
                        onVoicePressed: kIsWeb
                            ? null
                            : () => unawaited(_startVoiceRecording()),
                        stickerAction: StickerButton(
                          textEditingController: _inputController,
                          controller: _stickerController,
                          onStickerSelected: _sendSticker,
                          onGifSelected:
                              ({
                                required url,
                                required previewUrl,
                                required width,
                                required height,
                              }) => widget.account.message().sendRemoteImage(
                                conversationId: widget.conversation.id,
                                url: url,
                                previewUrl: previewUrl,
                                width: width,
                                height: height,
                                mimeType: 'image/gif',
                                silent: false,
                              ),
                          onStickerSent: () {},
                          onEmojiUsed: (_) {
                            widget.onDraftChanged(_inputController.text);
                            _inputFocusNode.requestFocus();
                          },
                          child: const ChatInputAction(
                            actionKey: Key('chat-sticker'),
                            asset: MixinAssets.sticker,
                          ),
                        ),
                      ),
                    ),
                ],
              ),
            ),
          ),
        ),
      ],
    ),
  );
}

class _UnavailableVoiceRecorderBackend implements VoiceRecorderBackend {
  const _UnavailableVoiceRecorderBackend();

  @override
  Future<void> start(String path) =>
      Future.error(StateError('Voice recording is unavailable'));

  @override
  Future<VoiceRecording> stop() =>
      Future.error(StateError('Voice recording is unavailable'));

  @override
  Future<void> cancel() async {}
}
