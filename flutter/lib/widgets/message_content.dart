import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:markdown_widget/markdown_widget.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/message_audio.dart';
import 'package:mixin_desktop_ui/widgets/message_layout.dart';
import 'package:mixin_desktop_ui/widgets/message_items/special_message_items.dart';
import 'package:mixin_desktop_ui/widgets/message_selectable_text.dart';
import 'package:mixin_desktop_ui/widgets/post_markdown.dart';
import 'package:lottie/lottie.dart';
import 'package:provider/provider.dart';

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
    this.onOpenTranscript,
    this.onOpenSnapshot,
    this.onOpenImage,
    this.onOpenVideo,
    this.onOpenPost,
    this.onOpenFile,
    this.onOpenIdentityNumber,
    this.onMarkAudioRead,
    this.onDownloadAttachment,
    this.onCancelAttachment,
    this.currentUserId,
    this.mentionNames = const {},
    this.showNip = false,
    this.highlighted = false,
  });

  final MessageListEntry message;
  final bool isCurrentUser;
  final Widget dateAndStatus;
  final Widget overlayDateAndStatus;
  final MessageUriCallback? onOpenUri;
  final MessageStringCallback? onOpenUser;
  final MessageStringCallback? onOpenMessage;
  final MessageStringCallback? onAction;
  final MessageStringCallback? onOpenTranscript;
  final MessageStringCallback? onOpenSnapshot;
  final MessageEntryCallback? onOpenImage;
  final MessageEntryCallback? onOpenVideo;
  final MessageEntryCallback? onOpenPost;
  final MessageEntryCallback? onOpenFile;
  final MessageStringCallback? onOpenIdentityNumber;
  final MessageEntryCallback? onMarkAudioRead;
  final MessageEntryCallback? onDownloadAttachment;
  final MessageEntryCallback? onCancelAttachment;
  final String? currentUserId;
  final Map<String, String> mentionNames;
  final bool showNip;
  final bool highlighted;

  @override
  Widget build(BuildContext context) {
    final status = message.status.toUpperCase();
    if (status == 'UNKNOWN') {
      return MessageLayout(
        spacing: 6,
        content: _UnknownMessage(message: message),
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
      return PinMessageItem(message: message);
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
            child: Text(
              isCurrentUser
                  ? l10n?.youDeletedThisMessage ?? 'You deleted this message'
                  : l10n?.thisMessageWasDeleted ?? 'This message was deleted',
              style: TextStyle(
                fontSize: context.messageFontSize(16),
                color: context.theme.text,
              ),
            ),
          ),
        ],
      );
    }

    Widget content;
    if (message.category.endsWith('_TRANSCRIPT')) {
      content = TranscriptMessageItem(
        message: message,
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
      content = AppButtonGroupMessageItem(message: message, onAction: onAction);
    } else if (message.category == 'APP_CARD') {
      content = AppCardMessageItem(
        message: message,
        onAction: onAction,
        isCurrentUser: isCurrentUser,
        showNip: showNip,
        highlighted: highlighted,
        dateAndStatus: dateAndStatus,
      );
    } else {
      content = _standardContent();
    }
    final quote = message.quoteContent;
    if (quote == null || quote.isEmpty) return content;
    return IntrinsicWidth(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SizedBox(
            width: message.isImage || message.isVideo ? 0 : null,
            child: QuoteMessagePreview(
              raw: quote,
              messageId: message.quoteMessageId,
              onOpenMessage: onOpenMessage,
            ),
          ),
          content,
        ],
      ),
    );
  }

  Widget _standardContent() {
    if (message.isImage) {
      return _ImageMessage(
        message: message,
        dateAndStatus: dateAndStatus,
        overlayDateAndStatus: overlayDateAndStatus,
        onOpen: onOpenImage,
        onDownloadAttachment: onDownloadAttachment,
        onCancelAttachment: onCancelAttachment,
        onOpenUri: onOpenUri,
        onOpenIdentityNumber: onOpenIdentityNumber,
        mentionNames: mentionNames,
      );
    }
    if (message.isVideo) {
      return _VideoMessage(
        message: message,
        dateAndStatus: dateAndStatus,
        overlayDateAndStatus: overlayDateAndStatus,
        onOpen: onOpenVideo,
        onDownloadAttachment: onDownloadAttachment,
        onCancelAttachment: onCancelAttachment,
        onOpenUri: onOpenUri,
        onOpenIdentityNumber: onOpenIdentityNumber,
        mentionNames: mentionNames,
      );
    }
    if (message.isAudio) {
      return AudioMessageWidget(
        message: message,
        onMarkRead: onMarkAudioRead,
        onDownloadAttachment: onDownloadAttachment,
        onCancelAttachment: onCancelAttachment,
      );
    }
    if (message.isSticker) {
      return _StickerMessage(message: message);
    }
    if (message.category.endsWith('_DATA')) {
      return _FileMessage(
        message: message,
        onOpen: onOpenFile,
        onDownloadAttachment: onDownloadAttachment,
        onCancelAttachment: onCancelAttachment,
      );
    }
    if (message.isPost) {
      return MessageLayout(
        spacing: 6,
        content: _PostMessage(message: message, onOpen: onOpenPost),
        dateAndStatus: dateAndStatus,
      );
    }
    if (message.isText) {
      return _TextMessage(
        message: message,
        dateAndStatus: dateAndStatus,
        onOpenUri: onOpenUri,
        onOpenIdentityNumber: onOpenIdentityNumber,
        mentionNames: mentionNames,
      );
    }
    return MessageLayout(
      spacing: 6,
      content: _UnknownMessage(message: message),
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
  });

  final MessageListEntry message;
  final Widget dateAndStatus;
  final MessageUriCallback? onOpenUri;
  final MessageStringCallback? onOpenIdentityNumber;
  final Map<String, String> mentionNames;

  @override
  Widget build(BuildContext context) => MessageLayout(
    spacing: 6,
    content: SelectableMessageText(
      content: message.content,
      onOpenUri: onOpenUri,
      onOpenIdentityNumber: onOpenIdentityNumber,
      mentionNames: mentionNames,
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
    required this.dateAndStatus,
    required this.overlayDateAndStatus,
    this.onOpen,
    this.onDownloadAttachment,
    this.onCancelAttachment,
    this.onOpenUri,
    this.onOpenIdentityNumber,
    this.mentionNames = const {},
  });

  final MessageListEntry message;
  final Widget dateAndStatus;
  final Widget overlayDateAndStatus;
  final MessageEntryCallback? onOpen;
  final MessageEntryCallback? onDownloadAttachment;
  final MessageEntryCallback? onCancelAttachment;
  final MessageUriCallback? onOpenUri;
  final MessageStringCallback? onOpenIdentityNumber;
  final Map<String, String> mentionNames;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final size = _mediaSize(
        context,
        constraints,
        mediaWidth: message.mediaWidth,
        mediaHeight: message.mediaHeight,
        maxWidth: 300,
        minimumWidth: 200,
      );
      final caption = message.caption?.trim() ?? '';
      final image = _InteractiveMessageCard(
        message: message,
        onTap: _attachmentAction(
          message,
          onOpen: onOpen,
          onDownload: onDownloadAttachment,
          onCancel: onCancelAttachment,
        ),
        child: ClipRRect(
          borderRadius: const BorderRadius.all(Radius.circular(8)),
          child: SizedBox.fromSize(
            size: size,
            child: Stack(
              fit: StackFit.expand,
              children: [
                _MediaImage(
                  message: message,
                  fit: BoxFit.cover,
                  placeholder: _MediaPlaceholder(category: message.category),
                ),
                _MediaStatusOverlay(status: message.mediaStatus),
                if (caption.isEmpty)
                  Positioned(
                    right: 4,
                    bottom: 4,
                    child: _TimestampPill(child: overlayDateAndStatus),
                  ),
              ],
            ),
          ),
        ),
      );
      return ConstrainedBox(
        key: Key('message-media-image-${message.id}'),
        constraints: BoxConstraints(maxWidth: size.width),
        child: caption.isEmpty
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
                    child: MessageLayout(
                      spacing: 6,
                      content: SelectableMessageText(
                        content: caption,
                        onOpenUri: onOpenUri,
                        onOpenIdentityNumber: onOpenIdentityNumber,
                        mentionNames: mentionNames,
                        style: TextStyle(
                          color: context.theme.text,
                          fontSize: context.messageFontSize(16),
                        ),
                      ),
                      dateAndStatus: dateAndStatus,
                    ),
                  ),
                ],
              ),
      );
    },
  );
}

class _VideoMessage extends StatelessWidget {
  const _VideoMessage({
    required this.message,
    required this.dateAndStatus,
    required this.overlayDateAndStatus,
    this.onOpen,
    this.onDownloadAttachment,
    this.onCancelAttachment,
    this.onOpenUri,
    this.onOpenIdentityNumber,
    this.mentionNames = const {},
  });

  final MessageListEntry message;
  final Widget dateAndStatus;
  final Widget overlayDateAndStatus;
  final MessageEntryCallback? onOpen;
  final MessageEntryCallback? onDownloadAttachment;
  final MessageEntryCallback? onCancelAttachment;
  final MessageUriCallback? onOpenUri;
  final MessageStringCallback? onOpenIdentityNumber;
  final Map<String, String> mentionNames;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final size = _mediaSize(
        context,
        constraints,
        mediaWidth: message.mediaWidth,
        mediaHeight: message.mediaHeight,
        maxWidth: 200,
        minimumWidth: 120,
      );
      final caption = message.caption?.trim() ?? '';
      final preview = _InteractiveMessageCard(
        message: message,
        onTap: _attachmentAction(
          message,
          onOpen: onOpen,
          onDownload: onDownloadAttachment,
          onCancel: onCancelAttachment,
        ),
        child: ClipRRect(
          borderRadius: const BorderRadius.all(Radius.circular(8)),
          child: SizedBox.fromSize(
            size: size,
            child: Stack(
              fit: StackFit.expand,
              children: [
                _MediaImage(
                  message: message,
                  useMediaUrl: false,
                  fit: BoxFit.cover,
                  placeholder: _MediaPlaceholder(category: message.category),
                ),
                Center(
                  child: _MediaStatusOverlay(
                    status: message.mediaStatus,
                    done: _StaticMediaIcon(category: message.category),
                  ),
                ),
                Positioned(
                  top: 6,
                  left: 6,
                  child: _MediaLabel(
                    text: _formatDuration(message.mediaDuration),
                  ),
                ),
                if (caption.isEmpty)
                  Positioned(
                    right: 4,
                    bottom: 4,
                    child: _TimestampPill(child: overlayDateAndStatus),
                  ),
              ],
            ),
          ),
        ),
      );
      return ConstrainedBox(
        key: Key('message-media-video-${message.id}'),
        constraints: BoxConstraints(maxWidth: size.width),
        child: caption.isEmpty
            ? preview
            : Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  preview,
                  Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 10,
                    ),
                    child: MessageLayout(
                      spacing: 6,
                      content: SelectableMessageText(
                        content: caption,
                        onOpenUri: onOpenUri,
                        onOpenIdentityNumber: onOpenIdentityNumber,
                        mentionNames: mentionNames,
                        style: TextStyle(
                          color: context.theme.text,
                          fontSize: context.messageFontSize(16),
                        ),
                      ),
                      dateAndStatus: dateAndStatus,
                    ),
                  ),
                ],
              ),
      );
    },
  );
}

class _StickerMessage extends StatelessWidget {
  const _StickerMessage({required this.message});

  final MessageListEntry message;

  @override
  Widget build(BuildContext context) {
    final size = _stickerSize(
      context,
      message.stickerAssetWidth ?? message.mediaWidth,
      message.stickerAssetHeight ?? message.mediaHeight,
    );
    final placeholder = _MediaPlaceholder(category: message.category);
    final assetUrl = message.stickerAssetUrl ?? message.mediaUrl;
    final sticker = assetUrl == null || assetUrl.isEmpty
        ? null
        : _StickerAsset(
            source: assetUrl,
            assetType: message.stickerAssetType,
            placeholder: placeholder,
          );
    return SizedBox.fromSize(
      key: Key('message-media-sticker-${message.id}'),
      size: size,
      child: sticker ?? placeholder,
    );
  }
}

class _StickerAsset extends StatelessWidget {
  const _StickerAsset({
    required this.source,
    required this.assetType,
    required this.placeholder,
  });

  final String source;
  final String? assetType;
  final Widget placeholder;

  @override
  Widget build(BuildContext context) {
    if (assetType?.toLowerCase() == 'json') {
      final uri = Uri.tryParse(source);
      if (uri != null && (uri.scheme == 'http' || uri.scheme == 'https')) {
        return Lottie.network(
          source,
          fit: BoxFit.contain,
          repeat: true,
          errorBuilder: (_, _, _) => placeholder,
        );
      }
      final file = uri?.scheme == 'file' ? File.fromUri(uri!) : File(source);
      if (file.existsSync()) {
        return Lottie.file(
          file,
          fit: BoxFit.contain,
          repeat: true,
          errorBuilder: (_, _, _) => placeholder,
        );
      }
      return placeholder;
    }
    return _imageForSource(
          source,
          fit: BoxFit.contain,
          allowEmbeddedData: false,
          errorBuilder: placeholder,
        ) ??
        placeholder;
  }
}

class _FileMessage extends StatelessWidget {
  const _FileMessage({
    required this.message,
    this.onOpen,
    this.onDownloadAttachment,
    this.onCancelAttachment,
  });

  final MessageListEntry message;
  final MessageEntryCallback? onOpen;
  final MessageEntryCallback? onDownloadAttachment;
  final MessageEntryCallback? onCancelAttachment;

  @override
  Widget build(BuildContext context) {
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    final name = _mediaName(message, l10n?.file ?? 'File');
    final extension = _fileExtension(name, message.mediaMimeType);
    final action = _attachmentAction(
      message,
      onOpen: onOpen,
      onDownload: onDownloadAttachment,
      onCancel: onCancelAttachment,
    );
    return _InteractiveMessageCard(
      message: message,
      onTap: action,
      child: ConstrainedBox(
        key: Key('message-media-file-${message.id}'),
        constraints: const BoxConstraints(maxWidth: 300, minWidth: 180),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Stack(
              alignment: Alignment.center,
              children: [
                Container(
                  width: 38,
                  height: 38,
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    color: context.theme.statusBackground,
                    shape: BoxShape.circle,
                  ),
                  child: Text(
                    extension,
                    maxLines: 1,
                    overflow: TextOverflow.clip,
                    style: TextStyle(
                      color: context.theme.secondaryText,
                      fontSize: extension.length > 4 ? 9 : 11,
                    ),
                  ),
                ),
                _MediaStatusOverlay(status: message.mediaStatus),
              ],
            ),
            const SizedBox(width: 8),
            Flexible(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    name,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      fontSize: context.messageFontSize(14),
                      color: context.theme.text,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    _formatBytes(message.mediaSize),
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
      ),
    );
  }
}

class _PostMessage extends StatelessWidget {
  const _PostMessage({required this.message, this.onOpen});

  final MessageListEntry message;
  final MessageEntryCallback? onOpen;

  @override
  Widget build(BuildContext context) {
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    final content = message.content.trim();
    final preview = const LineSplitter()
        .convert(content.isEmpty ? l10n?.post ?? 'Post' : content)
        .take(10)
        .join('\r\n');
    final fontSize = context.messageFontSize(16);
    return _InteractiveMessageCard(
      message: message,
      onTap: onOpen,
      child: ConstrainedBox(
        key: Key('message-media-post-${message.id}'),
        constraints: const BoxConstraints(
          maxWidth: 400,
          minWidth: 128,
          maxHeight: 400,
        ),
        child: ScrollConfiguration(
          behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false),
          child: SingleChildScrollView(
            physics: const NeverScrollableScrollPhysics(),
            child: DefaultTextStyle.merge(
              style: TextStyle(fontSize: fontSize, color: context.theme.text),
              child: SelectionArea(
                contextMenuBuilder: (context, selectableState) =>
                    const SizedBox.shrink(),
                child: MarkdownBlock(
                  data: preview,
                  selectable: false,
                  config: postMarkdownConfig(context, fontSize: fontSize),
                ),
              ),
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
    return MouseRegion(
      cursor: SystemMouseCursors.click,
      child: GestureDetector(
        behavior: HitTestBehavior.deferToChild,
        onTap: () => onTap!(message),
        child: child,
      ),
    );
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
  'DONE' || 'READ' => onOpen,
  _ => null,
};

class _UnknownMessage extends StatelessWidget {
  const _UnknownMessage({required this.message});

  final MessageListEntry message;

  @override
  Widget build(BuildContext context) {
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    return Text(
      key: Key('message-media-unknown-${message.id}'),
      l10n?.messageNotSupport ?? 'This message is not supported',
      style: TextStyle(
        fontSize: context.messageFontSize(16),
        color: context.theme.text,
      ),
    );
  }
}

class _MediaImage extends StatelessWidget {
  const _MediaImage({
    required this.message,
    required this.placeholder,
    required this.fit,
    this.useMediaUrl = true,
  });

  final MessageListEntry message;
  final Widget placeholder;
  final BoxFit fit;
  final bool useMediaUrl;

  @override
  Widget build(BuildContext context) {
    final thumbnail = _imageForSource(
      message.thumbImage,
      fit: fit,
      allowEmbeddedData: true,
      errorBuilder: placeholder,
    );
    final fallback = thumbnail ?? placeholder;
    if (!useMediaUrl) return fallback;
    return _imageForSource(
          message.mediaUrl,
          fit: fit,
          allowEmbeddedData: false,
          errorBuilder: fallback,
        ) ??
        fallback;
  }
}

Widget? _imageForSource(
  String? source, {
  required BoxFit fit,
  required bool allowEmbeddedData,
  required Widget errorBuilder,
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
  return Image(
    key: ValueKey('media-image-$value'),
    image: provider,
    fit: fit,
    filterQuality: FilterQuality.medium,
    gaplessPlayback: true,
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

class _MediaPlaceholder extends StatelessWidget {
  const _MediaPlaceholder({required this.category});

  final String category;

  @override
  Widget build(BuildContext context) {
    final asset = MixinAssets.messageIcon(category);
    return ColoredBox(
      color: context.theme.stickerPlaceholderColor,
      child: Center(
        child: asset == null
            ? Icon(
                Icons.broken_image_outlined,
                color: context.theme.secondaryText,
              )
            : SvgPicture.asset(
                asset,
                width: 28,
                height: 28,
                colorFilter: ColorFilter.mode(
                  context.theme.secondaryText,
                  BlendMode.srcIn,
                ),
              ),
      ),
    );
  }
}

class _StaticMediaIcon extends StatelessWidget {
  const _StaticMediaIcon({required this.category});

  final String category;

  @override
  Widget build(BuildContext context) {
    final asset = MixinAssets.messageIcon(category);
    return ExcludeSemantics(
      child: Container(
        key: ValueKey('static-media-icon-$category'),
        width: 38,
        height: 38,
        alignment: Alignment.center,
        decoration: const BoxDecoration(
          color: Color.fromRGBO(0, 0, 0, 0.3),
          shape: BoxShape.circle,
        ),
        child: asset == null
            ? const SizedBox()
            : SvgPicture.asset(
                asset,
                width: 22,
                height: 22,
                colorFilter: const ColorFilter.mode(
                  Colors.white,
                  BlendMode.srcIn,
                ),
              ),
      ),
    );
  }
}

class _MediaStatusOverlay extends StatelessWidget {
  const _MediaStatusOverlay({required this.status, this.done});

  final String status;
  final Widget? done;

  @override
  Widget build(BuildContext context) => switch (status.toUpperCase()) {
    'PENDING' => const Center(
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: Color.fromRGBO(0, 0, 0, 0.3),
          shape: BoxShape.circle,
        ),
        child: SizedBox.square(
          dimension: 38,
          child: Padding(
            padding: EdgeInsets.all(6),
            child: CircularProgressIndicator(
              strokeWidth: 2,
              color: Colors.white,
            ),
          ),
        ),
      ),
    ),
    'EXPIRED' => const Center(
      child: CircleAvatar(
        radius: 19,
        backgroundColor: Color.fromRGBO(0, 0, 0, 0.3),
        child: Icon(Icons.warning_amber_rounded, color: Colors.white, size: 28),
      ),
    ),
    'CANCELED' => const Center(
      child: CircleAvatar(
        radius: 19,
        backgroundColor: Color.fromRGBO(0, 0, 0, 0.3),
        child: Icon(Icons.cloud_off_outlined, color: Colors.white, size: 26),
      ),
    ),
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

Size _mediaSize(
  BuildContext context,
  BoxConstraints constraints, {
  required int? mediaWidth,
  required int? mediaHeight,
  required double maxWidth,
  required double minimumWidth,
}) {
  final availableWidth = constraints.maxWidth.isFinite
      ? constraints.maxWidth
      : maxWidth;
  final boundedMaxWidth = math.max(1.0, math.min(availableWidth, maxWidth));
  final boundedMinimumWidth = math.min(minimumWidth, boundedMaxWidth);
  final pixelRatio = MediaQuery.devicePixelRatioOf(context);
  final sourceWidth = (mediaWidth ?? minimumWidth).toDouble();
  final sourceHeight = (mediaHeight ?? minimumWidth).toDouble();
  final width = (sourceWidth / pixelRatio).clamp(
    boundedMinimumWidth,
    boundedMaxWidth,
  );
  final aspectRatio = sourceWidth > 0 && sourceHeight > 0
      ? sourceWidth / sourceHeight
      : 1.0;
  final maxHeight = MediaQuery.sizeOf(context).height * 2 / 3;
  final height = (width / aspectRatio).clamp(60.0, math.max(60.0, maxHeight));
  return Size(width.toDouble(), height.toDouble());
}

Size _stickerSize(BuildContext context, int? width, int? height) {
  const maxSide = 140.0;
  const minSide = 60.0;
  final sourceWidth = (width ?? 0).toDouble();
  final sourceHeight = (height ?? 0).toDouble();
  if (sourceWidth <= 0 || sourceHeight <= 0) {
    return const Size.square(maxSide);
  }
  final ratio = sourceWidth / sourceHeight;
  var size = ratio >= 1
      ? Size(maxSide, (maxSide / ratio).clamp(minSide, maxSide))
      : Size((maxSide * ratio).clamp(minSide, maxSide), maxSide);
  final scale = maxSide / math.max(sourceWidth, sourceHeight);
  if (scale <= 0.5 && MediaQuery.devicePixelRatioOf(context) <= 1.5) {
    size *= 200 / maxSide;
  }
  return size;
}

String _formatDuration(String value) {
  final milliseconds = int.tryParse(value) ?? 0;
  final duration = Duration(milliseconds: math.max(0, milliseconds));
  final hours = duration.inHours;
  final minutes = duration.inMinutes.remainder(60);
  final seconds = duration.inSeconds.remainder(60);
  if (hours > 0) {
    return '$hours:${minutes.toString().padLeft(2, '0')}:'
        '${seconds.toString().padLeft(2, '0')}';
  }
  return '$minutes:${seconds.toString().padLeft(2, '0')}';
}

String _mediaName(MessageListEntry message, String fallback) {
  final content = message.content.trim();
  if (content.isNotEmpty) {
    try {
      final decoded = jsonDecode(content);
      if (decoded is Map<String, dynamic>) {
        for (final key in const ['name', 'filename', 'file_name']) {
          final value = decoded[key];
          if (value is String && value.trim().isNotEmpty) return value.trim();
        }
      }
    } on FormatException {
      if (!content.startsWith('{') && !content.startsWith('[')) return content;
    }
  }
  final mediaUrl = message.mediaUrl?.trim();
  if (mediaUrl != null && mediaUrl.isNotEmpty) {
    final uri = Uri.tryParse(mediaUrl);
    final path = (uri?.path.isNotEmpty ?? false) ? uri!.path : mediaUrl;
    final normalized = path.replaceAll('\\', '/');
    final name = normalized.split('/').last;
    if (name.isNotEmpty) {
      try {
        return Uri.decodeComponent(name);
      } on FormatException {
        return name;
      }
    }
  }
  return fallback;
}

String _fileExtension(String name, String? mimeType) {
  final index = name.lastIndexOf('.');
  if (index >= 0 && index + 1 < name.length) {
    final value = name.substring(index + 1).trim();
    if (value.isNotEmpty && value.length <= 6) return value.toUpperCase();
  }
  final subtype = mimeType?.split('/').last.trim() ?? '';
  if (subtype.isNotEmpty && subtype.length <= 6) return subtype.toUpperCase();
  return 'FILE';
}

String _formatBytes(int? value) {
  if (value == null || value <= 0) return '';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  var size = value.toDouble();
  var unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit++;
  }
  final digits = unit == 0 || size >= 10 ? 0 : 1;
  return '${size.toStringAsFixed(digits)} ${units[unit]}';
}
