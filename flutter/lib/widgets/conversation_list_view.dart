import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/constants/icon_fonts.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart'
    show AccountHandle, UserProfileItem;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/action_button.dart';
import 'package:mixin_desktop_ui/widgets/badges_widget.dart';
import 'package:mixin_desktop_ui/widgets/conversation_search_results.dart';
import 'package:mixin_desktop_ui/widgets/custom_popup_menu.dart';
import 'package:mixin_desktop_ui/widgets/custom_context_menu.dart';
import 'package:mixin_desktop_ui/widgets/high_light_text.dart';
import 'package:mixin_desktop_ui/widgets/mixin_dialog.dart';
import 'package:mixin_desktop_ui/widgets/interactive_decorated_box.dart';
import 'package:mixin_desktop_ui/widgets/mute_dialog.dart';
import 'package:mixin_desktop_ui/widgets/move_window.dart';
import 'package:mixin_desktop_ui/widgets/message_items/special_message_items.dart';
import 'package:mixin_desktop_ui/widgets/message_datetime_and_status.dart';
import 'package:mixin_desktop_ui/widgets/message_selectable_text.dart';
import 'package:mixin_desktop_ui/widgets/search_text_field.dart';
import 'package:super_context_menu/super_context_menu.dart';
import 'package:intl/intl.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';

enum ConversationCreateAction { searchContact, conversation, group, circle }

class ConversationListView extends StatefulWidget {
  const ConversationListView({
    required this.conversations,
    required this.initialized,
    required this.itemPositionsListener,
    required this.itemScrollController,
    required this.loading,
    required this.currentUserId,
    required this.circles,
    required this.currentCircleId,
    required this.filterUnseen,
    required this.selectedConversationId,
    required this.onQueryChanged,
    required this.onToggleUnseen,
    required this.onCreateActionSelected,
    required this.onSelected,
    required this.onPinned,
    required this.onMuted,
    required this.onDeleted,
    required this.onCircleChanged,
    super.key,
    this.query = '',
    this.account,
    this.searchMessages = const [],
    this.searchUsers = const [],
    this.searchMaoUser,
    this.searchMao,
    this.searchMessageConversations = const {},
    this.searchMessagesLoading = false,
    this.onSearchMessageSelected,
    this.onSearchUser,
    this.onLocalUserSelected,
    this.onMaoBotOpen,
    this.onOpenLink,
    this.audioPlayerBar = const SizedBox.shrink(),
    this.networkStatus = const SizedBox.shrink(),
  });

  final List<ConversationListEntry> conversations;
  final bool initialized;
  final ItemPositionsListener itemPositionsListener;
  final ItemScrollController itemScrollController;
  final bool loading;
  final String currentUserId;
  final AccountHandle? account;
  final Map<String, String> circles;
  final String? currentCircleId;
  final String query;
  final bool filterUnseen;
  final String? selectedConversationId;
  final ValueChanged<String> onQueryChanged;
  final List<MessageListEntry> searchMessages;
  final List<UserProfileItem> searchUsers;
  final UserProfileItem? searchMaoUser;
  final String? searchMao;
  final Map<String, ConversationListEntry> searchMessageConversations;
  final bool searchMessagesLoading;
  final ValueChanged<MessageListEntry>? onSearchMessageSelected;
  final ValueChanged<String>? onSearchUser;
  final ValueChanged<UserProfileItem>? onLocalUserSelected;
  final ValueChanged<UserProfileItem>? onMaoBotOpen;
  final ValueChanged<Uri>? onOpenLink;
  final VoidCallback onToggleUnseen;
  final ValueChanged<ConversationCreateAction> onCreateActionSelected;
  final ValueChanged<ConversationListEntry> onSelected;
  final ValueChanged<ConversationListEntry> onPinned;
  final void Function(ConversationListEntry, int) onMuted;
  final ValueChanged<ConversationListEntry> onDeleted;
  final void Function(ConversationListEntry, String, bool) onCircleChanged;
  final Widget audioPlayerBar;
  final Widget networkStatus;

  @override
  State<ConversationListView> createState() => _ConversationListViewState();
}

class _ConversationListViewState extends State<ConversationListView> {
  late final TextEditingController searchController;
  final searchFocusNode = FocusNode();

  @override
  void initState() {
    super.initState();
    searchController = TextEditingController(text: widget.query);
  }

  @override
  void didUpdateWidget(covariant ConversationListView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (searchController.text == widget.query) return;
    searchController.value = TextEditingValue(
      text: widget.query,
      selection: TextSelection.collapsed(offset: widget.query.length),
    );
  }

  @override
  void dispose() {
    searchController.dispose();
    searchFocusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final content = widget.query.trim().isEmpty
        ? _ConversationListBody(
            conversations: widget.conversations,
            initialized: widget.initialized,
            loading: widget.loading,
            filterUnseen: widget.filterUnseen,
            itemPositionsListener: widget.itemPositionsListener,
            itemScrollController: widget.itemScrollController,
            currentUserId: widget.currentUserId,
            account: widget.account,
            circles: widget.circles,
            currentCircleId: widget.currentCircleId,
            selectedConversationId: widget.selectedConversationId,
            onSelected: widget.onSelected,
            onPinned: widget.onPinned,
            onMuted: widget.onMuted,
            onDeleted: widget.onDeleted,
            onCircleChanged: widget.onCircleChanged,
          )
        : ConversationSearchResults(
            keyword: widget.query,
            users: widget.searchUsers,
            maoUser: widget.searchMaoUser,
            mao: widget.searchMao,
            conversations: widget.conversations,
            messages: widget.searchMessages,
            messageConversations: widget.searchMessageConversations,
            loadingMessages: widget.searchMessagesLoading,
            account: widget.account,
            onConversationSelected: widget.onSelected,
            onMessageSelected: widget.onSearchMessageSelected ?? (_) {},
            onSearchUser: widget.onSearchUser ?? (_) {},
            onUserSelected: widget.onLocalUserSelected ?? (_) {},
            onMaoBotOpen: widget.onMaoBotOpen ?? (_) {},
            onOpenLink: widget.onOpenLink ?? (_) {},
            onClear: () {
              searchController.clear();
              widget.onQueryChanged('');
              searchFocusNode.unfocus();
            },
          );
    return Material(
      color: context.mixinTheme.primary,
      child: Column(
        children: [
          _SearchBar(
            controller: searchController,
            focusNode: searchFocusNode,
            filterUnseen: widget.filterUnseen,
            onChanged: widget.onQueryChanged,
            onToggleUnseen: widget.onToggleUnseen,
            onCreateActionSelected: widget.onCreateActionSelected,
          ),
          widget.networkStatus,
          Expanded(child: content),
          AnimatedSize(
            duration: const Duration(milliseconds: 200),
            child: widget.audioPlayerBar,
          ),
        ],
      ),
    );
  }
}

class _ConversationListBody extends StatelessWidget {
  const _ConversationListBody({
    required this.conversations,
    required this.initialized,
    required this.loading,
    required this.filterUnseen,
    required this.itemPositionsListener,
    required this.itemScrollController,
    required this.currentUserId,
    required this.account,
    required this.circles,
    required this.currentCircleId,
    required this.selectedConversationId,
    required this.onSelected,
    required this.onPinned,
    required this.onMuted,
    required this.onDeleted,
    required this.onCircleChanged,
  });

  final List<ConversationListEntry> conversations;
  final bool initialized;
  final bool loading;
  final bool filterUnseen;
  final ItemPositionsListener itemPositionsListener;
  final ItemScrollController itemScrollController;
  final String currentUserId;
  final AccountHandle? account;
  final Map<String, String> circles;
  final String? currentCircleId;
  final String? selectedConversationId;
  final ValueChanged<ConversationListEntry> onSelected;
  final ValueChanged<ConversationListEntry> onPinned;
  final void Function(ConversationListEntry, int) onMuted;
  final ValueChanged<ConversationListEntry> onDeleted;
  final void Function(ConversationListEntry, String, bool) onCircleChanged;

  @override
  Widget build(BuildContext context) {
    if (loading && !initialized) {
      return Center(
        child: CircularProgressIndicator(
          strokeWidth: 2,
          color: context.mixinTheme.accent,
        ),
      );
    }
    if (conversations.isEmpty) {
      return _EmptyState(
        text: filterUnseen ? context.l10n.searchEmpty : context.l10n.noData,
      );
    }
    return ScrollablePositionedList.builder(
      itemPositionsListener: itemPositionsListener,
      itemScrollController: itemScrollController,
      itemCount: conversations.length,
      itemBuilder: (context, index) {
        final conversation = conversations[index];
        return _ConversationContextMenu(
          conversation: conversation,
          onPinned: onPinned,
          onMuted: onMuted,
          onDeleted: onDeleted,
          circles: circles,
          currentCircleId: currentCircleId,
          onCircleChanged: onCircleChanged,
          child: ConversationItem(
            conversation: conversation,
            currentUserId: currentUserId,
            account: account,
            selected: conversation.id == selectedConversationId,
            onTap: () => onSelected(conversation),
          ),
        );
      },
    );
  }
}

class _SearchBar extends StatelessWidget {
  const _SearchBar({
    required this.controller,
    required this.focusNode,
    required this.filterUnseen,
    required this.onChanged,
    required this.onToggleUnseen,
    required this.onCreateActionSelected,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final bool filterUnseen;
  final ValueChanged<String> onChanged;
  final VoidCallback onToggleUnseen;
  final ValueChanged<ConversationCreateAction> onCreateActionSelected;

  @override
  Widget build(BuildContext context) {
    final isMacOS = defaultTargetPlatform == TargetPlatform.macOS;
    final scaffold = Scaffold.maybeOf(context);
    return MoveWindow(
      child: SizedBox(
        height: 64,
        child: Row(
          children: [
            AnimatedSize(
              duration: const Duration(milliseconds: 150),
              child: AnimatedSwitcher(
                duration: const Duration(milliseconds: 150),
                child: scaffold?.hasDrawer ?? false
                    ? ActionButton(
                        key: const ValueKey('open-drawer'),
                        onTapUp: (_) => scaffold?.openDrawer(),
                        child: Icon(
                          Icons.menu,
                          size: 20,
                          color: context.mixinTheme.icon,
                        ),
                      )
                    : const SizedBox(
                        key: ValueKey('drawer-placeholder'),
                        width: 16,
                      ),
              ),
            ),
            Expanded(
              child: MoveWindowBarrier(
                child: FocusableActionDetector(
                  shortcuts: const {
                    SingleActivator(LogicalKeyboardKey.escape):
                        _ClearSearchIntent(),
                  },
                  actions: {
                    _ClearSearchIntent: CallbackAction<_ClearSearchIntent>(
                      onInvoke: (_) {
                        controller.clear();
                        onChanged('');
                        focusNode.unfocus();
                        return null;
                      },
                    ),
                  },
                  child: SearchTextField(
                    controller: controller,
                    focusNode: focusNode,
                    hintText: filterUnseen
                        ? context.l10n.searchUnread
                        : '${context.l10n.search} '
                              '(${isMacOS ? '⌘' : 'Ctrl '}K)',
                    onChanged: onChanged,
                  ),
                ),
              ),
            ),
            const SizedBox(width: 8),
            ActionButton(
              name: MixinAssets.filterUnseen,
              color: filterUnseen
                  ? context.mixinTheme.accent
                  : context.mixinTheme.icon,
              onTap: onToggleUnseen,
            ),
            const SizedBox(width: 4),
            CustomPopupMenuButton<ConversationCreateAction>(
              icon: MixinAssets.add,
              onSelected: onCreateActionSelected,
              itemBuilder: (context) => [
                CustomPopupMenuItem(
                  value: ConversationCreateAction.searchContact,
                  icon: MixinAssets.contextMenuSearchUser,
                  title: context.l10n.searchContact,
                ),
                CustomPopupMenuItem(
                  value: ConversationCreateAction.conversation,
                  icon: MixinAssets.contextMenuCreateConversation,
                  title: context.l10n.createConversation,
                ),
                CustomPopupMenuItem(
                  value: ConversationCreateAction.group,
                  icon: MixinAssets.contextMenuCreateGroup,
                  title: context.l10n.createGroup,
                ),
                CustomPopupMenuItem(
                  value: ConversationCreateAction.circle,
                  icon: MixinAssets.circle,
                  title: context.l10n.createCircle,
                ),
              ],
            ),
            const SizedBox(width: 12),
          ],
        ),
      ),
    );
  }
}

class _ClearSearchIntent extends Intent {
  const _ClearSearchIntent();
}

class ConversationItem extends StatelessWidget {
  const ConversationItem({
    required this.conversation,
    required this.currentUserId,
    required this.selected,
    required this.onTap,
    super.key,
    this.account,
  });

  final ConversationListEntry conversation;
  final String currentUserId;
  final AccountHandle? account;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colors = context.mixinTheme;
    return SizedBox(
      height: 78,
      child: InteractiveDecoratedBox(
        onTap: onTap,
        decoration: BoxDecoration(color: colors.primary),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8),
          child: DecoratedBox(
            decoration: selected
                ? BoxDecoration(
                    color: colors.listSelected,
                    borderRadius: const BorderRadius.all(Radius.circular(8)),
                  )
                : const BoxDecoration(),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12),
              child: Row(
                children: [
                  ConversationAvatarView(conversation: conversation, size: 50),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Padding(
                      padding: const EdgeInsets.symmetric(vertical: 16),
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Row(
                            children: [
                              Expanded(
                                child: Row(
                                  children: [
                                    Flexible(
                                      child: CustomText(
                                        conversation.name,
                                        maxLines: 1,
                                        overflow: TextOverflow.ellipsis,
                                        style: TextStyle(
                                          color: colors.text,
                                          fontSize: 16,
                                        ),
                                      ),
                                    ),
                                    _ConversationBadge(
                                      verified: conversation.isVerified,
                                      isBot: conversation.isBot,
                                      membership: conversation.membership,
                                    ),
                                  ],
                                ),
                              ),
                              Text(
                                _formatTime(context, conversation.updatedAt),
                                style: TextStyle(
                                  color: colors.secondaryText,
                                  fontSize: 12,
                                ),
                              ),
                            ],
                          ),
                          _Subtitle(
                            conversation: conversation,
                            currentUserId: currentUserId,
                            account: account,
                          ),
                        ],
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _ConversationBadge extends StatelessWidget {
  const _ConversationBadge({
    required this.verified,
    required this.isBot,
    required this.membership,
  });

  final bool verified;
  final bool isBot;
  final String? membership;

  @override
  Widget build(BuildContext context) =>
      BadgesWidget(verified: verified, isBot: isBot, membership: membership);
}

class _Subtitle extends StatelessWidget {
  const _Subtitle({
    required this.conversation,
    required this.currentUserId,
    required this.account,
  });

  final ConversationListEntry conversation;
  final String currentUserId;
  final AccountHandle? account;

  @override
  Widget build(BuildContext context) => SizedBox(
    height: 20,
    child: Row(
      children: [
        Expanded(
          child: _MessagePreview(
            conversation: conversation,
            currentUserId: currentUserId,
            account: account,
          ),
        ),
        _ConversationIndicators(conversation: conversation),
      ],
    ),
  );
}

class _MessagePreview extends StatelessWidget {
  const _MessagePreview({
    required this.conversation,
    required this.currentUserId,
    required this.account,
  });

  final ConversationListEntry conversation;
  final String currentUserId;
  final AccountHandle? account;

  @override
  Widget build(BuildContext context) {
    final hasDraft = conversation.status != 3 && conversation.draft.isNotEmpty;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        if (!hasDraft)
          _MessageStatus(
            conversation: conversation,
            currentUserId: currentUserId,
          ),
        if (!hasDraft) const SizedBox(width: 2),
        Expanded(
          child: _MessageContent(
            conversation: conversation,
            currentUserId: currentUserId,
            hasDraft: hasDraft,
            account: account,
          ),
        ),
      ],
    );
  }
}

class _MessageContent extends HookWidget {
  const _MessageContent({
    required this.conversation,
    required this.currentUserId,
    required this.hasDraft,
    required this.account,
  });

  final ConversationListEntry conversation;
  final String currentUserId;
  final bool hasDraft;
  final AccountHandle? account;

  @override
  Widget build(BuildContext context) {
    final preview = hasDraft
        ? conversation.draft
        : conversation.contentType == null
        ? ''
        : _messagePreview(context, conversation, currentUserId);
    final resolvedPreview = useResolvedMessageMentions(
      hasDraft ? null : account,
      preview,
      revision: conversation,
    );
    if (conversation.contentType == null && !hasDraft) {
      return const SizedBox();
    }
    final colors = context.mixinTheme;
    final icon = hasDraft ? null : _messagePreviewIcon(conversation);
    final text = hasDraft ? preview : resolvedPreview;
    return Row(
      children: [
        if (icon != null) ...[
          SvgPicture.asset(
            icon,
            colorFilter: ColorFilter.mode(
              colors.secondaryText,
              BlendMode.srcIn,
            ),
          ),
          const SizedBox(width: 4),
        ],
        if (hasDraft) ...[
          Text(
            '${context.l10n.draft}:',
            style: TextStyle(color: colors.red, fontSize: 14),
          ),
          const SizedBox(width: 4),
        ],
        Expanded(
          child: CustomText(
            text,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(color: colors.secondaryText, fontSize: 14),
          ),
        ),
      ],
    );
  }
}

class _MessageStatus extends StatelessWidget {
  const _MessageStatus({
    required this.conversation,
    required this.currentUserId,
  });

  final ConversationListEntry conversation;
  final String currentUserId;

  @override
  Widget build(BuildContext context) {
    if (conversation.senderId != currentUserId ||
        !_supportsMessageStatus(conversation.contentType)) {
      return const SizedBox();
    }
    return MessageStatusIcon(
      messageId: 'conversation-${conversation.id}',
      status: conversation.messageStatus ?? '',
    );
  }
}

class _ConversationIndicators extends StatelessWidget {
  const _ConversationIndicators({required this.conversation});

  final ConversationListEntry conversation;

  @override
  Widget build(BuildContext context) {
    final colors = context.mixinTheme;
    final children = <Widget>[];
    if (conversation.mentionCount > 0) {
      children.add(_UnreadBadge(text: '@', color: colors.accent));
    }
    if (conversation.unseenCount > 0) {
      if (children.isNotEmpty) children.add(const SizedBox(width: 8));
      children.add(
        _UnreadBadge(
          text: '${conversation.unseenCount}',
          color: conversation.isMuted ? colors.secondaryText : colors.accent,
        ),
      );
    } else {
      final statuses = <Widget>[
        if (conversation.isMuted)
          SvgPicture.asset(
            MixinAssets.mute,
            colorFilter: ColorFilter.mode(
              colors.secondaryText,
              BlendMode.srcIn,
            ),
          ),
        if (conversation.isPinned)
          SvgPicture.asset(
            'assets/images/pin.svg',
            colorFilter: ColorFilter.mode(
              colors.secondaryText,
              BlendMode.srcIn,
            ),
          ),
      ];
      if (statuses.isNotEmpty) {
        if (children.isNotEmpty) children.add(const SizedBox(width: 8));
        children.add(
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              for (var index = 0; index < statuses.length; index++) ...[
                if (index > 0) const SizedBox(width: 4),
                statuses[index],
              ],
            ],
          ),
        );
      }
    }
    return Row(mainAxisSize: MainAxisSize.min, children: children);
  }
}

class _UnreadBadge extends StatelessWidget {
  const _UnreadBadge({required this.text, required this.color});

  final String text;
  final Color color;

  @override
  Widget build(BuildContext context) => Container(
    constraints: const BoxConstraints(
      minWidth: 26,
      minHeight: 20,
      maxHeight: 20,
    ),
    padding: const EdgeInsets.symmetric(horizontal: 5),
    alignment: Alignment.center,
    decoration: BoxDecoration(
      color: color,
      borderRadius: const BorderRadius.all(Radius.circular(10)),
    ),
    child: Text(
      text,
      maxLines: 1,
      textAlign: TextAlign.center,
      style: const TextStyle(color: Colors.white, fontSize: 12, height: 1),
    ),
  );
}

class _EmptyState extends StatelessWidget {
  const _EmptyState({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final color = context.dynamicColor(const Color.fromRGBO(229, 233, 240, 1));
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          SvgPicture.asset(
            MixinAssets.empty,
            width: 58,
            height: 78,
            colorFilter: ColorFilter.mode(color, BlendMode.srcIn),
          ),
          const SizedBox(height: 24),
          Text(text, style: TextStyle(color: color, fontSize: 14)),
        ],
      ),
    );
  }
}

class _ConversationContextMenu extends StatelessWidget {
  const _ConversationContextMenu({
    required this.conversation,
    required this.onPinned,
    required this.onMuted,
    required this.onDeleted,
    required this.circles,
    required this.currentCircleId,
    required this.onCircleChanged,
    required this.child,
  });

  final ConversationListEntry conversation;
  final ValueChanged<ConversationListEntry> onPinned;
  final void Function(ConversationListEntry, int) onMuted;
  final ValueChanged<ConversationListEntry> onDeleted;
  final Map<String, String> circles;
  final String? currentCircleId;
  final void Function(ConversationListEntry, String, bool) onCircleChanged;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final canAddToCircle = circles.keys.any(
      (circleId) => !conversation.circleIds.contains(circleId),
    );
    return ContextMenuWidget(
      desktopMenuWidgetBuilder: CustomDesktopMenuWidgetBuilder(),
      menuProvider: (_) => Menu(
        children: [
          MenuAction(
            image: MenuImage.icon(
              conversation.isPinned ? IconFonts.unPin : IconFonts.pin,
            ),
            title: conversation.isPinned
                ? context.l10n.unpin
                : context.l10n.pinTitle,
            callback: () => onPinned(conversation),
          ),
          MenuAction(
            image: MenuImage.icon(
              conversation.isMuted ? IconFonts.unMute : IconFonts.mute,
            ),
            title: conversation.isMuted
                ? context.l10n.unmute
                : context.l10n.mute,
            callback: () async {
              if (conversation.isMuted) {
                onMuted(conversation, 0);
                return;
              }
              final duration = await showMuteDialog(context);
              if (duration != null) onMuted(conversation, duration);
            },
          ),
          MenuSeparator(),
          if (canAddToCircle) ...[
            Menu(
              title: context.l10n.addToCircle,
              children: circles.entries
                  .where((entry) => !conversation.circleIds.contains(entry.key))
                  .map(
                    (entry) => MenuAction(
                      title: entry.value,
                      callback: () =>
                          onCircleChanged(conversation, entry.key, true),
                    ),
                  )
                  .toList(),
            ),
            MenuSeparator(),
          ],
          MenuAction(
            image: MenuImage.icon(IconFonts.delete),
            title: context.l10n.deleteChat,
            callback: () async {
              final confirmed = await showConfirmMixinDialog(
                context,
                context.l10n.conversationDeleteTitle(conversation.name),
                description: context.l10n.deleteChatDescription,
              );
              if (confirmed != null) onDeleted(conversation);
            },
          ),
          if (currentCircleId != null &&
              conversation.circleIds.contains(currentCircleId))
            MenuAction(
              image: MenuImage.icon(IconFonts.delete),
              title: context.l10n.removeChatFromCircle,
              callback: () =>
                  onCircleChanged(conversation, currentCircleId!, false),
            ),
        ],
      ),
      child: child,
    );
  }
}

bool _supportsMessageStatus(String? category) {
  const unsupported = {
    'SYSTEM_CONVERSATION',
    'SYSTEM_ACCOUNT_SNAPSHOT',
    'MESSAGE_RECALL',
    'MESSAGE_PIN',
    'WEBRTC_AUDIO_CANCEL',
    'WEBRTC_AUDIO_DECLINE',
    'WEBRTC_AUDIO_END',
    'WEBRTC_AUDIO_BUSY',
    'WEBRTC_AUDIO_FAILED',
    'KRAKEN_END',
    'KRAKEN_DECLINE',
    'KRAKEN_CANCEL',
    'KRAKEN_INVITE',
  };
  return !unsupported.contains(category);
}

String? _messagePreviewIcon(ConversationListEntry conversation) {
  if (conversation.messageStatus == 'FAILED') return null;
  final category = conversation.contentType;
  if (category == 'SYSTEM_SAFE_INSCRIPTION') {
    return 'assets/images/transfer.svg';
  }
  if (category?.contains('TRANSCRIPT') ?? false) {
    return 'assets/images/file.svg';
  }
  if (_isCallMessage(category)) return 'assets/images/video_call.svg';
  return MixinAssets.messageIcon(category);
}

String _messagePreview(
  BuildContext context,
  ConversationListEntry conversation,
  String currentUserId,
) {
  final category = conversation.contentType;
  if (category == null) return '';
  String text;
  if (conversation.messageStatus == 'FAILED') {
    text = context.l10n.waitingForThisMessage;
  } else if (conversation.messageStatus == 'UNKNOWN') {
    text = context.l10n.messageNotSupport;
  } else if (category == 'SYSTEM_CONVERSATION') {
    text = generateSystemMessageText(
      context,
      action: conversation.lastMessageAction,
      participantId: conversation.lastMessageParticipantId,
      participantName: conversation.lastMessageParticipantName,
      senderId: conversation.senderId,
      senderName: conversation.senderName,
      currentUserId: currentUserId,
      expireIn: int.tryParse(conversation.content),
    );
  } else if (category == 'MESSAGE_PIN') {
    text = context.l10n.chatPinMessage(
      conversation.senderName ?? '',
      pinMessagePreview(context.l10n, conversation.content),
    );
  } else if (category.contains('TEXT')) {
    text = conversation.content.trim();
  } else if (category.contains('SNAPSHOT')) {
    text = '[${context.l10n.transfer}]';
  } else if (category.contains('STICKER')) {
    text = '[${context.l10n.sticker}]';
  } else if (category.contains('IMAGE')) {
    text = '[${context.l10n.image}]';
  } else if (category.contains('VIDEO')) {
    text = '[${context.l10n.video}]';
  } else if (category.contains('LIVE')) {
    text = '[${context.l10n.live}]';
  } else if (category.contains('DATA')) {
    text = '[${context.l10n.file}]';
  } else if (category.contains('POST')) {
    text = conversation.content.trim().isEmpty
        ? context.l10n.post
        : conversation.content.trim();
  } else if (category.contains('LOCATION')) {
    text = '[${context.l10n.location}]';
  } else if (category.contains('AUDIO')) {
    text = '[${context.l10n.audio}]';
  } else if (category == 'APP_BUTTON_GROUP') {
    text = _appButtonGroupPreview(conversation.content);
  } else if (category == 'APP_CARD') {
    text = _appCardPreview(context, conversation.content);
  } else if (category.contains('CONTACT')) {
    text = '[${context.l10n.contact}]';
  } else if (_isCallMessage(category)) {
    text = context.l10n.contentVoice;
  } else if (category.contains('RECALL')) {
    text = conversation.senderId == currentUserId
        ? '[${context.l10n.youDeletedThisMessage}]'
        : '[${context.l10n.thisMessageWasDeleted}]';
  } else if (category.contains('TRANSCRIPT')) {
    text = '[${context.l10n.transcript}]';
  } else if (category.contains('INSCRIPTION')) {
    text = '[${context.l10n.collectible}]';
  } else {
    text = context.l10n.messageNotSupport;
  }
  if (category == 'SYSTEM_CONVERSATION' || category == 'MESSAGE_PIN') {
    return text;
  }
  final showSender =
      conversation.isGroup || conversation.senderId != conversation.ownerId;
  if (text.isNotEmpty && showSender) {
    final sender = conversation.senderId == currentUserId
        ? context.l10n.you
        : conversation.senderName ?? '';
    return '$sender: $text';
  }
  return text;
}

bool _isCallMessage(String? category) => const {
  'WEBRTC_AUDIO_CANCEL',
  'WEBRTC_AUDIO_DECLINE',
  'WEBRTC_AUDIO_END',
  'WEBRTC_AUDIO_BUSY',
  'WEBRTC_AUDIO_FAILED',
}.contains(category);

String _appButtonGroupPreview(String content) {
  try {
    final items = jsonDecode(content) as List<dynamic>;
    return items
        .whereType<Map<String, dynamic>>()
        .map((item) => item['label'])
        .whereType<String>()
        .map((label) => '[$label]')
        .join();
  } on Object {
    return '';
  }
}

String _appCardPreview(BuildContext context, String content) {
  try {
    final card = jsonDecode(content) as Map<String, dynamic>;
    final title = card['title'];
    if (title is String) return '[$title]';
  } on Object {
    // Use the localized fallback below.
  }
  return '[${context.l10n.card}]';
}

String _formatTime(BuildContext context, DateTime value) {
  final now = DateTime.now();
  final local = value.toLocal();
  if (local.year == now.year &&
      local.month == now.month &&
      local.day == now.day) {
    return DateFormat.Hm().format(local);
  }
  final startOfWeek = now.subtract(Duration(days: now.weekday));
  final valueWeek = local.subtract(Duration(days: local.weekday));
  if (startOfWeek.year == valueWeek.year &&
      startOfWeek.month == valueWeek.month &&
      startOfWeek.day == valueWeek.day) {
    return DateFormat.E().format(local);
  }
  if (local.year == now.year) return DateFormat.MMMd().format(local);
  return DateFormat.yMMMd().format(local);
}
