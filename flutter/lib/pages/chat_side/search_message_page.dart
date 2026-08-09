import 'dart:async';
import 'dart:convert';

import 'package:equatable/equatable.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:intl/intl.dart';
import 'package:markdown/markdown.dart' as markdown;
import 'package:provider/provider.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';

import '../../constants/assets.dart';
import '../../controllers/chat_side_notifier.dart';
import '../../controllers/message_search_controller.dart';
import '../../l10n/l10n.dart';
import '../../models/message_list_entry.dart';
import '../../src/rust/desktop_api.dart' as rust;
import '../../theme.dart';
import '../../utils/app_logger.dart';
import '../../widgets/action_button.dart';
import '../../widgets/avatar_view.dart';
import '../../widgets/badges_widget.dart';
import '../../widgets/high_light_text.dart';
import '../../widgets/interactive_decorated_box.dart';
import '../../widgets/search_text_field.dart';
import '../../widgets/settings_widgets.dart';
import 'chat_side_scope.dart';

class SearchMessagePage extends HookWidget {
  const SearchMessagePage({super.key});

  @override
  Widget build(BuildContext context) {
    final categories = useMemoized(
      () => [
        _Category(context.l10n.text, const [
          'PLAIN_TEXT',
          'SIGNAL_TEXT',
          'ENCRYPTED_TEXT',
        ]),
        _Category(context.l10n.post, const [
          'PLAIN_POST',
          'SIGNAL_POST',
          'ENCRYPTED_POST',
        ]),
      ],
    );
    final selectedCategories = useState<List<String>?>(null);
    final focusNode = useFocusNode();
    final keywordNotifier = context.read<SearchConversationKeywordNotifier>();
    final editingController = useTextEditingController(
      text: keywordNotifier.value.$2,
    );
    final keywordIsEmpty = editingController.text.isEmpty;
    useListenable(editingController);
    final userMode = useState(false);
    final selectedUser = useState<rust.ConversationParticipantItem?>(null);
    final scope = ChatSideScope.of(context);

    useEffect(() {
      if (keywordIsEmpty) selectedCategories.value = null;
      return null;
    }, [keywordIsEmpty]);

    useEffect(() {
      var active = true;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (active && context.mounted) focusNode.requestFocus();
      });
      return () => active = false;
    }, [focusNode, userMode.value, selectedUser.value]);

    useEffect(() {
      SearchConversationKeywordNotifier.updateSelectedUser(
        context,
        selectedUser.value?.userId,
      );
      return null;
    }, [selectedUser.value]);

    void enableUserMode() {
      editingController.text = '';
      userMode.value = true;
    }

    return Scaffold(
      backgroundColor: context.theme.primary,
      appBar: MixinAppBar(
        title: Text(context.l10n.searchConversation),
        actions: [
          if (!Navigator.of(context).canPop())
            ActionButton(
              name: MixinAssets.close,
              color: context.theme.icon,
              onTap: scope.notifier.clear,
            ),
        ],
      ),
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
            child: Row(
              children: [
                Expanded(
                  child: FocusableActionDetector(
                    autofocus: true,
                    shortcuts: {
                      if (keywordIsEmpty &&
                          (userMode.value || selectedUser.value != null))
                        const SingleActivator(LogicalKeyboardKey.backspace):
                            const _ResetModeIntent(),
                    },
                    actions: {
                      _ResetModeIntent: CallbackAction<_ResetModeIntent>(
                        onInvoke: (_) {
                          if (selectedUser.value != null) {
                            selectedUser.value = null;
                          } else if (userMode.value) {
                            userMode.value = false;
                          }
                          return null;
                        },
                      ),
                    },
                    child: SearchTextField(
                      fontSize: 16,
                      controller: editingController,
                      focusNode: focusNode,
                      onChanged: (keyword) {
                        if (userMode.value && selectedUser.value == null) {
                          return;
                        }
                        SearchConversationKeywordNotifier.updateKeyword(
                          context,
                          keyword,
                        );
                      },
                      hintText: context.l10n.search,
                      showClear: userMode.value,
                      leading: userMode.value
                          ? Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                Text(
                                  context.l10n.fromWithColon,
                                  style: TextStyle(
                                    fontSize: 16,
                                    color: context.theme.text,
                                  ),
                                ),
                                if (selectedUser.value != null)
                                  Padding(
                                    padding: const EdgeInsets.only(right: 8),
                                    child: AvatarView(
                                      userId: selectedUser.value!.userId,
                                      name: selectedUser.value!.fullName,
                                      avatarUrl: selectedUser.value!.avatarUrl,
                                      size: 20,
                                    ),
                                  ),
                              ],
                            )
                          : null,
                      onTapClear: () {
                        userMode.value = false;
                        selectedUser.value = null;
                      },
                    ),
                  ),
                ),
                AnimatedSize(
                  duration: const Duration(milliseconds: 200),
                  alignment: Alignment.centerLeft,
                  child: userMode.value
                      ? const SizedBox()
                      : SizedBox(
                          height: 36,
                          child: ActionButton(
                            name: MixinAssets.userSearch,
                            color: context.theme.icon,
                            onTap: enableUserMode,
                          ),
                        ),
                ),
              ],
            ),
          ),
          AnimatedSize(
            alignment: Alignment.bottomCenter,
            duration: const Duration(milliseconds: 200),
            child:
                keywordIsEmpty || (userMode.value && selectedUser.value == null)
                ? const SizedBox(height: 8)
                : Padding(
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    child: Container(
                      padding: const EdgeInsets.only(left: 16),
                      height: 32,
                      alignment: Alignment.center,
                      child: ListView.separated(
                        scrollDirection: Axis.horizontal,
                        padding: EdgeInsets.zero,
                        itemCount: categories.length,
                        itemBuilder: (context, index) {
                          final category = categories[index];
                          return _CategoryItem(
                            name: category.name,
                            categories: category.categories,
                            selectedCategories: selectedCategories.value,
                            onSelected: (value) {
                              selectedCategories.value =
                                  listEquals(
                                    selectedCategories.value,
                                    value,
                                  )
                                  ? null
                                  : value;
                            },
                          );
                        },
                        separatorBuilder: (context, index) =>
                            const SizedBox(width: 8),
                      ),
                    ),
                  ),
          ),
          Expanded(
            child: userMode.value && selectedUser.value == null
                ? _SearchParticipantList(
                    account: scope.account,
                    editingController: editingController,
                    conversationId: scope.conversation.id,
                    isBot: scope.conversation.isBot,
                    onSelected: (user) {
                      editingController.text = '';
                      selectedUser.value = user;
                    },
                  )
                : _SearchMessageList(
                    account: scope.account,
                    conversationId: scope.conversation.id,
                    selectedUserId: selectedUser.value?.userId,
                    categories: selectedCategories.value,
                    onTap: (message) {
                      scope.onLocateMessage(message.id);
                      scope.notifier.closeAfterContentJump(
                        routeMode: scope.routeMode,
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }
}

class _SearchMessageList extends HookWidget {
  const _SearchMessageList({
    required this.account,
    required this.conversationId,
    required this.onTap,
    this.selectedUserId,
    this.categories,
  });

  final rust.AccountHandle account;
  final String conversationId;
  final String? selectedUserId;
  final List<String>? categories;
  final ValueChanged<MessageListEntry> onTap;

  @override
  Widget build(BuildContext context) {
    final keywordNotifier = context.read<SearchConversationKeywordNotifier>();
    final keywordState = useValueListenable(keywordNotifier);
    final rawKeyword = keywordState.$2.trim();
    final keyword = useState(rawKeyword);
    useEffect(() {
      final timer = Timer(
        const Duration(milliseconds: 150),
        () => keyword.value = rawKeyword,
      );
      return timer.cancel;
    }, [rawKeyword]);

    final controller = useMemoized(
      () => MessageSearchController(
        account: account,
        conversationId: conversationId,
        keyword: keyword.value,
        senderId: selectedUserId,
        categories: categories,
      ),
      [account, conversationId, keyword.value, selectedUserId, categories],
    );
    useEffect(() {
      final changes = account.messageChanges().listen((_) {
        unawaited(controller.reload());
      });
      return () {
        unawaited(changes.cancel());
        controller.dispose();
      };
    }, [account, controller]);

    final state = useValueListenable(controller);
    if (keyword.value.isEmpty || state.error != null) {
      return const SizedBox();
    }
    return ScrollablePositionedList.builder(
      itemPositionsListener: controller.itemPositionsListener,
      itemCount: state.items.length,
      itemBuilder: (context, index) {
        final message = state.items[index];
        return _SearchMessageTile(
          message: message,
          keyword: keyword.value,
          onTap: () => onTap(message),
        );
      },
    );
  }
}

class _SearchParticipantList extends HookWidget {
  const _SearchParticipantList({
    required this.account,
    required this.onSelected,
    required this.editingController,
    required this.conversationId,
    required this.isBot,
  });

  final rust.AccountHandle account;
  final ValueChanged<rust.ConversationParticipantItem> onSelected;
  final TextEditingController editingController;
  final String conversationId;
  final bool isBot;

  @override
  Widget build(BuildContext context) {
    useListenable(editingController);
    final keyword = editingController.text.trim();
    final participants = useState<List<rust.ConversationParticipantItem>>(
      const [],
    );

    useEffect(() {
      var active = true;
      participants.value = const [];
      unawaited(
        () async {
          try {
            final result = isBot
                ? await account.conversation().searchBotGroupUsers(
                    conversationId: conversationId,
                    keyword: keyword,
                  )
                : keyword.isEmpty
                ? await account.conversation().conversationParticipants(
                    conversationId: conversationId,
                  )
                : await account.conversation().searchGroupUsers(
                    conversationId: conversationId,
                    keyword: keyword,
                  );
            if (active) participants.value = result;
          } on Object catch (error, stackTrace) {
            e('Load conversation participants failed', error, stackTrace);
            if (active) participants.value = const [];
          }
        }(),
      );
      return () => active = false;
    }, [account, conversationId, isBot, keyword]);

    return ListView.builder(
      itemCount: participants.value.length,
      itemBuilder: (context, index) {
        final user = participants.value[index];
        return InteractiveDecoratedBox(
          onTap: () => onSelected(user),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 6),
            child: Row(
              children: [
                AvatarView(
                  userId: user.userId,
                  name: user.fullName,
                  avatarUrl: user.avatarUrl,
                  size: 38,
                ),
                const SizedBox(width: 16),
                Expanded(
                  child: Row(
                    children: [
                      Flexible(
                        child: CustomText(
                          user.fullName,
                          style: TextStyle(
                            fontSize: 16,
                            color: context.theme.text,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      BadgesWidget(
                        verified: user.isVerified,
                        isBot: user.isBot,
                        membership: user.membership,
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _CategoryItem extends StatelessWidget {
  const _CategoryItem({
    required this.name,
    required this.categories,
    required this.selectedCategories,
    required this.onSelected,
  });

  final String name;
  final List<String> categories;
  final List<String>? selectedCategories;
  final ValueChanged<List<String>> onSelected;

  @override
  Widget build(BuildContext context) {
    final selected = listEquals(categories, selectedCategories);
    return InteractiveDecoratedBox(
      onTap: () => onSelected(categories),
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        decoration: ShapeDecoration(
          color: selected ? context.theme.accent : context.theme.listSelected,
          shape: const StadiumBorder(),
        ),
        padding: const EdgeInsets.symmetric(horizontal: 12),
        height: 32,
        alignment: Alignment.center,
        child: Text(
          name,
          style: TextStyle(
            color: selected ? Colors.white : context.theme.text,
            fontSize: 16,
            height: 1,
          ),
        ),
      ),
    );
  }
}

class _Category extends Equatable {
  const _Category(this.name, this.categories);

  final String name;
  final List<String> categories;

  @override
  List<Object?> get props => [name, categories];
}

class _ResetModeIntent extends Intent {
  const _ResetModeIntent();
}

class _SearchMessageTile extends StatelessWidget {
  const _SearchMessageTile({
    required this.message,
    required this.keyword,
    required this.onTap,
  });

  final MessageListEntry message;
  final String keyword;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final selectedDecoration = BoxDecoration(
      borderRadius: const BorderRadius.all(Radius.circular(8)),
      color: context.theme.listSelected,
    );
    final icon = MixinAssets.messageIcon(message.category);
    final description = _searchMessagePreview(context, message);
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
              SizedBox.square(
                dimension: 50,
                child: AvatarView(
                  userId: message.senderId,
                  name: message.senderName,
                  avatarUrl: message.senderAvatarUrl,
                  size: 50,
                ),
              ),
              const SizedBox(width: 12),
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
                                child: CustomText(
                                  message.senderName,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: TextStyle(
                                    color: context.theme.text,
                                    fontSize: 16,
                                  ),
                                  textMatchers: [
                                    EmojiTextMatcher(),
                                    if (keyword.trim().isNotEmpty)
                                      MultiKeyWordTextMatcher.createKeywordMatcher(
                                        keyword: keyword,
                                        style: TextStyle(
                                          color: context.theme.accent,
                                        ),
                                        caseSensitive: false,
                                      ),
                                  ],
                                ),
                              ),
                              BadgesWidget(
                                verified: message.senderIsVerified,
                                isBot: message.senderIsBot,
                                membership: message.senderMembership,
                              ),
                            ],
                          ),
                        ),
                        Text(
                          _formattedSearchDate(message.createdAt),
                          style: TextStyle(
                            color: context.theme.secondaryText,
                            fontSize: 12,
                          ),
                        ),
                      ],
                    ),
                    Row(
                      children: [
                        if (icon != null)
                          Padding(
                            padding: const EdgeInsets.only(right: 2),
                            child: SvgPicture.asset(
                              icon,
                              colorFilter: ColorFilter.mode(
                                context.theme.secondaryText,
                                BlendMode.srcIn,
                              ),
                            ),
                          ),
                        Expanded(
                          child: CustomText(
                            description,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: TextStyle(
                              color: context.theme.secondaryText,
                              fontSize: 14,
                            ),
                            textMatchers: [
                              EmojiTextMatcher(),
                              if (keyword.trim().isNotEmpty)
                                MultiKeyWordTextMatcher.createKeywordMatcher(
                                  keyword: keyword,
                                  style: TextStyle(
                                    color: context.theme.accent,
                                  ),
                                  caseSensitive: false,
                                ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

String _searchMessagePreview(BuildContext context, MessageListEntry message) {
  final category = message.category;
  final status = message.status.toUpperCase();
  if (status == 'FAILED') return context.l10n.waitingForThisMessage;
  if (status == 'UNKNOWN') return context.l10n.messageNotSupport;
  if (message.isText) return message.content.trim();
  if (category == 'SYSTEM_ACCOUNT_SNAPSHOT' ||
      category == 'SYSTEM_SAFE_SNAPSHOT') {
    return '[${context.l10n.transfer}]';
  }
  if (message.isSticker) return '[${context.l10n.sticker}]';
  if (message.isImage) return '[${context.l10n.image}]';
  if (category.endsWith('_VIDEO')) return '[${context.l10n.video}]';
  if (message.isLive) return '[${context.l10n.live}]';
  if (category.endsWith('_DATA')) return '[${context.l10n.file}]';
  if (message.isPost) return _postOptimizeMarkdown(message.content.trim());
  if (category.endsWith('_LOCATION')) return '[${context.l10n.location}]';
  if (message.isAudio) return '[${context.l10n.audio}]';
  if (category == 'APP_BUTTON_GROUP') {
    try {
      final actions = jsonDecode(message.content) as List<dynamic>;
      return actions
          .map((item) => '[${(item as Map<String, dynamic>)['label'] ?? ''}]')
          .join();
    } on Object {
      return '';
    }
  }
  if (category == 'APP_CARD') {
    try {
      final card = jsonDecode(message.content) as Map<String, dynamic>;
      return '[${card['title'] ?? context.l10n.card}]';
    } on Object {
      return '[${context.l10n.card}]';
    }
  }
  if (category.endsWith('_CONTACT')) return '[${context.l10n.contact}]';
  if (message.isRecall) {
    final isCurrentUser =
        message.senderId == ChatSideScope.of(context).currentUserId;
    return '[${isCurrentUser ? context.l10n.youDeletedThisMessage : context.l10n.thisMessageWasDeleted}]';
  }
  if (category.endsWith('_TRANSCRIPT')) {
    return '[${context.l10n.transcript}]';
  }
  if (category == 'SYSTEM_SAFE_INSCRIPTION') {
    return '[${context.l10n.collectible}]';
  }
  return context.l10n.messageNotSupport;
}

String _postOptimizeMarkdown(String content) {
  final optimized = const LineSplitter().convert(content).take(10).join('\r\n');
  final truncated = optimized.length > 1024
      ? optimized.substring(0, 1024)
      : optimized;
  final nodes = markdown.Document().parseLines(
    const LineSplitter().convert(truncated),
  );
  return nodes
      .map((node) => node.textContent)
      .join()
      .replaceAll(RegExp(r'\s+'), ' ');
}

String _formattedSearchDate(DateTime value) {
  final now = DateTime.now();
  final local = value.toLocal();
  final sameDay =
      now.year == local.year &&
      now.month == local.month &&
      now.day == local.day;
  if (sameDay) return DateFormat.Hm().format(local);
  final startOfThisWeek = now.subtract(Duration(days: now.weekday));
  final startOfValueWeek = local.subtract(Duration(days: local.weekday));
  final sameWeek =
      startOfThisWeek.year == startOfValueWeek.year &&
      startOfThisWeek.month == startOfValueWeek.month &&
      startOfThisWeek.day == startOfValueWeek.day;
  if (sameWeek) return DateFormat.E().format(local);
  if (now.year == local.year) return DateFormat.MMMd().format(local);
  return DateFormat.yMMMd().format(local);
}
