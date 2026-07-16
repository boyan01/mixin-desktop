import 'dart:convert';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:latlng/latlng.dart';
import 'package:map/map.dart' as map;
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/message_bubble.dart';
import 'package:mixin_desktop_ui/widgets/message_layout.dart';
import 'package:mixin_desktop_ui/widgets/message_media_preview_pages.dart';
import 'package:mixin_desktop_ui/widgets/message_style.dart';

typedef MessageStringCallback = void Function(String value);
typedef MessageUriCallback = void Function(Uri uri);

class WaitingMessageItem extends StatelessWidget {
  const WaitingMessageItem({required this.onOpenHelp, super.key});

  final VoidCallback? onOpenHelp;

  @override
  Widget build(BuildContext context) {
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    return InkWell(
      onTap: onOpenHelp,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 2),
        child: Text(
          l10n?.waitingForThisMessage ?? 'Waiting for this message',
          style: TextStyle(color: context.theme.secondaryText, fontSize: 16),
        ),
      ),
    );
  }
}

class QuoteMessagePreview extends StatelessWidget {
  const QuoteMessagePreview({
    required this.raw,
    required this.messageId,
    required this.onOpenMessage,
    super.key,
  });

  final String raw;
  final String? messageId;
  final MessageStringCallback? onOpenMessage;

  @override
  Widget build(BuildContext context) {
    final quote = _QuoteData.parse(raw);
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    if (quote == null) {
      return _QuoteMessageBase(
        description: l10n?.messageNotFound ?? 'Message not found',
      );
    }
    if (!quote.supported) {
      return _QuoteMessageBase(
        description: l10n?.messageNotSupport ?? 'Message not supported',
      );
    }
    final image = _quoteImage(quote);
    final iconAsset = MixinAssets.messageIcon(quote.category);
    return InkWell(
      onTap: messageId == null || onOpenMessage == null
          ? null
          : () => onOpenMessage!(messageId!),
      child: _QuoteMessageBase(
        sender: quote.sender,
        description: quote.preview(l10n),
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
      ),
    );
  }

  Widget? _quoteImage(_QuoteData quote) {
    if (quote.category.endsWith('_CONTACT')) {
      return AvatarView(
        userId: quote.sharedUserId ?? '',
        name: quote.sharedUserName ?? '',
        avatarUrl: quote.sharedUserAvatarUrl ?? '',
        size: 48,
      );
    }
    final source = quote.category.endsWith('_STICKER')
        ? quote.stickerAssetUrl
        : quote.mediaUrl ?? quote.thumbUrl ?? quote.thumbImage;
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

class _QuoteMessageBase extends StatelessWidget {
  const _QuoteMessageBase({
    required this.description,
    this.sender,
    this.icon,
    this.image,
  });

  final String? sender;
  final String description;
  final Widget? icon;
  final Widget? image;

  @override
  Widget build(BuildContext context) {
    final lines = const LineSplitter().convert(description);
    final preview = lines.isEmpty
        ? ''
        : '${lines.first}${lines.length > 1 ? '...' : ''}';
    final accent = context.theme.accent;

    return ClipRRect(
      borderRadius: const BorderRadius.all(Radius.circular(8)),
      child: Container(
        key: const Key('quote-message-preview'),
        constraints: const BoxConstraints(minHeight: 50),
        color: const Color.fromRGBO(0, 0, 0, 0.04),
        child: Stack(
          children: [
            Positioned(
              left: 0,
              top: 0,
              bottom: 0,
              child: Container(
                key: const Key('quote-message-accent'),
                width: 6,
                color: accent,
              ),
            ),
            Padding(
              padding: const EdgeInsets.only(left: 6),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
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
                          if (sender?.isNotEmpty == true)
                            Padding(
                              padding: const EdgeInsets.only(bottom: 4),
                              child: Text(
                                sender!,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: TextStyle(
                                  color: accent,
                                  fontSize:
                                      context.messageStyle.secondaryFontSize,
                                  height: 1,
                                ),
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
                                child: Text(
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
                  const SizedBox(width: 8),
                  if (image != null)
                    SizedBox.square(
                      key: const Key('quote-message-image'),
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
          ],
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
    return InkWell(
      onTap: userId.isEmpty || onOpenUser == null
          ? null
          : () => onOpenUser!(userId),
      child: ConstrainedBox(
        constraints: const BoxConstraints(minWidth: 180, maxWidth: 300),
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
                          message.sharedUserFullName ?? '',
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            color: context.theme.text,
                            fontSize: 16,
                          ),
                        ),
                      ),
                      if (message.sharedUserIsVerified)
                        Padding(
                          padding: const EdgeInsets.only(left: 3),
                          child: Icon(
                            Icons.verified,
                            color: context.theme.accent,
                            size: 14,
                          ),
                        ),
                      if ((message.sharedUserAppId ?? '').isNotEmpty)
                        Padding(
                          padding: const EdgeInsets.only(left: 3),
                          child: Icon(
                            Icons.smart_toy_outlined,
                            color: context.theme.accent,
                            size: 14,
                          ),
                        ),
                    ],
                  ),
                  Text(
                    message.sharedUserIdentityNumber ?? '',
                    style: TextStyle(
                      color: context.theme.secondaryText,
                      fontSize: 14,
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
    return InkWell(
      onTap: onOpenUri == null ? null : () => onOpenUri!(uri),
      child: SizedBox(
        width: 260,
        height: 180,
        child: Stack(
          fit: StackFit.expand,
          children: [
            const _StaticMapPlaceholder(),
            map.MapLayout(
              controller: map.MapController(
                location: LatLng.degree(location.latitude, location.longitude),
              ),
              builder: (context, transformer) => map.TileLayer(
                builder: (context, x, y, z) {
                  final url =
                      'https://www.google.com/maps/vt/pb=!1m4!1m3!1i$z!2i$x!3i$y!2m3!1e0!2sm!3i420120488!3m7!2sen!5e1105!12m4!1e68!2m2!1sset!2sRoadmap!4e0!5m1!1e0!23i4111425';
                  return Image.network(
                    url,
                    fit: BoxFit.cover,
                    errorBuilder: (_, _, _) => const SizedBox.expand(),
                  );
                },
              ),
            ),
            Center(
              child: Padding(
                padding: const EdgeInsets.only(bottom: 16.5),
                child: Icon(
                  Icons.location_on,
                  color: context.theme.red,
                  size: 36,
                ),
              ),
            ),
            if (location.label.isNotEmpty)
              Align(
                alignment: Alignment.bottomCenter,
                child: Container(
                  width: double.infinity,
                  color: context.theme.primary.withValues(alpha: 0.92),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 7,
                  ),
                  child: Text(
                    location.label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(color: context.theme.text, fontSize: 12),
                  ),
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
    super.key,
  });

  final MessageListEntry message;
  final MessageStringCallback? onAction;

  @override
  Widget build(BuildContext context) {
    final actions = _ActionData.parseList(message.content);
    if (actions == null) {
      return UnknownSpecialMessage(category: message.category);
    }
    return ConstrainedBox(
      constraints: const BoxConstraints(minWidth: 240, maxWidth: 340),
      child: _ActionButtonLayout(
        children: actions
            .map((action) => _ActionButton(action: action, onAction: onAction))
            .toList(growable: false),
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
    required this.dateAndStatus,
    super.key,
  });

  final MessageListEntry message;
  final MessageStringCallback? onAction;
  final bool isCurrentUser;
  final bool showNip;
  final bool highlighted;
  final Widget dateAndStatus;

  @override
  Widget build(BuildContext context) {
    final card = _AppCardData.parse(message.content);
    if (card == null) {
      return MessageBubble(
        isCurrentUser: isCurrentUser,
        showNip: showNip,
        highlighted: highlighted,
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
        dateAndStatus: dateAndStatus,
      );
    }
    return MessageBubble(
      key: Key('app-card-compact-${message.id}'),
      isCurrentUser: isCurrentUser,
      showNip: showNip,
      highlighted: highlighted,
      outerTimeAndStatusWidget: dateAndStatus,
      child: InkWell(
        onTap: onAction == null ? null : () => onAction!(card.action),
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
    required this.dateAndStatus,
  });

  final String messageId;
  final _AppCardData card;
  final MessageStringCallback? onAction;
  final bool isCurrentUser;
  final bool showNip;
  final bool highlighted;
  final Widget dateAndStatus;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final availableWidth = constraints.maxWidth.isFinite
          ? constraints.maxWidth
          : 750.0;
      final width = (availableWidth * 0.5)
          .clamp(
            math.min(320.0, availableWidth),
            math.min(375.0, availableWidth),
          )
          .toDouble();
      return Align(
        alignment: isCurrentUser ? Alignment.centerRight : Alignment.centerLeft,
        child: Column(
          key: Key('app-card-actions-message-$messageId'),
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            SizedBox(
              key: Key('app-card-body-$messageId'),
              width: width,
              child: MessageBubble(
                isCurrentUser: isCurrentUser,
                showNip: showNip,
                highlighted: highlighted,
                padding: EdgeInsets.zero,
                child: SelectionArea(
                  contextMenuBuilder: (context, selectableState) =>
                      const SizedBox.shrink(),
                  child: _ActionsCardBody(card: card),
                ),
              ),
            ),
            if (card.actions.isNotEmpty) ...[
              const SizedBox(height: 8),
              MessageBubbleNipPadding(
                currentUser: isCurrentUser,
                child: SizedBox(
                  key: Key('app-card-actions-$messageId'),
                  width: width,
                  child: _ActionButtonLayout(
                    children: card.actions
                        .map(
                          (action) =>
                              _ActionButton(action: action, onAction: onAction),
                        )
                        .toList(growable: false),
                  ),
                ),
              ),
            ],
            Padding(
              padding: const EdgeInsets.only(right: 10, left: 10, top: 4),
              child: dateAndStatus,
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
    required this.overlayDateAndStatus,
    required this.onOpenTranscript,
    super.key,
  });

  final MessageListEntry message;
  final Widget overlayDateAndStatus;
  final MessageStringCallback? onOpenTranscript;

  @override
  Widget build(BuildContext context) {
    final lines = _TranscriptLine.parseList(message.content);
    if (lines == null) return UnknownSpecialMessage(category: message.category);
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    return InkWell(
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
                  padding: const EdgeInsets.only(left: 8, bottom: 4),
                  child: Row(
                    children: [
                      Text(
                        l10n?.transcript ?? 'Transcript',
                        style: TextStyle(
                          color: context.theme.text,
                          fontSize: 16,
                        ),
                      ),
                      const Spacer(),
                      Container(
                        decoration: const BoxDecoration(
                          color: Color.fromRGBO(0, 0, 0, 0.2),
                          borderRadius: BorderRadius.all(Radius.circular(8)),
                        ),
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
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      for (final line in lines.take(4))
                        Text(
                          '${line.name}: ${line.preview}',
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
            ),
            Positioned(
              right: 2,
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
    return InkWell(
      onTap: id == null || onOpenSnapshot == null
          ? null
          : () => onOpenSnapshot!(id),
      child: ConstrainedBox(
        constraints: BoxConstraints(
          minWidth: kind == SnapshotKind.safeInscription ? 260 : 200,
          maxWidth: 300,
        ),
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
                  _AssetIcon(
                    iconUrl: message.snapshotAssetIconUrl,
                    chainIconUrl: message.snapshotChainIconUrl,
                    size: 40,
                  ),
                  const SizedBox(width: 8),
                  Flexible(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(
                          amount,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            color: context.theme.text,
                            fontSize: 16,
                          ),
                        ),
                        Text(
                          symbol,
                          style: TextStyle(
                            color: context.theme.secondaryText,
                            fontSize: 12,
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
  Widget build(BuildContext context) {
    final text = _systemText(context, message, currentUserId);
    return SystemNoticeItem(text: text, maxLines: null);
  }
}

class PinMessageItem extends StatelessWidget {
  const PinMessageItem({required this.message, super.key});

  final MessageListEntry message;

  @override
  Widget build(BuildContext context) {
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations)!;
    final pin = _PinData.parse(message.content);
    final preview = pin?.preview ?? l10n.aMessage;
    return SystemNoticeItem(
      text: l10n.chatPinMessage(message.senderName, preview),
      maxLines: 1,
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
    return SystemNoticeItem(
      text: l10n?.messageE2ee ?? 'Messages are end-to-end encrypted',
      color: context.theme.encrypt,
      onTap: uri == null || onOpenUri == null ? null : () => onOpenUri!(uri),
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
          style: TextStyle(fontSize: 16, color: context.theme.text),
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

class SystemNoticeItem extends StatelessWidget {
  const SystemNoticeItem({
    required this.text,
    super.key,
    this.color,
    this.onTap,
    this.maxLines = 2,
  });

  final String text;
  final Color? color;
  final VoidCallback? onTap;
  final int? maxLines;

  @override
  Widget build(BuildContext context) => Center(
    child: InkWell(
      onTap: onTap,
      borderRadius: const BorderRadius.all(Radius.circular(10)),
      child: Container(
        constraints: const BoxConstraints(maxWidth: 400),
        padding: const EdgeInsets.symmetric(vertical: 5, horizontal: 10),
        decoration: BoxDecoration(
          color:
              color ??
              context.dynamicColor(
                const Color.fromRGBO(202, 234, 201, 1),
                darkColor: const Color.fromRGBO(72, 94, 76, 1),
              ),
          borderRadius: const BorderRadius.all(Radius.circular(10)),
        ),
        child: Text(
          text,
          maxLines: maxLines,
          overflow: maxLines == null
              ? TextOverflow.clip
              : TextOverflow.ellipsis,
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 14, color: context.theme.text),
        ),
      ),
    ),
  );
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
  const _ActionButton({required this.action, required this.onAction});

  final _ActionData action;
  final MessageStringCallback? onAction;

  @override
  Widget build(BuildContext context) => Material(
    color: context.theme.primary,
    borderRadius: const BorderRadius.all(Radius.circular(8)),
    child: InkWell(
      borderRadius: const BorderRadius.all(Radius.circular(8)),
      onTap: onAction == null ? null : () => onAction!(action.action),
      child: IntrinsicWidth(
        child: Stack(
          fit: StackFit.passthrough,
          children: [
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
              child: Text(
                action.label,
                textAlign: TextAlign.center,
                style: TextStyle(color: action.color, fontSize: 14, height: 1),
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
  );
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
  const _ActionsCardBody({required this.card});

  final _AppCardData card;

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
        child: Text(
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
        child: Text(
          card.description,
          style: TextStyle(color: context.theme.text, fontSize: 16),
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
  Widget build(BuildContext context) => ConstrainedBox(
    constraints: const BoxConstraints(minWidth: 180, maxWidth: 300),
    child: Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        ClipRRect(
          borderRadius: const BorderRadius.all(Radius.circular(4)),
          child: card.iconUrl.isEmpty
              ? Container(
                  width: 40,
                  height: 40,
                  color: context.theme.statusBackground,
                  child: const Icon(Icons.apps),
                )
              : Image.network(
                  card.iconUrl,
                  width: 40,
                  height: 40,
                  fit: BoxFit.cover,
                  errorBuilder: (_, _, _) =>
                      const SizedBox.square(dimension: 40),
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
    ),
  );
}

class _StaticMapPlaceholder extends StatelessWidget {
  const _StaticMapPlaceholder();

  @override
  Widget build(BuildContext context) => ColoredBox(
    color: context.theme.background,
    child: CustomPaint(painter: _MapPainter(color: context.theme.divider)),
  );
}

class _MapPainter extends CustomPainter {
  const _MapPainter({required this.color});
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..strokeWidth = 1;
    for (var x = 18.0; x < size.width; x += 34) {
      canvas.drawLine(Offset(x, 0), Offset(x + 50, size.height), paint);
    }
    for (var y = 22.0; y < size.height; y += 32) {
      canvas.drawLine(Offset(0, y), Offset(size.width, y - 18), paint);
    }
  }

  @override
  bool shouldRepaint(covariant _MapPainter oldDelegate) =>
      oldDelegate.color != color;
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
  Widget build(BuildContext context) => SizedBox(
    width: 162,
    height: 36,
    child: OutlinedButton(
      onPressed: onAction == null ? null : () => onAction!(action),
      child: Text(text),
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
        _InscriptionContent(
          contentType: message.inscriptionContentType ?? data.contentType,
          contentUrl: message.inscriptionContentUrl ?? data.contentUrl,
        ),
        Expanded(
          child: Padding(
            padding: const EdgeInsets.all(10),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  message.inscriptionName ?? data.name,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
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
                    Icon(
                      Icons.hexagon_outlined,
                      size: 22,
                      color: context.theme.accent,
                    ),
                    const Spacer(),
                    _AssetIcon(iconUrl: message.inscriptionIconUrl, size: 22),
                  ],
                ),
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
          child: ClipOval(
            child: iconUrl?.isNotEmpty == true
                ? Image.network(
                    iconUrl!,
                    fit: BoxFit.cover,
                    errorBuilder: (_, _, _) => _assetPlaceholder(context),
                  )
                : _assetPlaceholder(context),
          ),
        ),
        if (chainIconUrl?.isNotEmpty == true)
          Positioned(
            right: 0,
            bottom: 0,
            child: Container(
              width: size * 0.34,
              height: size * 0.34,
              padding: const EdgeInsets.all(1),
              decoration: BoxDecoration(
                color: context.theme.primary,
                shape: BoxShape.circle,
              ),
              child: ClipOval(
                child: Image.network(chainIconUrl!, fit: BoxFit.cover),
              ),
            ),
          ),
      ],
    ),
  );

  Widget _assetPlaceholder(BuildContext context) => ColoredBox(
    color: context.theme.statusBackground,
    child: Icon(Icons.paid_outlined, size: size * 0.55),
  );
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
    child: Padding(
      padding: const EdgeInsets.all(4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              _AssetIcon(iconUrl: iconUrl, size: 16),
              const SizedBox(width: 4),
              Text(
                symbol,
                style: TextStyle(color: context.theme.text, fontSize: 13),
              ),
            ],
          ),
          const SizedBox(height: 16),
          Text(
            amount,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(
              color: context.theme.text,
              fontSize: 30,
              height: 1,
              fontWeight: FontWeight.w500,
            ),
          ),
          if (memo.isNotEmpty) ...[
            const SizedBox(height: 10),
            Text(
              memo,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                color: context.theme.secondaryText,
                fontSize: 12,
              ),
            ),
          ] else
            const SizedBox(height: 6),
        ],
      ),
    ),
  );
}

class _InscriptionContent extends StatelessWidget {
  const _InscriptionContent({this.contentType, this.contentUrl});

  final String? contentType;
  final String? contentUrl;

  @override
  Widget build(BuildContext context) {
    final url = contentUrl;
    if (url != null &&
        url.isNotEmpty &&
        contentType?.startsWith('image/') == true) {
      return Image.network(
        url,
        width: 112,
        height: 112,
        fit: BoxFit.cover,
        errorBuilder: (_, _, _) => _placeholder(context),
      );
    }
    return _placeholder(context);
  }

  Widget _placeholder(BuildContext context) => Container(
    width: 112,
    height: 112,
    color: context.theme.statusBackground,
    child: Icon(Icons.hexagon_outlined, size: 46, color: context.theme.accent),
  );
}

class _QuoteData {
  const _QuoteData({
    required this.sender,
    required this.category,
    required this.content,
    required this.mediaName,
    required this.mediaUrl,
    required this.thumbUrl,
    required this.thumbImage,
    required this.stickerAssetUrl,
    required this.sharedUserId,
    required this.sharedUserName,
    required this.sharedUserAvatarUrl,
    required this.sharedUserIdentityNumber,
  });

  final String sender;
  final String category;
  final String content;
  final String? mediaName;
  final String? mediaUrl;
  final String? thumbUrl;
  final String? thumbImage;
  final String? stickerAssetUrl;
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

  String preview(AppLocalizations? l10n) {
    if (category.endsWith('_TEXT') || category.endsWith('_POST')) {
      return content;
    }
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
        sender: json['user_full_name']?.toString() ?? '',
        category: json['type']?.toString() ?? '',
        content: json['content']?.toString() ?? '',
        mediaName: _nonEmpty(json['media_name']),
        mediaUrl: _nonEmpty(json['media_url']),
        thumbUrl: _nonEmpty(json['thumb_url']),
        thumbImage: _nonEmpty(json['thumb_image']),
        stickerAssetUrl: _nonEmpty(json['asset_url']),
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

class _LocationData {
  const _LocationData(this.latitude, this.longitude, this.name, this.address);
  final double latitude;
  final double longitude;
  final String name;
  final String address;
  String get label => address.isNotEmpty ? address : name;
  Uri get googleMapsUri => Uri.parse(
    address.isEmpty
        ? 'https://www.google.com/maps/place/@$latitude,$longitude,17z'
        : 'https://www.google.com/maps/search/${Uri.encodeComponent(address)}/@$latitude,$longitude,17z',
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

  static List<_TranscriptLine>? parseList(String raw) {
    try {
      final json = jsonDecode(raw) as List<dynamic>;
      return json.map((item) {
        final data = item as Map<String, dynamic>;
        return _TranscriptLine(
          data['name']?.toString() ?? '',
          _categoryPreview(
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
        name: json['name']?.toString() ?? 'Inscription',
        contentType: _nonEmpty(json['content_type']),
        contentUrl: _nonEmpty(json['content_url']),
      );
    } on Object {
      return const _SnapshotData(
        id: null,
        amount: '',
        symbol: '',
        name: 'Inscription',
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

class _PinData {
  const _PinData(this.messageId, this.preview);
  final String? messageId;
  final String preview;

  static _PinData? parse(String raw) {
    try {
      final json = jsonDecode(raw) as Map<String, dynamic>;
      return _PinData(
        _nonEmpty(json['message_id']),
        _categoryPreview(
          json['type']?.toString() ?? '',
          json['content']?.toString() ?? '',
        ),
      );
    } on Object {
      return null;
    }
  }
}

String _systemText(
  BuildContext context,
  MessageListEntry message,
  String currentUserId,
) {
  final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  final participant = message.participantId == currentUserId
      ? l10n.you
      : message.participantFullName ?? '';
  final sender = message.senderId == currentUserId
      ? l10n.you
      : message.senderName;
  return switch (message.action?.toUpperCase()) {
    'JOIN' => l10n.chatGroupJoin(participant),
    'EXIT' => l10n.chatGroupExit(participant),
    'ADD' => l10n.chatGroupAdd(sender, participant),
    'REMOVE' => l10n.chatGroupRemove(sender, participant),
    'CREATE' => l10n.createdThisGroup(sender),
    'ROLE' => l10n.nowAnAddmin(participant),
    'EXPIRE' => _expireSystemText(l10n, sender, message.expireIn),
    'UPDATE' => l10n.messageNotSupport,
    _ => l10n.messageNotSupport,
  };
}

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
  String category,
  String content, {
  String? mediaName,
  String? sharedName,
}) {
  if (category.endsWith('_TEXT') || category.endsWith('_POST')) return content;
  if (category.endsWith('_IMAGE')) return '[Photo]';
  if (category.endsWith('_VIDEO')) return '[Video]';
  if (category.endsWith('_AUDIO')) return '[Audio]';
  if (category.endsWith('_DATA')) {
    return mediaName?.isNotEmpty == true ? mediaName! : '[File]';
  }
  if (category.endsWith('_STICKER')) return '[Sticker]';
  if (category.endsWith('_CONTACT')) {
    return sharedName?.isNotEmpty == true ? sharedName! : '[Contact]';
  }
  if (category.endsWith('_LOCATION')) return '[Location]';
  if (category.endsWith('_TRANSCRIPT')) return '[Transcript]';
  return content.isEmpty ? '[Message]' : content;
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
