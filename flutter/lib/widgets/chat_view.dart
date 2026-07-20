import 'dart:async';
import 'dart:io';
import 'dart:math' as math;
import 'dart:ui' as ui show BoxHeightStyle;

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:flutter_portal/flutter_portal.dart';
import 'package:file_selector/file_selector.dart';
import 'package:intl/intl.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/constants/icon_fonts.dart';
import 'package:mixin_desktop_ui/controllers/chat_side_notifier.dart';
import 'package:mixin_desktop_ui/controllers/message_controller.dart';
import 'package:mixin_desktop_ui/controllers/message_action_controller.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/controllers/sticker_controller.dart';
import 'package:mixin_desktop_ui/controllers/voice_recorder_controller.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/utils/app_logger.dart';
import 'package:mixin_desktop_ui/utils/system_clipboard.dart';
import 'package:mixin_desktop_ui/utils/name_color.dart';
import 'package:mixin_desktop_ui/utils/web_view.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/action_button.dart';
import 'package:mixin_desktop_ui/widgets/adaptive_selection_toolbar.dart';
import 'package:mixin_desktop_ui/widgets/animated_visibility.dart';
import 'package:mixin_desktop_ui/widgets/badges_widget.dart';
import 'package:mixin_desktop_ui/widgets/chat_drop_overlay.dart';
import 'package:mixin_desktop_ui/widgets/chat/chat_history_viewport.dart';
import 'package:mixin_desktop_ui/widgets/chat/chat_scroll_coordinator.dart';
import 'package:mixin_desktop_ui/widgets/high_light_text.dart';
import 'package:mixin_desktop_ui/widgets/interactive_decorated_box.dart';
import 'package:mixin_desktop_ui/widgets/message_bubble.dart';
import 'package:mixin_desktop_ui/widgets/message_action_policy.dart';
import 'package:mixin_desktop_ui/widgets/message_actions_menu.dart';
import 'package:mixin_desktop_ui/widgets/custom_context_menu.dart';
import 'package:mixin_desktop_ui/widgets/custom_popup_menu.dart';
import 'package:mixin_desktop_ui/widgets/message_audio.dart';
import 'package:mixin_desktop_ui/widgets/message_content.dart';
import 'package:mixin_desktop_ui/widgets/message_items/special_message_items.dart';
import 'package:mixin_desktop_ui/widgets/message_day_time.dart';
import 'package:mixin_desktop_ui/widgets/message_datetime_and_status.dart';
import 'package:mixin_desktop_ui/widgets/message_media_preview_pages.dart';
import 'package:mixin_desktop_ui/widgets/message_name.dart';
import 'package:mixin_desktop_ui/widgets/message_presentation.dart';
import 'package:mixin_desktop_ui/widgets/message_qr_dialog.dart';
import 'package:mixin_desktop_ui/widgets/message_rows.dart';
import 'package:mixin_desktop_ui/widgets/message_style.dart';
import 'package:mixin_desktop_ui/widgets/mixin_dialog.dart';
import 'package:mixin_desktop_ui/widgets/pin_message_bubble.dart';
import 'package:mixin_desktop_ui/widgets/show_message_user_dialog.dart';
import 'package:mixin_desktop_ui/widgets/show_snapshot_detail_dialog.dart';
import 'package:mixin_desktop_ui/widgets/show_attachment_preview_dialog.dart';
import 'package:mixin_desktop_ui/widgets/show_forward_conversation_selector.dart';
import 'package:mixin_desktop_ui/widgets/show_add_image_sticker_dialog.dart';
import 'package:mixin_desktop_ui/widgets/transcript_page.dart';
import 'package:mixin_desktop_ui/widgets/toast.dart';
import 'package:mixin_desktop_ui/widgets/waveform_widget.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_button.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_detail_page.dart';
import 'package:provider/provider.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:super_context_menu/super_context_menu.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:visibility_detector/visibility_detector.dart';

const _maxTextLength = 64 * 1024;

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
    this.onOpenPinnedMessages,
    this.onSelectConversation,
    this.onSelectConversationInfo,
    this.onOpenUri,
    this.showInfoAction = true,
    this.locateMessageId,
    this.locateRequest = 0,
  });

  final rust.AccountHandle account;
  final ConversationListEntry conversation;
  final String draft;
  final ValueChanged<String> onDraftChanged;
  final ValueChanged<ConversationListEntry>? onSelectConversation;
  final ValueChanged<ConversationListEntry>? onSelectConversationInfo;
  final ValueChanged<Uri>? onOpenUri;
  final VoidCallback? onBack;
  final VoidCallback? onSearch;
  final VoidCallback? onInfo;
  final VoidCallback? onOpenPinnedMessages;
  final bool showInfoAction;
  final String? locateMessageId;
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
  late final TextEditingController _inputController;
  late final FocusNode _inputFocusNode;
  late final VoiceRecorderController _voiceRecorderController;
  late StickerController _stickerController;
  MessageListEntry? _quoteMessage;
  String? _highlightedMessageId;
  double _highlightOpacity = 0;
  late final AnimationController _highlightController;
  final Set<String> _selectedMessageIds = {};
  Timer? _mentionTimer;
  int _mentionRevision = 0;
  List<rust.ConversationParticipantItem> _mentionUsers = const [];
  String _mentionKeyword = '';
  int _mentionIndex = 0;
  bool _isEncryptConversation = true;
  bool _showPinnedMessage = false;
  bool _showScamWarning = false;

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
    _scrollCoordinator = ChatScrollCoordinator();
    _createMessageController();
    unawaited(_loadOverlayPreferences());
    _inputController = TextEditingController(text: widget.draft);
    _inputController.addListener(_onInputChanged);
    _inputFocusNode = FocusNode(debugLabel: 'chat_input');
    _voiceRecorderController = VoiceRecorderController()
      ..addListener(_onVoiceRecorderChanged);
    _stickerController = StickerController(account: widget.account);
    _scheduleInputFocus();
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
      _messageController.dispose();
      _messageActions.dispose();
      _createMessageController();
      unawaited(_loadOverlayPreferences());
      _quoteMessage = null;
      _highlightedMessageId = null;
      _highlightOpacity = 0;
      _highlightController.reset();
      _selectedMessageIds.clear();
      _clearMentions();
      unawaited(_voiceRecorderController.cancel());
      if (!identical(oldWidget.account, widget.account)) {
        _stickerController.dispose();
        _stickerController = StickerController(account: widget.account);
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
        (_) => unawaited(_locateMessage(widget.locateMessageId!)),
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
    final preferences = await SharedPreferences.getInstance();
    final scamDismissedUntil = preferences.getInt(_scamWarningKey);
    if (!mounted || conversationId != widget.conversation.id) return;
    setState(() {
      _showPinnedMessage = preferences.getBool(_showPinnedMessageKey) ?? true;
      _showScamWarning =
          widget.conversation.isScam &&
          (scamDismissedUntil == null ||
              DateTime.now().millisecondsSinceEpoch > scamDismissedUntil);
    });
  }

  Future<void> _dismissPinnedMessage() async {
    setState(() => _showPinnedMessage = false);
    final preferences = await SharedPreferences.getInstance();
    await preferences.setBool(_showPinnedMessageKey, false);
  }

  Future<void> _dismissScamWarning() async {
    setState(() => _showScamWarning = false);
    final preferences = await SharedPreferences.getInstance();
    await preferences.setInt(
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
    } on Object catch (error) {
      e('resolve conversation encryption failed', error);
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
    _scheduleMentionSearch();
    setState(() {});
  }

  void _scheduleMentionSearch() {
    _mentionTimer?.cancel();
    if (!widget.conversation.isGroup && !widget.conversation.isBot) {
      _clearMentions();
      return;
    }
    final selection = _inputController.selection.baseOffset;
    if (selection < 0 || selection > _inputController.text.length) {
      _clearMentions();
      return;
    }
    final match = RegExp(
      r'(?:^|\s)@([^@\s]*)$',
    ).firstMatch(_inputController.text.substring(0, selection));
    if (match == null) {
      _clearMentions();
      return;
    }
    final keyword = match.group(1) ?? '';
    final revision = ++_mentionRevision;
    _mentionKeyword = keyword;
    _mentionTimer = Timer(const Duration(milliseconds: 150), () async {
      try {
        final result = await widget.account.conversation().searchBotGroupUsers(
          conversationId: widget.conversation.id,
          keyword: keyword,
        );
        if (!mounted || revision != _mentionRevision) return;
        setState(() {
          _mentionUsers = result;
          _mentionIndex = 0;
        });
      } on Object {
        if (mounted && revision == _mentionRevision) _clearMentions();
      }
    });
  }

  void _clearMentions() {
    _mentionTimer?.cancel();
    _mentionRevision++;
    _mentionUsers = const [];
    _mentionKeyword = '';
    _mentionIndex = 0;
  }

  void _moveMention(int delta) {
    if (_mentionUsers.isEmpty) return;
    setState(() {
      _mentionIndex = (_mentionIndex + delta).clamp(
        0,
        _mentionUsers.length - 1,
      );
    });
  }

  void _selectMention([int? index]) {
    if (_mentionUsers.isEmpty) return;
    final user = _mentionUsers[index ?? _mentionIndex];
    final selection = _inputController.selection.baseOffset.clamp(
      0,
      _inputController.text.length,
    );
    final before = _inputController.text.substring(0, selection);
    final replaced = before.replaceFirst(
      RegExp(r'@[^@\s]*$'),
      '@${user.identityNumber} ',
    );
    final value = replaced + _inputController.text.substring(selection);
    _inputController.value = TextEditingValue(
      text: value,
      selection: TextSelection.collapsed(offset: replaced.length),
    );
    widget.onDraftChanged(value);
    _clearMentions();
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
            name,
            width,
            height,
            durationMillis,
            thumbnail,
            caption,
            required silent,
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

  Future<void> _locateMessage(String messageId) async {
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
    final direction = await controller.restoreDirectionFromSource(
      sourceMessageId: controller.state.center?.id,
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
                '${message.isText ? message.content : _messagePreview(context, message)}',
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
    _mentionTimer?.cancel();
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
        _ChatHeader(
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
                          RepaintBoundary(
                            child: _MessageList(
                              account: widget.account,
                              messageController: _messageController,
                              actions: _messageActions,
                              scrollCoordinator: _scrollCoordinator,
                              currentUserId: _currentUserId,
                              conversation: widget.conversation,
                              selectedMessageIds: _selectedMessageIds,
                              highlightedMessageId: _highlightedMessageId,
                              highlightOpacity: _highlightOpacity,
                              onReply: _replyTo,
                              onForward: (messages) =>
                                  unawaited(_forwardMessages(messages)),
                              onSelect: _selectMessage,
                              onToggleSelection: _toggleMessageSelection,
                              onOpenMessage: _locateMessage,
                              onReedit: _reedit,
                              onSelectConversation: widget.onSelectConversation,
                              onSelectConversationInfo:
                                  widget.onSelectConversationInfo,
                              onOpenUri: widget.onOpenUri,
                              onStickerAlbumChanged:
                                  _stickerController.refreshLocal,
                            ),
                          ),
                          Positioned(
                            left: 6,
                            right: 6,
                            bottom: 6,
                            child: _ScamWarningBanner(
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
                                  _JumpMentionButton(
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
                                        _JumpCurrentButton(
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
                            builder: (context, child) => _PinMessagesBanner(
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
                    _SelectionBottomBar(
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
                    const _GroupCannotSendBar()
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
                        isEncryptConversation: _isEncryptConversation,
                        quoteMessage: _quoteMessage,
                        onChanged: widget.onDraftChanged,
                        onCancelQuote: () =>
                            setState(() => _quoteMessage = null),
                        onSend: _sendText,
                        onSendSilent: () => _sendText(silent: true),
                        onSendPost: _sendPost,
                        mentionUsers: _mentionUsers,
                        mentionKeyword: _mentionKeyword,
                        mentionIndex: _mentionIndex,
                        onMentionMove: _moveMention,
                        onMentionSelected: _selectMention,
                        onPasteFiles: _showAttachments,
                        onContactPressed: () => unawaited(_sendContact()),
                        onFilesPressed: () => unawaited(_pickAttachments()),
                        onPicturesPressed: () => unawaited(_pickAttachments()),
                        onVoicePressed: () => unawaited(_startVoiceRecording()),
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
                          child: const _ChatInputAction(
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

void _showDiscardRecordingWarningAlertOverlay(
  BuildContext context, {
  required VoidCallback onDiscard,
}) {
  final overlay = Overlay.of(context, rootOverlay: true);
  OverlayEntry? entry;

  void dismiss() {
    entry?.remove();
    entry = null;
  }

  entry = OverlayEntry(
    builder: (context) => Stack(
      children: [
        GestureDetector(
          behavior: HitTestBehavior.translucent,
          onTap: dismiss,
          child: const SizedBox.expand(
            child: ColoredBox(color: Color(0x80000000)),
          ),
        ),
        Center(
          child: SizedBox(
            width: 400,
            child: Material(
              borderRadius: const BorderRadius.all(Radius.circular(11)),
              color: context.theme.popUp,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 30),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const SizedBox(height: 40),
                    Text(
                      context.l10n.discardRecordingWarning,
                      style: TextStyle(
                        fontSize: 16,
                        height: 2,
                        color: context.theme.text,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(height: 36),
                    Row(
                      children: [
                        const Spacer(),
                        MixinButton(
                          backgroundTransparent: true,
                          onTap: dismiss,
                          child: Text(context.l10n.cancel),
                        ),
                        MixinButton(
                          onTap: () {
                            dismiss();
                            onDiscard();
                          },
                          child: Text(context.l10n.discard),
                        ),
                      ],
                    ),
                    const SizedBox(height: 30),
                  ],
                ),
              ),
            ),
          ),
        ),
      ],
    ),
  );
  overlay.insert(entry!);
}

class _GroupCannotSendBar extends StatelessWidget {
  const _GroupCannotSendBar();

  @override
  Widget build(BuildContext context) => Container(
    height: 56,
    alignment: Alignment.center,
    color: context.theme.primary,
    child: Text(
      context.l10n.groupCantSend,
      style: TextStyle(color: context.theme.secondaryText),
    ),
  );
}

class _ChatHeader extends StatelessWidget {
  const _ChatHeader({
    required this.conversation,
    required this.onBack,
    required this.onSearch,
    required this.onInfo,
    required this.showInfoAction,
    required this.selectionMode,
    required this.onCancelSelection,
    required this.onBot,
  });

  final ConversationListEntry conversation;
  final VoidCallback? onBack;
  final VoidCallback? onSearch;
  final VoidCallback? onInfo;
  final bool showInfoAction;
  final bool selectionMode;
  final VoidCallback onCancelSelection;
  final VoidCallback? onBot;

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
            _HeaderAction(
              key: const Key('chat-back'),
              asset: MixinAssets.back,
              onPressed: onBack,
            ),
          GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: onInfo,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                ConversationAvatarView(conversation: conversation, size: 36),
                const SizedBox(width: 10),
              ],
            ),
          ),
          Expanded(
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: onInfo,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Flexible(
                        child: CustomSelectableArea(
                          child: CustomText(
                            conversation.name,
                            maxLines: 1,
                            style: TextStyle(
                              color: context.theme.text,
                              fontSize: 16,
                              height: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                            textAlign: TextAlign.center,
                          ),
                        ),
                      ),
                      BadgesWidget(
                        verified: conversation.isVerified,
                        isBot: conversation.isBot,
                        membership: conversation.membership,
                      ),
                    ],
                  ),
                  if (subtitle.isNotEmpty) ...[
                    const SizedBox(height: 4),
                    CustomSelectableText(
                      subtitle,
                      maxLines: 1,
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
          ),
          AnimatedSize(
            duration: const Duration(milliseconds: 200),
            alignment: Alignment.centerLeft,
            child: onBot == null
                ? const SizedBox()
                : _HeaderAction(asset: MixinAssets.bots, onPressed: onBot),
          ),
          if (selectionMode)
            TextButton(
              onPressed: onCancelSelection,
              child: Text(context.l10n.cancel),
            )
          else ...[
            _HeaderAction(
              key: const Key('chat-search'),
              asset: MixinAssets.chatSearch,
              onPressed: onSearch,
            ),
            AnimatedSize(
              duration: const Duration(milliseconds: 200),
              alignment: Alignment.centerLeft,
              child: showInfoAction
                  ? _HeaderAction(
                      key: const Key('chat-info'),
                      asset: MixinAssets.chatInfo,
                      onPressed: onInfo,
                    )
                  : const SizedBox(),
            ),
          ],
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

class _ScamWarningBanner extends StatelessWidget {
  const _ScamWarningBanner({required this.visible, required this.onDismiss});

  final bool visible;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) => AnimatedVisibility(
    visible: visible,
    child: Container(
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        borderRadius: const BorderRadius.all(Radius.circular(4)),
        color: context.messageBubbleColor(false),
        boxShadow: const [
          BoxShadow(
            color: Color.fromRGBO(0, 0, 0, 0.15),
            offset: Offset(0, 2),
            blurRadius: 10,
          ),
        ],
      ),
      child: Row(
        children: [
          Padding(
            padding: const EdgeInsets.only(
              left: 8,
              right: 16,
              top: 8,
              bottom: 8,
            ),
            child: SvgPicture.asset(
              MixinAssets.warning,
              width: 26,
              height: 26,
              colorFilter: ColorFilter.mode(context.theme.red, BlendMode.srcIn),
            ),
          ),
          Expanded(
            child: Text(
              context.l10n.scamWarning,
              style: TextStyle(color: context.theme.text, fontSize: 14),
            ),
          ),
          ActionButton(
            name: MixinAssets.close,
            color: context.theme.icon,
            size: 20,
            onTap: onDismiss,
          ),
        ],
      ),
    ),
  );
}

class _JumpCurrentButton extends StatelessWidget {
  const _JumpCurrentButton({required this.visible, required this.onTap});

  final bool visible;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    if (!visible) return const SizedBox();
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: InteractiveDecoratedBox(
        onTap: onTap,
        child: Container(
          height: 40,
          width: 40,
          decoration: BoxDecoration(
            color: context.messageBubbleColor(false),
            boxShadow: const [
              BoxShadow(
                color: Color.fromRGBO(0, 0, 0, 0.15),
                offset: Offset(0, 2),
                blurRadius: 10,
              ),
            ],
            shape: BoxShape.circle,
          ),
          alignment: Alignment.center,
          child: SvgPicture.asset(
            MixinAssets.jumpCurrentArrow,
            colorFilter: ColorFilter.mode(context.theme.text, BlendMode.srcIn),
          ),
        ),
      ),
    );
  }
}

class _JumpMentionButton extends StatelessWidget {
  const _JumpMentionButton({
    required this.messageIds,
    required this.onJump,
    required this.onClear,
  });

  final List<String> messageIds;
  final ValueChanged<String> onJump;
  final VoidCallback onClear;

  @override
  Widget build(BuildContext context) {
    if (messageIds.isEmpty) return const SizedBox();
    return ContextMenuWidget(
      desktopMenuWidgetBuilder: CustomDesktopMenuWidgetBuilder(),
      menuProvider: (_) => Menu(
        children: [MenuAction(title: context.l10n.clear, callback: onClear)],
      ),
      child: InteractiveDecoratedBox(
        onTap: () => onJump(messageIds.first),
        child: SizedBox(
          height: 52,
          width: 40,
          child: Stack(
            children: [
              Positioned(
                top: 12,
                child: Container(
                  height: 40,
                  width: 40,
                  decoration: BoxDecoration(
                    color: context.messageBubbleColor(false),
                    boxShadow: const [
                      BoxShadow(
                        color: Color.fromRGBO(0, 0, 0, 0.15),
                        offset: Offset(0, 2),
                        blurRadius: 10,
                      ),
                    ],
                    shape: BoxShape.circle,
                  ),
                  alignment: Alignment.center,
                  child: Text(
                    '@',
                    style: TextStyle(
                      fontSize: 17,
                      height: 1,
                      color: context.theme.text,
                    ),
                  ),
                ),
              ),
              Container(
                width: 40,
                alignment: Alignment.topCenter,
                child: Container(
                  constraints: const BoxConstraints(
                    minWidth: 20,
                    maxWidth: 40,
                    minHeight: 20,
                    maxHeight: 20,
                  ),
                  padding: const EdgeInsets.symmetric(horizontal: 4),
                  decoration: BoxDecoration(
                    color: context.theme.accent,
                    borderRadius: const BorderRadius.all(Radius.circular(10)),
                  ),
                  child: Column(
                    children: [
                      const Spacer(),
                      Text(
                        messageIds.length > 99 ? '99+' : '${messageIds.length}',
                        maxLines: 1,
                        textAlign: TextAlign.center,
                        style: const TextStyle(
                          fontSize: 14,
                          height: 1,
                          color: Colors.white,
                        ),
                      ),
                      const Spacer(),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _PinMessagesBanner extends StatelessWidget {
  const _PinMessagesBanner({
    required this.messageIds,
    required this.preview,
    required this.visible,
    required this.onDismiss,
    required this.onLocate,
    this.onOpenAll,
  });

  final List<String> messageIds;
  final rust.PinMessagePreviewItem? preview;
  final bool visible;
  final VoidCallback onDismiss;
  final ValueChanged<String> onLocate;
  final VoidCallback? onOpenAll;

  @override
  Widget build(BuildContext context) {
    final pinnedMessageId = messageIds.firstOrNull;
    final showPreview = visible && pinnedMessageId != null;
    final preview = pinnedMessageId == null
        ? ''
        : context.l10n.chatPinMessage(
            this.preview?.senderName ?? '',
            this.preview == null
                ? context.l10n.aMessage
                : pinMessagePreview(context.l10n, this.preview!.content),
          );
    return Positioned(
      top: 12,
      right: 16,
      left: 10,
      height: 64,
      child: AnimatedVisibility(
        visible: showPreview || messageIds.isNotEmpty,
        child: Row(
          children: [
            Expanded(
              child: AnimatedVisibility(
                visible: showPreview,
                child: PinMessageBubble(
                  child: Row(
                    children: [
                      ActionButton(
                        name: MixinAssets.close,
                        color: context.theme.icon,
                        size: 20,
                        onTap: onDismiss,
                      ),
                      const SizedBox(width: 4),
                      Expanded(
                        child: InteractiveDecoratedBox(
                          cursor: SystemMouseCursors.click,
                          onTap: pinnedMessageId == null
                              ? null
                              : () => onLocate(pinnedMessageId),
                          child: CustomText(
                            preview,
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
            const SizedBox(width: 8),
            AnimatedVisibility(
              visible: messageIds.isNotEmpty,
              child: InteractiveDecoratedBox(
                onTap: onOpenAll,
                child: Container(
                  height: 34,
                  width: 34,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: context.messageBubbleColor(false),
                    boxShadow: const [
                      BoxShadow(
                        color: Color.fromRGBO(0, 0, 0, 0.15),
                        offset: Offset(0, 2),
                        blurRadius: 10,
                      ),
                    ],
                  ),
                  alignment: Alignment.center,
                  child: SvgPicture.asset(
                    MixinAssets.chatPin,
                    width: 34,
                    height: 34,
                    colorFilter: ColorFilter.mode(
                      context.theme.text,
                      BlendMode.srcIn,
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _MessageList extends StatelessWidget {
  const _MessageList({
    required this.account,
    required this.messageController,
    required this.actions,
    required this.scrollCoordinator,
    required this.currentUserId,
    required this.conversation,
    required this.selectedMessageIds,
    required this.highlightedMessageId,
    required this.highlightOpacity,
    required this.onReply,
    required this.onForward,
    required this.onSelect,
    required this.onToggleSelection,
    required this.onOpenMessage,
    required this.onReedit,
    required this.onSelectConversation,
    required this.onSelectConversationInfo,
    required this.onOpenUri,
    required this.onStickerAlbumChanged,
  });

  final rust.AccountHandle account;
  final MessageController messageController;
  final MessageActionController actions;
  final ChatScrollCoordinator scrollCoordinator;
  final String currentUserId;
  final ConversationListEntry conversation;
  final Set<String> selectedMessageIds;
  final String? highlightedMessageId;
  final double highlightOpacity;
  final ValueChanged<MessageListEntry> onReply;
  final ValueChanged<List<MessageListEntry>> onForward;
  final ValueChanged<MessageListEntry> onSelect;
  final ValueChanged<MessageListEntry> onToggleSelection;
  final ValueChanged<String> onOpenMessage;
  final ValueChanged<String> onReedit;
  final ValueChanged<ConversationListEntry>? onSelectConversation;
  final ValueChanged<ConversationListEntry>? onSelectConversationInfo;
  final ValueChanged<Uri>? onOpenUri;
  final Future<void> Function() onStickerAlbumChanged;

  @override
  Widget build(BuildContext context) => ChatHistoryViewport(
    messageController: messageController,
    presentationListenable: actions,
    scrollCoordinator: scrollCoordinator,
    unreadBar: const _UnreadMessageBar(),
    messageBuilder: (row, dayTimeKey) {
      final message = row.message;
      return _ChatMessage(
        dayTimeKey: dayTimeKey,
        account: account,
        message: message,
        previous: row.previous,
        next: row.next,
        currentUserId: currentUserId,
        isGroup: conversation.isGroup,
        isBot: conversation.isBot,
        isBotGroup: actions.isBotGroup,
        conversationOwnerId: conversation.ownerId,
        currentUserRole: actions.currentUserRole,
        messages: messageController.state.list,
        mentionNames: actions.mentionNames,
        selected: selectedMessageIds.contains(message.id),
        highlightOpacity: highlightedMessageId == message.id
            ? highlightOpacity
            : 0,
        inSelectionMode: selectedMessageIds.isNotEmpty,
        onReply: () => onReply(message),
        onForward: () => onForward([message]),
        onSelect: () => onSelect(message),
        onToggleSelection: () => onToggleSelection(message),
        onTogglePin: () => unawaited(
          _ignoreMutation(actions.setMessagePinned(message, !message.pinned)),
        ),
        onRecall: () =>
            unawaited(_ignoreMutation(actions.recallMessages([message]))),
        onDelete: () =>
            unawaited(_ignoreMutation(actions.deleteMessages([message]))),
        onOpenMessage: onOpenMessage,
        recalledText: actions.recalledText(message.id),
        onReedit: onReedit,
        onSelectConversation: onSelectConversation,
        onSelectConversationInfo: onSelectConversationInfo,
        onOpenUri: onOpenUri,
        onMarkMentionRead: () => unawaited(actions.markMentionRead(message)),
        onMarkAudioRead: (audio) => unawaited(actions.markAudioRead(audio)),
        onDownloadAttachment: () =>
            unawaited(_ignoreMutation(actions.downloadAttachment(message))),
        onCancelAttachment: () =>
            unawaited(_ignoreMutation(actions.cancelAttachment(message))),
        onAddSticker: () => actions.addSticker(message),
        onAddImageAsSticker: () => actions.addImageAsSticker(message),
        onStrangerAction: (action) =>
            actions.handleStrangerAction(message, action),
        onStickerAlbumChanged: onStickerAlbumChanged,
      );
    },
  );
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
    required this.dayTimeKey,
    required this.account,
    required this.message,
    required this.previous,
    required this.next,
    required this.currentUserId,
    required this.isGroup,
    required this.isBot,
    required this.isBotGroup,
    required this.conversationOwnerId,
    required this.currentUserRole,
    required this.messages,
    required this.mentionNames,
    required this.selected,
    required this.highlightOpacity,
    required this.inSelectionMode,
    required this.onReply,
    required this.onForward,
    required this.onSelect,
    required this.onToggleSelection,
    required this.onTogglePin,
    required this.onRecall,
    required this.onDelete,
    required this.onOpenMessage,
    required this.recalledText,
    required this.onReedit,
    required this.onSelectConversation,
    required this.onSelectConversationInfo,
    required this.onOpenUri,
    required this.onMarkMentionRead,
    required this.onMarkAudioRead,
    required this.onDownloadAttachment,
    required this.onCancelAttachment,
    required this.onAddSticker,
    required this.onAddImageAsSticker,
    required this.onStrangerAction,
    required this.onStickerAlbumChanged,
  });

  final GlobalKey? dayTimeKey;
  final rust.AccountHandle account;
  final MessageListEntry message;
  final MessageListEntry? previous;
  final MessageListEntry? next;
  final String currentUserId;
  final bool isGroup;
  final bool isBot;
  final bool isBotGroup;
  final String conversationOwnerId;
  final String? currentUserRole;
  final List<MessageListEntry> messages;
  final Map<String, String> mentionNames;
  final bool selected;
  final double highlightOpacity;
  final bool inSelectionMode;
  final VoidCallback onReply;
  final VoidCallback onForward;
  final VoidCallback onSelect;
  final VoidCallback onToggleSelection;
  final VoidCallback onTogglePin;
  final VoidCallback onRecall;
  final VoidCallback onDelete;
  final ValueChanged<String> onOpenMessage;
  final String? recalledText;
  final ValueChanged<String> onReedit;
  final ValueChanged<ConversationListEntry>? onSelectConversation;
  final ValueChanged<ConversationListEntry>? onSelectConversationInfo;
  final ValueChanged<Uri>? onOpenUri;
  final VoidCallback onMarkMentionRead;
  final ValueChanged<MessageListEntry> onMarkAudioRead;
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
        isBotGroup: widget.isBotGroup,
        conversationOwnerId: widget.conversationOwnerId,
      ),
      enableShowAvatar: settings.messageShowAvatar,
    );
    final showIdentityNumber =
        settings.messageShowIdentityNumber &&
        widget.message.senderIdentityNumber.isNotEmpty &&
        widget.message.senderIdentityNumber != '0';
    final search = context.watch<SearchConversationKeywordNotifier?>()?.value;
    final (selectedUserId, searchKeyword) = search ?? (null, '');
    final messageKeyword =
        selectedUserId == null || selectedUserId == widget.message.senderId
        ? searchKeyword
        : '';
    final messageContent = MessageContent(
      message: widget.message,
      isCurrentUser: presentation.isCurrentUser,
      currentUserId: widget.currentUserId,
      onOpenUri: widget.onOpenUri ?? (uri) => unawaited(launchUrl(uri)),
      onOpenMessage: widget.onOpenMessage,
      recalledText: widget.recalledText,
      onReedit: widget.onReedit,
      onOpenSnapshot: (_) =>
          showSnapshotDetailDialog(context, message: widget.message),
      onAction: widget.message.category == 'STRANGER'
          ? _handleStrangerAction
          : null,
      onAppAction: _openAction,
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
      keyword: messageKeyword,
      audioPlaylist: widget.messages,
      showNip: presentation.showNip,
      highlighted: _menuHighlighted,
      highlightOpacity: widget.highlightOpacity,
      onMarkAudioRead: widget.onMarkAudioRead,
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
        !widget.message.isInvalidSpecialMessage &&
        (bypassActions ||
            widget.message.category == 'APP_BUTTON_GROUP' ||
            widget.message.category == 'APP_CARD');
    final quote = buildMessageQuotePreview(
      widget.message,
      onOpenMessage: widget.onOpenMessage,
      mentionNames: widget.mentionNames,
    );
    final renderedMessage = bypassActions
        ? messageContent
        : bypassBubble
        ? Align(
            key: Key('message-bubble-${widget.message.id}'),
            alignment: presentation.isCurrentUser
                ? Alignment.centerRight
                : Alignment.centerLeft,
            child: messageContent,
          )
        : MessageBubble(
            key: Key('message-bubble-${widget.message.id}'),
            isCurrentUser: presentation.isCurrentUser,
            showNip: presentation.showNip,
            showBubble: widget.message.showMessageBubble,
            includeNip: widget.message.includeMessageBubbleNip,
            clip: widget.message.clipMessageBubble,
            highlighted: _menuHighlighted,
            highlightOpacity: widget.highlightOpacity,
            padding: widget.message.messageBubblePadding,
            forceIsCurrentUserColor:
                widget.message.forceCurrentMessageBubbleColor,
            isDisappearingMessage: (widget.message.expireIn ?? 0) > 0,
            quote: quote,
            constrainQuoteWidth:
                widget.message.isImage ||
                widget.message.isVideo ||
                widget.message.isLive,
            highlightMedia:
                !widget.message.showMessageBubble &&
                quote == null &&
                (widget.message.isImage ||
                    widget.message.isVideo ||
                    widget.message.isLive ||
                    widget.message.isSticker),
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
      desktopMenuWidgetBuilder: CustomDesktopMenuWidgetBuilder(),
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
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Flexible(
                child: MessageName(
                  userName: widget.message.senderName,
                  userId: widget.message.senderId,
                  userIdentityNumber: widget.message.senderIdentityNumber,
                  verified: widget.message.senderIsVerified,
                  isBot: widget.message.senderIsBot,
                  membership: widget.message.senderMembership,
                  showIdentityNumber: showIdentityNumber,
                  onTap: () => _showUser(userId: widget.message.senderId),
                ),
              ),
            ],
          ),
        interactiveMessage,
      ],
    );

    Widget child;
    if (bypassActions) {
      child = interactiveMessage;
    } else if (presentation.showSender && presentation.showAvatar) {
      child = Row(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const SizedBox(width: 8),
          InteractiveDecoratedBox(
            cursor: SystemMouseCursors.click,
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
                _AnimatedSelectionIcon(
                  selected: widget.selected,
                  inSelectionMode: widget.inSelectionMode,
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
          if (row.dateTime != null)
            MessageDayTime(key: widget.dayTimeKey, dateTime: row.dateTime!),
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
    unawaited(_showUserAndHandle(userId, identityNumber));
  }

  Future<void> _showUserAndHandle(
    String? userId,
    String? identityNumber,
  ) async {
    final result = await showMessageUserDialog(
      context,
      account: widget.account,
      userId: userId,
      identityNumber: identityNumber,
    );
    final onSelectConversation = widget.onSelectConversation;
    if (!mounted || result == null || onSelectConversation == null) return;
    await handleMessageUserDialogResult(
      context,
      account: widget.account,
      result: result,
      onSelectConversation: onSelectConversation,
      onSelectConversationInfo: widget.onSelectConversationInfo,
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
    final target = await showConversationSelector(
      context,
      account: widget.account,
      title: context.l10n.forward,
    );
    if (target == null || !mounted) return false;
    await widget.account.message().forwardMessages(
      targetConversationId: target.id,
      sourceMessageIds: [message.id],
    );
    return true;
  }

  void _openImage(MessageListEntry message) {
    final source = _messageMediaSource(message);
    if (source.isEmpty) return;
    unawaited(
      ImagePreviewPage.show(
        context,
        ImagePreviewPage(
          images: [
            ImagePreviewEntry(
              id: message.id,
              source: source,
              name: message.mediaName,
              thumbImage: message.thumbImage,
              canForward: _canForwardPreview(message),
              userId: message.senderId,
              userFullName: message.senderName,
              userIdentityNumber: message.senderIdentityNumber,
              avatarUrl: message.senderAvatarUrl,
            ),
          ],
          onCopy: (image) {
            final file = existingLocalFile(image.source);
            if (file != null) return _copyImage(file);
          },
          onSave: (image) => _saveSource(image.source, image.name),
          onForward: (image) => _forwardPreviewImage(image.id),
          loadOlder: (boundaryId) =>
              _loadConversationImages(boundaryId, before: 1, after: 0),
          loadNewer: (boundaryId) =>
              _loadConversationImages(boundaryId, before: 0, after: 1),
        ),
      ),
    );
  }

  Future<List<ImagePreviewEntry>> _loadConversationImages(
    String targetMessageId, {
    required int before,
    required int after,
  }) async {
    final items = await widget.account.message().imageMessagesAround(
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
            thumbImage: item.thumbImage,
            canForward: item.canForward,
            userId: item.userId,
            userFullName: item.userFullName,
            userIdentityNumber: item.userIdentityNumber,
            avatarUrl: item.avatarUrl,
          ),
        )
        .toList(growable: false);
  }

  Future<bool> _forwardPreviewImage(String messageId) async {
    final target = await showConversationSelector(
      context,
      account: widget.account,
      title: context.l10n.forward,
    );
    if (target == null || !mounted) return false;
    await widget.account.message().forwardMessages(
      targetConversationId: target.id,
      sourceMessageIds: [messageId],
    );
    return true;
  }

  void _openVideo(MessageListEntry message) {
    final source = _messageMediaSource(message);
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
          onForward: _canForwardPreview(message)
              ? () => _forwardPreviewMessage(message)
              : null,
        ),
      ),
    );
  }

  void _openPost(MessageListEntry message) {
    unawaited(
      PostPreviewPage.show(
        context,
        PostPreviewPage(content: message.content, title: message.senderName),
      ),
    );
  }

  void _openTranscript(String transcriptId) {
    final onSelectConversation = widget.onSelectConversation;
    if (onSelectConversation == null) return;
    unawaited(
      showTranscriptDialog(
        context,
        account: widget.account,
        transcriptId: transcriptId,
        sentByCurrentUser:
            widget.message.senderRelationship.toUpperCase() == 'ME',
        currentUserId: widget.currentUserId,
        onSelectConversation: onSelectConversation,
        onSelectConversationInfo:
            widget.onSelectConversationInfo ?? onSelectConversation,
        onOpenUri: widget.onOpenUri ?? (uri) => unawaited(launchUrl(uri)),
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
    await openOrSaveMessageFile(context, source, mediaName: message.mediaName);
  }

  Future<void> _saveMessage(MessageListEntry message) =>
      _saveSource(_messageMediaSource(message), message.mediaName);

  Future<void> _copyImage(File file) async {
    await copyLocalFileToClipboard(file);
  }

  void _openAction(String action, {String? title}) {
    openMessageAction(
      context: context,
      account: widget.account,
      conversationId: widget.message.conversationId,
      action: action,
      title: title,
      onOpenUri: widget.onOpenUri,
    );
  }

  Future<void> _addSticker() async {
    try {
      await widget.onAddSticker();
      if (!mounted) return;
      showToastSuccessful();
    } on Object {
      if (!mounted) return;
      showToastFailed(ToastError(context.l10n.addStickerFailed));
    }
  }

  void _handleStrangerAction(String action) {
    unawaited(() async {
      try {
        final uri = await widget.onStrangerAction(action);
        if (!mounted) return;
        if (mounted && (action == 'block' || action == 'add_contact')) {
          setState(() => _strangerResolved = true);
        }
        if (uri != null && action == 'open_home') {
          await openBotWebViewWindow(
            context: context,
            url: uri,
            title: widget.message.senderName,
            conversationId: widget.message.conversationId,
            currency: widget.account.profile().fiatCurrency,
          );
        } else if (uri != null) {
          _openAction(uri);
        }
      } on Object catch (error) {
        if (!mounted) return;
        showToastFailed(error);
      }
    }());
  }

  Future<void> _saveSource(String source, String? name) async {
    if (source.isEmpty) return;
    await saveMessageFileAs(source, suggestedName: name);
  }
}

class _AnimatedSelectionIcon extends StatefulWidget {
  const _AnimatedSelectionIcon({
    required this.selected,
    required this.inSelectionMode,
  });

  final bool selected;
  final bool inSelectionMode;

  @override
  State<_AnimatedSelectionIcon> createState() => _AnimatedSelectionIconState();
}

class _AnimatedSelectionIconState extends State<_AnimatedSelectionIcon>
    with SingleTickerProviderStateMixin {
  late final AnimationController controller = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 300),
  );
  late final Animation<double> width = controller.drive(
    Tween<double>(begin: 0, end: 48).chain(CurveTween(curve: Curves.easeInOut)),
  );

  @override
  void initState() {
    super.initState();
    if (widget.inSelectionMode) controller.value = 1;
  }

  @override
  void didUpdateWidget(_AnimatedSelectionIcon oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.inSelectionMode == widget.inSelectionMode) return;
    if (widget.inSelectionMode) {
      controller.forward();
    } else {
      controller.reverse();
    }
  }

  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
    animation: width,
    builder: (context, _) {
      if (width.isDismissed) return const SizedBox();
      return SizedBox(
        width: width.value,
        height: 20,
        child: Center(
          child: ClipOval(
            child: Container(
              width: 16,
              height: 16,
              alignment: const Alignment(0, -0.2),
              color: widget.selected
                  ? context.theme.accent
                  : context.theme.secondaryText,
              child: SvgPicture.asset(
                MixinAssets.selected,
                width: 10,
                height: 10,
              ),
            ),
          ),
        ),
      );
    },
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

class VoiceRecorderBarOverlayComposition extends StatefulWidget {
  const VoiceRecorderBarOverlayComposition({
    required this.controller,
    required this.onCancel,
    required this.onStop,
    required this.onRetry,
    required this.onSend,
    required this.child,
    super.key,
  });

  final VoiceRecorderController controller;
  final Future<void> Function() onCancel;
  final Future<void> Function() onStop;
  final Future<void> Function() onRetry;
  final Future<void> Function() onSend;
  final Widget child;

  @override
  State<VoiceRecorderBarOverlayComposition> createState() =>
      _VoiceRecorderBarOverlayCompositionState();
}

class _VoiceRecorderBarOverlayCompositionState
    extends State<VoiceRecorderBarOverlayComposition> {
  final LayerLink _link = LayerLink();
  OverlayEntry? _entry;
  double _layoutWidth = 0;
  bool _syncScheduled = false;

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_onRecorderChanged);
  }

  @override
  void didUpdateWidget(covariant VoiceRecorderBarOverlayComposition oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.controller, widget.controller)) {
      oldWidget.controller.removeListener(_onRecorderChanged);
      widget.controller.addListener(_onRecorderChanged);
    }
    _scheduleOverlaySync();
  }

  void _onRecorderChanged() {
    _entry?.markNeedsBuild();
    _scheduleOverlaySync();
  }

  void _scheduleOverlaySync() {
    if (_syncScheduled || !mounted) return;
    _syncScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _syncScheduled = false;
      if (mounted) _syncOverlay();
    });
  }

  void _syncOverlay() {
    final recorderMode =
        widget.controller.value.status != VoiceRecorderStatus.idle;
    if (!recorderMode) {
      _entry?.remove();
      _entry = null;
      return;
    }
    if (_entry != null) {
      _entry!.markNeedsBuild();
      return;
    }
    final entry = OverlayEntry(
      builder: (context) => Stack(
        fit: StackFit.expand,
        children: [
          if (widget.controller.value.status == VoiceRecorderStatus.recording)
            GestureDetector(
              behavior: HitTestBehavior.translucent,
              onTap: () => _showDiscardRecordingWarningAlertOverlay(
                context,
                onDiscard: () => unawaited(widget.onCancel()),
              ),
              child: const SizedBox.expand(),
            ),
          UnconstrainedBox(
            child: CompositedTransformFollower(
              link: _link,
              showWhenUnlinked: false,
              targetAnchor: Alignment.bottomCenter,
              followerAnchor: Alignment.bottomCenter,
              child: SizedBox(
                width: _layoutWidth,
                child: Material(
                  child: VoiceRecorderBar(
                    state: widget.controller.value,
                    onCancel: widget.onCancel,
                    onStop: widget.onStop,
                    onRetry: widget.onRetry,
                    onSend: widget.onSend,
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
    _entry = entry;
    (Navigator.of(context).overlay ?? Overlay.of(context)).insert(entry);
  }

  @override
  void dispose() {
    widget.controller.removeListener(_onRecorderChanged);
    _entry?.remove();
    _entry = null;
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      if (_layoutWidth != constraints.maxWidth) {
        _layoutWidth = constraints.maxWidth;
        _scheduleOverlaySync();
      }
      return CompositedTransformTarget(link: _link, child: widget.child);
    },
  );
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
    return Container(
      key: const Key('voice-recorder-bar'),
      height: 56,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      color: context.theme.primary,
      child: Row(
        children: [
          ActionButton(
            key: const Key('voice-cancel'),
            name: MixinAssets.closeOvalRecord,
            color: context.theme.icon,
            onTap: () => unawaited(onCancel()),
          ),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: recording == null
                  ? _RecordingDuration(duration: state.elapsed)
                  : _VoiceRecordingPreview(recording: recording),
            ),
          ),
          if (isRecording)
            ActionButton(
              key: const Key('voice-stop'),
              name: MixinAssets.recordStop,
              color: context.theme.accent,
              onTap: () => unawaited(onStop()),
            )
          else
            ActionButton(
              key: const Key('voice-retry'),
              name: MixinAssets.recordRetry,
              color: context.theme.icon,
              onTap: () => unawaited(onRetry()),
            ),
          ActionButton(
            key: const Key('voice-send'),
            name: MixinAssets.send,
            color: context.theme.icon,
            onTap: () => unawaited(onSend()),
          ),
        ],
      ),
    );
  }
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
    await _coordinator.playPreview(_previewId, widget.recording.path);
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
    return SizedBox(
      height: 32,
      child: Material(
        color: context.theme.listSelected,
        borderRadius: const BorderRadius.all(Radius.circular(15)),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const SizedBox(width: 2),
            ActionButton(
              key: const Key('voice-preview-play'),
              name: playing
                  ? MixinAssets.recordPreviewStop
                  : MixinAssets.recordPreviewPlay,
              onTap: () => unawaited(_togglePlayback()),
            ),
            const SizedBox(width: 2),
            Expanded(
              child: SizedBox(
                height: 20,
                child: WaveformWidget(
                  waveform: widget.recording.waveform,
                  value: progress,
                  backgroundColor: context.theme.waveformBackground,
                  foregroundColor: context.theme.waveformForeground,
                  maxBarCount: null,
                  alignment: WaveBarAlignment.center,
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
    required this.onChanged,
    required this.onCancelQuote,
    required this.onSend,
    this.onSendSilent,
    this.onPasteFiles,
    this.onSendPost,
    this.mentionUsers = const [],
    this.mentionKeyword = '',
    this.mentionIndex = 0,
    this.onMentionMove,
    this.onMentionSelected,
    this.onContactPressed,
    this.onFilesPressed,
    this.onPicturesPressed,
    this.onStickerPressed,
    this.stickerAction,
    this.onVoicePressed,
    this.voiceButtonActive = false,
    this.isEncryptConversation = false,
    super.key,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final MessageListEntry? quoteMessage;
  final ValueChanged<String> onChanged;
  final VoidCallback onCancelQuote;
  final Future<void> Function() onSend;
  final Future<void> Function()? onSendSilent;
  final Future<void> Function()? onSendPost;
  final Future<void> Function(List<XFile> files)? onPasteFiles;
  final List<rust.ConversationParticipantItem> mentionUsers;
  final String mentionKeyword;
  final int mentionIndex;
  final ValueChanged<int>? onMentionMove;
  final ValueChanged<int?>? onMentionSelected;
  final VoidCallback? onContactPressed;
  final VoidCallback? onFilesPressed;
  final VoidCallback? onPicturesPressed;
  final VoidCallback? onStickerPressed;
  final Widget? stickerAction;
  final VoidCallback? onVoicePressed;
  final bool voiceButtonActive;
  final bool isEncryptConversation;

  @override
  State<ChatInputBar> createState() => _ChatInputBarState();
}

class _ChatInputBarState extends State<ChatInputBar> {
  MessageListEntry? lastQuoteMessage;

  @override
  void initState() {
    super.initState();
    lastQuoteMessage = widget.quoteMessage;
    widget.controller.addListener(_handleTextChanged);
  }

  @override
  void didUpdateWidget(covariant ChatInputBar oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.quoteMessage != null) lastQuoteMessage = widget.quoteMessage;
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
        widget.controller.text.trim().isNotEmpty &&
        widget.controller.value.composing.isCollapsed;
    return Container(
      key: const Key('chat-input-bar'),
      color: context.theme.primary,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          TweenAnimationBuilder<double>(
            tween: Tween(begin: 0, end: widget.quoteMessage == null ? 0 : 1),
            duration: const Duration(milliseconds: 200),
            curve: Curves.easeOut,
            builder: (context, progress, child) => ClipRect(
              child: Align(
                alignment: AlignmentDirectional.topCenter,
                heightFactor: progress,
                child: child,
              ),
            ),
            child: lastQuoteMessage == null
                ? const SizedBox()
                : _QuoteInputPreview(
                    message: lastQuoteMessage!,
                    onCancel: widget.onCancelQuote,
                  ),
          ),
          Container(
            constraints: const BoxConstraints(minHeight: 56),
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                _SendActionTypeButton(
                  onContact: widget.onContactPressed,
                  onFiles: widget.onFilesPressed,
                  onPictures: widget.onPicturesPressed,
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
                    child: LayoutBuilder(
                      builder: (context, constraints) => PortalTarget(
                        visible: widget.mentionUsers.isNotEmpty,
                        anchor: const Aligned(
                          follower: Alignment.bottomCenter,
                          target: Alignment.topCenter,
                        ),
                        closeDuration: const Duration(milliseconds: 150),
                        portalFollower: SizedBox(
                          width: constraints.maxWidth,
                          child: ConstrainedBox(
                            constraints: const BoxConstraints(maxHeight: 200),
                            child: _MentionPanel(
                              users: widget.mentionUsers,
                              keyword: widget.mentionKeyword,
                              selectedIndex: widget.mentionIndex,
                              onSelected: widget.onMentionSelected,
                            ),
                          ),
                        ),
                        child: Shortcuts(
                          shortcuts: {
                            if (widget.mentionUsers.isNotEmpty) ...{
                              const SingleActivator(
                                LogicalKeyboardKey.arrowDown,
                              ): const _MoveMentionIntent(
                                1,
                              ),
                              const SingleActivator(LogicalKeyboardKey.arrowUp):
                                  const _MoveMentionIntent(-1),
                              const SingleActivator(LogicalKeyboardKey.tab):
                                  const _MoveMentionIntent(1),
                              const SingleActivator(LogicalKeyboardKey.enter):
                                  const _SelectMentionIntent(),
                            } else if (sendable)
                              const SingleActivator(LogicalKeyboardKey.enter):
                                  const _SendInputIntent(),
                            if (widget.onSendPost != null)
                              SingleActivator(
                                LogicalKeyboardKey.enter,
                                meta:
                                    defaultTargetPlatform ==
                                    TargetPlatform.macOS,
                                shift: true,
                                alt:
                                    defaultTargetPlatform !=
                                    TargetPlatform.macOS,
                              ): const _SendPostInputIntent(),
                            const SingleActivator(LogicalKeyboardKey.escape):
                                const _CancelQuoteInputIntent(),
                          },
                          child: Actions(
                            actions: {
                              if (widget.onPasteFiles != null)
                                PasteTextIntent: _PasteFilesAction(
                                  context,
                                  widget.onPasteFiles!,
                                ),
                              _MoveMentionIntent:
                                  CallbackAction<_MoveMentionIntent>(
                                    onInvoke: (intent) {
                                      widget.onMentionMove?.call(intent.delta);
                                      return null;
                                    },
                                  ),
                              _SelectMentionIntent:
                                  CallbackAction<_SelectMentionIntent>(
                                    onInvoke: (_) {
                                      widget.onMentionSelected?.call(null);
                                      return null;
                                    },
                                  ),
                              _SendInputIntent:
                                  CallbackAction<_SendInputIntent>(
                                    onInvoke: (_) {
                                      unawaited(widget.onSend());
                                      return null;
                                    },
                                  ),
                              _SendPostInputIntent:
                                  CallbackAction<_SendPostInputIntent>(
                                    onInvoke: (_) {
                                      unawaited(widget.onSendPost?.call());
                                      return null;
                                    },
                                  ),
                              _CancelQuoteInputIntent:
                                  CallbackAction<_CancelQuoteInputIntent>(
                                    onInvoke: (_) {
                                      widget.onCancelQuote();
                                      return null;
                                    },
                                  ),
                            },
                            child: Stack(
                              children: [
                                TextField(
                                  key: const Key('chat-input'),
                                  controller: widget.controller,
                                  focusNode: widget.focusNode,
                                  onChanged: widget.onChanged,
                                  minLines: 1,
                                  maxLines: 7,
                                  textInputAction: TextInputAction.newline,
                                  inputFormatters: [
                                    LengthLimitingTextInputFormatter(
                                      _maxTextLength,
                                    ),
                                  ],
                                  textAlignVertical: TextAlignVertical.center,
                                  selectionHeightStyle: ui
                                      .BoxHeightStyle
                                      .includeLineSpacingMiddle,
                                  contextMenuBuilder: (context, state) =>
                                      MixinAdaptiveSelectionToolbar(
                                        editableTextState: state,
                                      ),
                                  cursorColor: context.theme.accent,
                                  style: TextStyle(
                                    fontSize: 14,
                                    color: context.theme.text,
                                  ),
                                  decoration: InputDecoration(
                                    isDense: true,
                                    border: InputBorder.none,
                                    enabledBorder: InputBorder.none,
                                    focusedBorder: InputBorder.none,
                                    contentPadding: const EdgeInsets.only(
                                      left: 8,
                                      top: 8,
                                      bottom: 8,
                                    ),
                                  ),
                                ),
                                if (widget.controller.text.isEmpty)
                                  Positioned.fill(
                                    left: 8,
                                    child: Align(
                                      alignment: Alignment.centerLeft,
                                      child: IgnorePointer(
                                        child: Text(
                                          widget.isEncryptConversation
                                              ? l10n?.chatHintE2e ??
                                                    'End-to-end encrypted'
                                              : l10n?.typeMessage ??
                                                    'Type message',
                                          style: TextStyle(
                                            color: context.theme.secondaryText,
                                            fontSize: 14,
                                          ),
                                          maxLines: 1,
                                          overflow: TextOverflow.ellipsis,
                                        ),
                                      ),
                                    ),
                                  ),
                              ],
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 16),
                Stack(
                  fit: StackFit.passthrough,
                  children: [
                    if (sendable)
                      Transform.scale(
                        scale: 1,
                        child: ContextMenuWidget(
                          desktopMenuWidgetBuilder:
                              CustomDesktopMenuWidgetBuilder(),
                          menuProvider: (_) => Menu(
                            children: [
                              MenuAction(
                                image: MenuImage.icon(IconFonts.mute),
                                title: context.l10n.sendWithoutSound,
                                callback: widget.onSendSilent ?? widget.onSend,
                              ),
                            ],
                          ),
                          child: _ChatInputAction(
                            actionKey: const Key('chat-send'),
                            asset: MixinAssets.send,
                            onPressed: widget.onSend,
                          ),
                        ),
                      )
                    else
                      Transform.scale(
                        scale: 1,
                        child: _ChatInputAction(
                          actionKey: const Key('chat-voice'),
                          asset: MixinAssets.microphone,
                          active: widget.voiceButtonActive,
                          onPressed: widget.onVoicePressed,
                        ),
                      ),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _MentionPanel extends StatelessWidget {
  const _MentionPanel({
    required this.users,
    required this.keyword,
    required this.selectedIndex,
    required this.onSelected,
  });

  final List<rust.ConversationParticipantItem> users;
  final String keyword;
  final int selectedIndex;
  final ValueChanged<int?>? onSelected;

  @override
  Widget build(BuildContext context) => DecoratedBox(
    decoration: BoxDecoration(color: context.theme.popUp),
    child: ListView.builder(
      itemCount: users.length,
      shrinkWrap: true,
      itemBuilder: (context, index) {
        final user = users[index];
        return InteractiveDecoratedBox.color(
          decoration: selectedIndex == index
              ? BoxDecoration(color: context.theme.listSelected)
              : null,
          onTap: () => onSelected?.call(index),
          child: Container(
            height: 50,
            padding: const EdgeInsets.all(8),
            child: Row(
              children: [
                AvatarView(
                  userId: user.userId,
                  name: user.fullName,
                  avatarUrl: user.avatarUrl,
                  size: 32,
                ),
                const SizedBox(width: 6),
                Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    CustomText(
                      user.fullName,
                      style: TextStyle(
                        fontSize: 14,
                        color: context.theme.text,
                        height: 1,
                      ),
                      textMatchers: [
                        EmojiTextMatcher(),
                        if (keyword.trim().isNotEmpty)
                          MultiKeyWordTextMatcher.createKeywordMatcher(
                            keyword: keyword,
                            style: TextStyle(color: context.theme.accent),
                            caseSensitive: false,
                          ),
                      ],
                      maxLines: 1,
                    ),
                    const SizedBox(height: 2),
                    CustomText(
                      user.identityNumber,
                      style: TextStyle(
                        fontSize: 12,
                        color: context.theme.secondaryText,
                      ),
                      textMatchers: [
                        EmojiTextMatcher(),
                        if (keyword.trim().isNotEmpty)
                          MultiKeyWordTextMatcher.createKeywordMatcher(
                            keyword: keyword,
                            style: TextStyle(color: context.theme.accent),
                            caseSensitive: false,
                          ),
                      ],
                      maxLines: 1,
                    ),
                  ],
                ),
              ],
            ),
          ),
        );
      },
    ),
  );
}

class _MoveMentionIntent extends Intent {
  const _MoveMentionIntent(this.delta);

  final int delta;
}

class _SelectMentionIntent extends Intent {
  const _SelectMentionIntent();
}

class _SendInputIntent extends Intent {
  const _SendInputIntent();
}

class _SendPostInputIntent extends Intent {
  const _SendPostInputIntent();
}

class _CancelQuoteInputIntent extends Intent {
  const _CancelQuoteInputIntent();
}

class _PasteFilesAction extends Action<PasteTextIntent> {
  _PasteFilesAction(this.context, this.onPasteFiles);

  final BuildContext context;
  final Future<void> Function(List<XFile> files) onPasteFiles;

  @override
  Object? invoke(PasteTextIntent intent) {
    final defaultAction = callingAction;
    scheduleMicrotask(() async {
      final files = await readClipboardFiles();
      if (files.isEmpty) {
        defaultAction?.invoke(intent);
        return;
      }
      if (!context.mounted) return;
      await onPasteFiles(
        files.map((file) => XFile(file.path)).toList(growable: false),
      );
    });
    return null;
  }
}

enum _SendActionType { contact, files, picturesAndVideos }

class _SendActionTypeButton extends StatefulWidget {
  const _SendActionTypeButton({
    required this.onContact,
    required this.onFiles,
    required this.onPictures,
  });

  final VoidCallback? onContact;
  final VoidCallback? onFiles;
  final VoidCallback? onPictures;

  @override
  State<_SendActionTypeButton> createState() => _SendActionTypeButtonState();
}

class _SendActionTypeButtonState extends State<_SendActionTypeButton>
    with SingleTickerProviderStateMixin {
  late final AnimationController animationController = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 200),
  );

  @override
  void dispose() {
    animationController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(left: 6),
    child: AnimatedBuilder(
      animation: animationController,
      builder: (context, child) {
        final value = Curves.easeInOut.transform(animationController.value);
        final scale = value < 0.5
            ? 1 - 0.2 * Curves.easeIn.transform(value * 2)
            : 0.8 + 0.2 * Curves.easeOut.transform((value - 0.5) * 2);
        return Transform.scale(
          scale: scale,
          child: Transform.rotate(angle: value * math.pi / 4, child: child),
        );
      },
      child: CustomPopupMenuButton<_SendActionType>(
        key: const Key('chat-add'),
        alignment: Alignment.topCenter,
        icon: MixinAssets.add,
        color: context.theme.icon,
        size: 32,
        padding: const EdgeInsets.all(4),
        onVisibilityChanged: (value) {
          if (!mounted) return;
          if (value) {
            animationController.forward();
          } else {
            animationController.reverse();
          }
        },
        itemBuilder: (context) => [
          CustomPopupMenuItem(
            icon: MixinAssets.contact,
            title: context.l10n.contact,
            value: _SendActionType.contact,
          ),
          CustomPopupMenuItem(
            icon: MixinAssets.file,
            title: context.l10n.files,
            value: _SendActionType.files,
          ),
          CustomPopupMenuItem(
            icon: MixinAssets.filePreviewImages,
            title: context.l10n.picturesAndVideos,
            value: _SendActionType.picturesAndVideos,
          ),
        ],
        onSelected: (value) {
          switch (value) {
            case _SendActionType.contact:
              widget.onContact?.call();
            case _SendActionType.files:
              widget.onFiles?.call();
            case _SendActionType.picturesAndVideos:
              widget.onPictures?.call();
          }
        },
      ),
    ),
  );
}

class _ChatInputAction extends StatelessWidget {
  const _ChatInputAction({
    required this.asset,
    required this.actionKey,
    this.onPressed,
    this.active = false,
  });

  final String asset;
  final Key actionKey;
  final VoidCallback? onPressed;
  final bool active;

  @override
  Widget build(BuildContext context) => ActionButton(
    key: actionKey,
    name: asset,
    onTap: onPressed,
    color: active ? context.theme.accent : context.theme.icon,
  );
}

class _QuoteInputPreview extends StatelessWidget {
  const _QuoteInputPreview({required this.message, required this.onCancel});

  final MessageListEntry message;
  final VoidCallback onCancel;

  @override
  Widget build(BuildContext context) {
    final color = nameColorForId(message.senderId) ?? context.theme.accent;
    final iconAsset = MixinAssets.messageIcon(message.category);
    final source = message.isSticker
        ? message.stickerAssetUrl
        : message.mediaUrl ?? message.thumbUrl ?? message.thumbImage;
    final imageProvider = source == null
        ? null
        : imageProviderForSource(source);
    Widget? image;
    if (message.category.endsWith('_CONTACT')) {
      image = AvatarView(
        userId: message.sharedUserId ?? '',
        name: message.sharedUserFullName ?? '',
        avatarUrl: message.sharedUserAvatarUrl ?? '',
        size: 48,
      );
    } else if (imageProvider != null) {
      image = Image(image: imageProvider, fit: BoxFit.cover);
    }

    return DecoratedBox(
      key: const Key('chat-quote-preview'),
      decoration: BoxDecoration(color: context.theme.popUp),
      child: Row(
        children: [
          Expanded(
            child: Container(
              constraints: const BoxConstraints(minHeight: 50),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Flexible(
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Container(
                          key: const Key('chat-quote-accent'),
                          width: 6,
                          color: color,
                        ),
                        Flexible(
                          child: Padding(
                            padding: const EdgeInsets.only(
                              top: 6,
                              left: 6,
                              bottom: 6,
                            ),
                            child: Column(
                              mainAxisSize: MainAxisSize.min,
                              crossAxisAlignment: CrossAxisAlignment.start,
                              mainAxisAlignment: MainAxisAlignment.spaceBetween,
                              children: [
                                Padding(
                                  padding: const EdgeInsets.only(bottom: 4),
                                  child: Row(
                                    mainAxisSize: MainAxisSize.min,
                                    children: [
                                      Flexible(
                                        child: Text(
                                          message.senderName,
                                          maxLines: 1,
                                          overflow: TextOverflow.ellipsis,
                                          style: TextStyle(
                                            color: color,
                                            fontSize: context
                                                .messageStyle
                                                .secondaryFontSize,
                                            height: 1,
                                          ),
                                        ),
                                      ),
                                      BadgesWidget(
                                        verified: false,
                                        isBot: false,
                                        membership: message.senderMembership,
                                      ),
                                    ],
                                  ),
                                ),
                                Row(
                                  mainAxisSize: MainAxisSize.min,
                                  children: [
                                    if (iconAsset != null)
                                      Padding(
                                        padding: const EdgeInsets.only(
                                          right: 4,
                                        ),
                                        child: SvgPicture.asset(
                                          iconAsset,
                                          width: 16,
                                          height: 16,
                                          colorFilter: ColorFilter.mode(
                                            context.theme.secondaryText,
                                            BlendMode.srcIn,
                                          ),
                                        ),
                                      ),
                                    Flexible(
                                      child: Text(
                                        _messagePreview(context, message),
                                        maxLines: 1,
                                        overflow: TextOverflow.ellipsis,
                                        style: TextStyle(
                                          color: context.theme.secondaryText,
                                          fontSize: context
                                              .messageStyle
                                              .tertiaryFontSize,
                                        ),
                                      ),
                                    ),
                                  ],
                                ),
                              ],
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 8),
                  if (image != null)
                    SizedBox.square(
                      dimension: 48,
                      child: RepaintBoundary(
                        child: ClipRRect(
                          borderRadius: const BorderRadius.all(
                            Radius.circular(6),
                          ),
                          child: image,
                        ),
                      ),
                    ),
                ],
              ),
            ),
          ),
          GestureDetector(
            key: const Key('chat-quote-cancel'),
            onTap: onCancel,
            behavior: HitTestBehavior.opaque,
            child: Container(
              padding: const EdgeInsets.all(14),
              child: SvgPicture.asset(
                MixinAssets.closeOval,
                height: 22,
                width: 22,
              ),
            ),
          ),
        ],
      ),
    );
  }
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

    return SizedBox(
      key: const Key('chat-selection-bar'),
      height: 80,
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceAround,
        children: [
          _SelectionAction(
            label: context.l10n.combineAndForward,
            iconAssetName: MixinAssets.messageTranscriptForward,
            enabled: canCombineForward,
            onTap: canCombineForward
                ? () => onCombineForward(selectedMessages)
                : null,
          ),
          _SelectionAction(
            label: context.l10n.oneByOneForward,
            iconAssetName: MixinAssets.contextMenuForward,
            enabled: canForward,
            onTap: canForward ? () => onForward(selectedMessages) : null,
          ),
          _SelectionAction(
            label: context.l10n.copy,
            iconAssetName: MixinAssets.previewCopy,
            onTap: onCopy,
          ),
          _SelectionAction(
            label: context.l10n.delete,
            iconAssetName: MixinAssets.contextMenuDelete,
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
    required this.iconAssetName,
    this.enabled = true,
    this.onTap,
  });

  final String label;
  final String iconAssetName;
  final bool enabled;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final active = enabled && onTap != null;
    Widget child = Center(
      child: Padding(
        padding: const EdgeInsets.all(8),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            SvgPicture.asset(
              iconAssetName,
              width: 24,
              height: 24,
              colorFilter: ColorFilter.mode(
                context.theme.icon,
                BlendMode.srcIn,
              ),
            ),
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
    );
    if (active) {
      child = InteractiveDecoratedBox.color(
        decoration: const BoxDecoration(),
        hoveringColor: context.dynamicColor(
          const Color.fromRGBO(0, 0, 0, 0.03),
          darkColor: const Color.fromRGBO(255, 255, 255, 0.2),
        ),
        onTap: onTap,
        child: child,
      );
    } else {
      child = Opacity(opacity: 0.5, child: child);
    }
    return Expanded(child: child);
  }
}

Future<void> _showSelectionDeleteDialog(
  BuildContext context, {
  required List<MessageListEntry> selectedMessages,
  required bool canRecall,
  required Future<void> Function(List<MessageListEntry>) onDelete,
  required Future<void> Function(List<MessageListEntry>) onRecall,
}) async {
  final action = await showConfirmMixinDialog(
    context,
    context.l10n.chatDeleteMessage(
      selectedMessages.length,
      selectedMessages.length,
    ),
    positiveText: context.l10n.delete,
    neutralText: canRecall ? context.l10n.deleteForEveryone : null,
  );
  switch (action) {
    case DialogEvent.positive:
      await onDelete(selectedMessages);
    case DialogEvent.neutral:
      await onRecall(selectedMessages);
    case null:
      return;
  }
}

Future<void> _ignoreMutation(Future<void> mutation) async {
  try {
    await mutation;
  } on Object {
    // MessageActionController already logs mutation failures with context.
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
