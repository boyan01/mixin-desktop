import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:intl/intl.dart';
import 'package:markdown/markdown.dart' as markdown;
import 'package:provider/provider.dart';

import '../../constants/assets.dart';
import '../../controllers/chat_side_notifier.dart';
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
import 'chat_side_scope.dart';

class SearchMessagePage extends StatefulWidget {
  const SearchMessagePage({super.key});

  @override
  State<SearchMessagePage> createState() => _SearchMessagePageState();
}

class _SearchMessagePageState extends State<SearchMessagePage> {
  static const textCategories = ['PLAIN_TEXT', 'SIGNAL_TEXT', 'ENCRYPTED_TEXT'];
  static const postCategories = ['PLAIN_POST', 'SIGNAL_POST', 'ENCRYPTED_POST'];

  final controller = TextEditingController();
  final focusNode = FocusNode();
  Timer? debounce;
  List<MessageListEntry> messages = const [];
  List<rust.ConversationParticipantItem> participants = const [];
  rust.ConversationParticipantItem? selectedUser;
  List<String>? selectedCategories;
  bool userMode = false;
  bool loading = false;
  bool loadingMore = false;
  bool participantLoading = false;
  bool hasMore = true;
  Object? error;
  StreamSubscription<BigInt>? changes;
  int searchGeneration = 0;
  int participantGeneration = 0;
  bool restoredKeyword = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (!restoredKeyword) {
      restoredKeyword = true;
      controller.text = context
          .read<SearchConversationKeywordNotifier>()
          .value
          .$2;
      if (controller.text.trim().isNotEmpty) unawaited(_search());
    }
    changes ??= ChatSideScope.of(context).account.messageChanges().listen(
      (_) {
        if (controller.text.trim().isNotEmpty) unawaited(_search());
      },
      onError: (Object exception) {
        if (mounted) setState(() => error = exception);
      },
    );
  }

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback(
      (_) => focusNode.requestFocus(),
    );
  }

  @override
  void dispose() {
    unawaited(changes?.cancel());
    debounce?.cancel();
    controller.dispose();
    focusNode.dispose();
    super.dispose();
  }

  void _changed(String value) {
    if (userMode && selectedUser == null) {
      debounce?.cancel();
      final scope = ChatSideScope.of(context);
      if (scope.conversation.isBot) {
        debounce = Timer(
          const Duration(milliseconds: 100),
          () => unawaited(_loadParticipants(value.trim())),
        );
      } else {
        setState(() {});
      }
      return;
    }
    context.read<SearchConversationKeywordNotifier>().value = (
      selectedUser?.userId,
      value,
    );
    debounce?.cancel();
    debounce = Timer(const Duration(milliseconds: 150), _search);
  }

  Future<void> _search({bool append = false}) async {
    final query = controller.text.trim();
    if (query.isEmpty) {
      searchGeneration++;
      if (mounted) setState(() => messages = const []);
      return;
    }
    final scope = ChatSideScope.of(context);
    final requestGeneration = append ? searchGeneration : ++searchGeneration;
    if (append && (loadingMore || !hasMore)) return;
    setState(() {
      if (append) {
        loadingMore = true;
      } else {
        loading = true;
        hasMore = true;
      }
      error = null;
    });
    try {
      final result = await scope.account.message().searchMessages(
        conversationId: scope.conversation.id,
        query: query,
        senderId: selectedUser?.userId,
        categories: selectedCategories ?? const [],
        anchorMessageId: append ? messages.last.id : null,
        limit: 60,
      );
      if (!mounted || requestGeneration != searchGeneration) return;
      setState(() {
        final loaded = result.map(MessageListEntry.fromRust).toList();
        messages = append ? [...messages, ...loaded] : loaded;
        hasMore = loaded.length == 60;
        loading = false;
        loadingMore = false;
      });
    } on Object catch (exception, stackTrace) {
      e('Search conversation messages failed', exception, stackTrace);
      if (!mounted || requestGeneration != searchGeneration) return;
      setState(() {
        loading = false;
        loadingMore = false;
        error = exception;
      });
    }
  }

  Future<void> _enableUserMode() async {
    setState(() {
      userMode = true;
      selectedUser = null;
      messages = const [];
      controller.clear();
    });
    if (participants.isNotEmpty) return;
    await _loadParticipants('');
  }

  Future<void> _loadParticipants(String keyword) async {
    final scope = ChatSideScope.of(context);
    final requestGeneration = ++participantGeneration;
    setState(() {
      participantLoading = true;
      error = null;
    });
    try {
      final value = scope.conversation.isBot
          ? await scope.account.conversation().searchBotGroupUsers(
              conversationId: scope.conversation.id,
              keyword: keyword,
            )
          : await scope.account.conversation().conversationParticipants(
              conversationId: scope.conversation.id,
            );
      if (mounted && requestGeneration == participantGeneration) {
        setState(() {
          participants = value;
          participantLoading = false;
        });
      }
    } on Object catch (exception, stackTrace) {
      e('Load conversation participants failed', exception, stackTrace);
      if (mounted && requestGeneration == participantGeneration) {
        setState(() {
          participantLoading = false;
          error = exception;
        });
      }
    }
  }

  void _selectUser(rust.ConversationParticipantItem user) {
    controller.clear();
    setState(() => selectedUser = user);
    context.read<SearchConversationKeywordNotifier>().value = (user.userId, '');
    focusNode.requestFocus();
  }

  void _locate(String messageId) {
    final scope = ChatSideScope.of(context);
    scope.onLocateMessage(messageId);
    scope.notifier.closeAfterContentJump(routeMode: scope.routeMode);
  }

  @override
  Widget build(BuildContext context) => ChatSidePageScaffold(
    title: context.l10n.searchConversation,
    body: Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
          child: Row(
            children: [
              Expanded(
                child: FocusableActionDetector(
                  shortcuts: {
                    if (controller.text.isEmpty &&
                        (userMode || selectedUser != null))
                      const SingleActivator(LogicalKeyboardKey.backspace):
                          const _ResetModeIntent(),
                  },
                  actions: {
                    _ResetModeIntent: CallbackAction<_ResetModeIntent>(
                      onInvoke: (_) {
                        setState(() {
                          if (selectedUser != null) {
                            selectedUser = null;
                          } else {
                            userMode = false;
                          }
                        });
                        return null;
                      },
                    ),
                  },
                  child: SearchTextField(
                    fontSize: 16,
                    controller: controller,
                    focusNode: focusNode,
                    onChanged: _changed,
                    hintText: context.l10n.search,
                    showClear: userMode,
                    leading: userMode
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
                              if (selectedUser != null)
                                Padding(
                                  padding: const EdgeInsets.only(right: 8),
                                  child: AvatarView(
                                    userId: selectedUser!.userId,
                                    name: selectedUser!.fullName,
                                    avatarUrl: selectedUser!.avatarUrl,
                                    size: 20,
                                  ),
                                ),
                            ],
                          )
                        : null,
                    onTapClear: () {
                      setState(() {
                        userMode = false;
                        selectedUser = null;
                        messages = const [];
                      });
                    },
                  ),
                ),
              ),
              AnimatedSize(
                duration: const Duration(milliseconds: 200),
                alignment: Alignment.centerLeft,
                child: userMode
                    ? const SizedBox()
                    : SizedBox(
                        height: 36,
                        child: ActionButton(
                          name: MixinAssets.userSearch,
                          color: context.theme.icon,
                          onTap: _enableUserMode,
                        ),
                      ),
              ),
            ],
          ),
        ),
        AnimatedSize(
          alignment: Alignment.bottomCenter,
          duration: const Duration(milliseconds: 200),
          child: controller.text.isEmpty || (userMode && selectedUser == null)
              ? const SizedBox(height: 8)
              : Padding(
                  padding: const EdgeInsets.symmetric(vertical: 8),
                  child: Container(
                    padding: const EdgeInsets.only(left: 16),
                    height: 32,
                    alignment: Alignment.center,
                    child: ListView(
                      scrollDirection: Axis.horizontal,
                      padding: EdgeInsets.zero,
                      children: [
                        _FilterChip(
                          label: context.l10n.text,
                          selected: selectedCategories == textCategories,
                          onSelected: () {
                            setState(() {
                              selectedCategories =
                                  selectedCategories == textCategories
                                  ? null
                                  : textCategories;
                            });
                            unawaited(_search());
                          },
                        ),
                        const SizedBox(width: 8),
                        _FilterChip(
                          label: context.l10n.post,
                          selected: selectedCategories == postCategories,
                          onSelected: () {
                            setState(() {
                              selectedCategories =
                                  selectedCategories == postCategories
                                  ? null
                                  : postCategories;
                            });
                            unawaited(_search());
                          },
                        ),
                      ],
                    ),
                  ),
                ),
        ),
        Expanded(child: _body()),
      ],
    ),
  );

  Widget _body() {
    if (userMode && selectedUser == null) {
      if (participantLoading || error != null) return const SizedBox();
      final keyword = controller.text.trim().toLowerCase();
      final filtered = participants
          .where(
            (item) =>
                keyword.isEmpty ||
                item.fullName.toLowerCase().contains(keyword) ||
                item.identityNumber.contains(keyword),
          )
          .toList(growable: false);
      return ListView.builder(
        itemCount: filtered.length,
        itemBuilder: (context, index) {
          final participant = filtered.elementAt(index);
          return InteractiveDecoratedBox(
            onTap: () => _selectUser(participant),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 6),
              child: Row(
                children: [
                  AvatarView(
                    userId: participant.userId,
                    name: participant.fullName,
                    avatarUrl: participant.avatarUrl,
                    size: 38,
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: Row(
                      children: [
                        Flexible(
                          child: CustomText(
                            participant.fullName,
                            style: TextStyle(
                              fontSize: 16,
                              color: context.theme.text,
                            ),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        BadgesWidget(
                          verified: participant.isVerified,
                          isBot: participant.isBot,
                          membership: participant.membership,
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
    if (controller.text.trim().isEmpty || loading || error != null) {
      return const SizedBox();
    }
    return NotificationListener<ScrollNotification>(
      onNotification: (notification) {
        if (notification.metrics.extentAfter < 240) {
          unawaited(_search(append: true));
        }
        return false;
      },
      child: ListView.builder(
        itemCount: messages.length,
        itemBuilder: (context, index) {
          final message = messages[index];
          return _SearchMessageTile(
            message: message,
            keyword: controller.text.trim(),
            onTap: () => _locate(message.id),
          );
        },
      ),
    );
  }
}

class _ResetModeIntent extends Intent {
  const _ResetModeIntent();
}

class _FilterChip extends StatelessWidget {
  const _FilterChip({
    required this.label,
    required this.selected,
    required this.onSelected,
  });

  final String label;
  final bool selected;
  final VoidCallback onSelected;

  @override
  Widget build(BuildContext context) => InteractiveDecoratedBox(
    onTap: onSelected,
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
        label,
        style: TextStyle(
          color: selected ? Colors.white : context.theme.text,
          fontSize: 16,
          height: 1,
        ),
      ),
    ),
  );
}

class _HighlightedText extends StatelessWidget {
  const _HighlightedText({required this.text, required this.query});

  final String text;
  final String query;

  @override
  Widget build(BuildContext context) {
    final style = TextStyle(color: context.theme.secondaryText, fontSize: 14);
    final keywords = query
        .trim()
        .split(RegExp(r'\s+'))
        .where((value) => value.isNotEmpty)
        .toList();
    if (keywords.isEmpty) {
      return Text(
        text,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: style,
      );
    }
    final matcher = RegExp(
      '(${keywords.map(RegExp.escape).join('|')})',
      caseSensitive: false,
    );
    final matches = matcher.allMatches(text).toList();
    if (matches.isEmpty) {
      return Text(
        text,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: style,
      );
    }
    var offset = 0;
    final spans = <InlineSpan>[];
    for (final match in matches) {
      if (match.start > offset) {
        spans.add(TextSpan(text: text.substring(offset, match.start)));
      }
      spans.add(
        TextSpan(
          text: text.substring(match.start, match.end),
          style: TextStyle(color: context.theme.accent),
        ),
      );
      offset = match.end;
    }
    if (offset < text.length) spans.add(TextSpan(text: text.substring(offset)));
    return Text.rich(
      TextSpan(children: spans),
      style: style,
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
    );
  }
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
                                child: Text(
                                  message.senderName,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: TextStyle(
                                    color: context.theme.text,
                                    fontSize: 16,
                                  ),
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
                          child: _HighlightedText(
                            text: description,
                            query: keyword,
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
