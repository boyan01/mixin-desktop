import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/constants/icon_fonts.dart';
import 'package:mixin_desktop_ui/controllers/paging_controller.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:super_context_menu/super_context_menu.dart';
import 'package:intl/intl.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';

class ConversationListView extends StatefulWidget {
  const ConversationListView({
    required this.pagingState,
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
    required this.onSelected,
    required this.onPinned,
    required this.onMuted,
    required this.onDeleted,
    required this.onCircleChanged,
    required this.onRetry,
    super.key,
    this.error,
  });

  final PagingState<ConversationListEntry> pagingState;
  final ItemPositionsListener itemPositionsListener;
  final ItemScrollController itemScrollController;
  final bool loading;
  final String? error;
  final String currentUserId;
  final Map<String, String> circles;
  final String? currentCircleId;
  final bool filterUnseen;
  final String? selectedConversationId;
  final ValueChanged<String> onQueryChanged;
  final VoidCallback onToggleUnseen;
  final ValueChanged<ConversationListEntry> onSelected;
  final ValueChanged<ConversationListEntry> onPinned;
  final void Function(ConversationListEntry, int) onMuted;
  final ValueChanged<ConversationListEntry> onDeleted;
  final void Function(ConversationListEntry, String, bool) onCircleChanged;
  final VoidCallback onRetry;

  @override
  State<ConversationListView> createState() => _ConversationListViewState();
}

class _ConversationListViewState extends State<ConversationListView> {
  final searchController = TextEditingController();
  final searchFocusNode = FocusNode();

  @override
  void dispose() {
    searchController.dispose();
    searchFocusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Material(
    color: context.mixinTheme.primary,
    child: Column(
      children: [
        _SearchBar(
          controller: searchController,
          focusNode: searchFocusNode,
          filterUnseen: widget.filterUnseen,
          onChanged: widget.onQueryChanged,
          onToggleUnseen: widget.onToggleUnseen,
        ),
        Expanded(child: _buildList()),
      ],
    ),
  );

  Widget _buildList() {
    if (widget.loading && !widget.pagingState.initialized) {
      return Center(
        child: CircularProgressIndicator(
          strokeWidth: 2,
          color: context.mixinTheme.accent,
        ),
      );
    }
    if (widget.error != null && widget.pagingState.count == 0) {
      return Center(
        child: TextButton.icon(
          onPressed: widget.onRetry,
          icon: const Icon(Icons.refresh),
          label: Text(context.l10n.retry),
        ),
      );
    }
    if (widget.pagingState.count == 0) {
      final searching = searchController.text.isNotEmpty || widget.filterUnseen;
      return _EmptyState(
        text: searching ? context.l10n.searchEmpty : context.l10n.noData,
      );
    }
    return ScrollablePositionedList.builder(
      itemPositionsListener: widget.itemPositionsListener,
      itemScrollController: widget.itemScrollController,
      itemCount: widget.pagingState.count,
      itemBuilder: (context, index) {
        final conversation = widget.pagingState.map[index];
        if (conversation == null) return const SizedBox(height: 78);
        return _ConversationContextMenu(
          conversation: conversation,
          onPinned: widget.onPinned,
          onMuted: widget.onMuted,
          onDeleted: widget.onDeleted,
          circles: widget.circles,
          currentCircleId: widget.currentCircleId,
          onCircleChanged: widget.onCircleChanged,
          child: ConversationItem(
            conversation: conversation,
            currentUserId: widget.currentUserId,
            selected: conversation.id == widget.selectedConversationId,
            onTap: () => widget.onSelected(conversation),
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
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final bool filterUnseen;
  final ValueChanged<String> onChanged;
  final VoidCallback onToggleUnseen;

  @override
  Widget build(BuildContext context) {
    final isMacOS = defaultTargetPlatform == TargetPlatform.macOS;
    return SizedBox(
      height: 64,
      child: Row(
        children: [
          const SizedBox(width: 16),
          Expanded(
            child: CallbackShortcuts(
              bindings: {
                const SingleActivator(LogicalKeyboardKey.escape): () {
                  controller.clear();
                  onChanged('');
                  focusNode.unfocus();
                },
                SingleActivator(
                  LogicalKeyboardKey.keyK,
                  meta: isMacOS,
                  control: !isMacOS,
                ): focusNode.requestFocus,
              },
              child: _SearchField(
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
          const SizedBox(width: 8),
          _ActionButton(
            asset: MixinAssets.filterUnseen,
            color: filterUnseen
                ? context.mixinTheme.accent
                : context.mixinTheme.icon,
            onTap: onToggleUnseen,
          ),
          const SizedBox(width: 4),
          PopupMenuButton<void>(
            tooltip: '',
            padding: const EdgeInsets.all(8),
            iconSize: 24,
            style: _actionButtonStyle(context),
            icon: SvgPicture.asset(
              MixinAssets.add,
              width: 24,
              height: 24,
              colorFilter: ColorFilter.mode(
                context.mixinTheme.icon,
                BlendMode.srcIn,
              ),
            ),
            itemBuilder: (context) => [
              PopupMenuItem(child: Text(context.l10n.searchContact)),
              PopupMenuItem(child: Text(context.l10n.createConversation)),
              PopupMenuItem(child: Text(context.l10n.createGroup)),
              PopupMenuItem(child: Text(context.l10n.createCircle)),
            ],
          ),
          const SizedBox(width: 12),
        ],
      ),
    );
  }
}

class _SearchField extends StatefulWidget {
  const _SearchField({
    required this.controller,
    required this.focusNode,
    required this.hintText,
    required this.onChanged,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final String hintText;
  final ValueChanged<String> onChanged;

  @override
  State<_SearchField> createState() => _SearchFieldState();
}

class _SearchFieldState extends State<_SearchField> {
  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_changed);
  }

  @override
  void didUpdateWidget(covariant _SearchField oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      oldWidget.controller.removeListener(_changed);
      widget.controller.addListener(_changed);
    }
  }

  void _changed() => setState(() {});

  @override
  void dispose() {
    widget.controller.removeListener(_changed);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.mixinTheme;
    final background = Theme.of(context).brightness == Brightness.light
        ? const Color.fromRGBO(245, 247, 250, 1)
        : Colors.white.withValues(alpha: 0.08);
    return Container(
      height: 36,
      decoration: ShapeDecoration(
        color: background,
        shape: const StadiumBorder(),
      ),
      child: Row(
        children: [
          Padding(
            padding: const EdgeInsets.only(left: 16, right: 8),
            child: SvgPicture.asset(
              MixinAssets.search,
              colorFilter: ColorFilter.mode(
                colors.secondaryText,
                BlendMode.srcIn,
              ),
            ),
          ),
          Expanded(
            child: TextField(
              controller: widget.controller,
              focusNode: widget.focusNode,
              onChanged: widget.onChanged,
              style: TextStyle(color: colors.text, fontSize: 14),
              decoration: InputDecoration(
                isDense: true,
                border: InputBorder.none,
                hintText: widget.hintText,
                hintStyle: TextStyle(color: colors.secondaryText),
                contentPadding: EdgeInsets.zero,
              ),
            ),
          ),
          if (widget.controller.text.isNotEmpty)
            InkWell(
              onTap: () {
                widget.controller.clear();
                widget.onChanged('');
              },
              child: Padding(
                padding: const EdgeInsets.only(right: 16, left: 8),
                child: Icon(Icons.close, color: colors.secondaryText, size: 16),
              ),
            )
          else
            const SizedBox(width: 40),
        ],
      ),
    );
  }
}

class _ActionButton extends StatelessWidget {
  const _ActionButton({
    required this.asset,
    required this.color,
    required this.onTap,
  });

  final String asset;
  final Color color;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => IconButton(
    onPressed: onTap,
    padding: const EdgeInsets.all(8),
    iconSize: 24,
    style: _actionButtonStyle(context),
    icon: SvgPicture.asset(
      asset,
      width: 24,
      height: 24,
      colorFilter: ColorFilter.mode(color, BlendMode.srcIn),
    ),
  );
}

ButtonStyle _actionButtonStyle(BuildContext context) {
  final hoverColor = Theme.of(context).brightness == Brightness.light
      ? const Color.fromRGBO(0, 0, 0, 0.03)
      : const Color.fromRGBO(255, 255, 255, 0.2);
  return ButtonStyle(
    fixedSize: const WidgetStatePropertyAll(Size.square(40)),
    minimumSize: const WidgetStatePropertyAll(Size.square(40)),
    maximumSize: const WidgetStatePropertyAll(Size.square(40)),
    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
    shape: const WidgetStatePropertyAll(CircleBorder()),
    splashFactory: NoSplash.splashFactory,
    backgroundColor: WidgetStateProperty.resolveWith((states) {
      if (states.contains(WidgetState.hovered) ||
          states.contains(WidgetState.pressed)) {
        return hoverColor;
      }
      return Colors.transparent;
    }),
    overlayColor: const WidgetStatePropertyAll(Colors.transparent),
  );
}

class ConversationItem extends StatelessWidget {
  const ConversationItem({
    required this.conversation,
    required this.currentUserId,
    required this.selected,
    required this.onTap,
    super.key,
  });

  final ConversationListEntry conversation;
  final String currentUserId;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colors = context.mixinTheme;
    return Material(
      color: colors.primary,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8),
        child: InkWell(
          onTap: onTap,
          borderRadius: const BorderRadius.all(Radius.circular(8)),
          child: Container(
            decoration: BoxDecoration(
              color: selected ? colors.listSelected : null,
              borderRadius: const BorderRadius.all(Radius.circular(8)),
            ),
            padding: const EdgeInsets.symmetric(horizontal: 12),
            child: Row(
              children: [
                _ConversationAvatar(conversation: conversation),
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
                                    child: Text(
                                      conversation.name,
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                      style: TextStyle(
                                        color: colors.text,
                                        fontSize: 16,
                                      ),
                                    ),
                                  ),
                                  if (conversation.isVerified)
                                    Padding(
                                      padding: const EdgeInsets.only(left: 3),
                                      child: SvgPicture.asset(
                                        MixinAssets.verified,
                                        width: 14,
                                        height: 14,
                                      ),
                                    ),
                                  if (conversation.isBot)
                                    Padding(
                                      padding: const EdgeInsets.only(left: 3),
                                      child: SvgPicture.asset(
                                        MixinAssets.bots,
                                        width: 14,
                                        height: 14,
                                        colorFilter: ColorFilter.mode(
                                          colors.accent,
                                          BlendMode.srcIn,
                                        ),
                                      ),
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
    );
  }
}

class _Subtitle extends StatelessWidget {
  const _Subtitle({required this.conversation, required this.currentUserId});

  final ConversationListEntry conversation;
  final String currentUserId;

  @override
  Widget build(BuildContext context) {
    final colors = context.mixinTheme;
    final hasDraft = conversation.status != 3 && conversation.draft.isNotEmpty;
    final icon = hasDraft
        ? null
        : MixinAssets.messageIcon(conversation.contentType);
    return SizedBox(
      height: 20,
      child: Row(
        children: [
          if (!hasDraft &&
              conversation.senderId == currentUserId &&
              conversation.messageStatus != null)
            _MessageStatus(status: conversation.messageStatus!),
          if (icon != null)
            Padding(
              padding: const EdgeInsets.only(right: 4),
              child: SvgPicture.asset(
                icon,
                width: 14,
                height: 14,
                colorFilter: ColorFilter.mode(
                  colors.secondaryText,
                  BlendMode.srcIn,
                ),
              ),
            ),
          if (hasDraft)
            Text(
              '${context.l10n.draft}:',
              style: TextStyle(color: colors.red, fontSize: 14),
            ),
          Expanded(
            child: Text(
              hasDraft
                  ? conversation.draft
                  : _messagePreview(context, conversation, currentUserId),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(color: colors.secondaryText, fontSize: 14),
            ),
          ),
          if (conversation.mentionCount > 0)
            _UnreadBadge(text: '@', color: colors.accent),
          if (conversation.unseenCount > 0)
            Padding(
              padding: const EdgeInsets.only(left: 8),
              child: _UnreadBadge(
                text: '${conversation.unseenCount}',
                color: conversation.isMuted
                    ? colors.secondaryText
                    : colors.accent,
              ),
            )
          else ...[
            if (conversation.isMuted)
              SvgPicture.asset(
                MixinAssets.mute,
                colorFilter: ColorFilter.mode(
                  colors.secondaryText,
                  BlendMode.srcIn,
                ),
              ),
            if (conversation.isPinned)
              Padding(
                padding: const EdgeInsets.only(left: 4),
                child: SvgPicture.asset(
                  'assets/images/pin.svg',
                  colorFilter: ColorFilter.mode(
                    colors.secondaryText,
                    BlendMode.srcIn,
                  ),
                ),
              ),
          ],
        ],
      ),
    );
  }
}

class _MessageStatus extends StatelessWidget {
  const _MessageStatus({required this.status});

  final String status;

  @override
  Widget build(BuildContext context) {
    final asset = switch (status) {
      'FAILED' => 'assets/images/failed.svg',
      'READ' => 'assets/images/read.svg',
      'DELIVERED' => 'assets/images/delivered.svg',
      _ => null,
    };
    if (asset == null) return const SizedBox();
    return Padding(
      padding: const EdgeInsets.only(right: 2),
      child: SvgPicture.asset(
        asset,
        colorFilter: ColorFilter.mode(
          context.mixinTheme.secondaryText,
          BlendMode.srcIn,
        ),
      ),
    );
  }
}

class _ConversationAvatar extends StatelessWidget {
  const _ConversationAvatar({required this.conversation});

  final ConversationListEntry conversation;

  @override
  Widget build(BuildContext context) {
    if (!conversation.isGroup) {
      return AvatarView(
        userId: conversation.ownerId,
        name: conversation.name,
        avatarUrl: conversation.avatarUrl,
        size: 50,
      );
    }
    final avatars = conversation.groupAvatars.take(4).toList();
    if (avatars.isEmpty && conversation.avatarUrl.isNotEmpty) {
      return AvatarView(
        userId: conversation.id,
        name: conversation.name,
        avatarUrl: conversation.avatarUrl,
        size: 50,
      );
    }
    return ClipOval(
      child: SizedBox.square(
        dimension: 50,
        child: GridView.count(
          physics: const NeverScrollableScrollPhysics(),
          crossAxisCount: avatars.length == 1 ? 1 : 2,
          children: avatars
              .map(
                (avatar) => AvatarView(
                  userId: avatar.userId,
                  name: avatar.name,
                  avatarUrl: avatar.avatarUrl,
                  size: 25,
                  clipOval: false,
                ),
              )
              .toList(),
        ),
      ),
    );
  }
}

class _UnreadBadge extends StatelessWidget {
  const _UnreadBadge({required this.text, required this.color});

  final String text;
  final Color color;

  @override
  Widget build(BuildContext context) => Container(
    constraints: const BoxConstraints(minWidth: 20, minHeight: 20),
    padding: const EdgeInsets.symmetric(horizontal: 6),
    alignment: Alignment.center,
    decoration: BoxDecoration(
      color: color,
      borderRadius: const BorderRadius.all(Radius.circular(10)),
    ),
    child: Text(
      text,
      style: const TextStyle(color: Colors.white, fontSize: 11),
    ),
  );
}

class _EmptyState extends StatelessWidget {
  const _EmptyState({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) => Center(
    child: Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        SvgPicture.asset(
          MixinAssets.empty,
          width: 58,
          height: 78,
          colorFilter: ColorFilter.mode(
            context.mixinTheme.divider,
            BlendMode.srcIn,
          ),
        ),
        const SizedBox(height: 24),
        Text(
          text,
          style: TextStyle(color: context.mixinTheme.divider, fontSize: 14),
        ),
      ],
    ),
  );
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
  Widget build(BuildContext context) => ContextMenuWidget(
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
          title: conversation.isMuted ? context.l10n.unmute : context.l10n.mute,
          callback: () async {
            if (conversation.isMuted) {
              onMuted(conversation, 0);
              return;
            }
            final duration = await _showMuteDialog(context);
            if (duration != null) onMuted(conversation, duration);
          },
        ),
        if (circles.keys.any(
          (circleId) => !conversation.circleIds.contains(circleId),
        ))
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
        MenuAction(
          image: MenuImage.icon(IconFonts.delete),
          title: context.l10n.deleteChat,
          attributes: const MenuActionAttributes(destructive: true),
          callback: () async {
            final confirmed = await showDialog<bool>(
              context: context,
              builder: (context) => AlertDialog(
                title: Text(
                  context.l10n.conversationDeleteTitle(conversation.name),
                ),
                content: Text(context.l10n.deleteChatDescription),
                actions: [
                  TextButton(
                    onPressed: () => Navigator.pop(context, false),
                    child: Text(context.l10n.cancel),
                  ),
                  TextButton(
                    onPressed: () => Navigator.pop(context, true),
                    child: Text(context.l10n.delete),
                  ),
                ],
              ),
            );
            if (confirmed ?? false) onDeleted(conversation);
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

Future<int?> _showMuteDialog(BuildContext context) => showDialog<int>(
  context: context,
  builder: (context) => SimpleDialog(
    title: Text(context.l10n.contactMuteTitle),
    children:
        [
              (context.l10n.oneHour, 60 * 60),
              (context.l10n.hour(8, 8), 8 * 60 * 60),
              ('1 ${context.l10n.unitWeek(1)}', 7 * 24 * 60 * 60),
              (context.l10n.oneYear, 365 * 24 * 60 * 60),
            ]
            .map(
              (option) => SimpleDialogOption(
                onPressed: () => Navigator.pop(context, option.$2),
                child: Text(option.$1),
              ),
            )
            .toList(),
  ),
);

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
  } else if (category == 'APP_CARD') {
    text = '[${context.l10n.card}]';
  } else if (category.contains('CONTACT')) {
    text = '[${context.l10n.contact}]';
  } else if (category.contains('CALL')) {
    text = context.l10n.contentVoice;
  } else if (category.contains('RECALL')) {
    text = conversation.senderId == currentUserId
        ? context.l10n.youDeletedThisMessage
        : context.l10n.thisMessageWasDeleted;
  } else if (category.contains('TRANSCRIPT')) {
    text = '[${context.l10n.transcript}]';
  } else if (category.contains('INSCRIPTION')) {
    text = '[${context.l10n.collectible}]';
  } else {
    text = context.l10n.messageNotSupport;
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
