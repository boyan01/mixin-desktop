import 'dart:convert';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:auto_size_text/auto_size_text.dart';
import 'package:decimal/decimal.dart';
import 'package:decimal/intl.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:hexagon/hexagon.dart';
import 'package:http/http.dart' as http;
import 'package:intl/intl.dart' as intl;
import 'package:latlng/latlng.dart';
import 'package:map/map.dart' as map;
import 'package:markdown/markdown.dart' as markdown;
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/utils/name_color.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/badges_widget.dart';
import 'package:mixin_desktop_ui/widgets/high_light_text.dart';
import 'package:mixin_desktop_ui/widgets/interactive_decorated_box.dart';
import 'package:mixin_desktop_ui/widgets/message_bubble.dart';
import 'package:mixin_desktop_ui/widgets/message_layout.dart';
import 'package:mixin_desktop_ui/widgets/message_media_preview_pages.dart';
import 'package:mixin_desktop_ui/widgets/message_selectable_text.dart';
import 'package:mixin_desktop_ui/widgets/message_style.dart';
import 'package:mixin_desktop_ui/widgets/mixin_image.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_item.dart';
import 'package:mixin_desktop_ui/widgets/toast.dart';
import 'package:pointycastle/digests/sha3.dart';

typedef MessageStringCallback = void Function(String value);
typedef MessageUriCallback = void Function(Uri uri);
typedef MessageActionCallback = void Function(String value, {String? title});

class WaitingMessageItem extends StatelessWidget {
  const WaitingMessageItem({
    required this.messageId,
    required this.subject,
    required this.dateAndStatus,
    required this.onOpenHelp,
    super.key,
  });

  final String messageId;
  final String subject;
  final Widget dateAndStatus;
  final VoidCallback? onOpenHelp;

  @override
  Widget build(BuildContext context) {
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    return MessageLayout(
      spacing: 6,
      content: RichText(
        key: Key('message-media-waiting-$messageId'),
        text: TextSpan(
          text:
              l10n?.chatDecryptionFailedHint(subject) ??
              'Waiting for $subject to get online and establish an encrypted session.',
          style: TextStyle(
            fontSize: context.messageStyle.primaryFontSize,
            color: context.theme.text,
          ),
          children: [
            TextSpan(
              mouseCursor: SystemMouseCursors.click,
              text: l10n?.learnMore ?? 'Learn More',
              style: TextStyle(
                fontSize: context.messageStyle.primaryFontSize,
                color: context.theme.accent,
              ),
              recognizer: TapGestureRecognizer()..onTap = onOpenHelp,
            ),
          ],
        ),
      ),
      dateAndStatus: dateAndStatus,
    );
  }
}

class QuoteMessagePreview extends StatelessWidget {
  const QuoteMessagePreview({
    required this.raw,
    required this.messageId,
    required this.membership,
    required this.mentionNames,
    required this.onOpenMessage,
    super.key,
  });

  final String raw;
  final String? messageId;
  final String? membership;
  final Map<String, String> mentionNames;
  final MessageStringCallback? onOpenMessage;

  @override
  Widget build(BuildContext context) {
    final quote = _QuoteData.parse(raw);
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    if (quote == null) {
      return _QuoteMessageBase(
        description: l10n?.messageNotFound ?? 'Message not found',
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
        description: l10n?.messageNotSupport ?? 'Message not supported',
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
      description: quote.preview(l10n, mentionNames),
      onTap: messageId == null || onOpenMessage == null
          ? null
          : () => onOpenMessage!(messageId!),
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
    final source = quote.mediaUrl ?? quote.thumbUrl ?? quote.thumbImage;
    if (source == null) return null;
    final provider = imageProviderForSource(source);
    if (provider == null) return null;
    return ClipRRect(
      borderRadius: const BorderRadius.all(Radius.circular(6)),
      child: Image(
        image: provider,
        width: 48,
        height: 48,
        fit: BoxFit.cover,
        errorBuilder: (_, _, _) => const SizedBox.square(dimension: 48),
      ),
    );
  }
}

Widget? buildMessageQuotePreview(
  MessageListEntry message, {
  MessageStringCallback? onOpenMessage,
  Map<String, String> mentionNames = const {},
}) {
  final raw = message.quoteContent;
  if (raw == null || raw.isEmpty) return null;
  return QuoteMessagePreview(
    raw: raw,
    messageId: message.quoteMessageId,
    membership: message.quoteUserMembership,
    mentionNames: mentionNames,
    onOpenMessage: onOpenMessage,
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
                SizedBox.square(
                  key: const Key('quote-message-image'),
                  dimension: 48,
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

class ContactMessageItem extends StatelessWidget {
  const ContactMessageItem({
    required this.message,
    required this.onOpenUser,
    super.key,
  });

  final MessageListEntry message;
  final MessageStringCallback? onOpenUser;

  @override
  Widget build(BuildContext context) {
    final userId = message.sharedUserId ?? '';
    return InteractiveDecoratedBox(
      onTap: userId.isEmpty || onOpenUser == null
          ? null
          : () => onOpenUser!(userId),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          AvatarView(
            userId: userId,
            name: message.sharedUserFullName ?? '',
            avatarUrl: message.sharedUserAvatarUrl ?? '',
            size: 40,
          ),
          const SizedBox(width: 8),
          Flexible(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Flexible(
                      child: Text(
                        Characters(message.sharedUserFullName ?? '')
                            .replaceAll(Characters(''), Characters('\u200B'))
                            .toString(),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          color: context.theme.text,
                          fontSize: context.messageStyle.primaryFontSize,
                        ),
                      ),
                    ),
                    BadgesWidget(
                      verified: message.sharedUserIsVerified,
                      isBot: message.sharedUserAppId != null,
                      membership: message.sharedUserMembership,
                    ),
                  ],
                ),
                Text(
                  message.sharedUserIdentityNumber ?? '',
                  style: TextStyle(
                    color: context.theme.secondaryText,
                    fontSize: context.messageStyle.secondaryFontSize,
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

class LocationMessageItem extends StatelessWidget {
  const LocationMessageItem({
    required this.message,
    required this.onOpenUri,
    super.key,
  });

  final MessageListEntry message;
  final MessageUriCallback? onOpenUri;

  @override
  Widget build(BuildContext context) {
    final location = _LocationData.parse(message.content);
    if (location == null) {
      return UnknownSpecialMessage(category: message.category);
    }
    final uri = location.googleMapsUri;
    return SizedBox(
      width: 260,
      height: 180,
      child: InteractiveDecoratedBox(
        onTap: onOpenUri == null ? null : () => onOpenUri!(uri),
        child: Stack(
          children: [
            map.MapLayout(
              controller: map.MapController(
                location: LatLng.degree(location.latitude, location.longitude),
              ),
              builder: (context, transformer) => map.TileLayer(
                builder: (context, x, y, z) {
                  final url =
                      'https://www.google.com/maps/vt/pb=!1m4!1m3!1i$z!2i$x!3i$y!2m3!1e0!2sm!3i420120488!3m7!2sen!5e1105!12m4!1e68!2m2!1sset!2sRoadmap!4e0!5m1!1e0!23i4111425';
                  return MixinImage.network(url);
                },
              ),
            ),
            Center(
              child: Padding(
                padding: const EdgeInsets.only(bottom: 16.5),
                child: SvgPicture.asset(MixinAssets.locationMark),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class AppButtonGroupMessageItem extends StatelessWidget {
  const AppButtonGroupMessageItem({
    required this.message,
    required this.onAction,
    required this.isCurrentUser,
    required this.showNip,
    required this.highlighted,
    this.highlightOpacity = 0,
    super.key,
    this.quote,
    this.onOpenUri,
    this.onOpenIdentityNumber,
    this.mentionNames = const {},
    this.keyword = '',
    this.isPinnedPage = false,
    this.onPinnedMessageTap,
  });

  final MessageListEntry message;
  final MessageActionCallback? onAction;
  final bool isCurrentUser;
  final bool showNip;
  final bool highlighted;
  final double highlightOpacity;
  final Widget? quote;
  final MessageUriCallback? onOpenUri;
  final MessageStringCallback? onOpenIdentityNumber;
  final Map<String, String> mentionNames;
  final String keyword;
  final bool isPinnedPage;
  final VoidCallback? onPinnedMessageTap;

  @override
  Widget build(BuildContext context) {
    final actions = _ActionData.parseList(message.content);
    if (actions == null) {
      return UnknownSpecialMessage(category: message.category);
    }
    return MessageBubble(
      isCurrentUser: isCurrentUser,
      showNip: showNip,
      showBubble: false,
      highlighted: false,
      padding: EdgeInsets.zero,
      quote: quote,
      isPinnedPage: isPinnedPage,
      onPinnedMessageTap: onPinnedMessageTap,
      isDisappearingMessage: (message.expireIn ?? 0) > 0,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minWidth: 240, maxWidth: 340),
        child: _ActionButtonLayout(
          children: actions
              .map(
                (action) => _ActionButton(
                  action: action,
                  onAction: onAction,
                  highlighted: highlighted,
                  highlightOpacity: highlightOpacity,
                ),
              )
              .toList(growable: false),
        ),
      ),
    );
  }
}

class AppCardMessageItem extends StatelessWidget {
  const AppCardMessageItem({
    required this.message,
    required this.onAction,
    required this.isCurrentUser,
    required this.showNip,
    required this.highlighted,
    this.highlightOpacity = 0,
    required this.dateAndStatus,
    super.key,
    this.quote,
    this.onOpenUri,
    this.onOpenIdentityNumber,
    this.mentionNames = const {},
    this.keyword = '',
    this.isPinnedPage = false,
    this.onPinnedMessageTap,
  });

  final MessageListEntry message;
  final MessageActionCallback? onAction;
  final bool isCurrentUser;
  final bool showNip;
  final bool highlighted;
  final double highlightOpacity;
  final Widget dateAndStatus;
  final Widget? quote;
  final MessageUriCallback? onOpenUri;
  final MessageStringCallback? onOpenIdentityNumber;
  final Map<String, String> mentionNames;
  final String keyword;
  final bool isPinnedPage;
  final VoidCallback? onPinnedMessageTap;

  @override
  Widget build(BuildContext context) {
    final card = _AppCardData.parse(message.content);
    if (card == null) {
      return MessageBubble(
        isCurrentUser: isCurrentUser,
        showNip: showNip,
        highlighted: highlighted,
        highlightOpacity: highlightOpacity,
        quote: quote,
        isPinnedPage: isPinnedPage,
        onPinnedMessageTap: onPinnedMessageTap,
        isDisappearingMessage: (message.expireIn ?? 0) > 0,
        child: MessageLayout(
          spacing: 6,
          content: UnknownSpecialMessage(category: message.category),
          dateAndStatus: dateAndStatus,
        ),
      );
    }
    if (card.action.isEmpty) {
      return _ActionsCardMessage(
        messageId: message.id,
        card: card,
        onAction: onAction,
        isCurrentUser: isCurrentUser,
        showNip: showNip,
        highlighted: highlighted,
        highlightOpacity: highlightOpacity,
        quote: quote,
        onOpenUri: onOpenUri,
        onOpenIdentityNumber: onOpenIdentityNumber,
        mentionNames: mentionNames,
        keyword: keyword,
        isPinnedPage: isPinnedPage,
        onPinnedMessageTap: onPinnedMessageTap,
        isDisappearingMessage: (message.expireIn ?? 0) > 0,
      );
    }
    return MessageBubble(
      key: Key('app-card-compact-${message.id}'),
      isCurrentUser: isCurrentUser,
      showNip: showNip,
      highlighted: highlighted,
      highlightOpacity: highlightOpacity,
      quote: quote,
      isPinnedPage: isPinnedPage,
      onPinnedMessageTap: onPinnedMessageTap,
      isDisappearingMessage: (message.expireIn ?? 0) > 0,
      outerTimeAndStatusWidget: dateAndStatus,
      child: InteractiveDecoratedBox(
        onTap: onAction == null
            ? null
            : () => onAction!(card.action, title: card.title),
        child: SelectionArea(
          contextMenuBuilder: (context, selectableState) =>
              const SizedBox.shrink(),
          child: _AppCardHeader(card: card),
        ),
      ),
    );
  }
}

class _ActionsCardMessage extends StatelessWidget {
  const _ActionsCardMessage({
    required this.messageId,
    required this.card,
    required this.onAction,
    required this.isCurrentUser,
    required this.showNip,
    required this.highlighted,
    required this.highlightOpacity,
    this.quote,
    this.onOpenUri,
    this.onOpenIdentityNumber,
    this.mentionNames = const {},
    this.keyword = '',
    this.isPinnedPage = false,
    this.onPinnedMessageTap,
    this.isDisappearingMessage = false,
  });

  final String messageId;
  final _AppCardData card;
  final MessageActionCallback? onAction;
  final bool isCurrentUser;
  final bool showNip;
  final bool highlighted;
  final double highlightOpacity;
  final Widget? quote;
  final MessageUriCallback? onOpenUri;
  final MessageStringCallback? onOpenIdentityNumber;
  final Map<String, String> mentionNames;
  final String keyword;
  final bool isPinnedPage;
  final VoidCallback? onPinnedMessageTap;
  final bool isDisappearingMessage;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final width = (constraints.maxWidth * 0.5).clamp(320.0, 375.0);
      return MessageBubble(
        isCurrentUser: isCurrentUser,
        showNip: showNip,
        showBubble: false,
        padding: EdgeInsets.zero,
        includeNip: true,
        highlighted: false,
        quote: quote,
        isPinnedPage: isPinnedPage,
        onPinnedMessageTap: onPinnedMessageTap,
        isDisappearingMessage: isDisappearingMessage,
        child: Column(
          key: Key('app-card-actions-message-$messageId'),
          children: [
            MessageBubble(
              key: Key('app-card-body-$messageId'),
              isCurrentUser: isCurrentUser,
              showNip: true,
              highlighted: highlighted,
              highlightOpacity: highlightOpacity,
              padding: EdgeInsets.zero,
              clip: true,
              includeNip: false,
              child: ConstrainedBox(
                constraints: BoxConstraints(minWidth: width, maxWidth: width),
                child: SelectionArea(
                  contextMenuBuilder: (context, selectableState) =>
                      const SizedBox.shrink(),
                  child: _ActionsCardBody(
                    card: card,
                    onOpenUri: onOpenUri,
                    onOpenIdentityNumber: onOpenIdentityNumber,
                    mentionNames: mentionNames,
                    keyword: keyword,
                  ),
                ),
              ),
            ),
            const SizedBox(height: 8),
            MessageBubbleNipPadding(
              currentUser: isCurrentUser,
              child: ConstrainedBox(
                key: Key('app-card-actions-$messageId'),
                constraints: BoxConstraints(minWidth: width, maxWidth: width),
                child: _ActionButtonLayout(
                  children: card.actions
                      .map(
                        (action) => _ActionButton(
                          action: action,
                          onAction: onAction,
                          highlighted: highlighted,
                          highlightOpacity: highlightOpacity,
                        ),
                      )
                      .toList(growable: false),
                ),
              ),
            ),
          ],
        ),
      );
    },
  );
}

class TranscriptMessageItem extends StatelessWidget {
  const TranscriptMessageItem({
    required this.message,
    required this.isCurrentUser,
    required this.overlayDateAndStatus,
    required this.onOpenTranscript,
    super.key,
  });

  final MessageListEntry message;
  final bool isCurrentUser;
  final Widget overlayDateAndStatus;
  final MessageStringCallback? onOpenTranscript;

  @override
  Widget build(BuildContext context) {
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    final lines = _TranscriptLine.parseList(message.content, l10n);
    if (lines == null) return UnknownSpecialMessage(category: message.category);
    return InteractiveDecoratedBox(
      onTap: onOpenTranscript == null
          ? null
          : () => onOpenTranscript!(message.id),
      child: SizedBox(
        width: 260,
        child: Stack(
          children: [
            Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Padding(
                  padding: const EdgeInsets.only(left: 4, bottom: 4),
                  child: Row(
                    children: [
                      const SizedBox(width: 4),
                      Text(
                        l10n?.transcript ?? 'Transcript',
                        style: TextStyle(
                          color: context.theme.text,
                          fontSize: context.messageStyle.primaryFontSize,
                        ),
                      ),
                      const Spacer(),
                      Container(
                        decoration: const BoxDecoration(
                          color: Color.fromRGBO(0, 0, 0, 0.2),
                          borderRadius: BorderRadius.all(Radius.circular(8)),
                        ),
                        alignment: Alignment.center,
                        child: SvgPicture.asset(
                          MixinAssets.postDetail,
                          width: 20,
                          height: 20,
                        ),
                      ),
                    ],
                  ),
                ),
                Container(
                  width: double.infinity,
                  padding: const EdgeInsets.symmetric(
                    horizontal: 6,
                    vertical: 4,
                  ),
                  decoration: const BoxDecoration(
                    color: Color.fromRGBO(0, 0, 0, 0.04),
                    borderRadius: BorderRadius.all(Radius.circular(6)),
                  ),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      for (final line in lines.take(4))
                        Text(
                          Characters('${line.name}: ${line.preview}')
                              .replaceAll(Characters(''), Characters('\u200B'))
                              .toString(),
                          maxLines: 1,
                          style: TextStyle(
                            color: context.theme.secondaryText,
                            fontSize: context.messageStyle.tertiaryFontSize,
                          ),
                        ),
                    ],
                  ),
                ),
              ],
            ),
            Positioned(
              right: isCurrentUser ? 1 : 2,
              bottom: 2,
              child: DecoratedBox(
                decoration: const ShapeDecoration(
                  color: Color.fromRGBO(0, 0, 0, 0.3),
                  shape: StadiumBorder(),
                ),
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    vertical: 3,
                    horizontal: 5,
                  ),
                  child: overlayDateAndStatus,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class SnapshotMessageItem extends StatelessWidget {
  const SnapshotMessageItem({
    required this.message,
    required this.kind,
    required this.onOpenSnapshot,
    super.key,
  });

  final MessageListEntry message;
  final SnapshotKind kind;
  final MessageStringCallback? onOpenSnapshot;

  @override
  Widget build(BuildContext context) {
    final data = _SnapshotData.parse(message.content);
    final id = message.snapshotId ?? message.inscriptionHash ?? data.id;
    final amount = message.snapshotAmount ?? data.amount;
    final symbol = message.snapshotAssetSymbol ?? data.symbol;
    final memo = _decodeSafeMemo(message.snapshotMemo ?? '');
    return InteractiveDecoratedBox(
      onTap: onOpenSnapshot == null
          ? null
          : () {
              if (id != null) {
                onOpenSnapshot!(id);
              } else if (kind == SnapshotKind.safeInscription) {
                showToastFailed(
                  ToastError(context.l10n.dataLoading),
                  context: context,
                );
              }
            },
      child: kind == SnapshotKind.safeInscription
          ? _InscriptionCard(message: message, data: data)
          : kind == SnapshotKind.safe
          ? _SafeSnapshotCard(
              amount: amount,
              symbol: symbol,
              iconUrl: message.snapshotAssetIconUrl,
              memo: memo,
            )
          : Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                if (message.snapshotAssetIconUrl?.isNotEmpty == true)
                  _AssetIcon(
                    iconUrl: message.snapshotAssetIconUrl,
                    chainIconUrl: message.snapshotChainIconUrl,
                    size: 40,
                  )
                else
                  const SizedBox(width: 40, height: 40),
                const SizedBox(width: 8),
                Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Flexible(
                          child: amount.isEmpty
                              ? const SizedBox()
                              : Text(
                                  _numberFormat(amount),
                                  style: TextStyle(
                                    color: context.theme.text,
                                    fontSize:
                                        context.messageStyle.secondaryFontSize,
                                  ),
                                ),
                        ),
                      ],
                    ),
                    Text(
                      symbol,
                      style: TextStyle(
                        color: context.theme.secondaryText,
                        fontSize: context.messageStyle.tertiaryFontSize,
                      ),
                    ),
                  ],
                ),
              ],
            ),
    );
  }
}

enum SnapshotKind { account, safe, safeInscription }

class SystemConversationMessageItem extends StatelessWidget {
  const SystemConversationMessageItem({
    required this.message,
    required this.currentUserId,
    super.key,
  });

  final MessageListEntry message;
  final String currentUserId;

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.symmetric(vertical: 4, horizontal: 8),
      child: MouseRegion(
        cursor: SystemMouseCursors.click,
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: context.dynamicColor(const Color.fromRGBO(202, 234, 201, 1)),
            borderRadius: const BorderRadius.all(Radius.circular(10)),
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 5, horizontal: 10),
            child: CustomText(
              generateSystemMessageText(
                context,
                action: message.action,
                participantId: message.participantId,
                participantName: message.participantFullName,
                senderId: message.senderId,
                senderName: message.senderName,
                currentUserId: currentUserId,
                expireIn: message.expireIn,
              ),
              style: TextStyle(
                fontSize: context.messageStyle.secondaryFontSize,
                color: context.dynamicColor(const Color.fromRGBO(0, 0, 0, 1)),
              ),
            ),
          ),
        ),
      ),
    ),
  );
}

class PinMessageItem extends StatelessWidget {
  const PinMessageItem({
    required this.message,
    this.mentionNames = const {},
    super.key,
  });

  final MessageListEntry message;
  final Map<String, String> mentionNames;

  @override
  Widget build(BuildContext context) {
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations)!;
    final pin = _PinData.parse(message.content);
    var preview = pin?.preview(l10n) ?? l10n.aMessage;
    for (final entry in mentionNames.entries) {
      preview = preview.replaceAll('@${entry.key}', '@${entry.value}');
    }
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 4, horizontal: 8),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 400),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: context.dynamicColor(
                const Color.fromRGBO(202, 234, 201, 1),
              ),
              borderRadius: const BorderRadius.all(Radius.circular(10)),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 5, horizontal: 10),
              child: CustomText(
                l10n.chatPinMessage(message.senderName, preview),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: context.messageStyle.secondaryFontSize,
                  color: context.dynamicColor(const Color.fromRGBO(0, 0, 0, 1)),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class SecretMessageItem extends StatelessWidget {
  const SecretMessageItem({required this.onOpenUri, super.key});

  final MessageUriCallback? onOpenUri;

  @override
  Widget build(BuildContext context) {
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    final uri = Uri.tryParse(l10n?.secretUrl ?? 'https://mixin.one');
    return Center(
      child: Padding(
        padding: const EdgeInsets.only(left: 8, right: 8, bottom: 4),
        child: MouseRegion(
          cursor: SystemMouseCursors.click,
          child: GestureDetector(
            onTap: uri == null || onOpenUri == null
                ? null
                : () => onOpenUri!(uri),
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: context.theme.encrypt,
                borderRadius: const BorderRadius.all(Radius.circular(10)),
              ),
              child: Padding(
                padding: const EdgeInsets.all(10),
                child: Text(
                  l10n?.messageE2ee ?? 'Messages are end-to-end encrypted',
                  style: TextStyle(
                    fontSize: context.messageStyle.secondaryFontSize,
                    color: context.dynamicColor(
                      const Color.fromRGBO(0, 0, 0, 1),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class StrangerMessageItem extends StatelessWidget {
  const StrangerMessageItem({
    required this.message,
    required this.onAction,
    super.key,
  });

  final MessageListEntry message;
  final MessageStringCallback? onAction;

  @override
  Widget build(BuildContext context) {
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    final bot = message.senderIsBot;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          bot
              ? (l10n?.chatBotReceptionTitle ?? 'Tap to interact with the bot')
              : (l10n?.strangerHint ?? 'This person is not in your contacts'),
          style: TextStyle(
            fontSize: context.messageStyle.primaryFontSize,
            color: context.theme.text,
          ),
        ),
        const SizedBox(height: 10),
        Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            _StrangerButton(
              text: bot
                  ? (l10n?.openHomePage ?? 'Open Home Page')
                  : (l10n?.block ?? 'Block'),
              action: bot ? 'open_home' : 'block',
              onAction: onAction,
            ),
            const SizedBox(width: 16),
            _StrangerButton(
              text: bot
                  ? (l10n?.sayHi ?? 'Say Hi')
                  : (l10n?.addContact ?? 'Add Contact'),
              action: bot ? 'say_hi' : 'add_contact',
              onAction: onAction,
            ),
          ],
        ),
      ],
    );
  }
}

class UnknownSpecialMessage extends StatelessWidget {
  const UnknownSpecialMessage({required this.category, super.key});

  final String category;

  @override
  Widget build(BuildContext context) {
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    return Text(
      l10n?.messageNotSupport ?? 'This message is not supported',
      style: TextStyle(color: context.theme.text, fontSize: 16),
    );
  }
}

class _ActionButton extends StatelessWidget {
  const _ActionButton({
    required this.action,
    required this.onAction,
    required this.highlighted,
    required this.highlightOpacity,
  });

  final _ActionData action;
  final MessageActionCallback? onAction;
  final bool highlighted;
  final double highlightOpacity;

  @override
  Widget build(BuildContext context) {
    final bubbleClipper = BubbleClipper(
      currentUser: false,
      showNip: false,
      nipPadding: false,
    );
    return InteractiveDecoratedBox.color(
      cursor: SystemMouseCursors.click,
      onTap: onAction == null
          ? null
          : () => onAction!(action.action, title: action.label),
      child: MessageBubbleHighlight(
        clipper: bubbleClipper,
        currentUser: false,
        media: true,
        enabled: highlighted || highlightOpacity > 0,
        opacity: highlighted ? 1 : highlightOpacity,
        child: CustomPaint(
          painter: BubblePainter(
            color: context.theme.primary,
            clipper: bubbleClipper,
          ),
          child: IntrinsicWidth(
            child: Stack(
              fit: StackFit.passthrough,
              children: [
                Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 14,
                    vertical: 8,
                  ),
                  child: Text(
                    action.label,
                    textAlign: TextAlign.center,
                    style: TextStyle(
                      color: action.color,
                      fontSize: context.messageStyle.secondaryFontSize,
                      height: 1,
                    ),
                  ),
                ),
                if (action.isSendUserLink)
                  Align(
                    alignment: Alignment.topRight,
                    child: Padding(
                      padding: const EdgeInsets.all(6),
                      child: SvgPicture.asset(
                        MixinAssets.linkSend,
                        width: 8,
                        height: 8,
                      ),
                    ),
                  )
                else if (action.isExternalLink)
                  Align(
                    alignment: Alignment.topRight,
                    child: Padding(
                      padding: const EdgeInsets.all(6),
                      child: SvgPicture.asset(
                        MixinAssets.externalLink,
                        width: 6,
                        height: 6,
                      ),
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

class _ActionButtonLayout extends MultiChildRenderObjectWidget {
  const _ActionButtonLayout({required super.children});

  @override
  RenderObject createRenderObject(BuildContext context) =>
      _RenderActionButtonLayout(horizontalSpacing: 8, verticalSpacing: 8);
}

class _ActionButtonParentData extends ContainerBoxParentData<RenderBox> {
  int row = 0;
  double height = 0;
}

class _RenderActionButtonLayout extends RenderBox
    with
        ContainerRenderObjectMixin<RenderBox, _ActionButtonParentData>,
        RenderBoxContainerDefaultsMixin<RenderBox, _ActionButtonParentData> {
  _RenderActionButtonLayout({
    required this.horizontalSpacing,
    required this.verticalSpacing,
  });

  final double horizontalSpacing;
  final double verticalSpacing;

  @override
  void setupParentData(covariant RenderObject child) {
    if (child.parentData is! _ActionButtonParentData) {
      child.parentData = _ActionButtonParentData();
    }
  }

  @override
  Size computeDryLayout(covariant BoxConstraints constraints) {
    final childWidths = [
      (constraints.maxWidth - horizontalSpacing * 2) / 3,
      (constraints.maxWidth - horizontalSpacing) / 2,
    ];
    final childConstraints = BoxConstraints(
      maxWidth: constraints.maxWidth,
      maxHeight: constraints.maxHeight,
    );
    var height = 0.0;
    var rowItems = 0;
    var maxRowHeight = 0.0;
    var row = 0;
    var child = firstChild;
    while (child != null) {
      final parentData = child.parentData! as _ActionButtonParentData;
      final childSize = child.getDryLayout(childConstraints);
      final lastMaxRowHeight = maxRowHeight;
      maxRowHeight = math.max(childSize.height, maxRowHeight);
      parentData
        ..row = row
        ..height = maxRowHeight;

      if (childSize.width <= childWidths[0]) {
        rowItems = rowItems == 0 || rowItems == 2 || rowItems == 4
            ? rowItems + 2
            : 6;
      } else if (childSize.width <= childWidths[1]) {
        if (rowItems == 0) {
          rowItems = 3;
        } else if (rowItems == 2 || rowItems == 3) {
          rowItems = 6;
        } else {
          rowItems = 2;
          height += lastMaxRowHeight + verticalSpacing;
          maxRowHeight = childSize.height;
          row++;
          parentData
            ..row = row
            ..height = maxRowHeight;
        }
      } else if (rowItems == 0) {
        rowItems = 6;
      } else {
        rowItems = 0;
        height += lastMaxRowHeight + verticalSpacing;
        height += childSize.height + verticalSpacing;
        maxRowHeight = 0;
        parentData
          ..row = row + 1
          ..height = childSize.height;
        row += 2;
      }

      if (rowItems >= 6) {
        rowItems = 0;
        height += maxRowHeight + verticalSpacing;
        maxRowHeight = 0;
        row++;
      }
      child = parentData.nextSibling;
    }
    if (maxRowHeight != 0) height += maxRowHeight + verticalSpacing;
    if (height > 0) height -= verticalSpacing;
    return constraints.constrain(Size(constraints.maxWidth, height));
  }

  @override
  void performLayout() {
    size = computeDryLayout(constraints);
    final rows = <int, List<RenderBox>>{};
    var child = firstChild;
    while (child != null) {
      final parentData = child.parentData! as _ActionButtonParentData;
      (rows[parentData.row] ??= []).add(child);
      child = parentData.nextSibling;
    }

    var offsetY = 0.0;
    for (final row in rows.values) {
      final childWidth =
          (constraints.maxWidth - (row.length - 1) * horizontalSpacing) /
          row.length;
      final rowHeight = row.fold<double>(
        0,
        (value, item) => math.max(
          value,
          (item.parentData! as _ActionButtonParentData).height,
        ),
      );
      final childConstraints = BoxConstraints.expand(
        width: childWidth,
        height: rowHeight,
      );
      var offsetX = 0.0;
      for (final item in row) {
        item.layout(childConstraints);
        (item.parentData! as _ActionButtonParentData).offset = Offset(
          offsetX,
          offsetY,
        );
        offsetX += childWidth + horizontalSpacing;
      }
      offsetY += rowHeight + verticalSpacing;
    }
  }

  @override
  void paint(PaintingContext context, Offset offset) =>
      defaultPaint(context, offset);

  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) =>
      defaultHitTestChildren(result, position: position);
}

class _ActionsCardBody extends StatelessWidget {
  const _ActionsCardBody({
    required this.card,
    required this.onOpenUri,
    required this.onOpenIdentityNumber,
    required this.mentionNames,
    required this.keyword,
  });

  final _AppCardData card;
  final MessageUriCallback? onOpenUri;
  final MessageStringCallback? onOpenIdentityNumber;
  final Map<String, String> mentionNames;
  final String keyword;

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      if (card.coverUrl.isNotEmpty)
        AspectRatio(
          aspectRatio: 1,
          child: _NetworkCardImage(url: card.coverUrl),
        )
      else if (card.cover != null)
        AspectRatio(
          aspectRatio: math.max(card.cover!.aspectRatio, 1.5),
          child: _NetworkCardImage(url: card.cover!.url),
        ),
      const SizedBox(height: 10),
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12),
        child: CustomText(
          card.title,
          style: TextStyle(
            color: context.theme.text,
            fontSize: 16,
            fontWeight: FontWeight.bold,
          ),
        ),
      ),
      const SizedBox(height: 8),
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12),
        child: SelectableMessageText(
          content: card.description,
          enableSelection: false,
          onOpenUri: onOpenUri,
          onOpenIdentityNumber: onOpenIdentityNumber,
          mentionNames: mentionNames,
          keyword: keyword,
          style: TextStyle(
            color: context.theme.text,
            fontSize: context.messageStyle.primaryFontSize,
          ),
        ),
      ),
      const SizedBox(height: 10),
    ],
  );
}

class _NetworkCardImage extends StatelessWidget {
  const _NetworkCardImage({required this.url});

  final String url;

  @override
  Widget build(BuildContext context) => Image.network(
    url,
    fit: BoxFit.cover,
    errorBuilder: (_, _, _) => ColoredBox(color: context.theme.background),
  );
}

class _AppCardHeader extends StatelessWidget {
  const _AppCardHeader({required this.card});

  final _AppCardData card;

  @override
  Widget build(BuildContext context) => Row(
    mainAxisSize: MainAxisSize.min,
    children: [
      ClipRRect(
        borderRadius: const BorderRadius.all(Radius.circular(4)),
        child: card.iconUrl.isEmpty
            ? const SizedBox.square(dimension: 40)
            : Image.network(
                card.iconUrl,
                width: 40,
                height: 40,
                fit: BoxFit.cover,
                errorBuilder: (_, _, _) => const SizedBox.square(dimension: 40),
              ),
      ),
      const SizedBox(width: 8),
      Flexible(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              card.title,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(color: context.theme.text, fontSize: 14),
            ),
            Text(
              card.description.split('\n').first,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                color: context.theme.secondaryText,
                fontSize: 12,
              ),
            ),
          ],
        ),
      ),
    ],
  );
}

class _StrangerButton extends StatelessWidget {
  const _StrangerButton({
    required this.text,
    required this.action,
    required this.onAction,
  });
  final String text;
  final String action;
  final MessageStringCallback? onAction;

  @override
  Widget build(BuildContext context) => InteractiveDecoratedBox.color(
    onTap: onAction == null ? null : () => onAction!(action),
    decoration: BoxDecoration(
      color: context.theme.primary,
      borderRadius: const BorderRadius.all(Radius.circular(8)),
    ),
    child: ConstrainedBox(
      constraints: const BoxConstraints(
        minWidth: 162,
        minHeight: 36,
        maxHeight: 36,
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Center(
          child: Text(
            text,
            style: TextStyle(
              fontSize: context.messageStyle.primaryFontSize,
              color: context.theme.accent,
            ),
          ),
        ),
      ),
    ),
  );
}

class _InscriptionCard extends StatelessWidget {
  const _InscriptionCard({required this.message, required this.data});
  final MessageListEntry message;
  final _SnapshotData data;

  @override
  Widget build(BuildContext context) => SizedBox(
    width: 260,
    height: 112,
    child: Row(
      children: [
        InscriptionContent(
          contentType: message.inscriptionContentType ?? data.contentType,
          contentUrl: message.inscriptionContentUrl ?? data.contentUrl,
          iconUrl: message.inscriptionIconUrl,
          mode: InscriptionContentMode.small,
        ),
        Expanded(
          child: Padding(
            padding: const EdgeInsets.all(10),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  message.inscriptionName ?? data.name,
                  style: TextStyle(color: context.theme.text, fontSize: 14),
                ),
                const SizedBox(height: 4),
                Text(
                  message.inscriptionSequence == null
                      ? ''
                      : '#${message.inscriptionSequence}',
                  style: TextStyle(
                    color: context.theme.secondaryText,
                    fontSize: 12,
                  ),
                ),
                const Spacer(),
                Row(
                  children: [
                    ColoredHashWidget(inscriptionHex: message.inscriptionHash),
                    const SizedBox(width: 10),
                    HexagonWidget(
                      type: HexagonType.FLAT,
                      cornerRadius: 4,
                      height: 22,
                      width: 22,
                      child: SizedBox.square(
                        dimension: 22,
                        child: Image.network(
                          message.inscriptionIconUrl ?? '',
                          fit: BoxFit.cover,
                          errorBuilder: (_, _, _) => SvgPicture.asset(
                            MixinAssets.collectionPlaceholder,
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(width: 2),
                  ],
                ),
                const SizedBox(height: 2),
              ],
            ),
          ),
        ),
      ],
    ),
  );
}

class _AssetIcon extends StatelessWidget {
  const _AssetIcon({this.iconUrl, this.chainIconUrl, required this.size});

  final String? iconUrl;
  final String? chainIconUrl;
  final double size;

  @override
  Widget build(BuildContext context) => SizedBox.square(
    dimension: size,
    child: Stack(
      children: [
        Positioned.fill(
          child: ClipPath(
            clipper: _SymbolCustomClipper(
              chainPlaceholderSize: chainIconUrl?.isNotEmpty == true ? 14 : 0,
            ),
            clipBehavior: Clip.antiAliasWithSaveLayer,
            child: iconUrl?.isNotEmpty == true
                ? Image.network(
                    iconUrl!,
                    fit: BoxFit.cover,
                    errorBuilder: (_, _, _) => const SizedBox.expand(),
                  )
                : const SizedBox.expand(),
          ),
        ),
        if (chainIconUrl?.isNotEmpty == true)
          Positioned(
            right: 1,
            bottom: 1,
            child: Image.network(
              chainIconUrl!,
              width: 12,
              height: 12,
              fit: BoxFit.cover,
              errorBuilder: (_, _, _) => const SizedBox.square(dimension: 12),
            ),
          ),
      ],
    ),
  );
}

class _SymbolCustomClipper extends CustomClipper<Path> {
  const _SymbolCustomClipper({required this.chainPlaceholderSize});

  final double chainPlaceholderSize;

  @override
  Path getClip(Size size) {
    final symbol = Path()..addOval(Offset.zero & size);
    if (chainPlaceholderSize <= 0) return symbol;
    final chain = Path()
      ..addOval(
        Offset(
              size.width - chainPlaceholderSize,
              size.height - chainPlaceholderSize,
            ) &
            Size.square(chainPlaceholderSize),
      );
    return Path.combine(PathOperation.difference, symbol, chain);
  }

  @override
  bool shouldReclip(covariant _SymbolCustomClipper oldClipper) =>
      oldClipper.chainPlaceholderSize != chainPlaceholderSize;
}

class _SafeSnapshotCard extends StatelessWidget {
  const _SafeSnapshotCard({
    required this.amount,
    required this.symbol,
    required this.iconUrl,
    required this.memo,
  });

  final String amount;
  final String symbol;
  final String? iconUrl;
  final String memo;

  @override
  Widget build(BuildContext context) => SizedBox(
    width: switch (amount.length) {
      < 10 => 174,
      < 15 => 190,
      < 25 => 216,
      _ => 232,
    },
    child: Stack(
      children: [
        Align(
          alignment: Alignment.topRight,
          child: SvgPicture.asset(MixinAssets.bgSnapshot),
        ),
        Padding(
          padding: const EdgeInsets.all(4),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  if (iconUrl?.isNotEmpty != true)
                    const SizedBox.square(dimension: 16)
                  else
                    ClipOval(
                      child: Image.network(
                        iconUrl!,
                        width: 16,
                        height: 16,
                        fit: BoxFit.cover,
                        errorBuilder: (_, _, _) =>
                            const SizedBox.square(dimension: 16),
                      ),
                    ),
                  const SizedBox(width: 4),
                  Text(
                    symbol,
                    style: TextStyle(color: context.theme.text, fontSize: 13),
                  ),
                ],
              ),
              const SizedBox(height: 16),
              AutoSizeText(
                _numberFormat(amount),
                maxFontSize: 36,
                minFontSize: 24,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  color: context.theme.text,
                  fontFamily: 'MixinCondensed',
                  fontSize: 36,
                  height: 1,
                ),
              ),
              if (memo.isNotEmpty) ...[
                const SizedBox(height: 10),
                CustomText(
                  memo,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: context.theme.secondaryText,
                    fontSize: 12,
                  ),
                ),
                const SizedBox(height: 2),
              ] else
                const SizedBox(height: 6),
            ],
          ),
        ),
      ],
    ),
  );
}

enum InscriptionContentMode {
  small(2, 14),
  large(5, 24);

  const InscriptionContentMode(this.maxLines, this.fontSize);

  final int maxLines;
  final double fontSize;
}

class InscriptionContent extends StatelessWidget {
  const InscriptionContent({
    required this.mode,
    this.contentType,
    this.contentUrl,
    this.iconUrl,
    super.key,
  });

  final String? contentType;
  final String? contentUrl;
  final String? iconUrl;
  final InscriptionContentMode mode;

  @override
  Widget build(BuildContext context) {
    final url = contentUrl;
    final placeholder = SvgPicture.asset(MixinAssets.inscriptionPlaceholder);
    return AspectRatio(
      aspectRatio: 1,
      child: switch (contentType) {
        final type when type?.startsWith('image') == true && url != null =>
          Image.network(
            url,
            fit: BoxFit.cover,
            errorBuilder: (_, _, _) => placeholder,
          ),
        final type
            when type?.startsWith('text') == true &&
                url != null &&
                iconUrl != null =>
          _TextInscriptionContent(
            contentUrl: url,
            iconUrl: iconUrl!,
            mode: mode,
          ),
        _ => placeholder,
      },
    );
  }
}

class _TextInscriptionContent extends StatefulWidget {
  const _TextInscriptionContent({
    required this.contentUrl,
    required this.iconUrl,
    required this.mode,
  });

  final String contentUrl;
  final String iconUrl;
  final InscriptionContentMode mode;

  @override
  State<_TextInscriptionContent> createState() =>
      _TextInscriptionContentState();
}

class _TextInscriptionContentState extends State<_TextInscriptionContent> {
  late Future<String> _content = _loadContent();

  Future<String> _loadContent() async {
    final response = await http.get(Uri.parse(widget.contentUrl));
    return _inscriptionDisplayContent(
      utf8.decode(response.bodyBytes, allowMalformed: true),
    );
  }

  @override
  void didUpdateWidget(covariant _TextInscriptionContent oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.contentUrl != widget.contentUrl) {
      _content = _loadContent();
    }
  }

  @override
  Widget build(BuildContext context) => Stack(
    fit: StackFit.expand,
    children: [
      Image.asset(MixinAssets.inscriptionTextBackground),
      LayoutBuilder(
        builder: (context, constraints) {
          final textStyle = TextStyle(
            color: Color.fromRGBO(255, 167, 36, 1),
            fontSize: widget.mode.fontSize,
            fontWeight: FontWeight.bold,
          );
          return Column(
            mainAxisSize: MainAxisSize.min,
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              HexagonWidget(
                type: HexagonType.FLAT,
                cornerRadius: constraints.maxWidth / 3 / 5,
                height: constraints.maxWidth / 3,
                width: constraints.maxWidth / 3,
                child: SizedBox.square(
                  dimension: constraints.maxWidth / 3,
                  child: Image.network(
                    widget.iconUrl,
                    fit: BoxFit.cover,
                    errorBuilder: (_, _, _) =>
                        SvgPicture.asset(MixinAssets.collectionPlaceholder),
                  ),
                ),
              ),
              Padding(
                padding: EdgeInsets.only(
                  top: constraints.maxWidth / 40,
                  right: constraints.maxWidth / 10,
                  left: constraints.maxWidth / 10,
                ),
                child: FutureBuilder<String>(
                  future: _content,
                  builder: (context, snapshot) => _MinLinesWrapper(
                    text: snapshot.data,
                    style: textStyle,
                    minLines: widget.mode.maxLines,
                    child: AutoSizeText(
                      snapshot.data ?? '',
                      maxLines: widget.mode.maxLines,
                      maxFontSize: 24,
                      minFontSize: widget.mode.fontSize,
                      overflow: TextOverflow.ellipsis,
                      style: textStyle,
                      textAlign: TextAlign.center,
                    ),
                  ),
                ),
              ),
            ],
          );
        },
      ),
    ],
  );
}

class _MinLinesWrapper extends StatelessWidget {
  const _MinLinesWrapper({
    required this.text,
    required this.style,
    required this.minLines,
    required this.child,
  });

  final String? text;
  final TextStyle style;
  final int minLines;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final textPainter = TextPainter(
      text: TextSpan(text: text, style: style),
      textDirection: TextDirection.ltr,
      maxLines: minLines,
    )..layout(maxWidth: MediaQuery.sizeOf(context).width);
    return ConstrainedBox(
      constraints: BoxConstraints(
        minHeight: textPainter.preferredLineHeight * minLines,
      ),
      child: child,
    );
  }
}

class ColoredHashWidget extends StatelessWidget {
  const ColoredHashWidget({
    required this.inscriptionHex,
    this.blockSize = const Size(5, 16),
    this.space = 3,
    super.key,
  });

  final String? inscriptionHex;
  final Size blockSize;
  final double space;

  @override
  Widget build(BuildContext context) {
    final colors = _inscriptionColors(inscriptionHex);
    return Row(
      children: [
        for (var index = 0; index < colors.length; index++) ...[
          if (index > 0) SizedBox(width: space),
          Container(
            width: blockSize.width,
            height: blockSize.height,
            decoration: BoxDecoration(
              color: colors[index],
              borderRadius: BorderRadius.circular(1),
            ),
          ),
        ],
      ],
    );
  }
}

List<Color> _inscriptionColors(String? value) {
  final bytes = _hexBytes(value);
  if (bytes == null) return List<Color>.filled(12, Colors.black12);
  final digest = SHA3Digest(256).process(Uint8List.fromList(bytes));
  final data = [...bytes, ...digest.take(4)];
  return [
    for (var index = 0; index + 2 < data.length; index += 3)
      Color.fromARGB(0xFF, data[index], data[index + 1], data[index + 2]),
  ];
}

List<int>? _hexBytes(String? value) {
  if (value == null ||
      value.length.isOdd ||
      !RegExp(r'^[0-9a-fA-F]+$').hasMatch(value)) {
    return null;
  }
  return [
    for (var index = 0; index < value.length; index += 2)
      int.parse(value.substring(index, index + 2), radix: 16),
  ];
}

String _inscriptionDisplayContent(String content) => content.replaceAll(
  RegExp(r'[\s\p{Other}\p{Cf}\p{Cc}\p{Cn}]', unicode: true),
  '■',
);

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
  final String sender;
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

  String preview(AppLocalizations? l10n, Map<String, String> mentionNames) {
    if (category.endsWith('_TEXT')) {
      return replaceMessageMentions(content, mentionNames);
    }
    if (category.endsWith('_POST')) return _postOptimizeMarkdown(content);
    if (category.endsWith('_IMAGE')) return l10n?.image ?? 'Image';
    if (category.endsWith('_VIDEO')) return l10n?.video ?? 'Video';
    if (category.endsWith('_LIVE')) return l10n?.live ?? 'Live';
    if (category.endsWith('_AUDIO')) return l10n?.audio ?? 'Audio';
    if (category.endsWith('_DATA')) {
      return mediaName?.isNotEmpty == true
          ? mediaName!
          : (l10n?.file ?? 'File');
    }
    if (category.endsWith('_STICKER')) return l10n?.sticker ?? 'Sticker';
    if (category.endsWith('_CONTACT')) {
      return sharedUserIdentityNumber?.isNotEmpty == true
          ? sharedUserIdentityNumber!
          : (sharedUserName ?? l10n?.contact ?? 'Contact');
    }
    if (category.endsWith('_LOCATION')) return l10n?.location ?? 'Location';
    if (category.endsWith('_TRANSCRIPT')) {
      return l10n?.transcript ?? 'Transcript';
    }
    if (category == 'APP_CARD') {
      try {
        final card = jsonDecode(content) as Map<String, dynamic>;
        return card['title']?.toString() ?? l10n?.bots ?? 'Bots';
      } on Object {
        return l10n?.bots ?? 'Bots';
      }
    }
    if (category == 'APP_BUTTON_GROUP') {
      try {
        final buttons = jsonDecode(content) as List<dynamic>;
        return buttons
            .map((item) => '[${(item as Map<String, dynamic>)['label'] ?? ''}]')
            .join();
      } on Object {
        return l10n?.bots ?? 'Bots';
      }
    }
    return l10n?.messageNotSupport ?? 'Message not supported';
  }

  static _QuoteData? parse(String raw) {
    try {
      final json = jsonDecode(raw) as Map<String, dynamic>;
      return _QuoteData(
        senderId: _nonEmpty(json['user_id']),
        sender: json['user_full_name']?.toString() ?? '',
        category: json['type']?.toString() ?? '',
        content: json['content']?.toString() ?? '',
        mediaName: _nonEmpty(json['media_name']),
        mediaUrl: _nonEmpty(json['media_url']),
        thumbUrl: _nonEmpty(json['thumb_url']),
        thumbImage: _nonEmpty(json['thumb_image']),
        stickerAssetUrl: _nonEmpty(json['asset_url']),
        stickerAssetType: _nonEmpty(json['asset_type']),
        stickerId: _nonEmpty(json['sticker_id']),
        sharedUserId: _nonEmpty(json['shared_user_id']),
        sharedUserName: _nonEmpty(json['shared_user_full_name']),
        sharedUserAvatarUrl: _nonEmpty(json['shared_user_avatar_url']),
        sharedUserIdentityNumber: _nonEmpty(
          json['shared_user_identity_number'],
        ),
      );
    } on Object {
      return null;
    }
  }
}

String _postOptimizeMarkdown(String content) {
  var optimized = const LineSplitter().convert(content).take(10).join('\r\n');
  if (optimized.length > 1024) optimized = optimized.substring(0, 1024);
  return markdown.Document()
      .parseLines(const LineSplitter().convert(optimized))
      .map((node) => node.textContent)
      .join()
      .replaceAll(RegExp(r'\s+'), ' ');
}

class _LocationData {
  const _LocationData(this.latitude, this.longitude, this.name, this.address);
  final double latitude;
  final double longitude;
  final String name;
  final String address;
  String get label => address.isNotEmpty ? address : name;
  Uri get googleMapsUri => Uri.parse(
    address.isEmpty
        ? 'https://www.google.com/maps/place/@$latitude,$longitude,17z?hl=zh-CN'
        : 'https://www.google.com/maps/search/${Uri.encodeComponent(address)}/@$latitude,$longitude,17z?hl=zh-CN',
  );

  static _LocationData? parse(String raw) {
    try {
      final json = jsonDecode(raw) as Map<String, dynamic>;
      return _LocationData(
        (json['latitude'] as num).toDouble(),
        (json['longitude'] as num).toDouble(),
        json['name']?.toString() ?? '',
        json['address']?.toString() ?? '',
      );
    } on Object {
      return null;
    }
  }
}

class _ActionData {
  const _ActionData(this.label, this.action, this.color);
  final String label;
  final String action;
  final Color color;

  Uri? get _uri => Uri.tryParse(action);

  bool get isExternalLink {
    final uri = _uri;
    if (uri == null || (uri.scheme != 'http' && uri.scheme != 'https')) {
      return false;
    }
    const mixinActionPaths = {
      'codes',
      'pay',
      'users',
      'transfer',
      'device',
      'send',
      'address',
      'withdrawal',
      'apps',
      'snapshots',
      'conversations',
      'multisigs',
      'swap',
      'markets',
      'membership',
    };
    final isMixinHost = uri.host == 'mixin.one' || uri.host == 'www.mixin.one';
    final isMixinAction =
        isMixinHost &&
        uri.pathSegments.isNotEmpty &&
        mixinActionPaths.contains(uri.pathSegments.first);
    return !isMixinAction;
  }

  bool get isSendUserLink {
    final uri = _uri;
    if (uri == null || uri.queryParameters['user']?.trim().isEmpty != false) {
      return false;
    }
    return (uri.scheme == 'mixin' && uri.host == 'send') ||
        ((uri.host == 'mixin.one' || uri.host == 'www.mixin.one') &&
            uri.pathSegments.isNotEmpty &&
            uri.pathSegments.first == 'send');
  }

  static List<_ActionData>? parseList(String raw) {
    try {
      final json = jsonDecode(raw) as List<dynamic>;
      return json
          .map((item) => _ActionData.fromJson(item as Map<String, dynamic>))
          .toList();
    } on Object {
      return null;
    }
  }

  factory _ActionData.fromJson(Map<String, dynamic> json) => _ActionData(
    json['label']?.toString() ?? '',
    json['action']?.toString() ?? '',
    _parseColor(json['color']?.toString()),
  );
}

class _AppCardData {
  const _AppCardData(
    this.iconUrl,
    this.coverUrl,
    this.cover,
    this.title,
    this.description,
    this.action,
    this.actions,
    this.shareable,
  );
  final String iconUrl;
  final String coverUrl;
  final _AppCardCover? cover;
  final String title;
  final String description;
  final String action;
  final List<_ActionData> actions;
  final bool shareable;

  static _AppCardData? parse(String raw) {
    try {
      final json = jsonDecode(raw) as Map<String, dynamic>;
      final actions = (json['actions'] as List<dynamic>? ?? const [])
          .map((item) => _ActionData.fromJson(item as Map<String, dynamic>))
          .toList();
      return _AppCardData(
        json['icon_url']?.toString() ?? '',
        json['cover_url']?.toString() ?? '',
        _AppCardCover.parse(json['cover']),
        json['title']?.toString() ?? '',
        json['description']?.toString() ?? '',
        json['action']?.toString() ?? '',
        actions,
        json['shareable'] is bool ? json['shareable'] as bool : true,
      );
    } on Object {
      return null;
    }
  }
}

class _AppCardCover {
  const _AppCardCover({
    required this.url,
    required this.width,
    required this.height,
  });

  final String url;
  final int width;
  final int height;

  double get aspectRatio => height <= 0 ? 1 : width / height;

  static _AppCardCover? parse(Object? value) {
    if (value is! Map<String, dynamic>) return null;
    final url = value['url']?.toString().trim() ?? '';
    if (url.isEmpty) return null;
    return _AppCardCover(
      url: url,
      width: (value['width'] as num?)?.toInt() ?? 0,
      height: (value['height'] as num?)?.toInt() ?? 0,
    );
  }
}

class _TranscriptLine {
  const _TranscriptLine(this.name, this.preview);
  final String name;
  final String preview;

  static List<_TranscriptLine>? parseList(String raw, AppLocalizations? l10n) {
    try {
      final json = jsonDecode(raw) as List<dynamic>;
      return json.map((item) {
        final data = item as Map<String, dynamic>;
        return _TranscriptLine(
          data['name']?.toString() ?? '',
          _categoryPreview(
            l10n,
            data['category']?.toString() ?? data['type']?.toString() ?? '',
            data['content']?.toString() ?? '',
            mediaName: data['media_name']?.toString(),
          ),
        );
      }).toList();
    } on Object {
      return null;
    }
  }
}

class _SnapshotData {
  const _SnapshotData({
    required this.id,
    required this.amount,
    required this.symbol,
    required this.name,
    required this.contentType,
    required this.contentUrl,
  });
  final String? id;
  final String amount;
  final String symbol;
  final String name;
  final String? contentType;
  final String? contentUrl;

  static _SnapshotData parse(String raw) {
    try {
      final json = jsonDecode(raw) as Map<String, dynamic>;
      return _SnapshotData(
        id:
            _nonEmpty(json['snapshot_id']) ??
            _nonEmpty(json['inscription_hash']),
        amount: json['amount']?.toString() ?? '',
        symbol:
            json['symbol']?.toString() ??
            json['asset_symbol']?.toString() ??
            '',
        name: json['name']?.toString() ?? '',
        contentType: _nonEmpty(json['content_type']),
        contentUrl: _nonEmpty(json['content_url']),
      );
    } on Object {
      return const _SnapshotData(
        id: null,
        amount: '',
        symbol: '',
        name: '',
        contentType: null,
        contentUrl: null,
      );
    }
  }
}

String _decodeSafeMemo(String raw) {
  if (raw.isEmpty ||
      raw.length.isOdd ||
      !RegExp(r'^[0-9a-fA-F]+$').hasMatch(raw)) {
    return raw;
  }
  try {
    final bytes = <int>[
      for (var index = 0; index < raw.length; index += 2)
        int.parse(raw.substring(index, index + 2), radix: 16),
    ];
    return utf8.decode(bytes);
  } on Object {
    return raw;
  }
}

String _numberFormat(String value) {
  if (value.isEmpty) return value;
  try {
    return DecimalFormatter(
      intl.NumberFormat('#,###.########'),
    ).format(Decimal.parse(value));
  } on Object {
    return value;
  }
}

class _PinData {
  const _PinData(this.messageId, this.category, this.content);
  final String? messageId;
  final String category;
  final String content;

  String preview(AppLocalizations? l10n) {
    if (category.endsWith('_TEXT')) return content.trim();
    final preview = _pinCategoryPreview(l10n, category, content);
    return preview.isEmpty ? preview : ': $preview';
  }

  static _PinData? parse(String raw) {
    try {
      final json = jsonDecode(raw) as Map<String, dynamic>;
      return _PinData(
        _nonEmpty(json['message_id']),
        json['type']?.toString() ?? '',
        json['content']?.toString() ?? '',
      );
    } on Object {
      return null;
    }
  }
}

String _pinCategoryPreview(
  AppLocalizations? l10n,
  String category,
  String content,
) {
  if (category.endsWith('_POST')) {
    return content.trim().isEmpty
        ? (l10n?.post ?? 'Post')
        : _postOptimizeMarkdown(content.trim());
  }
  if (category == 'APP_BUTTON_GROUP') {
    try {
      return (jsonDecode(content) as List<dynamic>)
          .map(
            (item) =>
                '[${_ActionData.fromJson(item as Map<String, dynamic>).label}]',
          )
          .join();
    } on Object {
      return '';
    }
  }
  if (category == 'APP_CARD') {
    final title = _AppCardData.parse(content)?.title;
    return '[${title ?? (l10n?.card ?? 'Card')}]';
  }
  if (category.startsWith('WEBRTC') || category.startsWith('KRAKEN')) {
    return l10n?.contentVoice ?? 'Voice call';
  }
  if (category == 'MESSAGE_RECALL') {
    return '[${l10n?.thisMessageWasDeleted ?? 'This message was deleted'}]';
  }
  return _categoryPreview(l10n, category, content);
}

String generateSystemMessageText(
  BuildContext context, {
  required String? action,
  required String? participantId,
  required String? participantName,
  required String? senderId,
  required String? senderName,
  required String currentUserId,
  required int? expireIn,
}) {
  final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  final participant = participantId == currentUserId
      ? l10n.you
      : participantName ?? '';
  final sender = senderId == currentUserId ? l10n.you : senderName ?? '';
  return switch (action?.toUpperCase()) {
    'JOIN' => l10n.chatGroupJoin(participant),
    'EXIT' => l10n.chatGroupExit(participant),
    'ADD' => l10n.chatGroupAdd(sender, participant),
    'REMOVE' => l10n.chatGroupRemove(sender, participant),
    'CREATE' => l10n.createdThisGroup(sender),
    'ROLE' => l10n.nowAnAddmin(participant),
    'EXPIRE' => _expireSystemText(l10n, sender, expireIn),
    'UPDATE' => l10n.messageNotSupport,
    _ => l10n.messageNotSupport,
  };
}

String pinMessagePreview(AppLocalizations l10n, String raw) =>
    _PinData.parse(raw)?.preview(l10n) ?? l10n.aMessage;

String _expireSystemText(AppLocalizations l10n, String sender, int? expireIn) {
  if (expireIn == null) {
    return l10n.changedDisappearingMessageSettings(sender);
  }
  if (expireIn <= 0) return l10n.disableDisappearingMessage(sender);
  return l10n.setDisappearingMessageTimeTo(
    sender,
    _formatConversationExpireIn(l10n, expireIn),
  );
}

String _formatConversationExpireIn(AppLocalizations l10n, int seconds) {
  final duration = Duration(seconds: seconds);
  if (seconds < 60) return '$seconds ${l10n.unitSecond(seconds)}';
  final minutes = duration.inMinutes;
  if (minutes < 60) return '$minutes ${l10n.unitMinute(minutes)}';
  final hours = duration.inHours;
  if (hours < 24) return '$hours ${l10n.unitHour(hours)}';
  final days = duration.inDays;
  if (days < 7) return '$days ${l10n.unitDay(days)}';
  final weeks = days ~/ 7;
  return '$weeks ${l10n.unitWeek(weeks)}';
}

String _categoryPreview(
  AppLocalizations? l10n,
  String category,
  String content, {
  String? mediaName,
  String? sharedName,
}) {
  if (category.endsWith('_TEXT')) return content.trim();
  if (category.endsWith('_POST')) {
    return content.trim().isEmpty ? (l10n?.post ?? 'Post') : content.trim();
  }
  if (category == 'SYSTEM_ACCOUNT_SNAPSHOT' ||
      category == 'SYSTEM_SAFE_SNAPSHOT') {
    return '[${l10n?.transfer ?? 'Transfer'}]';
  }
  if (category.endsWith('_IMAGE')) return '[${l10n?.image ?? 'Image'}]';
  if (category.endsWith('_VIDEO')) return '[${l10n?.video ?? 'Video'}]';
  if (category.endsWith('_LIVE')) return '[${l10n?.live ?? 'Live'}]';
  if (category.endsWith('_AUDIO')) return '[${l10n?.audio ?? 'Audio'}]';
  if (category.endsWith('_DATA')) {
    return mediaName?.isNotEmpty == true
        ? mediaName!
        : '[${l10n?.file ?? 'File'}]';
  }
  if (category.endsWith('_STICKER')) {
    return '[${l10n?.sticker ?? 'Sticker'}]';
  }
  if (category.endsWith('_CONTACT')) {
    return sharedName?.isNotEmpty == true
        ? sharedName!
        : '[${l10n?.contact ?? 'Contact'}]';
  }
  if (category.endsWith('_LOCATION')) {
    return '[${l10n?.location ?? 'Location'}]';
  }
  if (category.endsWith('_TRANSCRIPT')) {
    return '[${l10n?.transcript ?? 'Transcript'}]';
  }
  if (category == 'SYSTEM_SAFE_INSCRIPTION') {
    return '[${l10n?.collectible ?? 'Collectible'}]';
  }
  return l10n?.messageNotSupport ?? 'Message not supported';
}

String? _nonEmpty(Object? value) {
  final text = value?.toString().trim();
  return text == null || text.isEmpty ? null : text;
}

Color _parseColor(String? raw) {
  final value = raw?.replaceFirst('#', '') ?? '';
  final parsed = int.tryParse(value, radix: 16);
  if (parsed == null) return Colors.black;
  return Color(value.length == 6 ? 0xFF000000 | parsed : parsed);
}
