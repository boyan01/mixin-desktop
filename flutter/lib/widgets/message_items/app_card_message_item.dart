import 'dart:convert';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../models/message_list_entry.dart';
import '../../src/rust/desktop_api.dart' as rust;
import '../../theme.dart';
import '../../utils/web_view.dart';
import '../high_light_text.dart';
import '../interactive_decorated_box.dart';
import '../message_bubble.dart';
import '../message_layout.dart';
import '../message_selectable_text.dart';
import '../message_style.dart';
import 'app_button_group_message_item.dart';
import 'unknown_special_message.dart';

class AppCardMessageItem extends StatelessWidget {
  const AppCardMessageItem({
    required this.message,
    required this.isCurrentUser,
    required this.showNip,
    required this.highlighted,
    required this.dateAndStatus,
    this.highlightOpacity = 0,
    super.key,
    this.quote,
    this.mentionNames = const {},
    this.keyword = '',
    this.isPinnedPage = false,
    this.interactive = true,
  });

  final MessageListEntry message;
  final bool isCurrentUser;
  final bool showNip;
  final bool highlighted;
  final double highlightOpacity;
  final Widget dateAndStatus;
  final Widget? quote;
  final Map<String, String> mentionNames;
  final String keyword;
  final bool isPinnedPage;
  final bool interactive;

  @override
  Widget build(BuildContext context) {
    final card = AppCardData.parse(message.content);
    if (card == null) {
      return MessageBubble(
        isCurrentUser: isCurrentUser,
        showNip: showNip,
        highlighted: highlighted,
        highlightOpacity: highlightOpacity,
        quote: quote,
        isPinnedPage: isPinnedPage,
        pinnedMessageId: message.id,
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
        conversationId: message.conversationId,
        isCurrentUser: isCurrentUser,
        showNip: showNip,
        highlighted: highlighted,
        highlightOpacity: highlightOpacity,
        dateAndStatus: dateAndStatus,
        quote: quote,
        mentionNames: mentionNames,
        keyword: keyword,
        isPinnedPage: isPinnedPage,
        isDisappearingMessage: (message.expireIn ?? 0) > 0,
        interactive: interactive,
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
      pinnedMessageId: message.id,
      isDisappearingMessage: (message.expireIn ?? 0) > 0,
      outerTimeAndStatusWidget: dateAndStatus,
      child: InteractiveDecoratedBox(
        onTap: interactive
            ? () => openMessageAction(
                context: context,
                account: context.read<rust.AccountHandle>(),
                conversationId: message.conversationId,
                action: card.action,
                title: card.title,
              )
            : null,
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
    required this.conversationId,
    required this.isCurrentUser,
    required this.showNip,
    required this.highlighted,
    required this.highlightOpacity,
    required this.dateAndStatus,
    this.quote,
    this.mentionNames = const {},
    this.keyword = '',
    this.isPinnedPage = false,
    this.isDisappearingMessage = false,
    this.interactive = true,
  });

  final String messageId;
  final AppCardData card;
  final String conversationId;
  final bool isCurrentUser;
  final bool showNip;
  final bool highlighted;
  final double highlightOpacity;
  final Widget dateAndStatus;
  final Widget? quote;
  final Map<String, String> mentionNames;
  final String keyword;
  final bool isPinnedPage;
  final bool isDisappearingMessage;
  final bool interactive;

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
        quote: quote,
        isPinnedPage: isPinnedPage,
        pinnedMessageId: messageId,
        isDisappearingMessage: isDisappearingMessage,
        outerTimeAndStatusWidget: dateAndStatus,
        child: Column(
          key: Key('app-card-actions-message-$messageId'),
          children: [
            MessageBubble(
              isCurrentUser: isCurrentUser,
              showNip: true,
              highlighted: highlighted,
              highlightOpacity: highlightOpacity,
              padding: EdgeInsets.zero,
              clip: true,
              shrinkWrap: true,
              child: ConstrainedBox(
                key: Key('app-card-body-$messageId'),
                constraints: BoxConstraints(minWidth: width, maxWidth: width),
                child: SelectionArea(
                  contextMenuBuilder: (context, selectableState) =>
                      const SizedBox.shrink(),
                  child: _ActionsCardBody(
                    card: card,
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
                child: ActionButtonLayout(
                  children: card.actions
                      .map(
                        (action) => ActionButton(
                          action: action,
                          conversationId: conversationId,
                          highlighted: highlighted,
                          highlightOpacity: highlightOpacity,
                          interactive: interactive,
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

class _ActionsCardBody extends StatelessWidget {
  const _ActionsCardBody({
    required this.card,
    required this.mentionNames,
    required this.keyword,
  });

  final AppCardData card;
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

  final AppCardData card;

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

class AppCardData {
  const AppCardData(
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
  final AppCardCover? cover;
  final String title;
  final String description;
  final String action;
  final List<ActionData> actions;
  final bool shareable;

  static AppCardData? parse(String raw) {
    try {
      final json = jsonDecode(raw) as Map<String, dynamic>;
      final actions = (json['actions'] as List<dynamic>? ?? const [])
          .map((item) => ActionData.fromJson(item as Map<String, dynamic>))
          .toList();
      return AppCardData(
        json['icon_url']?.toString() ?? '',
        json['cover_url']?.toString() ?? '',
        AppCardCover.parse(json['cover']),
        json['title']?.toString() ?? '',
        json['description']?.toString() ?? '',
        json['action']?.toString() ?? '',
        actions,
        json['shareable'] is! bool || json['shareable'] as bool,
      );
    } on Object {
      return null;
    }
  }
}

class AppCardCover {
  const AppCardCover({
    required this.url,
    required this.width,
    required this.height,
  });

  final String url;
  final int width;
  final int height;

  double get aspectRatio => height <= 0 ? 1 : width / height;

  static AppCardCover? parse(Object? value) {
    if (value is! Map<String, dynamic>) return null;
    final url = value['url']?.toString().trim() ?? '';
    if (url.isEmpty) return null;
    return AppCardCover(
      url: url,
      width: (value['width'] as num?)?.toInt() ?? 0,
      height: (value['height'] as num?)?.toInt() ?? 0,
    );
  }
}
