import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_portal/flutter_portal.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:super_context_menu/super_context_menu.dart';

import '../constants/assets.dart';
import '../constants/icon_fonts.dart';
import '../controllers/conversation_list_controller.dart';
import '../l10n/l10n.dart';
import '../models/conversation_list_entry.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../theme.dart';
import '../utils/app_logger.dart';
import 'avatar_view.dart';
import 'custom_context_menu.dart';
import 'mixin_dialog.dart';
import 'show_forward_conversation_selector.dart';
import 'toast.dart';

class HomeSidebar extends HookWidget {
  const HomeSidebar({
    required this.controller,
    required this.collapsed,
    required this.showCollapse,
    required this.profileSelected,
    required this.onProfileSelected,
    required this.onCategorySelected,
    required this.onToggleCollapsed,
    super.key,
  });

  final ConversationListController controller;
  final bool collapsed;
  final bool showCollapse;
  final bool profileSelected;
  final VoidCallback onProfileSelected;
  final VoidCallback onCategorySelected;
  final VoidCallback onToggleCollapsed;

  @override
  Widget build(BuildContext context) {
    final colors = context.mixinTheme;
    final unseenCounts = useStream(
      useMemoized(controller.account.unseenCountChanges, [controller.account]),
      initialData: const <rust.ConversationUnseenCount>[],
    ).data!;
    final circles = useStream(
      useMemoized(controller.account.circleChanges, [controller.account]),
      initialData: const <rust.CircleItem>[],
    ).data!;
    final orderedCircles = useState(circles);
    useEffect(() {
      orderedCircles.value = circles;
      return null;
    }, [circles]);
    final visibleCircles = orderedCircles.value;
    return RepaintBoundary(
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: Theme.of(context).brightness == Brightness.light
              ? Colors.black.withValues(alpha: 0.03)
              : Colors.white.withValues(alpha: 0.01),
          border: Border(right: BorderSide(color: colors.divider)),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              SizedBox(
                height: defaultTargetPlatform == TargetPlatform.macOS ? 64 : 16,
              ),
              _ProfileItem(
                account: controller.account,
                collapsed: collapsed,
                selected: profileSelected,
                onTap: onProfileSelected,
              ),
              const SizedBox(height: 24),
              _CategoryItem(
                asset: MixinAssets.chat,
                title: context.l10n.allChats,
                filter: ConversationCategoryFilter.chats,
                controller: controller,
                unseenCounts: unseenCounts,
                collapsed: collapsed,
                onSelected: onCategorySelected,
              ),
              const SizedBox(height: 12),
              const _Divider(),
              const SizedBox(height: 12),
              _CategoryItem(
                asset: MixinAssets.contacts,
                title: context.l10n.contactTitle,
                filter: ConversationCategoryFilter.contacts,
                controller: controller,
                unseenCounts: unseenCounts,
                collapsed: collapsed,
                onSelected: onCategorySelected,
              ),
              const SizedBox(height: 8),
              _CategoryItem(
                asset: MixinAssets.groups,
                title: context.l10n.groups,
                filter: ConversationCategoryFilter.groups,
                controller: controller,
                unseenCounts: unseenCounts,
                collapsed: collapsed,
                onSelected: onCategorySelected,
              ),
              const SizedBox(height: 8),
              _CategoryItem(
                asset: MixinAssets.bots,
                title: context.l10n.botsTitle,
                filter: ConversationCategoryFilter.bots,
                controller: controller,
                unseenCounts: unseenCounts,
                collapsed: collapsed,
                onSelected: onCategorySelected,
              ),
              const SizedBox(height: 8),
              _CategoryItem(
                asset: MixinAssets.strangers,
                title: context.l10n.strangers,
                filter: ConversationCategoryFilter.strangers,
                controller: controller,
                unseenCounts: unseenCounts,
                collapsed: collapsed,
                onSelected: onCategorySelected,
              ),
              const SizedBox(height: 16),
              if (visibleCircles.isNotEmpty) const _Divider(),
              if (visibleCircles.isNotEmpty) const SizedBox(height: 12),
              Expanded(
                child: ReorderableListView.builder(
                  padding: EdgeInsets.zero,
                  buildDefaultDragHandles: false,
                  onReorderItem: (oldIndex, newIndex) => unawaited(
                    _reorderCircles(
                      context,
                      controller,
                      visibleCircles,
                      orderedCircles,
                      oldIndex,
                      newIndex,
                    ),
                  ),
                  itemCount: visibleCircles.length,
                  itemBuilder: (context, index) {
                    final circle = visibleCircles[index];
                    return Listener(
                      key: ValueKey(circle.circleId),
                      onPointerDown: (event) {
                        if (event.buttons != kPrimaryButton) return;
                        ReorderableList.maybeOf(context)?.startItemDragReorder(
                          index: index,
                          event: event,
                          recognizer: ImmediateMultiDragGestureRecognizer(
                            supportedDevices: const {
                              PointerDeviceKind.touch,
                              PointerDeviceKind.mouse,
                            },
                          ),
                        );
                      },
                      child: ContextMenuWidget(
                        desktopMenuWidgetBuilder:
                            CustomDesktopMenuWidgetBuilder(),
                        menuProvider: (_) => _circleMenu(
                          context,
                          controller,
                          circle,
                          onCategorySelected,
                        ),
                        child: Material(
                          color: colors.primary,
                          child: Padding(
                            padding: const EdgeInsets.symmetric(vertical: 4),
                            child: _CategoryItem(
                              asset: MixinAssets.circle,
                              iconColor: _circleColor(circle.circleId),
                              title: circle.name,
                              filter: ConversationCategoryFilter.circle,
                              circleId: circle.circleId,
                              controller: controller,
                              unseenCounts: unseenCounts,
                              collapsed: collapsed,
                              onSelected: onCategorySelected,
                            ),
                          ),
                        ),
                      ),
                    );
                  },
                ),
              ),
              TweenAnimationBuilder<double>(
                tween: Tween(end: showCollapse ? 1 : 0),
                duration: const Duration(milliseconds: 200),
                builder: (context, value, child) => ClipRect(
                  child: Align(
                    heightFactor: value,
                    alignment: Alignment.bottomCenter,
                    child: Opacity(
                      opacity: value,
                      child: IgnorePointer(ignoring: value == 0, child: child),
                    ),
                  ),
                ),
                child: _SidebarItem(
                  asset: collapsed
                      ? MixinAssets.expanded
                      : MixinAssets.collapse,
                  title: context.l10n.collapse,
                  selected: false,
                  count: 0,
                  mutedCount: 0,
                  collapsed: collapsed,
                  onTap: onToggleCollapsed,
                ),
              ),
              const SizedBox(height: 4),
            ],
          ),
        ),
      ),
    );
  }

  Menu _circleMenu(
    BuildContext context,
    ConversationListController controller,
    rust.CircleItem circle,
    VoidCallback onCategorySelected,
  ) => Menu(
    children: [
      MenuAction(
        image: MenuImage.icon(IconFonts.edit),
        title: context.l10n.editCircleName,
        callback: () => unawaited(_renameCircle(context, controller, circle)),
      ),
      MenuAction(
        image: MenuImage.icon(IconFonts.manageCircle),
        title: context.l10n.editConversations,
        callback: () =>
            unawaited(_manageCircleConversations(context, controller, circle)),
      ),
      MenuSeparator(),
      MenuAction(
        image: MenuImage.icon(IconFonts.delete),
        title: context.l10n.deleteCircle,
        attributes: const MenuActionAttributes(destructive: true),
        callback: () => unawaited(
          _deleteCircle(context, controller, circle, onCategorySelected),
        ),
      ),
    ],
  );

  Future<void> _renameCircle(
    BuildContext context,
    ConversationListController controller,
    rust.CircleItem circle,
  ) async {
    final name = await _showCircleNameDialog(context, circle.name);
    if (name == null || !context.mounted) return;
    try {
      await controller.account.conversation().updateCircle(
        circleId: circle.circleId,
        name: name.trim(),
      );
    } on Object catch (error, stackTrace) {
      e('Update circle failed: ${circle.circleId}', error, stackTrace);
      if (context.mounted) _showFailure(context);
    }
  }

  Future<void> _reorderCircles(
    BuildContext context,
    ConversationListController controller,
    List<rust.CircleItem> circles,
    ValueNotifier<List<rust.CircleItem>> orderedCircles,
    int oldIndex,
    int newIndex,
  ) async {
    if (oldIndex == newIndex) return;
    final reordered = [...circles];
    final circle = reordered.removeAt(oldIndex);
    reordered.insert(newIndex, circle);
    orderedCircles.value = reordered;
    try {
      await controller.account.conversation().reorderCircles(
        circleIds: reordered
            .map((circle) => circle.circleId)
            .toList(growable: false),
      );
    } on Object catch (error, stackTrace) {
      e('Reorder circles failed', error, stackTrace);
      orderedCircles.value = circles;
      if (context.mounted) _showFailure(context);
    }
  }

  Future<void> _manageCircleConversations(
    BuildContext context,
    ConversationListController controller,
    rust.CircleItem circle,
  ) async {
    final result = await showConversationMultiSelector(
      context,
      account: controller.account,
      title: circle.name,
      category: ConversationCategoryFilter.chats,
      initialCircleId: circle.circleId,
      allowEmpty: true,
      confirmedText: context.l10n.done,
    );
    if (result == null || !context.mounted) return;
    try {
      final account = controller.account;
      final existing = await account.conversation().conversationItems();
      final selectedIds = result.map((item) => item.id).toSet();
      for (final item in result) {
        if (item.circleIds.contains(circle.circleId)) continue;
        await account.conversation().editCircleConversation(
          circleId: circle.circleId,
          conversationId: item.id,
          ownerId: item.ownerId,
          isGroup: item.isGroup,
          add: true,
        );
      }
      for (final item in existing) {
        if (!item.circleIds.contains(circle.circleId) ||
            selectedIds.contains(item.conversationId)) {
          continue;
        }
        await account.conversation().editCircleConversation(
          circleId: circle.circleId,
          conversationId: item.conversationId,
          ownerId: item.ownerId,
          isGroup: item.category == 'GROUP',
          add: false,
        );
      }
      await controller.refresh();
    } on Object catch (error, stackTrace) {
      e(
        'Replace circle conversations failed: ${circle.circleId}',
        error,
        stackTrace,
      );
      if (context.mounted) _showFailure(context);
    }
  }

  Future<void> _deleteCircle(
    BuildContext context,
    ConversationListController controller,
    rust.CircleItem circle,
    VoidCallback onCategorySelected,
  ) async {
    final confirmed = await showConfirmMixinDialog(
      context,
      context.l10n.deleteTheCircle(circle.name),
    );
    if (confirmed == null || !context.mounted) return;
    try {
      await controller.account.conversation().deleteCircle(
        circleId: circle.circleId,
      );
      if (controller.category == ConversationCategoryFilter.circle &&
          controller.circleId == circle.circleId) {
        controller.selectCategory(ConversationCategoryFilter.chats);
      }
      onCategorySelected();
    } on Object catch (error, stackTrace) {
      e('Delete circle failed: ${circle.circleId}', error, stackTrace);
      if (context.mounted) _showFailure(context);
    }
  }
}

Future<String?> _showCircleNameDialog(
  BuildContext context,
  String initialName,
) async {
  final result = await showMixinDialog<String>(
    context: context,
    child: EditDialog(
      editText: initialName,
      title: Text(context.l10n.circles),
      hintText: context.l10n.editCircleName,
      positiveAction: context.l10n.edit,
      maxLength: 64,
    ),
  );
  return result?.trim().isEmpty == true ? null : result?.trim();
}

void _showFailure(BuildContext context) {
  showToastFailed(null);
}

class _Divider extends StatelessWidget {
  const _Divider();

  @override
  Widget build(BuildContext context) => Container(
    height: 1.5,
    margin: const EdgeInsets.symmetric(horizontal: 8),
    decoration: ShapeDecoration(
      color: context.mixinTheme.listSelected,
      shape: const StadiumBorder(),
    ),
  );
}

class _ProfileItem extends HookWidget {
  const _ProfileItem({
    required this.account,
    required this.collapsed,
    required this.selected,
    required this.onTap,
  });

  final rust.AccountHandle account;
  final bool collapsed;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final profile = useStream(
      useMemoized(account.profileChanges, [account]),
      initialData: account.profile(),
    ).data!;
    return _SidebarItem(
      customIcon: Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: AvatarView(
          userId: profile.userId,
          name: profile.fullName,
          avatarUrl: profile.avatarUrl,
          size: 24,
        ),
      ),
      title: profile.fullName,
      subtitle: profile.identityNumber,
      selected: selected,
      count: 0,
      mutedCount: 0,
      collapsed: collapsed,
      onTap: () {
        onTap();
        final scaffold = Scaffold.maybeOf(context);
        if (scaffold?.isDrawerOpen ?? false) Navigator.pop(context);
      },
    );
  }
}

class _CategoryItem extends StatelessWidget {
  const _CategoryItem({
    required this.asset,
    required this.title,
    required this.filter,
    required this.controller,
    required this.unseenCounts,
    required this.collapsed,
    required this.onSelected,
    this.circleId,
    this.iconColor,
  });

  final String asset;
  final String title;
  final ConversationCategoryFilter filter;
  final ConversationListController controller;
  final List<rust.ConversationUnseenCount> unseenCounts;
  final bool collapsed;
  final VoidCallback onSelected;
  final String? circleId;
  final Color? iconColor;

  @override
  Widget build(BuildContext context) => _SidebarItem(
    asset: asset,
    title: title,
    selected:
        controller.category == filter &&
        (filter != ConversationCategoryFilter.circle ||
            controller.circleId == circleId),
    count: _unseenCountFor(unseenCounts, filter, circleId),
    mutedCount: _mutedUnseenCountFor(unseenCounts, filter, circleId),
    collapsed: collapsed,
    iconColor: iconColor,
    onTap: () {
      controller.selectCategory(filter, circle: circleId);
      onSelected();
      final scaffold = Scaffold.maybeOf(context);
      if (scaffold?.isDrawerOpen ?? false) Navigator.pop(context);
    },
  );
}

int _unseenCountFor(
  List<rust.ConversationUnseenCount> counts,
  ConversationCategoryFilter filter,
  String? circleId,
) {
  for (final item in counts) {
    if (item.category == filter.name &&
        (filter != ConversationCategoryFilter.circle ||
            item.circleId == circleId)) {
      return item.count;
    }
  }
  return 0;
}

int _mutedUnseenCountFor(
  List<rust.ConversationUnseenCount> counts,
  ConversationCategoryFilter filter,
  String? circleId,
) {
  for (final item in counts) {
    if (item.category == filter.name &&
        (filter != ConversationCategoryFilter.circle ||
            item.circleId == circleId)) {
      return item.mutedCount;
    }
  }
  return 0;
}

class _SidebarItem extends StatefulWidget {
  const _SidebarItem({
    required this.title,
    required this.selected,
    required this.count,
    required this.mutedCount,
    required this.collapsed,
    required this.onTap,
    this.asset,
    this.customIcon,
    this.subtitle,
    this.iconColor,
  });

  final String? asset;
  final Widget? customIcon;
  final String title;
  final String? subtitle;
  final Color? iconColor;
  final bool selected;
  final int count;
  final int mutedCount;
  final bool collapsed;
  final VoidCallback onTap;

  @override
  State<_SidebarItem> createState() => _SidebarItemState();
}

class _SidebarItemState extends State<_SidebarItem> {
  bool hovering = false;
  bool hoveringTooltip = false;

  @override
  Widget build(BuildContext context) {
    final colors = context.mixinTheme;
    final dynamicColor = Theme.of(context).brightness == Brightness.light
        ? const Color.fromRGBO(51, 51, 51, 0.16)
        : const Color.fromRGBO(255, 255, 255, 0.4);
    final title = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          widget.title,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: TextStyle(color: colors.text, fontSize: 14),
        ),
        if (widget.subtitle != null) ...[
          const SizedBox(height: 2),
          Text(
            widget.subtitle!,
            style: TextStyle(color: colors.secondaryText, fontSize: 12),
          ),
        ],
      ],
    );
    final badge = _SidebarBadge(count: widget.count);
    final item = MouseRegion(
      onEnter: (_) => setState(() => hovering = true),
      onExit: (_) => setState(() => hovering = false),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: Duration(milliseconds: hovering ? 120 : 60),
          curve: Curves.decelerate,
          decoration: BoxDecoration(
            color: widget.selected
                ? colors.sidebarSelected
                : hovering
                ? colors.sidebarSelected.withValues(
                    alpha: colors.sidebarSelected.a / 2,
                  )
                : Colors.transparent,
            borderRadius: const BorderRadius.all(Radius.circular(8)),
          ),
          child: Stack(
            children: [
              Padding(
                padding: const EdgeInsets.all(8),
                child: Row(
                  children: [
                    widget.customIcon ??
                        SvgPicture.asset(
                          widget.asset!,
                          width: 24,
                          height: 24,
                          colorFilter: ColorFilter.mode(
                            widget.iconColor ?? colors.text,
                            BlendMode.srcIn,
                          ),
                        ),
                    if (!widget.collapsed) ...[
                      const SizedBox(width: 8),
                      Expanded(child: title),
                      if (widget.count > 0) badge,
                    ],
                  ],
                ),
              ),
              if (widget.collapsed && widget.count > 0)
                Positioned(
                  top: 6,
                  left: 28,
                  child: Container(
                    width: 6,
                    height: 6,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      color: widget.count == widget.mutedCount
                          ? dynamicColor
                          : colors.red,
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
    return PortalTarget(
      visible: widget.collapsed && (hovering || hoveringTooltip),
      anchor: const Aligned(
        follower: Alignment.centerLeft,
        target: Alignment.centerRight,
      ),
      portalFollower: MouseRegion(
        onEnter: (_) => setState(() => hoveringTooltip = true),
        onExit: (_) => setState(() => hoveringTooltip = false),
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: widget.onTap,
          child: Padding(
            padding: const EdgeInsets.only(left: 8),
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
              decoration: BoxDecoration(
                color: colors.background,
                borderRadius: const BorderRadius.all(Radius.circular(8)),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  title,
                  if (widget.count > 0) ...[const SizedBox(width: 12), badge],
                ],
              ),
            ),
          ),
        ),
      ),
      child: item,
    );
  }
}

class _SidebarBadge extends StatelessWidget {
  const _SidebarBadge({required this.count});

  final int count;

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
      color: Theme.of(context).brightness == Brightness.light
          ? const Color.fromRGBO(51, 51, 51, 0.16)
          : const Color.fromRGBO(255, 255, 255, 0.4),
      borderRadius: const BorderRadius.all(Radius.circular(10)),
    ),
    child: Text(
      '$count',
      maxLines: 1,
      style: TextStyle(color: context.mixinTheme.text, fontSize: 12, height: 1),
      textAlign: TextAlign.center,
    ),
  );
}

const _circleColors = [
  Color(0xFF8E7BFF),
  Color(0xFF657CFB),
  Color(0xFFA739C2),
  Color(0xFFBD6DDA),
  Color(0xFFFD89F1),
  Color(0xFFFA7B95),
  Color(0xFFE94156),
  Color(0xFFFA9652),
  Color(0xFFF1D22B),
  Color(0xFFBAE361),
  Color(0xFF5EDD5E),
  Color(0xFF4BE6FF),
  Color(0xFF45B7FE),
  Color(0xFF00ECD0),
  Color(0xFFFFCCC0),
  Color(0xFFCEA06B),
];

Color _circleColor(String circleId) {
  try {
    final components = circleId.trim().split('-');
    if (components.length != 5) throw const FormatException();
    final mostSignificantBits =
        (int.parse(components[0], radix: 16) << 32) |
        (int.parse(components[1], radix: 16) << 16) |
        int.parse(components[2], radix: 16);
    final leastSignificantBits =
        (int.parse(components[3], radix: 16) << 48) |
        int.parse(components[4], radix: 16);
    final hilo = mostSignificantBits ^ leastSignificantBits;
    final hash = (hilo >> 32) ^ hilo.toSigned(32);
    return _circleColors[hash.abs() % _circleColors.length];
  } on FormatException {
    return _circleColors[circleId.hashCode.abs() % _circleColors.length];
  }
}
