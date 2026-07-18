import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:intl/intl.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart'
    show UserProfileItem;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/badges_widget.dart';
import 'package:mixin_desktop_ui/widgets/high_light_text.dart';
import 'package:mixin_desktop_ui/widgets/interactive_decorated_box.dart';
import 'package:mixin_desktop_ui/widgets/message_selectable_text.dart';

const _defaultLimit = 3;

enum _ShowMoreType { contact, conversation, message }

class ConversationSearchResults extends StatefulWidget {
  const ConversationSearchResults({
    required this.keyword,
    required this.users,
    required this.maoUser,
    required this.mao,
    required this.conversations,
    required this.messages,
    required this.messageConversations,
    required this.loadingMessages,
    required this.mentionNames,
    required this.onConversationSelected,
    required this.onMessageSelected,
    required this.onSearchUser,
    required this.onUserSelected,
    required this.onMaoBotOpen,
    required this.onOpenLink,
    required this.onClear,
    super.key,
  });

  final String keyword;
  final List<UserProfileItem> users;
  final UserProfileItem? maoUser;
  final String? mao;
  final List<ConversationListEntry> conversations;
  final List<MessageListEntry> messages;
  final Map<String, ConversationListEntry> messageConversations;
  final bool loadingMessages;
  final Map<String, String> mentionNames;
  final ValueChanged<ConversationListEntry> onConversationSelected;
  final ValueChanged<MessageListEntry> onMessageSelected;
  final ValueChanged<String> onSearchUser;
  final ValueChanged<UserProfileItem> onUserSelected;
  final ValueChanged<UserProfileItem> onMaoBotOpen;
  final ValueChanged<Uri> onOpenLink;
  final VoidCallback onClear;

  @override
  State<ConversationSearchResults> createState() =>
      _ConversationSearchResultsState();
}

class _ConversationSearchResultsState extends State<ConversationSearchResults> {
  _ShowMoreType? type;

  @override
  void didUpdateWidget(covariant ConversationSearchResults oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.keyword != widget.keyword) type = null;
  }

  @override
  Widget build(BuildContext context) {
    final keyword = widget.keyword.trim();
    final uri = Uri.tryParse(keyword);
    final isUrl =
        uri != null && (uri.scheme == 'http' || uri.scheme == 'https');
    final isMixinNumber = RegExp(r'^\+?\d+$').hasMatch(keyword);

    if (type == _ShowMoreType.message) {
      return Column(
        children: [
          _SearchHeader(
            title: context.l10n.messages,
            showMore: true,
            more: false,
            onTap: () => setState(() => type = null),
          ),
          Expanded(
            child: ListView.builder(
              itemCount: widget.messages.length,
              itemBuilder: (context, index) =>
                  _messageItem(widget.messages[index]),
            ),
          ),
        ],
      );
    }

    final empty =
        widget.maoUser == null &&
        widget.users.isEmpty &&
        widget.conversations.isEmpty &&
        widget.messages.isEmpty &&
        !isUrl &&
        !isMixinNumber;
    if (empty && !widget.loadingMessages) {
      return _SearchEmpty(onClear: widget.onClear);
    }

    return CustomScrollView(
      slivers: [
        if (widget.maoUser case final maoUser?)
          SliverToBoxAdapter(child: _maoUserItem(maoUser)),
        if (isUrl)
          SliverToBoxAdapter(
            child: _SearchItem(
              name: context.l10n.openLink(keyword),
              keyword: keyword,
              maxLines: true,
              onTap: () => widget.onOpenLink(uri),
            ),
          ),
        if (isMixinNumber)
          SliverToBoxAdapter(
            child: _SearchItem(
              name: '${context.l10n.searchPlaceholderNumber}$keyword',
              keyword: keyword,
              maxLines: true,
              onTap: () => widget.onSearchUser(keyword),
            ),
          ),
        ..._userSection(
          context,
          title: context.l10n.contact,
          items: widget.users,
          sectionType: _ShowMoreType.contact,
        ),
        ..._conversationSection(
          context,
          title: context.l10n.conversation,
          items: widget.conversations,
          sectionType: _ShowMoreType.conversation,
        ),
        if (widget.messages.isNotEmpty)
          SliverToBoxAdapter(
            child: _SearchHeader(
              title: context.l10n.messages,
              showMore: widget.messages.length > _defaultLimit,
              more: type != _ShowMoreType.message,
              onTap: () => setState(() => type = _ShowMoreType.message),
            ),
          ),
        if (widget.messages.isNotEmpty)
          SliverList(
            delegate: SliverChildBuilderDelegate(
              (context, index) => _messageItem(widget.messages[index]),
              childCount: min(widget.messages.length, _defaultLimit),
            ),
          ),
        if (widget.messages.isNotEmpty)
          const SliverToBoxAdapter(child: SizedBox(height: 10)),
        if (widget.loadingMessages)
          SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 24),
              child: Center(
                child: CircularProgressIndicator(
                  strokeWidth: 2,
                  color: context.mixinTheme.accent,
                ),
              ),
            ),
          ),
      ],
    );
  }

  Widget _maoUserItem(UserProfileItem user) {
    const maoIcon =
        'https://kernel.mixin.dev/objects/fe75a8e48aeffb486df622c91bebfe4056ada7009f3151fb49e2a18340bbd615/icon';
    final placeholder = SvgPicture.asset(
      MixinAssets.inscriptionPlaceholder,
      width: 14,
      height: 14,
    );
    return _SearchItem(
      avatar: AvatarView(
        userId: user.userId,
        name: user.fullName,
        avatarUrl: user.avatarUrl,
        size: 50,
      ),
      name: user.fullName,
      description: widget.mao ?? '',
      descriptionIconWidget: ClipOval(
        child: Image.network(
          maoIcon,
          width: 14,
          height: 14,
          errorBuilder: (_, _, _) => placeholder,
        ),
      ),
      trailing: BadgesWidget(
        verified: user.isVerified,
        isBot: user.isBot,
        membership: user.membership,
      ),
      contentTrailing: user.isBot
          ? ElevatedButton(
              style: ElevatedButton.styleFrom(
                backgroundColor: context.mixinTheme.accent,
                foregroundColor: Colors.white,
                textStyle: const TextStyle(fontSize: 12),
                minimumSize: Size.zero,
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 12,
                ),
              ),
              onPressed: () => widget.onMaoBotOpen(user),
              child: Text(context.l10n.open),
            )
          : null,
      keyword: widget.keyword,
      onTap: () => widget.onUserSelected(user),
    );
  }

  List<Widget> _userSection(
    BuildContext context, {
    required String title,
    required List<UserProfileItem> items,
    required _ShowMoreType sectionType,
  }) {
    if (items.isEmpty) return const [];
    final expanded = type == sectionType;
    return [
      SliverToBoxAdapter(
        child: _SearchHeader(
          title: title,
          showMore: items.length > _defaultLimit,
          more: !expanded,
          onTap: () => setState(() => type = expanded ? null : sectionType),
        ),
      ),
      SliverList(
        delegate: SliverChildBuilderDelegate(
          (context, index) {
            final user = items[index];
            return _SearchItem(
              avatar: AvatarView(
                userId: user.userId,
                name: user.fullName,
                avatarUrl: user.avatarUrl,
                size: 50,
              ),
              name: user.fullName,
              description: context.l10n.contactMixinId(user.identityNumber),
              trailing: BadgesWidget(
                verified: user.isVerified,
                isBot: user.isBot,
                membership: user.membership,
              ),
              keyword: widget.keyword,
              onTap: () => widget.onUserSelected(user),
            );
          },
          childCount: expanded
              ? items.length
              : min(items.length, _defaultLimit),
        ),
      ),
      const SliverToBoxAdapter(child: SizedBox(height: 10)),
    ];
  }

  List<Widget> _conversationSection(
    BuildContext context, {
    required String title,
    required List<ConversationListEntry> items,
    required _ShowMoreType sectionType,
  }) {
    if (items.isEmpty) return const [];
    final expanded = type == sectionType;
    return [
      SliverToBoxAdapter(
        child: _SearchHeader(
          title: title,
          showMore: items.length > _defaultLimit,
          more: !expanded,
          onTap: () => setState(() => type = expanded ? null : sectionType),
        ),
      ),
      SliverList(
        delegate: SliverChildBuilderDelegate(
          (context, index) {
            final conversation = items[index];
            return _SearchItem(
              avatar: ConversationAvatarView(
                conversation: conversation,
                size: 50,
              ),
              name: conversation.name,
              description: replaceMessageMentions(
                _conversationDescription(context, conversation),
                widget.mentionNames,
              ),
              trailing: BadgesWidget(
                verified: conversation.isVerified,
                isBot: conversation.isBot,
                membership: conversation.membership,
              ),
              keyword: widget.keyword,
              onTap: () => widget.onConversationSelected(conversation),
            );
          },
          childCount: expanded
              ? items.length
              : min(items.length, _defaultLimit),
        ),
      ),
      const SliverToBoxAdapter(child: SizedBox(height: 10)),
    ];
  }

  Widget _messageItem(MessageListEntry message) {
    final conversation = widget.messageConversations[message.conversationId];
    return _SearchItem(
      avatar: conversation == null
          ? AvatarView(
              userId: message.senderId,
              name: message.senderName,
              avatarUrl: message.senderAvatarUrl,
              size: 50,
            )
          : ConversationAvatarView(conversation: conversation, size: 50),
      name: conversation?.name ?? message.senderName,
      keyword: widget.keyword,
      nameHighlight: false,
      descriptionIcon: MixinAssets.messageIcon(message.category),
      description: replaceMessageMentions(
        _messageDescription(message),
        widget.mentionNames,
      ),
      date: message.createdAt,
      onTap: () => widget.onMessageSelected(message),
    );
  }
}

class _SearchHeader extends StatelessWidget {
  const _SearchHeader({
    required this.title,
    required this.showMore,
    required this.onTap,
    required this.more,
  });

  final String title;
  final bool showMore;
  final VoidCallback onTap;
  final bool more;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.only(top: 16, bottom: 10, right: 20, left: 12),
    child: Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(
          title,
          style: TextStyle(fontSize: 14, color: context.mixinTheme.text),
        ),
        if (showMore)
          GestureDetector(
            onTap: onTap,
            child: Text(
              more ? context.l10n.more : context.l10n.less,
              style: TextStyle(fontSize: 14, color: context.mixinTheme.accent),
            ),
          ),
      ],
    ),
  );
}

class _SearchItem extends StatelessWidget {
  const _SearchItem({
    required this.name,
    required this.keyword,
    required this.onTap,
    this.avatar,
    this.nameHighlight = true,
    this.description,
    this.descriptionIcon,
    this.descriptionIconWidget,
    this.date,
    this.trailing,
    this.contentTrailing,
    this.maxLines = false,
  });

  final Widget? avatar;
  final Widget? trailing;
  final Widget? contentTrailing;
  final String name;
  final String keyword;
  final bool nameHighlight;
  final VoidCallback onTap;
  final String? description;
  final String? descriptionIcon;
  final Widget? descriptionIconWidget;
  final DateTime? date;
  final bool maxLines;

  @override
  Widget build(BuildContext context) {
    final selectedDecoration = BoxDecoration(
      borderRadius: const BorderRadius.all(Radius.circular(8)),
      color: context.mixinTheme.listSelected,
    );
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 6),
      child: InteractiveDecoratedBox(
        decoration: const BoxDecoration(),
        hoveringDecoration: selectedDecoration,
        onTap: onTap,
        child: Container(
          constraints: const BoxConstraints(minHeight: 72),
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 12),
          child: Row(
            children: [
              if (avatar != null) SizedBox.square(dimension: 50, child: avatar),
              if (avatar != null) const SizedBox(width: 12),
              Expanded(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Expanded(
                          child: Row(
                            children: [
                              Flexible(
                                child: _HighlightedText(
                                  text: name,
                                  query: nameHighlight ? keyword : '',
                                  maxLines: maxLines ? null : 1,
                                ),
                              ),
                              ?trailing,
                            ],
                          ),
                        ),
                        if (date != null)
                          Text(
                            DateFormat.Hm().format(date!),
                            style: TextStyle(
                              color: context.mixinTheme.secondaryText,
                              fontSize: 12,
                            ),
                          ),
                      ],
                    ),
                    if (description != null)
                      Row(
                        children: [
                          if (descriptionIcon != null ||
                              descriptionIconWidget != null)
                            Padding(
                              padding: const EdgeInsets.only(right: 2),
                              child:
                                  descriptionIconWidget ??
                                  SvgPicture.asset(
                                    descriptionIcon!,
                                    width: 14,
                                    height: 14,
                                    colorFilter: ColorFilter.mode(
                                      context.mixinTheme.secondaryText,
                                      BlendMode.srcIn,
                                    ),
                                  ),
                            ),
                          Expanded(
                            child: DefaultTextStyle(
                              style: TextStyle(
                                color: context.mixinTheme.secondaryText,
                                fontSize: 14,
                              ),
                              child: _HighlightedText(
                                text: description!,
                                query: keyword,
                                maxLines: 1,
                              ),
                            ),
                          ),
                        ],
                      ),
                  ],
                ),
              ),
              ?contentTrailing,
            ],
          ),
        ),
      ),
    );
  }
}

class _HighlightedText extends StatelessWidget {
  const _HighlightedText({
    required this.text,
    required this.query,
    required this.maxLines,
  });

  final String text;
  final String query;
  final int? maxLines;

  @override
  Widget build(BuildContext context) {
    final style = DefaultTextStyle.of(context).style.copyWith(
      color:
          DefaultTextStyle.of(context).style.color ?? context.mixinTheme.text,
      fontSize: DefaultTextStyle.of(context).style.fontSize ?? 16,
    );
    return CustomText(
      text,
      maxLines: maxLines,
      overflow: maxLines == null ? null : TextOverflow.ellipsis,
      textMatchers: [
        EmojiTextMatcher(),
        if (query.trim().isNotEmpty)
          MultiKeyWordTextMatcher.createKeywordMatcher(
            keyword: query,
            style: TextStyle(color: context.mixinTheme.accent),
            caseSensitive: false,
          ),
      ],
      style: style,
    );
  }
}

class _SearchEmpty extends StatelessWidget {
  const _SearchEmpty({required this.onClear});

  final VoidCallback onClear;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 43, vertical: 86),
    width: double.infinity,
    alignment: Alignment.topCenter,
    child: Column(
      children: [
        Text(
          context.l10n.searchEmpty,
          style: TextStyle(
            fontSize: 14,
            color: context.mixinTheme.secondaryText,
          ),
        ),
        const SizedBox(height: 4),
        TextButton(
          onPressed: onClear,
          child: Text(
            context.l10n.clearFilter,
            style: TextStyle(fontSize: 14, color: context.mixinTheme.accent),
          ),
        ),
      ],
    ),
  );
}

String _conversationDescription(
  BuildContext context,
  ConversationListEntry conversation,
) {
  if (conversation.contentType == null) return '';
  if (conversation.contentType!.contains('TEXT')) {
    return conversation.content;
  }
  if (conversation.contentType!.contains('IMAGE')) {
    return '[${context.l10n.image}]';
  }
  if (conversation.contentType!.contains('VIDEO')) {
    return '[${context.l10n.video}]';
  }
  if (conversation.contentType!.contains('AUDIO')) {
    return '[${context.l10n.audio}]';
  }
  if (conversation.contentType!.contains('STICKER')) {
    return '[${context.l10n.sticker}]';
  }
  if (conversation.contentType!.contains('DATA')) {
    return '[${context.l10n.file}]';
  }
  return conversation.content;
}

String _messageDescription(MessageListEntry message) {
  if (message.category.contains('TEXT') || message.category.contains('POST')) {
    return message.content;
  }
  return message.caption?.trim().isNotEmpty == true
      ? message.caption!.trim()
      : message.mediaName ?? message.content;
}
