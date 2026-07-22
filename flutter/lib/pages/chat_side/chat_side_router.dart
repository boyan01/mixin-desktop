import 'package:flutter/material.dart';

import '../../controllers/chat_side_notifier.dart';
import '../../models/conversation_list_entry.dart';
import '../../src/rust/desktop_api.dart' as rust;
import '../conversation_info_destination.dart';
import 'chat_info_page.dart';
import 'chat_side_scope.dart';
import 'circle_manager_page.dart';
import 'disappear_message_page.dart';
import 'group_participants_page.dart';
import 'groups_in_common_page.dart';
import 'pin_messages_page.dart';
import 'search_message_page.dart';
import 'shared_apps_page.dart';
import 'shared_media_page.dart';

const kChatSidePageWidth = 300.0;

class ChatSideRouter extends StatelessWidget {
  const ChatSideRouter({
    required this.account,
    required this.conversation,
    required this.notifier,
    required this.destinations,
    required this.constraints,
    required this.routeMode,
    required this.onLocateMessage,
    required this.onSelectConversation,
    required this.onOpenUri,
    required this.onConversationDeleted,
    this.leadingPages = const [],
    super.key,
  });

  final rust.AccountHandle account;
  final ConversationListEntry conversation;
  final ChatSideNotifier notifier;
  final List<ConversationInfoDestination> destinations;
  final List<Page<dynamic>> leadingPages;
  final BoxConstraints constraints;
  final bool routeMode;
  final ValueChanged<String> onLocateMessage;
  final ValueChanged<ConversationListEntry> onSelectConversation;
  final ValueChanged<Uri> onOpenUri;
  final VoidCallback onConversationDeleted;

  @override
  Widget build(BuildContext context) {
    final pages = [...leadingPages, ...destinations.map(_pageFor)];
    final navigator = routeMode
        ? SizedBox(
            width: constraints.maxWidth,
            child: Navigator(
              pages: pages,
              onDidRemovePage: (_) => notifier.onPopPage(),
            ),
          )
        : _AnimatedChatSlide(
            constraints: constraints,
            pages: pages,
            onDidRemovePage: (_) => notifier.onPopPage(),
          );
    return ChatSideScope(
      account: account,
      conversation: conversation,
      notifier: notifier,
      currentUserId: account.accountId(),
      routeMode: routeMode,
      onLocateMessage: onLocateMessage,
      onSelectConversation: onSelectConversation,
      onOpenUri: onOpenUri,
      onConversationDeleted: onConversationDeleted,
      child: navigator,
    );
  }
}

MaterialPage<void> _pageFor(
  ConversationInfoDestination destination,
) => MaterialPage<void>(
  key: ValueKey(destination),
  name: destination.name,
  child: switch (destination) {
    ConversationInfoDestination.infoPage => const ChatInfoPage(),
    ConversationInfoDestination.circles => const CircleManagerPage(),
    ConversationInfoDestination.searchMessageHistory =>
      const SearchMessagePage(),
    ConversationInfoDestination.sharedMedia => const SharedMediaPage(),
    ConversationInfoDestination.participants => const GroupParticipantsPage(),
    ConversationInfoDestination.pinMessages => const PinMessagesPage(),
    ConversationInfoDestination.sharedApps => const SharedAppsPage(),
    ConversationInfoDestination.groupsInCommon => const GroupsInCommonPage(),
    ConversationInfoDestination.disappearMessages =>
      const DisappearMessagePage(),
  },
);

class _AnimatedChatSlide extends StatefulWidget {
  const _AnimatedChatSlide({
    required this.pages,
    required this.constraints,
    required this.onDidRemovePage,
  });

  final List<Page<dynamic>> pages;
  final DidRemovePageCallback onDidRemovePage;
  final BoxConstraints constraints;

  @override
  State<_AnimatedChatSlide> createState() => _AnimatedChatSlideState();
}

class _AnimatedChatSlideState extends State<_AnimatedChatSlide>
    with SingleTickerProviderStateMixin {
  late final controller = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 300),
  );
  List<Page<dynamic>> displayedPages = const [];

  @override
  void initState() {
    super.initState();
    _updatePages();
  }

  @override
  void didUpdateWidget(_AnimatedChatSlide oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.pages != widget.pages) _updatePages();
  }

  void _updatePages() {
    if (widget.pages.isNotEmpty) {
      displayedPages = widget.pages;
      controller.forward();
    } else {
      controller.reverse().whenComplete(() {
        if (mounted && widget.pages.isEmpty) {
          setState(() => displayedPages = const []);
        }
      });
    }
  }

  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
    animation: controller,
    builder: (context, child) => SizedBox(
      width: kChatSidePageWidth * Curves.easeInOut.transform(controller.value),
      height: widget.constraints.maxHeight,
      child: controller.value == 0 ? null : child,
    ),
    child: ClipRect(
      child: OverflowBox(
        alignment: AlignmentDirectional.centerStart,
        minWidth: kChatSidePageWidth,
        maxWidth: kChatSidePageWidth,
        minHeight: widget.constraints.maxHeight,
        maxHeight: widget.constraints.maxHeight,
        child: Navigator(
          pages: displayedPages,
          onDidRemovePage: widget.onDidRemovePage,
        ),
      ),
    ),
  );
}
