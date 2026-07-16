import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/controllers/conversation_list_controller.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';

class HomeSidebar extends StatelessWidget {
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
                controller: controller,
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
                collapsed: collapsed,
                onSelected: onCategorySelected,
              ),
              const SizedBox(height: 8),
              _CategoryItem(
                asset: MixinAssets.groups,
                title: context.l10n.groups,
                filter: ConversationCategoryFilter.groups,
                controller: controller,
                collapsed: collapsed,
                onSelected: onCategorySelected,
              ),
              const SizedBox(height: 8),
              _CategoryItem(
                asset: MixinAssets.bots,
                title: context.l10n.botsTitle,
                filter: ConversationCategoryFilter.bots,
                controller: controller,
                collapsed: collapsed,
                onSelected: onCategorySelected,
              ),
              const SizedBox(height: 8),
              _CategoryItem(
                asset: MixinAssets.strangers,
                title: context.l10n.strangers,
                filter: ConversationCategoryFilter.strangers,
                controller: controller,
                collapsed: collapsed,
                onSelected: onCategorySelected,
              ),
              const SizedBox(height: 16),
              if (controller.circles.isNotEmpty) const _Divider(),
              if (controller.circles.isNotEmpty) const SizedBox(height: 12),
              Expanded(
                child: ListView.builder(
                  padding: EdgeInsets.zero,
                  itemCount: controller.circles.length,
                  itemBuilder: (context, index) {
                    final circle = controller.circles[index];
                    return Material(
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
                          collapsed: collapsed,
                          onSelected: onCategorySelected,
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

class _ProfileItem extends StatelessWidget {
  const _ProfileItem({
    required this.controller,
    required this.collapsed,
    required this.selected,
    required this.onTap,
  });

  final ConversationListController controller;
  final bool collapsed;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => _SidebarItem(
    customIcon: Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: AvatarView(
        userId: controller.profile.userId,
        name: controller.profile.fullName,
        avatarUrl: controller.profile.avatarUrl,
        size: 24,
      ),
    ),
    title: controller.profile.fullName,
    subtitle: controller.profile.identityNumber,
    selected: selected,
    count: 0,
    collapsed: collapsed,
    onTap: () {
      onTap();
      final scaffold = Scaffold.maybeOf(context);
      if (scaffold?.isDrawerOpen ?? false) Navigator.pop(context);
    },
  );
}

class _CategoryItem extends StatelessWidget {
  const _CategoryItem({
    required this.asset,
    required this.title,
    required this.filter,
    required this.controller,
    required this.collapsed,
    required this.onSelected,
    this.circleId,
    this.iconColor,
  });

  final String asset;
  final String title;
  final ConversationCategoryFilter filter;
  final ConversationListController controller;
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
    count: controller.countFor(filter, circle: circleId),
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

class _SidebarItem extends StatefulWidget {
  const _SidebarItem({
    required this.title,
    required this.selected,
    required this.count,
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
  final bool collapsed;
  final VoidCallback onTap;

  @override
  State<_SidebarItem> createState() => _SidebarItemState();
}

class _SidebarItemState extends State<_SidebarItem> {
  bool hovering = false;

  @override
  Widget build(BuildContext context) {
    final colors = context.mixinTheme;
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
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Text(
                              widget.title,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: TextStyle(
                                color: colors.text,
                                fontSize: 14,
                              ),
                            ),
                            if (widget.subtitle != null) ...[
                              const SizedBox(height: 2),
                              Text(
                                widget.subtitle!,
                                style: TextStyle(
                                  color: colors.secondaryText,
                                  fontSize: 12,
                                ),
                              ),
                            ],
                          ],
                        ),
                      ),
                      if (widget.count > 0) _SidebarBadge(count: widget.count),
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
                      color: colors.red,
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
    return widget.collapsed
        ? Tooltip(message: widget.title, child: item)
        : item;
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
