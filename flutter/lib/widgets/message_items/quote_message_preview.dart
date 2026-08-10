import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';

import '../../constants/assets.dart';
import '../../l10n/generated/app_localizations.dart';
import '../../l10n/l10n.dart';
import '../../models/message_list_entry.dart';
import '../../theme.dart';
import '../../utils/name_color.dart';
import '../avatar_view.dart';
import '../badges_widget.dart';
import '../high_light_text.dart';
import '../image_by_blur_hash.dart';
import '../message_media_preview_pages.dart';
import '../message_navigation_scope.dart';
import '../message_selectable_text.dart';
import '../message_style.dart';
import '../mixin_image.dart';
import '../sticker_page/sticker_item.dart';
import 'message_preview_utils.dart';

class QuoteMessagePreview extends StatelessWidget {
  const QuoteMessagePreview({
    required this.raw,
    required this.messageId,
    required this.membership,
    required this.mentionNames,
    this.sourceMessageId,
    super.key,
  });

  final String raw;
  final String? messageId;
  final String? sourceMessageId;
  final String? membership;
  final Map<String, String> mentionNames;

  @override
  Widget build(BuildContext context) {
    final quote = _QuoteData.parse(raw);
    if (quote == null) {
      return _QuoteMessageBase(
        description: context.l10n.messageNotFound,
        icon: SvgPicture.asset(
          MixinAssets.recall,
          colorFilter: ColorFilter.mode(
            context.theme.secondaryText,
            BlendMode.srcIn,
          ),
        ),
      );
    }
    if (!quote.supported) {
      return _QuoteMessageBase(
        description: context.l10n.messageNotSupport,
        icon: SvgPicture.asset(
          MixinAssets.recall,
          colorFilter: ColorFilter.mode(
            context.theme.secondaryText,
            BlendMode.srcIn,
          ),
        ),
      );
    }
    final image = _quoteImage(quote);
    final iconAsset = MixinAssets.messageIcon(quote.category);
    return _QuoteMessageBase(
      senderId: quote.senderId,
      sender: quote.sender,
      membership: membership,
      description: quote.preview(context.l10n, mentionNames),
      onTap: messageId == null
          ? null
          : () =>
                MessageNavigationScope.maybeOf(
                  context,
                )?.onLocateMessage(
                  messageId!,
                  sourceMessageId: sourceMessageId,
                ),
      icon: iconAsset == null
          ? null
          : SvgPicture.asset(
              iconAsset,
              width: 16,
              height: 16,
              colorFilter: ColorFilter.mode(
                context.theme.secondaryText,
                BlendMode.srcIn,
              ),
            ),
      image: image,
    );
  }

  Widget? _quoteImage(_QuoteData quote) {
    if (quote.category.endsWith('_CONTACT')) {
      return Padding(
        padding: const EdgeInsets.all(6),
        child: AvatarView(
          userId: quote.sharedUserId ?? '',
          name: quote.sharedUserName ?? '',
          avatarUrl: quote.sharedUserAvatarUrl ?? '',
          size: 48,
        ),
      );
    }
    if (quote.category.endsWith('_STICKER')) {
      return StickerItem(
        stickerId: quote.stickerId,
        assetUrl: quote.stickerAssetUrl ?? '',
        assetType: quote.stickerAssetType,
      );
    }
    if (quote.category.endsWith('_IMAGE')) {
      return _QuoteImage(quote: quote);
    }
    if (quote.category.endsWith('_VIDEO')) {
      final thumbnail = quote.thumbImage;
      return thumbnail == null
          ? const SizedBox()
          : ImageByBlurHashOrBase64(imageData: thumbnail);
    }
    if (quote.category.endsWith('_LIVE')) {
      final thumbnail = quote.thumbImage;
      final placeholder = thumbnail == null
          ? const SizedBox()
          : ImageByBlurHashOrBase64(imageData: thumbnail);
      return MixinImage.network(
        quote.thumbUrl ?? '',
        placeholder: () => placeholder,
        errorBuilder: (_, _, _) => placeholder,
      );
    }
    return null;
  }
}

class _QuoteImage extends StatelessWidget {
  const _QuoteImage({required this.quote});

  final _QuoteData quote;

  @override
  Widget build(BuildContext context) {
    final source = quote.mediaUrl;
    final provider = source == null ? null : imageProviderForSource(source);
    final thumbnail = quote.thumbImage;
    final fallback = thumbnail == null
        ? const SizedBox()
        : ImageByBlurHashOrBase64(imageData: thumbnail);
    if (provider == null) return fallback;
    return MixinImage(image: provider, errorBuilder: (_, _, _) => fallback);
  }
}

Widget? buildMessageQuotePreview(
  MessageListEntry message, {
  Map<String, String> mentionNames = const {},
}) {
  final messageId = message.quoteMessageId;
  if (messageId == null || messageId.isEmpty) return null;
  return QuoteMessagePreview(
    // Keep the quote surface while its local payload is unavailable, matching
    // flutter-app's "Message not found" fallback instead of flattening the
    // reply into an ordinary bubble.
    raw: message.quoteContent ?? '',
    messageId: messageId,
    sourceMessageId: message.id,
    membership: message.quoteUserMembership,
    mentionNames: mentionNames,
  );
}

class _QuoteMessageBase extends StatelessWidget {
  const _QuoteMessageBase({
    required this.description,
    this.senderId,
    this.sender,
    this.membership,
    this.icon,
    this.image,
    this.onTap,
  });

  final String? senderId;
  final String? sender;
  final String? membership;
  final String description;
  final Widget? icon;
  final Widget? image;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final lines = const LineSplitter().convert(description);
    final preview = lines.isEmpty
        ? ''
        : '${lines.first}${lines.length > 1 ? '...' : ''}';
    final color = nameColorForId(senderId) ?? context.theme.accent;

    return ClipRRect(
      borderRadius: const BorderRadius.all(Radius.circular(8)),
      child: GestureDetector(
        onTap: onTap,
        behavior: HitTestBehavior.opaque,
        child: Container(
          key: const Key('quote-message-preview'),
          constraints: const BoxConstraints(minHeight: 50),
          color: const Color.fromRGBO(0, 0, 0, 0.04),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Flexible(
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      key: const Key('quote-message-accent'),
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
                            if (sender != null)
                              Padding(
                                padding: const EdgeInsets.only(bottom: 4),
                                child: Row(
                                  mainAxisSize: MainAxisSize.min,
                                  children: [
                                    CustomText(
                                      sender!,
                                      maxLines: 1,
                                      style: TextStyle(
                                        color: color,
                                        fontSize: context
                                            .messageStyle
                                            .secondaryFontSize,
                                        height: 1,
                                      ),
                                    ),
                                    BadgesWidget(
                                      verified: false,
                                      isBot: false,
                                      membership: membership,
                                    ),
                                  ],
                                ),
                              ),
                            Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                if (icon != null)
                                  Padding(
                                    padding: const EdgeInsets.only(right: 4),
                                    child: icon,
                                  ),
                                Flexible(
                                  child: CustomText(
                                    preview,
                                    maxLines: 1,
                                    overflow: TextOverflow.ellipsis,
                                    style: TextStyle(
                                      color: context.theme.secondaryText,
                                      fontSize:
                                          context.messageStyle.tertiaryFontSize,
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
                SizedBox(
                  key: const Key('quote-message-image'),
                  width: 48,
                  height: 48,
                  child: RepaintBoundary(
                    child: ClipRRect(
                      borderRadius: const BorderRadius.all(Radius.circular(6)),
                      child: image,
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

class _QuoteData {
  const _QuoteData({
    required this.senderId,
    required this.sender,
    required this.category,
    required this.content,
    required this.mediaName,
    required this.mediaUrl,
    required this.thumbUrl,
    required this.thumbImage,
    required this.stickerAssetUrl,
    required this.stickerAssetType,
    required this.stickerId,
    required this.sharedUserId,
    required this.sharedUserName,
    required this.sharedUserAvatarUrl,
    required this.sharedUserIdentityNumber,
  });

  final String? senderId;
  final String? sender;
  final String category;
  final String content;
  final String? mediaName;
  final String? mediaUrl;
  final String? thumbUrl;
  final String? thumbImage;
  final String? stickerAssetUrl;
  final String? stickerAssetType;
  final String? stickerId;
  final String? sharedUserId;
  final String? sharedUserName;
  final String? sharedUserAvatarUrl;
  final String? sharedUserIdentityNumber;

  bool get supported =>
      category.endsWith('_TEXT') ||
      category.endsWith('_IMAGE') ||
      category.endsWith('_VIDEO') ||
      category.endsWith('_LIVE') ||
      category.endsWith('_AUDIO') ||
      category.endsWith('_DATA') ||
      category.endsWith('_STICKER') ||
      category.endsWith('_CONTACT') ||
      category.endsWith('_LOCATION') ||
      category.endsWith('_TRANSCRIPT') ||
      category.endsWith('_POST') ||
      category == 'APP_CARD' ||
      category == 'APP_BUTTON_GROUP';

  String preview(AppLocalizations l10n, Map<String, String> mentionNames) {
    if (category.endsWith('_TEXT')) {
      return replaceMessageMentions(content, mentionNames);
    }
    if (category.endsWith('_POST')) return postOptimizeMarkdown(content);
    if (category.endsWith('_IMAGE')) return l10n.image;
    if (category.endsWith('_VIDEO')) return l10n.video;
    if (category.endsWith('_LIVE')) return l10n.live;
    if (category.endsWith('_AUDIO')) return l10n.audio;
    if (category.endsWith('_DATA')) {
      return mediaName?.isNotEmpty == true ? mediaName! : l10n.file;
    }
    if (category.endsWith('_STICKER')) return l10n.sticker;
    if (category.endsWith('_CONTACT')) {
      return sharedUserIdentityNumber?.isNotEmpty == true
          ? sharedUserIdentityNumber!
          : (sharedUserName ?? l10n.contact);
    }
    if (category.endsWith('_LOCATION')) return l10n.location;
    if (category.endsWith('_TRANSCRIPT')) {
      return l10n.transcript;
    }
    if (category == 'APP_CARD') {
      try {
        final card = jsonDecode(content) as Map<String, dynamic>;
        return card['title']?.toString() ?? l10n.bots;
      } on Object {
        return l10n.bots;
      }
    }
    if (category == 'APP_BUTTON_GROUP') {
      try {
        final buttons = jsonDecode(content) as List<dynamic>;
        return buttons
            .map((item) => '[${(item as Map<String, dynamic>)['label'] ?? ''}]')
            .join();
      } on Object {
        return l10n.bots;
      }
    }
    return l10n.messageNotSupport;
  }

  static _QuoteData? parse(String raw) {
    try {
      final json = jsonDecode(raw) as Map<String, dynamic>;
      return _QuoteData(
        senderId: nonEmpty(json['user_id']),
        sender: json['user_full_name']?.toString(),
        category: json['type']?.toString() ?? '',
        content: json['content']?.toString() ?? '',
        mediaName: nonEmpty(json['media_name']),
        mediaUrl: nonEmpty(json['media_url']),
        thumbUrl: nonEmpty(json['thumb_url']),
        thumbImage: nonEmpty(json['thumb_image']),
        stickerAssetUrl: nonEmpty(json['asset_url']),
        stickerAssetType: nonEmpty(json['asset_type']),
        stickerId: nonEmpty(json['sticker_id']),
        sharedUserId: nonEmpty(json['shared_user_id']),
        sharedUserName: nonEmpty(json['shared_user_full_name']),
        sharedUserAvatarUrl: nonEmpty(json['shared_user_avatar_url']),
        sharedUserIdentityNumber: nonEmpty(
          json['shared_user_identity_number'],
        ),
      );
    } on Object {
      return null;
    }
  }
}
