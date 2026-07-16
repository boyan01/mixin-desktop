import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart' as rust;
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
import 'package:mixin_desktop_ui/widgets/transcript_page.dart';
import 'package:provider/provider.dart';
import 'package:super_context_menu/super_context_menu.dart';
import 'package:url_launcher/url_launcher.dart';

Future<void> showPinnedMessagesDialog(
  BuildContext context, {
  required rust.AccountHandle account,
  required String conversationId,
  required String currentUserId,
  required String? currentUserRole,
  required ValueChanged<String> onLocate,
}) => showDialog<void>(
  context: context,
  builder: (_) => Dialog(
    clipBehavior: Clip.antiAlias,
    child: _PinnedMessagesPage(
      account: account,
      conversationId: conversationId,
      currentUserId: currentUserId,
      currentUserRole: currentUserRole ?? 'MEMBER',
      onLocate: onLocate,
    ),
  ),
);

class _PinnedMessagesPage extends StatefulWidget {
  const _PinnedMessagesPage({
    required this.account,
    required this.conversationId,
    required this.currentUserId,
    required this.currentUserRole,
    required this.onLocate,
  });

  final rust.AccountHandle account;
  final String conversationId;
  final String currentUserId;
  final String? currentUserRole;
  final ValueChanged<String> onLocate;

  @override
  State<_PinnedMessagesPage> createState() => _PinnedMessagesPageState();
}

class _PinnedMessagesPageState extends State<_PinnedMessagesPage> {
  List<MessageListEntry> _messages = const [];
  StreamSubscription<BigInt>? _changes;
  bool _loading = true;
  bool _refreshing = false;
  bool _refreshPending = false;
  String? _error;

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
        final result = await widget.account.pinnedMessages(
          conversationId: widget.conversationId,
        );
        if (!mounted) return;
        setState(() {
          _messages = result.map(MessageListEntry.fromRust).toList();
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

  void _locate(String messageId) {
    Navigator.pop(context);
    widget.onLocate(messageId);
  }

  Future<void> _unpin(MessageListEntry message) async {
    try {
      await widget.account.setMessagePinned(
        conversationId: widget.conversationId,
        messageId: message.id,
        pinned: false,
      );
      await _refresh();
      if (mounted && _messages.isEmpty) Navigator.pop(context);
    } on Object catch (error) {
      if (mounted) setState(() => _error = error.toString());
    }
  }

  Future<void> _download(MessageListEntry message) async {
    try {
      await widget.account.downloadAttachment(messageId: message.id);
      await _refresh();
    } on Object catch (error) {
      if (mounted) setState(() => _error = error.toString());
    }
  }

  Future<void> _cancel(MessageListEntry message) async {
    try {
      await widget.account.cancelAttachment(messageId: message.id);
      await _refresh();
    } on Object catch (error) {
      if (mounted) setState(() => _error = error.toString());
    }
  }

  Future<void> _markAudioRead(MessageListEntry message) async {
    if (message.mediaStatus.toUpperCase() != 'DONE') return;
    await widget.account.markAudioRead(messageId: message.id);
    await _refresh();
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
                    context.l10n.pinnedMessageTitle(
                      _messages.length,
                      _messages.length,
                    ),
                    textAlign: TextAlign.center,
                    style: TextStyle(color: context.theme.text, fontSize: 16),
                  ),
                ),
                const SizedBox(width: 56),
              ],
            ),
          ),
          Divider(height: 1, color: context.theme.divider),
          Expanded(child: _body()),
        ],
      ),
    ),
  );

  Widget _body() {
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
          Padding(
            padding: const EdgeInsets.all(8),
            child: Text(
              _error!,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(color: context.theme.secondaryText),
            ),
          ),
        Expanded(
          child: ListView.builder(
            padding: const EdgeInsets.only(bottom: 16),
            itemCount: _messages.length,
            itemBuilder: (context, index) => _PinnedMessage(
              account: widget.account,
              message: _messages[index],
              previous: index == 0 ? null : _messages[index - 1],
              next: index + 1 == _messages.length ? null : _messages[index + 1],
              messages: _messages,
              currentUserId: widget.currentUserId,
              currentUserRole: widget.currentUserRole,
              onLocate: _locate,
              onUnpin: _unpin,
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

class _PinnedMessage extends StatelessWidget {
  const _PinnedMessage({
    required this.account,
    required this.message,
    required this.previous,
    required this.next,
    required this.messages,
    required this.currentUserId,
    required this.currentUserRole,
    required this.onLocate,
    required this.onUnpin,
    required this.onDownload,
    required this.onCancel,
    required this.onMarkAudioRead,
  });

  final rust.AccountHandle account;
  final MessageListEntry message;
  final MessageListEntry? previous;
  final MessageListEntry? next;
  final List<MessageListEntry> messages;
  final String currentUserId;
  final String? currentUserRole;
  final ValueChanged<String> onLocate;
  final MessageEntryCallback onUnpin;
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
      onOpenMessage: onLocate,
      onOpenTranscript: (id) => unawaited(
        showTranscriptDialog(
          context,
          account: account,
          transcriptId: id,
          currentUserId: currentUserId,
        ),
      ),
      onOpenImage: (item) => _openImage(context, item),
      onOpenVideo: (item) => _openVideo(context, item),
      onOpenPost: (item) => _openPost(context, item),
      onOpenFile: (item) => unawaited(_openFile(context, item)),
      onOpenUser: (id) => _showUser(context, userId: id),
      onOpenIdentityNumber: (id) => _showUser(context, identityNumber: id),
      onMarkAudioRead: onMarkAudioRead,
      onDownloadAttachment: onDownload,
      onCancelAttachment: onCancel,
    );
    final rendered = message.category == 'APP_CARD'
        ? content
        : MessageBubble(
            isCurrentUser: presentation.isCurrentUser,
            showNip: presentation.showNip,
            showBubble: message.showMessageBubble,
            includeNip: message.includeMessageBubbleNip,
            clip: message.clipMessageBubble,
            padding: message.messageBubblePadding,
            forceIsCurrentUserColor: message.forceCurrentMessageBubbleColor,
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
    final localImage = message.isImage
        ? existingLocalFile(message.mediaUrl)
        : null;
    final interactive = ContextMenuWidget(
      menuProvider: (_) => buildMessageActionsMenu(
        context: context,
        message: message,
        policy: policy,
        callbacks: MessageActionCallbacks(
          onLocateToChat: () => onLocate(message.id),
          onTogglePin: () => onUnpin(message),
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
    final column = Column(
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
              AvatarView(
                userId: message.senderId,
                name: message.senderName,
                avatarUrl: message.senderAvatarUrl,
                size: 32,
              ),
              Flexible(
                child: Padding(
                  padding: const EdgeInsets.only(top: 2, bottom: 2),
                  child: column,
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
            child: column,
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

  void _openVideo(BuildContext context, MessageListEntry item) {
    final source = item.mediaUrl?.trim() ?? '';
    if (source.isEmpty) return;
    Navigator.of(context, rootNavigator: true).push(
      MaterialPageRoute<void>(
        builder: (_) => VideoPreviewPage(source: source, title: item.mediaName),
      ),
    );
  }

  void _openPost(BuildContext context, MessageListEntry item) {
    Navigator.of(context, rootNavigator: true).push(
      MaterialPageRoute<void>(
        builder: (_) =>
            PostPreviewPage(content: item.content, title: item.senderName),
      ),
    );
  }

  Future<void> _openFile(BuildContext context, MessageListEntry item) async {
    final source = item.mediaUrl?.trim() ?? '';
    if (source.isEmpty) return;
    final result = await openMessageFile(source);
    if (!context.mounted || result.type.name == 'done') return;
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(result.message)));
  }

  Future<void> _save(MessageListEntry item) async {
    final source = item.mediaUrl?.trim() ?? '';
    if (source.isEmpty) return;
    await saveMessageFileAs(source, suggestedName: item.mediaName);
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
