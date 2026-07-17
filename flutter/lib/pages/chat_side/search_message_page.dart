import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:intl/intl.dart';
import 'package:mixin_desktop_ui/controllers/chat_side_notifier.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/pages/chat_side/chat_side_scope.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:provider/provider.dart';

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
    debounce = Timer(const Duration(milliseconds: 300), _search);
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
        offset: append ? messages.length : 0,
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
    } on Object catch (exception) {
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
    } on Object catch (exception) {
      if (mounted && requestGeneration == participantGeneration) {
        setState(() {
          participantLoading = false;
          error = exception;
        });
      }
    }
  }

  void _selectUser(rust.ConversationParticipantItem user) {
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
                  child: TextField(
                    controller: controller,
                    focusNode: focusNode,
                    onChanged: _changed,
                    decoration: InputDecoration(
                      isDense: true,
                      prefixIcon: const Icon(Icons.search),
                      prefix: selectedUser == null
                          ? null
                          : Padding(
                              padding: const EdgeInsets.only(right: 8),
                              child: Text(
                                '${context.l10n.fromWithColon} ${selectedUser!.fullName}',
                              ),
                            ),
                      hintText: context.l10n.search,
                      suffixIcon: userMode
                          ? IconButton(
                              onPressed: () {
                                setState(() {
                                  userMode = false;
                                  selectedUser = null;
                                  controller.clear();
                                  messages = const [];
                                });
                              },
                              icon: const Icon(Icons.close),
                            )
                          : null,
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(18),
                        borderSide: BorderSide.none,
                      ),
                      filled: true,
                    ),
                  ),
                ),
              ),
              AnimatedSize(
                duration: const Duration(milliseconds: 200),
                child: userMode
                    ? const SizedBox()
                    : IconButton(
                        onPressed: _enableUserMode,
                        icon: const Icon(Icons.person_search_outlined),
                      ),
              ),
            ],
          ),
        ),
        if (controller.text.isNotEmpty && selectedUser != null ||
            controller.text.isNotEmpty && !userMode)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Row(
              children: [
                _FilterChip(
                  label: context.l10n.text,
                  selected: selectedCategories == textCategories,
                  onSelected: () {
                    setState(() {
                      selectedCategories = selectedCategories == textCategories
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
                      selectedCategories = selectedCategories == postCategories
                          ? null
                          : postCategories;
                    });
                    unawaited(_search());
                  },
                ),
              ],
            ),
          ),
        Expanded(child: _body()),
      ],
    ),
  );

  Widget _body() {
    if (userMode && selectedUser == null) {
      if (participantLoading) {
        return const Center(child: CircularProgressIndicator());
      }
      if (error != null) {
        return ChatSideError(
          error: error!,
          onRetry: () => _loadParticipants(controller.text.trim()),
        );
      }
      final keyword = controller.text.trim().toLowerCase();
      final filtered = participants.where(
        (item) =>
            keyword.isEmpty ||
            item.fullName.toLowerCase().contains(keyword) ||
            item.identityNumber.contains(keyword),
      );
      return ListView(
        children: [
          for (final participant in filtered)
            ListTile(
              leading: AvatarView(
                userId: participant.userId,
                name: participant.fullName,
                avatarUrl: participant.avatarUrl,
                size: 40,
              ),
              title: Text(participant.fullName),
              subtitle: Text(participant.identityNumber),
              onTap: () => _selectUser(participant),
            ),
        ],
      );
    }
    if (loading) return const Center(child: CircularProgressIndicator());
    if (error != null) return ChatSideError(error: error!, onRetry: _search);
    if (controller.text.trim().isEmpty) return const SizedBox();
    if (messages.isEmpty) {
      return Center(
        child: Text(
          context.l10n.noResults,
          style: TextStyle(color: context.theme.secondaryText),
        ),
      );
    }
    return NotificationListener<ScrollNotification>(
      onNotification: (notification) {
        if (notification.metrics.extentAfter < 240) {
          unawaited(_search(append: true));
        }
        return false;
      },
      child: ListView.builder(
        itemCount: messages.length + (loadingMore ? 1 : 0),
        itemBuilder: (context, index) {
          if (index == messages.length) {
            return const Padding(
              padding: EdgeInsets.all(12),
              child: Center(child: CircularProgressIndicator()),
            );
          }
          final message = messages[index];
          return ListTile(
            leading: AvatarView(
              userId: message.senderId,
              name: message.senderName,
              avatarUrl: message.senderAvatarUrl,
              size: 40,
            ),
            title: Row(
              children: [
                Expanded(
                  child: Text(
                    message.senderName,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                Text(
                  DateFormat.yMd().add_Hm().format(message.createdAt.toLocal()),
                  style: TextStyle(
                    color: context.theme.secondaryText,
                    fontSize: 11,
                  ),
                ),
              ],
            ),
            subtitle: _HighlightedText(
              text: message.content,
              query: controller.text.trim(),
            ),
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
  Widget build(BuildContext context) => ChoiceChip(
    label: Text(label),
    selected: selected,
    onSelected: (_) => onSelected(),
  );
}

class _HighlightedText extends StatelessWidget {
  const _HighlightedText({required this.text, required this.query});

  final String text;
  final String query;

  @override
  Widget build(BuildContext context) {
    final index = text.toLowerCase().indexOf(query.toLowerCase());
    if (index < 0) {
      return Text(text, maxLines: 2, overflow: TextOverflow.ellipsis);
    }
    return Text.rich(
      TextSpan(
        children: [
          TextSpan(text: text.substring(0, index)),
          TextSpan(
            text: text.substring(index, index + query.length),
            style: TextStyle(color: context.theme.accent),
          ),
          TextSpan(text: text.substring(index + query.length)),
        ],
      ),
      maxLines: 2,
      overflow: TextOverflow.ellipsis,
    );
  }
}
