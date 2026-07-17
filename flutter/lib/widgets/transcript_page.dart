import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/utils/system_clipboard.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/message_action_policy.dart';
import 'package:mixin_desktop_ui/widgets/message_actions_menu.dart';
import 'package:mixin_desktop_ui/widgets/message_bubble.dart';
import 'package:mixin_desktop_ui/widgets/message_content.dart';
import 'package:mixin_desktop_ui/widgets/message_datetime_and_status.dart';
import 'package:mixin_desktop_ui/widgets/message_day_time.dart';
import 'package:mixin_desktop_ui/widgets/message_media_preview_pages.dart';
import 'package:mixin_desktop_ui/widgets/message_name.dart';
import 'package:mixin_desktop_ui/widgets/message_presentation.dart';
import 'package:mixin_desktop_ui/widgets/message_qr_dialog.dart';
import 'package:mixin_desktop_ui/widgets/message_rows.dart';
import 'package:mixin_desktop_ui/widgets/show_message_user_dialog.dart';
import 'package:provider/provider.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';
import 'package:super_context_menu/super_context_menu.dart';
import 'package:url_launcher/url_launcher.dart';

Future<void> showTranscriptDialog(
  BuildContext context, {
  required rust.AccountHandle account,
  required String transcriptId,
  required String currentUserId,
}) => showDialog<void>(
  context: context,
  builder: (_) => Dialog(
    clipBehavior: Clip.antiAlias,
    child: TranscriptPage(
      account: account,
      transcriptId: transcriptId,
      currentUserId: currentUserId,
    ),
  ),
);

class TranscriptPage extends StatefulWidget {
  const TranscriptPage({
    required this.account,
    required this.transcriptId,
    required this.currentUserId,
    super.key,
  });

  final rust.AccountHandle account;
  final String transcriptId;
  final String currentUserId;

  @override
  State<TranscriptPage> createState() => _TranscriptPageState();
}

class _TranscriptPageState extends State<TranscriptPage> {
  List<MessageListEntry> _messages = const [];
  final ItemScrollController _scrollController = ItemScrollController();
  StreamSubscription<BigInt>? _changes;
  bool _loading = true;
  bool _refreshing = false;
  bool _refreshPending = false;
  String? _error;
  String? _highlightedMessageId;
  int _highlightGeneration = 0;

  @override
  void initState() {
    super.initState();
    unawaited(_refresh());
    _changes = widget.account.messageChanges().listen(
      (_) => unawaited(_refresh()),
      onError: (Object error) {
        if (mounted) setState(() => _error = error.toString());
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
        final items = await widget.account.message().transcriptMessages(
          transcriptId: widget.transcriptId,
        );
        if (!mounted) return;
        setState(() {
          _messages = items.map(MessageListEntry.fromRust).toList();
          _loading = false;
          _error = null;
        });
      } on Object catch (error) {
        if (!mounted) return;
        setState(() {
          _loading = false;
          _error = error.toString();
        });
      }
    } while (_refreshPending && mounted);
    _refreshing = false;
  }

  Future<void> _download(MessageListEntry message) async {
    try {
      await widget.account.attachment().downloadTranscriptAttachment(
        transcriptId: widget.transcriptId,
        messageId: message.id,
      );
      await _refresh();
    } on Object catch (error) {
      if (mounted) setState(() => _error = error.toString());
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
      if (mounted) setState(() => _error = error.toString());
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
      if (mounted) setState(() => _error = error.toString());
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
    final generation = ++_highlightGeneration;
    for (var count = 0; count < 6; count++) {
      if (!mounted || generation != _highlightGeneration) return;
      setState(() {
        _highlightedMessageId = count.isEven ? messageId : null;
      });
      await Future<void>.delayed(const Duration(milliseconds: 165));
    }
    if (mounted && generation == _highlightGeneration) {
      setState(() => _highlightedMessageId = null);
    }
  }

  @override
  Widget build(BuildContext context) => SizedBox(
    width: 600,
    height: 800,
    child: Material(
      color: context.theme.primary,
      child: Column(
        children: [
          SizedBox(
            height: 56,
            child: Row(
              children: [
                const SizedBox(width: 8),
                IconButton(
                  onPressed: () => Navigator.pop(context),
                  icon: const Icon(Icons.close),
                ),
                Expanded(
                  child: Text(
                    context.l10n.transcript,
                    textAlign: TextAlign.center,
                    style: TextStyle(color: context.theme.text, fontSize: 16),
                  ),
                ),
                const SizedBox(width: 56),
              ],
            ),
          ),
          Divider(height: 1, color: context.theme.divider),
          Expanded(child: _buildBody()),
        ],
      ),
    ),
  );

  Widget _buildBody() {
    if (_loading) {
      return const Center(child: CircularProgressIndicator(strokeWidth: 2));
    }
    if (_messages.isEmpty) {
      return Center(
        child: Text(
          _error ?? context.l10n.noResults,
          style: TextStyle(color: context.theme.secondaryText),
        ),
      );
    }
    return Column(
      children: [
        if (_error != null)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            color: context.theme.primary,
            child: Text(
              _error!,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                color: context.theme.secondaryText,
                fontSize: 12,
              ),
            ),
          ),
        Expanded(
          child: ScrollablePositionedList.builder(
            itemScrollController: _scrollController,
            padding: const EdgeInsets.only(bottom: 16),
            itemCount: _messages.length,
            itemBuilder: (context, index) => _TranscriptMessage(
              account: widget.account,
              transcriptId: widget.transcriptId,
              currentUserId: widget.currentUserId,
              messages: _messages,
              message: _messages[index],
              previous: index == 0 ? null : _messages[index - 1],
              next: index + 1 == _messages.length ? null : _messages[index + 1],
              highlighted: _highlightedMessageId == _messages[index].id,
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
            ),
          ),
        ),
      ],
    );
  }
}

class _TranscriptMessage extends StatelessWidget {
  const _TranscriptMessage({
    required this.account,
    required this.transcriptId,
    required this.currentUserId,
    required this.messages,
    required this.message,
    required this.previous,
    required this.next,
    required this.highlighted,
    required this.onOpenMessage,
    required this.onDownload,
    required this.onCancel,
    required this.onMarkAudioRead,
  });

  final rust.AccountHandle account;
  final String transcriptId;
  final String currentUserId;
  final List<MessageListEntry> messages;
  final MessageListEntry message;
  final MessageListEntry? previous;
  final MessageListEntry? next;
  final bool highlighted;
  final ValueChanged<String>? onOpenMessage;
  final MessageEntryCallback onDownload;
  final MessageEntryCallback onCancel;
  final MessageEntryCallback onMarkAudioRead;

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
      onOpenUri: (uri) => unawaited(launchUrl(uri)),
      onOpenMessage: onOpenMessage,
      onAction: (action) {
        final uri = Uri.tryParse(action.trim());
        if (uri != null) unawaited(launchUrl(uri));
      },
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
          currentUserId: currentUserId,
        ),
      ),
      onMarkAudioRead: onMarkAudioRead,
      onDownloadAttachment: onDownload,
      onCancelAttachment: onCancel,
    );
    final rendered = message.category == 'APP_CARD'
        ? AnimatedContainer(
            duration: const Duration(milliseconds: 120),
            decoration: BoxDecoration(
              color: highlighted
                  ? context.theme.accent.withValues(alpha: 0.16)
                  : Colors.transparent,
              borderRadius: const BorderRadius.all(Radius.circular(8)),
            ),
            child: content,
          )
        : MessageBubble(
            isCurrentUser: presentation.isCurrentUser,
            showNip: presentation.showNip,
            showBubble: message.showMessageBubble,
            includeNip: message.includeMessageBubbleNip,
            clip: message.clipMessageBubble,
            highlighted: highlighted,
            padding: message.messageBubblePadding,
            forceIsCurrentUserColor: message.forceCurrentMessageBubbleColor,
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
          GestureDetector(
            onTap: () => _showUser(context, userId: message.senderId),
            child: MessageName(
              userName: message.senderName,
              userId: message.senderId,
              userIdentityNumber: message.senderIdentityNumber,
              verified: message.senderIsVerified,
              isBot: message.senderIsBot,
              showIdentityNumber: false,
            ),
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
              InkWell(
                customBorder: const CircleBorder(),
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
        if (row.dateTime != null) MessageDayTimeChip(dateTime: row.dateTime!),
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
          ),
        )
        .toList(growable: false);
    final index = images.indexWhere((item) => item.id == selected.id);
    if (index < 0) return;
    Navigator.of(context, rootNavigator: true).push(
      MaterialPageRoute<void>(
        builder: (_) => ImagePreviewPage(
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
    Navigator.of(context, rootNavigator: true).push(
      MaterialPageRoute<void>(
        builder: (_) =>
            VideoPreviewPage(source: source, title: message.mediaName),
      ),
    );
  }

  void _openPost(BuildContext context, MessageListEntry message) {
    Navigator.of(context, rootNavigator: true).push(
      MaterialPageRoute<void>(
        builder: (_) => PostPreviewPage(
          content: message.content,
          title: message.senderName,
        ),
      ),
    );
  }

  Future<void> _openFile(BuildContext context, MessageListEntry message) async {
    final source = message.mediaUrl?.trim() ?? '';
    if (source.isEmpty) return;
    final result = await openMessageFile(source);
    if (!context.mounted || result.type.name == 'done') return;
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(result.message)));
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
    unawaited(
      showMessageUserDialog(
        context,
        account: account,
        userId: userId,
        identityNumber: identityNumber,
      ),
    );
  }
}
