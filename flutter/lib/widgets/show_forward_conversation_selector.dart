import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/adaptive_selection_toolbar.dart';
import 'package:mixin_desktop_ui/widgets/badges_widget.dart';
import 'package:mixin_desktop_ui/widgets/buttons.dart';
import 'package:mixin_desktop_ui/widgets/high_light_text.dart';
import 'package:mixin_desktop_ui/widgets/interactive_decorated_box.dart';
import 'package:mixin_desktop_ui/widgets/mixin_dialog.dart';

Future<ConversationListEntry?> showConversationSelector(
  BuildContext context, {
  required rust.AccountHandle account,
  required String title,
  ConversationCategoryFilter category = ConversationCategoryFilter.chats,
  Widget? action,
}) async {
  final result = await _showConversationSelector(
    context,
    account: account,
    title: title,
    category: category,
    multiple: false,
    action: action,
  );
  return result == null || result.isEmpty ? null : result.first;
}

Future<List<ConversationListEntry>?> showConversationMultiSelector(
  BuildContext context, {
  required rust.AccountHandle account,
  required String title,
  ConversationCategoryFilter category = ConversationCategoryFilter.contacts,
  String? initialCircleId,
  bool allowEmpty = false,
  String? confirmedText,
  Set<String> filteredOwnerIds = const {},
  int? maxSelect,
  Widget? action,
}) => _showConversationSelector(
  context,
  account: account,
  title: title,
  category: category,
  multiple: true,
  initialCircleId: initialCircleId,
  allowEmpty: allowEmpty,
  confirmedText: confirmedText,
  filteredOwnerIds: filteredOwnerIds,
  maxSelect: maxSelect,
  action: action,
);

Future<List<ConversationListEntry>?> _showConversationSelector(
  BuildContext context, {
  required rust.AccountHandle account,
  required String title,
  required ConversationCategoryFilter category,
  required bool multiple,
  String? initialCircleId,
  bool allowEmpty = false,
  String? confirmedText,
  Set<String> filteredOwnerIds = const {},
  int? maxSelect,
  Widget? action,
}) async {
  final selected = await showMixinDialog<List<ConversationListEntry>>(
    context: context,
    child: _ConversationSelector(
      account: account,
      title: title,
      category: category,
      multiple: multiple,
      initialCircleId: initialCircleId,
      allowEmpty: allowEmpty,
      confirmedText: confirmedText,
      filteredOwnerIds: filteredOwnerIds,
      maxSelect: maxSelect,
      action: action,
    ),
  );
  if (selected == null) return null;
  final result = <ConversationListEntry>[];
  for (final item in selected) {
    if (item.status != -1) {
      result.add(item);
      continue;
    }
    final conversationId = await account.conversation().openUserConversation(
      userId: item.ownerId,
    );
    result.add(_copyConversationWithId(item, conversationId));
  }
  return result;
}

class _ConversationSelector extends StatefulWidget {
  const _ConversationSelector({
    required this.account,
    required this.title,
    required this.category,
    required this.multiple,
    required this.initialCircleId,
    required this.allowEmpty,
    required this.confirmedText,
    required this.filteredOwnerIds,
    required this.maxSelect,
    required this.action,
  });

  final rust.AccountHandle account;
  final String title;
  final ConversationCategoryFilter category;
  final bool multiple;
  final String? initialCircleId;
  final bool allowEmpty;
  final String? confirmedText;
  final Set<String> filteredOwnerIds;
  final int? maxSelect;
  final Widget? action;

  @override
  State<_ConversationSelector> createState() => _ConversationSelectorState();
}

class _ConversationSelectorState extends State<_ConversationSelector> {
  late final Future<List<ConversationListEntry>> _conversations = _load();
  final Map<String, ConversationListEntry> _selected = {};
  String _query = '';

  Future<List<ConversationListEntry>> _load() async {
    final result = <ConversationListEntry>[];
    final categories = widget.category == ConversationCategoryFilter.contacts
        ? const [
            ConversationCategoryFilter.contacts,
            ConversationCategoryFilter.bots,
          ]
        : [widget.category];
    for (final category in categories) {
      final count = await widget.account.conversation().conversationCount(
        category: category.name,
        circleId: category == ConversationCategoryFilter.circle
            ? widget.initialCircleId
            : null,
        keyword: '',
        unseenOnly: false,
      );
      var offset = 0;
      while (offset < count.toInt()) {
        final page = await widget.account.conversation().conversations(
          category: category.name,
          circleId: category == ConversationCategoryFilter.circle
              ? widget.initialCircleId
              : null,
          keyword: '',
          unseenOnly: false,
          limit: 200,
          offset: offset,
        );
        result.addAll(page.map(ConversationListEntry.fromRust));
        offset += page.length;
        if (page.length < 200) break;
      }
    }
    if (widget.category == ConversationCategoryFilter.chats ||
        widget.category == ConversationCategoryFilter.contacts ||
        widget.category == ConversationCategoryFilter.bots) {
      final existingOwnerIds = result.map((item) => item.ownerId).toSet();
      final users = await widget.account.user().selectableUsers();
      result.addAll(
        users
            .where(
              (user) =>
                  !existingOwnerIds.contains(user.userId) &&
                  (widget.category != ConversationCategoryFilter.bots ||
                      user.isBot),
            )
            .map(_conversationFromUser),
      );
    }
    final initialCircleId = widget.initialCircleId;
    if (initialCircleId != null) {
      _selected.addEntries(
        result
            .where((item) => item.circleIds.contains(initialCircleId))
            .map((item) => MapEntry(item.id, item)),
      );
    }
    return result
        .where((item) => !widget.filteredOwnerIds.contains(item.ownerId))
        .toList(growable: false);
  }

  @override
  Widget build(BuildContext context) => Material(
    color: Colors.transparent,
    child: Container(
      width: 480,
      height: 600,
      padding: const EdgeInsets.only(top: 16),
      child: Column(
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Expanded(
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: MixinCloseButton(
                      onTap: () => Navigator.pop(context),
                    ),
                  ),
                ),
                Expanded(
                  child: Align(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(
                          widget.title,
                          style: TextStyle(
                            color: context.theme.text,
                            fontSize: 16,
                          ),
                        ),
                        if (widget.multiple)
                          FutureBuilder<List<ConversationListEntry>>(
                            future: _conversations,
                            builder: (context, snapshot) => Text(
                              '${_selected.length} / ${snapshot.data?.length ?? 0}',
                              style: TextStyle(
                                fontSize: 12,
                                color: context.theme.secondaryText,
                              ),
                            ),
                          ),
                      ],
                    ),
                  ),
                ),
                Expanded(
                  child: Align(
                    alignment: Alignment.centerRight,
                    child:
                        widget.action ??
                        (widget.multiple &&
                                (widget.allowEmpty || _selected.isNotEmpty)
                            ? MixinButton(
                                backgroundTransparent: true,
                                padding: const EdgeInsets.all(8),
                                onTap: () => Navigator.pop(
                                  context,
                                  _selected.values.toList(growable: false),
                                ),
                                child: Text(
                                  widget.confirmedText ?? context.l10n.next,
                                ),
                              )
                            : const SizedBox()),
                  ),
                ),
              ],
            ),
          ),
          Container(
            height: 32,
            padding: const EdgeInsets.symmetric(horizontal: 16),
            margin: const EdgeInsets.only(top: 8, right: 24, left: 24),
            decoration: BoxDecoration(
              color: context.theme.background,
              borderRadius: const BorderRadius.all(Radius.circular(16)),
            ),
            alignment: Alignment.center,
            child: Stack(
              children: [
                TextField(
                  onChanged: (value) => setState(() => _query = value.trim()),
                  style: TextStyle(color: context.theme.text, fontSize: 14),
                  inputFormatters: [LengthLimitingTextInputFormatter(200)],
                  autofocus: true,
                  scrollPadding: EdgeInsets.zero,
                  contextMenuBuilder: (context, state) =>
                      MixinAdaptiveSelectionToolbar(editableTextState: state),
                  decoration: InputDecoration(
                    prefixIcon: Padding(
                      padding: const EdgeInsetsDirectional.only(end: 8),
                      child: SvgPicture.asset(
                        MixinAssets.search,
                        colorFilter: ColorFilter.mode(
                          context.theme.secondaryText,
                          BlendMode.srcIn,
                        ),
                      ),
                    ),
                    prefixIconConstraints: const BoxConstraints(
                      minHeight: 16,
                      minWidth: 16,
                    ),
                    isDense: true,
                    focusedBorder: InputBorder.none,
                    enabledBorder: InputBorder.none,
                  ),
                ),
                if (_query.isEmpty)
                  IgnorePointer(
                    child: Padding(
                      padding: const EdgeInsets.only(left: 24, top: 7),
                      child: Text(
                        context.l10n.search,
                        style: TextStyle(
                          color: context.theme.secondaryText,
                          height: 1,
                        ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ),
              ],
            ),
          ),
          AnimatedSize(
            alignment: Alignment.topCenter,
            duration: const Duration(milliseconds: 200),
            child: !widget.multiple || _selected.isEmpty
                ? const SizedBox(height: 8)
                : SizedBox(
                    height: 120,
                    child: ListView.separated(
                      scrollDirection: Axis.horizontal,
                      padding: const EdgeInsets.symmetric(horizontal: 24),
                      itemCount: _selected.length,
                      separatorBuilder: (_, _) => const SizedBox(width: 4),
                      itemBuilder: (context, index) {
                        final conversation = _selected.values.elementAt(index);
                        return SizedBox(
                          width: 66,
                          child: Column(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              const SizedBox(height: 20),
                              Stack(
                                children: [
                                  ConversationAvatarView(
                                    conversation: conversation,
                                    size: 50,
                                  ),
                                  _AvatarSmallCloseIcon(
                                    onTap: () => _toggle(conversation),
                                  ),
                                ],
                              ),
                              const SizedBox(height: 10),
                              Text(
                                conversation.name,
                                style: TextStyle(
                                  fontSize: 14,
                                  color: context.theme.text,
                                ),
                                textAlign: TextAlign.center,
                                overflow: TextOverflow.ellipsis,
                                maxLines: 2,
                              ),
                            ],
                          ),
                        );
                      },
                    ),
                  ),
          ),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: FutureBuilder<List<ConversationListEntry>>(
                future: _conversations,
                builder: (context, snapshot) {
                  final query = _query.toLowerCase();
                  final conversations = (snapshot.data ?? const [])
                      .where(
                        (item) =>
                            query.isEmpty ||
                            item.name.toLowerCase().contains(query) ||
                            item.identityNumber.contains(query),
                      )
                      .toList(growable: false);
                  final sections = _sections(context, conversations);
                  return CustomScrollView(
                    slivers: [
                      for (final section in sections) ...[
                        SliverPersistentHeader(
                          pinned: true,
                          delegate: _SectionHeaderDelegate(title: section.$1),
                        ),
                        SliverList.builder(
                          itemCount: section.$2.length,
                          itemBuilder: (context, index) =>
                              _buildItem(context, section.$2[index]),
                        ),
                      ],
                    ],
                  );
                },
              ),
            ),
          ),
        ],
      ),
    ),
  );

  void _toggle(ConversationListEntry conversation) {
    setState(() {
      if (_selected.remove(conversation.id) == null) {
        if (widget.maxSelect != null && _selected.length >= widget.maxSelect!) {
          return;
        }
        _selected[conversation.id] = conversation;
      }
    });
  }

  List<(String, List<ConversationListEntry>)> _sections(
    BuildContext context,
    List<ConversationListEntry> conversations,
  ) {
    if (widget.category == ConversationCategoryFilter.chats) {
      final recent = conversations
          .where((item) => item.status != -1)
          .toList(growable: false);
      final friends = conversations
          .where((item) => item.status == -1 && !item.isBot)
          .toList(growable: false);
      final bots = conversations
          .where((item) => item.status == -1 && item.isBot)
          .toList(growable: false);
      return [
        (context.l10n.recentChats, recent),
        (context.l10n.contactTitle, friends),
        (context.l10n.bots, bots),
      ].where((section) => section.$2.isNotEmpty).toList(growable: false);
    }
    if (widget.category == ConversationCategoryFilter.contacts) {
      return [
        (
          context.l10n.contactTitle,
          conversations.where((item) => !item.isBot).toList(growable: false),
        ),
        (
          context.l10n.bots,
          conversations.where((item) => item.isBot).toList(growable: false),
        ),
      ].where((section) => section.$2.isNotEmpty).toList(growable: false);
    }
    final title = switch (widget.category) {
      ConversationCategoryFilter.groups => context.l10n.groups,
      ConversationCategoryFilter.bots => context.l10n.bots,
      ConversationCategoryFilter.strangers => context.l10n.strangers,
      _ => context.l10n.recentChats,
    };
    return conversations.isEmpty ? const [] : [(title, conversations)];
  }

  Widget _buildItem(BuildContext context, ConversationListEntry conversation) {
    final selected = _selected.containsKey(conversation.id);
    return InteractiveDecoratedBox.color(
      decoration: const BoxDecoration(
        borderRadius: BorderRadius.all(Radius.circular(8)),
      ),
      hoveringColor: context.theme.listSelected,
      onTap: widget.multiple
          ? () => _toggle(conversation)
          : () => Navigator.pop(context, [conversation]),
      child: Container(
        height: 70,
        padding: const EdgeInsets.only(
          top: 10,
          bottom: 10,
          left: 14,
          right: 10,
        ),
        child: Row(
          children: [
            if (widget.multiple)
              Padding(
                padding: const EdgeInsets.only(right: 20),
                child: ClipOval(
                  child: Container(
                    height: 16,
                    width: 16,
                    color: selected
                        ? context.theme.accent
                        : context.theme.secondaryText,
                    alignment: Alignment.center,
                    child: SvgPicture.asset(MixinAssets.selected),
                  ),
                ),
              ),
            ConversationAvatarView(conversation: conversation, size: 50),
            const SizedBox(width: 16),
            Expanded(
              child: Row(
                children: [
                  Flexible(
                    child: _HighlightedText(
                      text: conversation.name,
                      keyword: _query,
                    ),
                  ),
                  BadgesWidget(
                    verified: conversation.isVerified,
                    isBot: conversation.isBot,
                    membership: conversation.membership,
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

class _SectionHeaderDelegate extends SliverPersistentHeaderDelegate {
  const _SectionHeaderDelegate({required this.title});

  final String title;

  @override
  double get minExtent => 42;

  @override
  double get maxExtent => 42;

  @override
  Widget build(
    BuildContext context,
    double shrinkOffset,
    bool overlapsContent,
  ) => Container(
    color: context.dynamicColor(
      const Color.fromRGBO(255, 255, 255, 1),
      darkColor: const Color.fromRGBO(62, 65, 72, 1),
    ),
    padding: const EdgeInsets.only(top: 10, bottom: 10, left: 14),
    alignment: Alignment.centerLeft,
    child: Text(
      title,
      style: TextStyle(fontSize: 16, color: context.theme.text),
    ),
  );

  @override
  bool shouldRebuild(covariant _SectionHeaderDelegate oldDelegate) =>
      title != oldDelegate.title;
}

class _AvatarSmallCloseIcon extends StatelessWidget {
  const _AvatarSmallCloseIcon({required this.onTap});

  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => Positioned(
    top: 0,
    right: 0,
    child: GestureDetector(
      onTap: onTap,
      child: Container(
        width: 22,
        height: 22,
        decoration: BoxDecoration(
          color: context.theme.popUp,
          shape: BoxShape.circle,
        ),
        child: Center(
          child: Container(
            width: 16,
            height: 16,
            decoration: BoxDecoration(
              color: context.dynamicColor(
                darkMixinColors.divider,
                darkColor: const Color.fromRGBO(142, 141, 143, 1),
              ),
              shape: BoxShape.circle,
            ),
            child: Center(
              child: SvgPicture.asset(
                MixinAssets.smallClose,
                colorFilter: ColorFilter.mode(
                  Colors.white.withValues(alpha: 0.9),
                  BlendMode.srcIn,
                ),
              ),
            ),
          ),
        ),
      ),
    ),
  );
}

class _HighlightedText extends StatelessWidget {
  const _HighlightedText({required this.text, required this.keyword});

  final String text;
  final String keyword;

  @override
  Widget build(BuildContext context) => CustomText(
    text,
    maxLines: 1,
    overflow: TextOverflow.ellipsis,
    textMatchers: [
      EmojiTextMatcher(),
      if (keyword.trim().isNotEmpty)
        MultiKeyWordTextMatcher.createKeywordMatcher(
          keyword: keyword,
          style: TextStyle(color: context.theme.accent),
          caseSensitive: false,
        ),
    ],
    style: TextStyle(fontSize: 16, color: context.theme.text),
  );
}

ConversationListEntry _conversationFromUser(rust.UserProfileItem user) =>
    ConversationListEntry(
      id: 'pending:${user.userId}',
      ownerId: user.userId,
      name: user.fullName,
      avatarUrl: user.avatarUrl,
      category: 'CONTACT',
      draft: '',
      status: -1,
      content: '',
      contentType: null,
      messageStatus: null,
      senderId: null,
      senderName: null,
      updatedAt: DateTime.fromMillisecondsSinceEpoch(0),
      unseenCount: 0,
      mentionCount: 0,
      isMuted: false,
      isVerified: user.isVerified,
      isBot: user.isBot,
      membership: user.membership,
      isPinned: false,
      relationship: user.relationship,
      identityNumber: user.identityNumber,
      circleIds: const [],
      groupAvatars: const [],
    );

ConversationListEntry _copyConversationWithId(
  ConversationListEntry item,
  String conversationId,
) => ConversationListEntry(
  id: conversationId,
  ownerId: item.ownerId,
  name: item.name,
  avatarUrl: item.avatarUrl,
  category: item.category,
  draft: item.draft,
  status: 0,
  lastReadMessageId: item.lastReadMessageId,
  content: item.content,
  contentType: item.contentType,
  messageStatus: item.messageStatus,
  senderId: item.senderId,
  senderName: item.senderName,
  updatedAt: item.updatedAt,
  unseenCount: item.unseenCount,
  mentionCount: item.mentionCount,
  isMuted: item.isMuted,
  isVerified: item.isVerified,
  isBot: item.isBot,
  membership: item.membership,
  isPinned: item.isPinned,
  relationship: item.relationship,
  identityNumber: item.identityNumber,
  circleIds: item.circleIds,
  groupAvatars: item.groupAvatars,
  participantCount: item.participantCount,
);
