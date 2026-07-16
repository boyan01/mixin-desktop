import 'dart:convert';

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
    this.query = '',
  });

  final PagingState<ConversationListEntry> pagingState;
  final ItemPositionsListener itemPositionsListener;
  final ItemScrollController itemScrollController;
  final bool loading;
  final String? error;
  final String currentUserId;
  final Map<String, String> circles;
  final String? currentCircleId;
  final String query;
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
      final searching = widget.query.isNotEmpty || widget.filterUnseen;
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
    final scaffold = Scaffold.maybeOf(context);
    return SizedBox(
      height: 64,
      child: Row(
        children: [
          AnimatedSize(
            duration: const Duration(milliseconds: 150),
            child: AnimatedSwitcher(
              duration: const Duration(milliseconds: 150),
              child: scaffold?.hasDrawer ?? false
                  ? IconButton(
                      key: const ValueKey('open-drawer'),
                      onPressed: scaffold?.openDrawer,
                      padding: const EdgeInsets.all(8),
                      iconSize: 20,
                      style: _actionButtonStyle(context),
                      icon: Icon(Icons.menu, color: context.mixinTheme.icon),
                    )
                  : const SizedBox(
                      key: ValueKey('drawer-placeholder'),
                      width: 16,
                    ),
            ),
          ),
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
          _ActionButton(
            asset: MixinAssets.add,
            color: context.mixinTheme.secondaryText,
            onTap: null,
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
              inputFormatters: [LengthLimitingTextInputFormatter(200)],
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
  final VoidCallback? onTap;

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
    return SizedBox(
      height: 78,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: onTap,
        child: ColoredBox(
          color: colors.primary,
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
                    ConversationAvatarView(
                      conversation: conversation,
                      size: 50,
                    ),
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
                                      _ConversationBadge(
                                        verified: conversation.isVerified,
                                        isBot: conversation.isBot,
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
        ),
      ),
    );
  }
}

class _ConversationBadge extends StatelessWidget {
  const _ConversationBadge({required this.verified, required this.isBot});

  final bool verified;
  final bool isBot;

  @override
  Widget build(BuildContext context) {
    final asset = verified
        ? MixinAssets.verified
        : isBot
        ? 'assets/images/bot_fill.svg'
        : null;
    if (asset == null) return const SizedBox();
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4),
      child: SvgPicture.asset(asset, width: 12, height: 12),
    );
  }
}

class _Subtitle extends StatelessWidget {
  const _Subtitle({required this.conversation, required this.currentUserId});

  final ConversationListEntry conversation;
  final String currentUserId;

  @override
  Widget build(BuildContext context) => SizedBox(
    height: 20,
    child: Row(
      children: [
        Expanded(
          child: _MessagePreview(
            conversation: conversation,
            currentUserId: currentUserId,
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
  });

  final ConversationListEntry conversation;
  final String currentUserId;

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
          ),
        ),
      ],
    );
  }
}

class _MessageContent extends StatelessWidget {
  const _MessageContent({
    required this.conversation,
    required this.currentUserId,
    required this.hasDraft,
  });

  final ConversationListEntry conversation;
  final String currentUserId;
  final bool hasDraft;

  @override
  Widget build(BuildContext context) {
    if (conversation.contentType == null && !hasDraft) {
      return const SizedBox();
    }
    final colors = context.mixinTheme;
    final icon = hasDraft ? null : _messagePreviewIcon(conversation);
    final text = hasDraft
        ? conversation.draft
        : _messagePreview(context, conversation, currentUserId);
    return Row(
      children: [
        if (icon != null) ...[
          SvgPicture.asset(
            icon,
            width: 14,
            height: 14,
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
          child: Text(
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
    final colors = context.mixinTheme;
    return switch (conversation.messageStatus) {
      'SENT' => SvgPicture.asset(
        MixinAssets.sent,
        colorFilter: ColorFilter.mode(colors.secondaryText, BlendMode.srcIn),
      ),
      'DELIVERED' => SvgPicture.asset(
        MixinAssets.delivered,
        colorFilter: ColorFilter.mode(colors.secondaryText, BlendMode.srcIn),
      ),
      'READ' => SvgPicture.asset(
        MixinAssets.read,
        colorFilter: ColorFilter.mode(colors.accent, BlendMode.srcIn),
      ),
      _ => CustomPaint(
        painter: _SendingStatusPainter(color: colors.secondaryText),
        child: const SizedBox.square(dimension: 14),
      ),
    };
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

class _SendingStatusPainter extends CustomPainter {
  const _SendingStatusPainter({required this.color});

  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeWidth = 1;
    final center = Offset(size.width / 2, size.height / 2);
    canvas.drawRRect(
      RRect.fromRectAndRadius(
        Rect.fromCenter(center: center, width: 11, height: 9),
        const Radius.circular(2.15),
      ),
      paint,
    );
    canvas
      ..drawLine(center, center.translate(0, -3), paint)
      ..drawLine(center, center.translate(3, 0), paint);
  }

  @override
  bool shouldRepaint(covariant _SendingStatusPainter oldDelegate) =>
      oldDelegate.color != color;
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
    const color = Color.fromRGBO(229, 233, 240, 1);
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          SvgPicture.asset(
            MixinAssets.empty,
            width: 58,
            height: 78,
            colorFilter: const ColorFilter.mode(color, BlendMode.srcIn),
          ),
          const SizedBox(height: 24),
          Text(text, style: const TextStyle(color: color, fontSize: 14)),
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
              final duration = await _showMuteDialog(context);
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
