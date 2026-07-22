import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:filesize/filesize.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:markdown_widget/markdown_widget.dart';
import 'package:mime/mime.dart';
import 'package:path/path.dart' as p;
import 'package:provider/provider.dart';

import '../constants/assets.dart';
import '../controllers/settings_controller.dart';
import '../l10n/generated/app_localizations.dart';
import '../models/message_list_entry.dart';
import '../theme.dart';
import 'attachment_status.dart';
import 'image_by_blur_hash.dart';
import 'interactive_decorated_box.dart';
import 'message_audio.dart';
import 'message_bubble.dart';
import 'message_items/special_message_items.dart';
import 'message_layout.dart';
import 'message_selectable_text.dart';
import 'mixin_image.dart';
import 'post_markdown.dart';
import 'sticker_page/sticker_item.dart';

typedef MessageEntryCallback = void Function(MessageListEntry message);

extension on BuildContext {
  double messageFontSize(double base) =>
      base + watch<SettingsController>().chatFontSizeDelta;
}

class MessageContent extends StatelessWidget {
  const MessageContent({
    required this.message,
    required this.isCurrentUser,
    required this.dateAndStatus,
    required this.overlayDateAndStatus,
    super.key,
    this.onOpenUri,
    this.onOpenUser,
    this.onOpenMessage,
    this.onAction,
    this.onAppAction,
    this.onOpenTranscript,
    this.onOpenSnapshot,
    this.onOpenImage,
    this.onOpenVideo,
    this.onOpenPost,
    this.onOpenFile,
    this.onOpenSticker,
    this.onOpenIdentityNumber,
    this.onMarkAudioRead,
    this.onDownloadAttachment,
    this.onCancelAttachment,
    this.currentUserId,
    this.mentionNames = const {},
    this.keyword = '',
    this.showNip = false,
    this.highlighted = false,
    this.highlightOpacity = 0,
    this.recalledText,
    this.onReedit,
    this.audioPlaylist = const [],
    this.attachmentSentByCurrentUser,
    this.isPinnedPage = false,
    this.onPinnedMessageTap,
  });

  final MessageListEntry message;
  final bool isCurrentUser;
  final Widget dateAndStatus;
  final Widget overlayDateAndStatus;
  final MessageUriCallback? onOpenUri;
  final MessageStringCallback? onOpenUser;
  final MessageStringCallback? onOpenMessage;
  final MessageStringCallback? onAction;
  final MessageActionCallback? onAppAction;
  final MessageStringCallback? onOpenTranscript;
  final MessageStringCallback? onOpenSnapshot;
  final MessageEntryCallback? onOpenImage;
  final MessageEntryCallback? onOpenVideo;
  final MessageEntryCallback? onOpenPost;
  final MessageEntryCallback? onOpenFile;
  final MessageEntryCallback? onOpenSticker;
  final MessageStringCallback? onOpenIdentityNumber;
  final MessageEntryCallback? onMarkAudioRead;
  final MessageEntryCallback? onDownloadAttachment;
  final MessageEntryCallback? onCancelAttachment;
  final String? currentUserId;
  final Map<String, String> mentionNames;
  final String keyword;
  final bool showNip;
  final bool highlighted;
  final double highlightOpacity;
  final String? recalledText;
  final ValueChanged<String>? onReedit;
  final List<MessageListEntry> audioPlaylist;
  final bool? attachmentSentByCurrentUser;
  final bool isPinnedPage;
  final VoidCallback? onPinnedMessageTap;

  @override
  Widget build(BuildContext context) {
    final status = message.status.toUpperCase();
    if (status == 'UNKNOWN' || message.isInvalidSpecialMessage) {
      final l10n = Localizations.of<AppLocalizations>(
        context,
        AppLocalizations,
      );
      final helpUri = Uri.tryParse(l10n?.chatNotSupportUrl ?? '');
      return MessageLayout(
        spacing: 6,
        content: _UnknownMessage(
          message: message,
          onOpenHelp: helpUri == null || onOpenUri == null
              ? null
              : () => onOpenUri!(helpUri),
        ),
        dateAndStatus: dateAndStatus,
      );
    }
    if (status == 'FAILED') {
      final l10n = Localizations.of<AppLocalizations>(
        context,
        AppLocalizations,
      );
      final helpUri = Uri.tryParse(l10n?.chatNotSupportUrl ?? '');
      return WaitingMessageItem(
        messageId: message.id,
        subject: isCurrentUser ? l10n?.linkedDevice ?? '' : message.senderName,
        dateAndStatus: dateAndStatus,
        onOpenHelp: helpUri == null || onOpenUri == null
            ? null
            : () => onOpenUri!(helpUri),
      );
    }
    if (message.category == 'SYSTEM_CONVERSATION') {
      return SystemConversationMessageItem(
        message: message,
        currentUserId: currentUserId ?? '',
      );
    }
    if (message.category == 'MESSAGE_PIN') {
      return PinMessageItem(message: message, mentionNames: mentionNames);
    }
    if (message.category == 'SECRET') {
      return SecretMessageItem(onOpenUri: onOpenUri);
    }
    if (message.category == 'STRANGER') {
      if (const {
        'FRIEND',
        'BLOCKED',
      }.contains(message.senderRelationship.toUpperCase())) {
        return const SizedBox.shrink();
      }
      return StrangerMessageItem(message: message, onAction: onAction);
    }
    if (message.isRecall) {
      final l10n = Localizations.of<AppLocalizations>(
        context,
        AppLocalizations,
      );
      return Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          SvgPicture.asset(
            MixinAssets.messageIcon(message.category)!,
            width: 16,
            height: 16,
            colorFilter: ColorFilter.mode(
              context.theme.secondaryText,
              BlendMode.srcIn,
            ),
          ),
          const SizedBox(width: 4),
          Flexible(
            child: Text.rich(
              TextSpan(
                children: [
                  TextSpan(
                    text: isCurrentUser
                        ? l10n?.youDeletedThisMessage ??
                              'You deleted this message'
                        : l10n?.thisMessageWasDeleted ??
                              'This message was deleted',
                  ),
                  if (recalledText != null)
                    TextSpan(
                      text: ' ${l10n?.reedit ?? 'Re-edit'}',
                      style: TextStyle(color: context.theme.accent),
                      recognizer: TapGestureRecognizer()
                        ..onTap = () => onReedit?.call(recalledText!),
                    ),
                ],
              ),
              style: TextStyle(
                fontSize: context.messageFontSize(16),
                color: context.theme.text,
              ),
            ),
          ),
        ],
      );
    }

    final quote = buildMessageQuotePreview(
      message,
      onOpenMessage: onOpenMessage,
      mentionNames: mentionNames,
    );
    Widget content;
    if (message.category.endsWith('_TRANSCRIPT')) {
      content = TranscriptMessageItem(
        message: message,
        isCurrentUser: isCurrentUser,
        overlayDateAndStatus: overlayDateAndStatus,
        onOpenTranscript: onOpenTranscript,
      );
    } else if (message.category.endsWith('_LOCATION')) {
      content = LocationMessageItem(message: message, onOpenUri: onOpenUri);
    } else if (message.category == 'SYSTEM_ACCOUNT_SNAPSHOT') {
      content = SnapshotMessageItem(
        message: message,
        kind: SnapshotKind.account,
        onOpenSnapshot: onOpenSnapshot,
      );
    } else if (message.category == 'SYSTEM_SAFE_SNAPSHOT') {
      content = SnapshotMessageItem(
        message: message,
        kind: SnapshotKind.safe,
        onOpenSnapshot: onOpenSnapshot,
      );
    } else if (message.category == 'SYSTEM_SAFE_INSCRIPTION') {
      content = SnapshotMessageItem(
        message: message,
        kind: SnapshotKind.safeInscription,
        onOpenSnapshot: onOpenSnapshot,
      );
    } else if (message.category.endsWith('_CONTACT')) {
      content = ContactMessageItem(message: message, onOpenUser: onOpenUser);
    } else if (message.category == 'APP_BUTTON_GROUP') {
      content = AppButtonGroupMessageItem(
        message: message,
        onAction: onAppAction,
        isCurrentUser: isCurrentUser,
        showNip: showNip,
        highlighted: highlighted,
        highlightOpacity: highlightOpacity,
        quote: quote,
        isPinnedPage: isPinnedPage,
        onPinnedMessageTap: onPinnedMessageTap,
      );
    } else if (message.category == 'APP_CARD') {
      content = AppCardMessageItem(
        message: message,
        onAction: onAppAction,
        isCurrentUser: isCurrentUser,
        showNip: showNip,
        highlighted: highlighted,
        highlightOpacity: highlightOpacity,
        dateAndStatus: dateAndStatus,
        quote: quote,
        onOpenUri: onOpenUri,
        onOpenIdentityNumber: onOpenIdentityNumber,
        mentionNames: mentionNames,
        keyword: keyword,
        isPinnedPage: isPinnedPage,
        onPinnedMessageTap: onPinnedMessageTap,
      );
    } else {
      content = _standardContent(context);
    }
    return content;
  }

  Widget _standardContent(BuildContext context) {
    final attachmentIsCurrentUser =
        attachmentSentByCurrentUser ?? isCurrentUser;
    if (message.isImage) {
      if (message.mediaWidth == null || message.mediaHeight == null) {
        final l10n = Localizations.of<AppLocalizations>(
          context,
          AppLocalizations,
        );
        final helpUri = Uri.tryParse(l10n?.chatNotSupportUrl ?? '');
        return MessageLayout(
          spacing: 6,
          content: _UnknownMessage(
            message: message,
            onOpenHelp: helpUri == null || onOpenUri == null
                ? null
                : () => onOpenUri!(helpUri),
          ),
          dateAndStatus: dateAndStatus,
        );
      }
      return _ImageMessage(
        message: message,
        isCurrentUser: isCurrentUser,
        sentByCurrentUser: attachmentIsCurrentUser,
        overlayDateAndStatus: overlayDateAndStatus,
        onOpen: onOpenImage,
        onDownloadAttachment: onDownloadAttachment,
        onCancelAttachment: onCancelAttachment,
        onOpenUri: onOpenUri,
        onOpenIdentityNumber: onOpenIdentityNumber,
        mentionNames: mentionNames,
        keyword: keyword,
      );
    }
    if (message.isVideo) {
      return _VideoMessage(
        message: message,
        isCurrentUser: isCurrentUser,
        sentByCurrentUser: attachmentIsCurrentUser,
        overlayDateAndStatus: overlayDateAndStatus,
        onOpen: onOpenVideo,
        onDownloadAttachment: onDownloadAttachment,
        onCancelAttachment: onCancelAttachment,
        onOpenUri: onOpenUri,
      );
    }
    if (message.isAudio) {
      return AudioMessageWidget(
        message: message,
        sentByCurrentUser: attachmentIsCurrentUser,
        playlist: audioPlaylist,
        onMarkRead: onMarkAudioRead,
        onDownloadAttachment: onDownloadAttachment,
        onCancelAttachment: onCancelAttachment,
      );
    }
    if (message.isSticker) {
      return _StickerMessage(message: message, onOpen: onOpenSticker);
    }
    if (message.category.endsWith('_DATA')) {
      return MessageFile(
        message: message,
        sentByCurrentUser: attachmentIsCurrentUser,
        onOpen: onOpenFile,
        onDownloadAttachment: onDownloadAttachment,
        onCancelAttachment: onCancelAttachment,
      );
    }
    if (message.isPost) {
      return MessagePost(
        message: message,
        onOpen: onOpenPost,
        overlayDateAndStatus: overlayDateAndStatus,
      );
    }
    if (message.isText) {
      return _TextMessage(
        message: message,
        dateAndStatus: dateAndStatus,
        onOpenUri: onOpenUri,
        onOpenIdentityNumber: onOpenIdentityNumber,
        mentionNames: mentionNames,
        keyword: keyword,
      );
    }
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    final helpUri = Uri.tryParse(l10n?.chatNotSupportUrl ?? '');
    return MessageLayout(
      spacing: 6,
      content: _UnknownMessage(
        message: message,
        onOpenHelp: helpUri == null || onOpenUri == null
            ? null
            : () => onOpenUri!(helpUri),
      ),
      dateAndStatus: dateAndStatus,
    );
  }
}

class _TextMessage extends StatelessWidget {
  const _TextMessage({
    required this.message,
    required this.dateAndStatus,
    this.onOpenUri,
    this.onOpenIdentityNumber,
    this.mentionNames = const {},
    this.keyword = '',
  });

  final MessageListEntry message;
  final Widget dateAndStatus;
  final MessageUriCallback? onOpenUri;
  final MessageStringCallback? onOpenIdentityNumber;
  final Map<String, String> mentionNames;
  final String keyword;

  @override
  Widget build(BuildContext context) => MessageLayout(
    spacing: 6,
    content: SelectableMessageText(
      content: message.content,
      onOpenUri: onOpenUri,
      onOpenIdentityNumber: onOpenIdentityNumber,
      mentionNames: mentionNames,
      keyword: keyword,
      style: TextStyle(
        fontSize: context.messageFontSize(16),
        color: context.theme.text,
      ),
    ),
    dateAndStatus: dateAndStatus,
  );
}

class _ImageMessage extends StatelessWidget {
  const _ImageMessage({
    required this.message,
    required this.isCurrentUser,
    required this.sentByCurrentUser,
    required this.overlayDateAndStatus,
    this.onOpen,
    this.onDownloadAttachment,
    this.onCancelAttachment,
    this.onOpenUri,
    this.onOpenIdentityNumber,
    this.mentionNames = const {},
    this.keyword = '',
  });

  final MessageListEntry message;
  final bool isCurrentUser;
  final bool sentByCurrentUser;
  final Widget overlayDateAndStatus;
  final MessageEntryCallback? onOpen;
  final MessageEntryCallback? onDownloadAttachment;
  final MessageEntryCallback? onCancelAttachment;
  final MessageUriCallback? onOpenUri;
  final MessageStringCallback? onOpenIdentityNumber;
  final Map<String, String> mentionNames;
  final String keyword;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final size = _imageMessageSize(
        context,
        constraints,
        mediaWidth: message.mediaWidth,
        mediaHeight: message.mediaHeight,
      );
      final caption = message.caption ?? '';
      final hasCaption = caption.trim().isNotEmpty;
      final image = MessageImage(
        message: message,
        size: size,
        showStatus: !hasCaption,
        isCurrentUser: isCurrentUser,
        sentByCurrentUser: sentByCurrentUser,
        overlayDateAndStatus: overlayDateAndStatus,
        onOpen: onOpen,
        onDownloadAttachment: onDownloadAttachment,
        onCancelAttachment: onCancelAttachment,
      );
      return ConstrainedBox(
        key: Key('message-media-image-${message.id}'),
        constraints: BoxConstraints(maxWidth: size.width),
        child: !hasCaption
            ? image
            : Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  image,
                  Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 10,
                    ),
                    child: SelectableMessageText(
                      content: caption,
                      onOpenUri: onOpenUri,
                      onOpenIdentityNumber: onOpenIdentityNumber,
                      mentionNames: mentionNames,
                      keyword: keyword,
                      style: TextStyle(
                        color: context.theme.text,
                        fontSize: context.messageFontSize(16),
                      ),
                    ),
                  ),
                ],
              ),
      );
    },
  );
}

class MessageImage extends StatelessWidget {
  const MessageImage({
    required this.message,
    required this.showStatus,
    required this.isCurrentUser,
    required this.sentByCurrentUser,
    required this.overlayDateAndStatus,
    this.size,
    this.onOpen,
    this.onDownloadAttachment,
    this.onCancelAttachment,
    super.key,
  });

  final MessageListEntry message;
  final bool showStatus;
  final bool isCurrentUser;
  final bool sentByCurrentUser;
  final Widget overlayDateAndStatus;
  final Size? size;
  final MessageEntryCallback? onOpen;
  final MessageEntryCallback? onDownloadAttachment;
  final MessageEntryCallback? onCancelAttachment;

  @override
  Widget build(BuildContext context) => _InteractiveMessageCard(
    message: message,
    onTap: _attachmentAction(
      message,
      onOpen: onOpen,
      onDownload: onDownloadAttachment,
      onCancel: onCancelAttachment,
    ),
    child: SizedBox.fromSize(
      size: size,
      child: Stack(
        fit: StackFit.expand,
        children: [
          _MediaImage(message: message, fit: BoxFit.cover),
          Center(
            child: MediaStatusOverlay(
              messageId: message.id,
              status: message.mediaStatus,
              upload: sentByCurrentUser && message.mediaUrl?.isNotEmpty == true,
            ),
          ),
          if (showStatus)
            Positioned(
              right: isCurrentUser ? 12 : 4,
              bottom: 4,
              child: _TimestampPill(child: overlayDateAndStatus),
            ),
          if (size != null && _needsImageExtendIcon(message, size!))
            Positioned(
              top: 8,
              right: isCurrentUser ? 16 : 8,
              child: const _PostDetailIcon(),
            ),
        ],
      ),
    ),
  );
}

class _VideoMessage extends StatelessWidget {
  const _VideoMessage({
    required this.message,
    required this.isCurrentUser,
    required this.sentByCurrentUser,
    required this.overlayDateAndStatus,
    this.onOpen,
    this.onDownloadAttachment,
    this.onCancelAttachment,
    this.onOpenUri,
  });

  final MessageListEntry message;
  final bool isCurrentUser;
  final bool sentByCurrentUser;
  final Widget overlayDateAndStatus;
  final MessageEntryCallback? onOpen;
  final MessageEntryCallback? onDownloadAttachment;
  final MessageEntryCallback? onCancelAttachment;
  final MessageUriCallback? onOpenUri;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final size = _videoMessageSize(
        constraints,
        mediaWidth: message.mediaWidth,
        mediaHeight: message.mediaHeight,
      );
      final preview = MessageVideo(
        message: message,
        size: size,
        isCurrentUser: isCurrentUser,
        sentByCurrentUser: sentByCurrentUser,
        overlayDateAndStatus: overlayDateAndStatus,
        onOpen: onOpen,
        onDownloadAttachment: onDownloadAttachment,
        onCancelAttachment: onCancelAttachment,
        onOpenUri: onOpenUri,
      );
      return ConstrainedBox(
        key: Key('message-media-video-${message.id}'),
        constraints: BoxConstraints(maxWidth: size.width),
        child: preview,
      );
    },
  );
}

class MessageVideo extends StatelessWidget {
  const MessageVideo({
    required this.message,
    required this.isCurrentUser,
    required this.sentByCurrentUser,
    required this.overlayDateAndStatus,
    this.size,
    this.overlay,
    this.onOpen,
    this.onDownloadAttachment,
    this.onCancelAttachment,
    this.onOpenUri,
    super.key,
  });

  final MessageListEntry message;
  final bool isCurrentUser;
  final bool sentByCurrentUser;
  final Widget overlayDateAndStatus;
  final Size? size;
  final Widget? overlay;
  final MessageEntryCallback? onOpen;
  final MessageEntryCallback? onDownloadAttachment;
  final MessageEntryCallback? onCancelAttachment;
  final MessageUriCallback? onOpenUri;

  @override
  Widget build(BuildContext context) => _InteractiveMessageCard(
    message: message,
    onTap: _videoAction,
    child: SizedBox.fromSize(
      size: size,
      child: Stack(
        fit: StackFit.expand,
        children: [
          _MediaImage(message: message, useMediaUrl: false, fit: BoxFit.cover),
          if (message.thumbUrl?.trim().isNotEmpty == true)
            _imageForSource(
                  message.thumbUrl,
                  fit: BoxFit.cover,
                  allowEmbeddedData: false,
                  errorBuilder: const SizedBox.shrink(),
                ) ??
                const SizedBox.shrink(),
          overlay ??
              Stack(
                fit: StackFit.expand,
                children: [
                  Center(
                    child: MediaStatusOverlay(
                      messageId: message.id,
                      status: message.mediaStatus,
                      upload:
                          sentByCurrentUser &&
                          message.mediaUrl?.isNotEmpty == true,
                      done: SvgPicture.asset(
                        MixinAssets.videoPlay,
                        width: 38,
                        height: 38,
                      ),
                    ),
                  ),
                  if (message.isVideo)
                    Positioned(
                      top: 6,
                      left: isCurrentUser ? 6 : 14,
                      child: _MediaLabel(
                        text: formatMessageDuration(message.mediaDuration),
                      ),
                    ),
                  Positioned(
                    right: isCurrentUser ? 12 : 4,
                    bottom: 4,
                    child: _TimestampPill(child: overlayDateAndStatus),
                  ),
                ],
              ),
        ],
      ),
    ),
  );

  void _videoAction(MessageListEntry _) {
    switch (message.mediaStatus.toUpperCase()) {
      case 'CANCELED':
        onDownloadAttachment?.call(message);
      case 'PENDING':
        onCancelAttachment?.call(message);
      case 'DONE':
        onOpen?.call(message);
      default:
        if (!message.isLive || onOpenUri == null) return;
        final uri = Uri.tryParse(message.mediaUrl?.trim() ?? '');
        if (uri != null) onOpenUri!(uri);
    }
  }
}

class _StickerMessage extends StatelessWidget {
  const _StickerMessage({required this.message, this.onOpen});

  final MessageListEntry message;
  final MessageEntryCallback? onOpen;

  @override
  Widget build(BuildContext context) {
    final size = _stickerSize(
      context,
      message.stickerAssetWidth ?? message.mediaWidth,
      message.stickerAssetHeight ?? message.mediaHeight,
      message.stickerAssetType,
    );
    final placeholder = Container(
      width: size.width,
      height: size.height,
      color: context.theme.stickerPlaceholderColor,
    );
    final assetUrl = message.stickerAssetUrl ?? message.mediaUrl;
    final sticker = assetUrl == null || assetUrl.isEmpty
        ? null
        : _StickerAsset(
            source: assetUrl,
            assetType: message.stickerAssetType,
            stickerId: message.stickerId,
            placeholder: placeholder,
            size: size,
          );
    final child = sticker == null
        ? placeholder
        : SizedBox.fromSize(
            key: Key('message-media-sticker-${message.id}'),
            size: size,
            child: sticker,
          );
    if (onOpen == null || message.stickerId?.isNotEmpty != true) return child;
    return InteractiveDecoratedBox(onTap: () => onOpen!(message), child: child);
  }
}

class _StickerAsset extends StatelessWidget {
  const _StickerAsset({
    required this.source,
    required this.assetType,
    required this.stickerId,
    required this.placeholder,
    required this.size,
  });

  final String source;
  final String? assetType;
  final String? stickerId;
  final Widget placeholder;
  final Size size;

  @override
  Widget build(BuildContext context) => StickerItem(
    assetUrl: source,
    assetType: assetType,
    stickerId: stickerId,
    errorWidget: placeholder,
    width: size.width,
    height: size.height,
  );
}

class MessageFile extends StatelessWidget {
  const MessageFile({
    required this.message,
    this.sentByCurrentUser,
    this.onOpen,
    this.onDownloadAttachment,
    this.onCancelAttachment,
    super.key,
  });

  final MessageListEntry message;
  final bool? sentByCurrentUser;
  final MessageEntryCallback? onOpen;
  final MessageEntryCallback? onDownloadAttachment;
  final MessageEntryCallback? onCancelAttachment;

  @override
  Widget build(BuildContext context) {
    final name = message.mediaName ?? '';
    final extension = _fileExtension(name);
    final action = _attachmentAction(
      message,
      onOpen: onOpen,
      onDownload: onDownloadAttachment,
      onCancel: onCancelAttachment,
    );
    final isCurrentUser =
        sentByCurrentUser ?? message.senderRelationship.toUpperCase() == 'ME';
    return InteractiveDecoratedBox(
      onTap: action == null ? null : () => action(message),
      child: Row(
        key: Key('message-media-file-${message.id}'),
        mainAxisSize: MainAxisSize.min,
        children: [
          switch (message.mediaStatus.toUpperCase()) {
            'CANCELED' =>
              isCurrentUser && message.mediaUrl?.isNotEmpty == true
                  ? const AttachmentStatusUpload()
                  : const AttachmentStatusDownload(),
            'PENDING' => AttachmentStatusPending(messageId: message.id),
            'EXPIRED' => const AttachmentStatusWarning(),
            _ => Container(
              width: 38,
              height: 38,
              alignment: Alignment.center,
              decoration: BoxDecoration(
                color: context.theme.statusBackground,
                shape: BoxShape.circle,
              ),
              child: Text(
                extension,
                style: const TextStyle(
                  color: Color.fromRGBO(184, 189, 199, 1),
                  fontSize: 12,
                ),
              ),
            ),
          },
          const SizedBox(width: 8),
          Flexible(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  Characters(
                    name,
                  ).replaceAll(Characters(''), Characters('\u200B')).toString(),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    fontSize: context.messageFontSize(14),
                    color: context.theme.text,
                  ),
                ),
                Text(
                  filesize(message.mediaSize),
                  maxLines: 1,
                  style: TextStyle(
                    fontSize: context.messageFontSize(12),
                    color: context.theme.secondaryText,
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

class MessagePost extends StatelessWidget {
  const MessagePost({
    required this.message,
    required this.overlayDateAndStatus,
    this.onOpen,
    this.showStatus = true,
    this.padding,
    this.decoration,
    super.key,
  });

  final MessageListEntry message;
  final Widget overlayDateAndStatus;
  final MessageEntryCallback? onOpen;
  final bool showStatus;
  final EdgeInsetsGeometry? padding;
  final Decoration? decoration;

  @override
  Widget build(BuildContext context) {
    final content = message.content;
    var preview = const LineSplitter().convert(content).take(10).join('\r\n');
    if (preview.length > 1024) preview = preview.substring(0, 1024);
    final fontSize = context.messageFontSize(16);
    return ConstrainedBox(
      key: Key('message-media-post-${message.id}'),
      constraints: const BoxConstraints(maxWidth: 400),
      child: DefaultTextStyle.merge(
        style: TextStyle(fontSize: fontSize),
        child: InteractiveDecoratedBox(
          onTap: onOpen == null ? null : () => onOpen!(message),
          behavior: HitTestBehavior.deferToChild,
          child: Container(
            padding: padding,
            decoration: decoration,
            child: Stack(
              children: [
                ConstrainedBox(
                  constraints: BoxConstraints(
                    minHeight: showStatus ? 48 : 0,
                    minWidth: 128,
                    maxHeight: 400,
                  ),
                  child: ScrollConfiguration(
                    behavior: ScrollConfiguration.of(
                      context,
                    ).copyWith(scrollbars: false),
                    child: SingleChildScrollView(
                      physics: const NeverScrollableScrollPhysics(),
                      child: SelectionArea(
                        contextMenuBuilder: (context, selectableState) =>
                            const SizedBox.shrink(),
                        child: MarkdownBlock(
                          data: preview,
                          selectable: false,
                          config: postMarkdownConfig(
                            context,
                            fontSize: fontSize,
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
                Positioned(
                  right: 0,
                  top: 0,
                  child: Container(
                    decoration: const BoxDecoration(
                      borderRadius: BorderRadius.all(Radius.circular(8)),
                      color: Color.fromRGBO(0, 0, 0, 0.2),
                    ),
                    alignment: Alignment.center,
                    child: SvgPicture.asset(
                      MixinAssets.postDetail,
                      width: 20,
                      height: 20,
                    ),
                  ),
                ),
                if (showStatus)
                  Positioned(
                    right: 0,
                    bottom: 0,
                    child: Container(
                      decoration: const BoxDecoration(
                        borderRadius: BorderRadius.all(Radius.circular(8)),
                        color: Color.fromRGBO(0, 0, 0, 0.2),
                      ),
                      padding: const EdgeInsets.symmetric(
                        horizontal: 6,
                        vertical: 2,
                      ),
                      child: overlayDateAndStatus,
                    ),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _InteractiveMessageCard extends StatelessWidget {
  const _InteractiveMessageCard({
    required this.message,
    required this.onTap,
    required this.child,
  });

  final MessageListEntry message;
  final MessageEntryCallback? onTap;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    if (onTap == null) return child;
    return InteractiveDecoratedBox(onTap: () => onTap!(message), child: child);
  }
}

MessageEntryCallback? _attachmentAction(
  MessageListEntry message, {
  required MessageEntryCallback? onOpen,
  required MessageEntryCallback? onDownload,
  required MessageEntryCallback? onCancel,
}) => switch (message.mediaStatus.toUpperCase()) {
  'CANCELED' => onDownload,
  'PENDING' => onCancel,
  'DONE' => onOpen,
  _ => null,
};

class _UnknownMessage extends StatelessWidget {
  const _UnknownMessage({required this.message, required this.onOpenHelp});

  final MessageListEntry message;
  final VoidCallback? onOpenHelp;

  @override
  Widget build(BuildContext context) {
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    return RichText(
      key: Key('message-media-unknown-${message.id}'),
      text: TextSpan(
        text: l10n?.messageNotSupport ?? 'This message is not supported',
        style: TextStyle(
          fontSize: context.messageFontSize(16),
          color: context.theme.text,
        ),
        children: [
          const TextSpan(text: ' '),
          TextSpan(
            mouseCursor: SystemMouseCursors.click,
            text: l10n?.learnMore ?? 'Learn More',
            style: TextStyle(
              fontSize: context.messageFontSize(16),
              color: context.theme.accent,
            ),
            recognizer: TapGestureRecognizer()..onTap = onOpenHelp,
          ),
        ],
      ),
    );
  }
}

class _MediaImage extends StatelessWidget {
  const _MediaImage({
    required this.message,
    required this.fit,
    this.useMediaUrl = true,
  });

  final MessageListEntry message;
  final BoxFit fit;
  final bool useMediaUrl;

  @override
  Widget build(BuildContext context) {
    final thumbImage = message.thumbImage;
    final isUndownloadedGiphy =
        message.mediaMimeType == 'image/gif' &&
        (message.mediaSize == null || message.mediaSize == 0);
    final thumbnail = isUndownloadedGiphy
        ? _imageForSource(
                thumbImage,
                fit: fit,
                allowEmbeddedData: false,
                placeholder: ColoredBox(color: context.theme.secondaryText),
                errorBuilder: ColoredBox(color: context.theme.secondaryText),
              ) ??
              ColoredBox(color: context.theme.secondaryText)
        : thumbImage == null
        ? const SizedBox()
        : ImageByBlurHashOrBase64(imageData: thumbImage, fit: fit);
    if (!useMediaUrl) return thumbnail;
    return _imageForSource(
          message.mediaUrl,
          fit: fit,
          allowEmbeddedData: false,
          placeholder: thumbnail,
          errorBuilder: thumbnail,
        ) ??
        thumbnail;
  }
}

Widget? _imageForSource(
  String? source, {
  required BoxFit fit,
  required bool allowEmbeddedData,
  required Widget errorBuilder,
  Widget? placeholder,
}) {
  final value = source?.trim();
  if (value == null || value.isEmpty) return null;
  final uri = Uri.tryParse(value);
  ImageProvider<Object>? provider;
  if (uri != null && (uri.scheme == 'http' || uri.scheme == 'https')) {
    provider = NetworkImage(value);
  } else if (uri != null && uri.scheme == 'file') {
    provider = FileImage(File.fromUri(uri));
  } else if (value.startsWith('data:')) {
    final bytes = _decodeImageData(value);
    if (bytes != null) provider = MemoryImage(bytes);
  } else if (File(value).existsSync()) {
    provider = FileImage(File(value));
  } else if (allowEmbeddedData) {
    final bytes = _decodeImageData(value);
    if (bytes != null) provider = MemoryImage(bytes);
  }
  provider ??= FileImage(File(value));
  return MixinImage(
    image: provider,
    fit: fit,
    placeholder: placeholder == null ? null : () => placeholder,
    errorBuilder: (_, _, _) => errorBuilder,
  );
}

Uint8List? _decodeImageData(String value) {
  try {
    final separator = value.indexOf(',');
    if (value.startsWith('data:') && separator < 0) return null;
    final payload = value.startsWith('data:')
        ? value.substring(separator + 1)
        : value;
    return base64Decode(payload);
  } on FormatException {
    return null;
  }
}

class MediaStatusOverlay extends StatelessWidget {
  const MediaStatusOverlay({
    required this.messageId,
    required this.status,
    this.upload = false,
    this.done,
    super.key,
  });

  final String messageId;
  final String status;
  final bool upload;
  final Widget? done;

  @override
  Widget build(BuildContext context) => switch (status.toUpperCase()) {
    'PENDING' => AttachmentStatusPending(messageId: messageId),
    'EXPIRED' => const AttachmentStatusWarning(),
    'CANCELED' =>
      upload
          ? const AttachmentStatusUpload()
          : const AttachmentStatusDownload(),
    _ => done ?? const SizedBox(),
  };
}

class _TimestampPill extends StatelessWidget {
  const _TimestampPill({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => DecoratedBox(
    decoration: const ShapeDecoration(
      color: Color.fromRGBO(0, 0, 0, 0.3),
      shape: StadiumBorder(),
    ),
    child: Padding(
      padding: const EdgeInsets.symmetric(vertical: 3, horizontal: 5),
      child: child,
    ),
  );
}

class _MediaLabel extends StatelessWidget {
  const _MediaLabel({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) => DecoratedBox(
    decoration: const BoxDecoration(
      color: Color.fromRGBO(0, 0, 0, 0.3),
      borderRadius: BorderRadius.all(Radius.circular(5)),
    ),
    child: Padding(
      padding: const EdgeInsets.all(4),
      child: Text(
        text,
        style: const TextStyle(fontSize: 10, color: Colors.white),
      ),
    ),
  );
}

Size _imageMessageSize(
  BuildContext context,
  BoxConstraints constraints, {
  required int? mediaWidth,
  required int? mediaHeight,
}) {
  final sourceWidth = math.max(1, (mediaWidth ?? 1).toDouble());
  final sourceHeight = math.max(1, (mediaHeight ?? 1).toDouble());
  final maxWidth = math.min(constraints.maxWidth * 0.6, 300);
  final minWidth = math.max(constraints.maxWidth * 0.2, 200);
  final width = math.max(
    math.min(sourceWidth / MediaQuery.devicePixelRatioOf(context), maxWidth),
    minWidth,
  );
  final height = math.min(
    width / (sourceWidth / sourceHeight),
    MediaQuery.sizeOf(context).height * 2 / 3,
  );
  return Size(width.toDouble(), height);
}

Size _videoMessageSize(
  BoxConstraints constraints, {
  required int? mediaWidth,
  required int? mediaHeight,
}) {
  const fallback = 200;
  final maxWidth = math.min(constraints.maxWidth * 0.6, 200);
  final width = math.min(mediaWidth ?? fallback, maxWidth).toDouble();
  final scale = (mediaWidth ?? fallback) / (mediaHeight ?? fallback);
  return Size(width, width / scale);
}

bool _needsImageExtendIcon(MessageListEntry message, Size size) {
  final mediaWidth = message.mediaWidth ?? 0;
  final mediaHeight = message.mediaHeight ?? 0;
  if (mediaWidth == 0 || mediaHeight == 0) return false;
  final layoutAspectRatio = size.aspectRatio;
  if (!layoutAspectRatio.isFinite || layoutAspectRatio == 0) return false;
  return layoutAspectRatio - mediaWidth / mediaHeight > 0.01;
}

class _PostDetailIcon extends StatelessWidget {
  const _PostDetailIcon();

  @override
  Widget build(BuildContext context) => Container(
    decoration: const BoxDecoration(
      borderRadius: BorderRadius.all(Radius.circular(8)),
      color: Color.fromRGBO(0, 0, 0, 0.2),
    ),
    alignment: Alignment.center,
    child: SvgPicture.asset(MixinAssets.postDetail, width: 20, height: 20),
  );
}

Size _stickerSize(
  BuildContext context,
  int? width,
  int? height,
  String? assetType,
) {
  final pixelRatio = MediaQuery.devicePixelRatioOf(context);
  final minSize = 60 * pixelRatio;
  final maxSize = 140 * pixelRatio;
  final sourceWidth = width?.toDouble();
  final sourceHeight = height?.toDouble();
  double outputWidth;
  double outputHeight;
  double scale;
  if (sourceWidth == null ||
      sourceHeight == null ||
      sourceWidth <= 0 ||
      sourceHeight <= 0) {
    outputWidth = maxSize;
    outputHeight = maxSize;
    scale = 1;
  } else if (sourceWidth < minSize) {
    scale = minSize / sourceWidth;
    if (scale * sourceHeight > maxSize) {
      outputWidth = maxSize;
      outputHeight = scale * sourceHeight;
    } else {
      outputWidth = scale * sourceWidth;
      outputHeight = scale * sourceHeight;
    }
  } else if (sourceHeight < minSize) {
    scale = minSize / sourceHeight;
    if (scale * sourceWidth > maxSize) {
      outputHeight = maxSize;
      outputWidth = scale * sourceWidth;
    } else {
      outputWidth = scale * sourceWidth;
      outputHeight = scale * sourceHeight;
    }
  } else if (sourceWidth > maxSize || sourceHeight > maxSize) {
    if (sourceWidth > sourceHeight) {
      scale = maxSize / sourceWidth;
      outputWidth = maxSize;
      outputHeight = scale * sourceHeight;
    } else {
      scale = maxSize / sourceHeight;
      outputHeight = maxSize;
      outputWidth = scale * sourceWidth;
    }
  } else {
    outputWidth = sourceWidth;
    outputHeight = sourceHeight;
    scale = 1;
  }
  var size = Size(outputWidth, outputHeight) / pixelRatio;
  if (scale <= 0.5 && MediaQuery.devicePixelRatioOf(context) <= 1.5) {
    if (assetType != 'json' && size.longestSide >= 140) {
      size *= 200 / 140;
    }
  }
  return size;
}

String formatMessageDuration(String value) {
  final milliseconds = int.tryParse(value) ?? 0;
  final duration = Duration(milliseconds: math.max(0, milliseconds));
  final normalized = duration < const Duration(seconds: 1)
      ? const Duration(seconds: 1)
      : duration;
  return '${normalized.inMinutes.toString().padLeft(2, '0')}:'
      '${normalized.inSeconds.remainder(60).toString().padLeft(2, '0')}';
}

String _fileExtension(String name) {
  if (lookupMimeType(name) == null) return 'FILE';
  final extension = p.extension(name).trim().replaceFirst('.', '');
  return extension.isEmpty ? 'FILE' : extension.toUpperCase();
}
