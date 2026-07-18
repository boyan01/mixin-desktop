import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/buttons.dart';
import 'package:mixin_desktop_ui/widgets/interactive_decorated_box.dart';
import 'package:mixin_desktop_ui/widgets/move_window.dart';

class CellGroup extends StatelessWidget {
  const CellGroup({
    required this.child,
    super.key,
    this.borderRadius = const BorderRadius.all(Radius.circular(8)),
    this.padding = const EdgeInsets.only(right: 10, left: 10, bottom: 10),
    this.cellBackgroundColor,
  });

  final BorderRadius borderRadius;
  final Widget child;
  final EdgeInsetsGeometry padding;
  final Color? cellBackgroundColor;

  @override
  Widget build(BuildContext context) => ConstrainedBox(
    constraints: const BoxConstraints(maxWidth: 600),
    child: Padding(
      padding: padding,
      child: ClipRRect(
        borderRadius: borderRadius,
        child: _CellStyle(
          backgroundColor:
              cellBackgroundColor ?? context.mixinTheme.listSelected,
          child: child,
        ),
      ),
    ),
  );
}

class _CellStyle extends InheritedWidget {
  const _CellStyle({required this.backgroundColor, required super.child});

  final Color backgroundColor;

  static _CellStyle of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<_CellStyle>()!;

  @override
  bool updateShouldNotify(_CellStyle oldWidget) =>
      oldWidget.backgroundColor == backgroundColor;
}

class CellItem extends StatelessWidget {
  const CellItem({
    required this.title,
    super.key,
    this.leading,
    this.color,
    this.onTap,
    this.selected = false,
    this.trailing = const Arrow(),
    this.description,
  });

  final Widget? leading;
  final Widget title;
  final Color? color;
  final VoidCallback? onTap;
  final bool selected;
  final Widget? trailing;
  final Widget? description;

  @override
  Widget build(BuildContext context) {
    final background = _CellStyle.of(context).backgroundColor;
    final selectedBackground = selected
        ? Color.alphaBlend(
            context.dynamicColor(
              const Color.fromRGBO(0, 0, 0, 0.05),
              darkColor: const Color.fromRGBO(255, 255, 255, 0.06),
            ),
            background,
          )
        : background;
    return InteractiveDecoratedBox(
      decoration: BoxDecoration(color: selectedBackground),
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.only(
          top: 17,
          bottom: 17,
          left: 16,
          right: 10,
        ),
        child: Row(
          children: [
            ?leading,
            if (leading != null) const SizedBox(width: 8),
            Expanded(
              child: DefaultTextStyle.merge(
                style: TextStyle(
                  fontSize: 16,
                  color: color ?? context.mixinTheme.text,
                ),
                child: title,
              ),
            ),
            if (description != null) const SizedBox(width: 4),
            if (description != null)
              DefaultTextStyle.merge(
                style: TextStyle(
                  color: context.mixinTheme.secondaryText,
                  fontSize: 14,
                ),
                child: description!,
              ),
            if (trailing != null) const SizedBox(width: 4),
            ?trailing,
          ],
        ),
      ),
    );
  }
}

class Arrow extends StatelessWidget {
  const Arrow({super.key});

  @override
  Widget build(BuildContext context) => SvgPicture.asset(
    MixinAssets.arrowRight,
    colorFilter: ColorFilter.mode(
      context.mixinTheme.secondaryText,
      BlendMode.srcIn,
    ),
    width: 30,
    height: 30,
  );
}

class MixinAppBar extends StatelessWidget implements PreferredSizeWidget {
  const MixinAppBar({
    super.key,
    this.title,
    this.actions = const [],
    this.backgroundColor,
    this.leading,
  });

  final Widget? title;
  final List<Widget> actions;
  final Color? backgroundColor;
  final Widget? leading;

  @override
  Widget build(BuildContext context) => MoveWindow(
    child: AppBar(
      toolbarHeight: 64,
      title: title == null
          ? null
          : DefaultTextStyle.merge(
              style: TextStyle(
                fontSize: 16,
                fontWeight: FontWeight.w600,
                color: context.mixinTheme.text,
              ),
              child: title!,
            ),
      actions: [
        ...actions.map(
          (action) => MoveWindowBarrier(
            child: DefaultTextStyle.merge(
              style: TextStyle(
                fontSize: 16,
                fontWeight: FontWeight.w500,
                color: context.mixinTheme.accent,
              ),
              child: action,
            ),
          ),
        ),
        const SizedBox(width: 8),
      ],
      elevation: 0,
      centerTitle: true,
      backgroundColor: backgroundColor ?? context.mixinTheme.primary,
      leading: MoveWindowBarrier(
        child: Builder(
          builder: (context) =>
              leading ??
              (ModalRoute.of(context)?.canPop ?? false
                  ? const Center(child: MixinBackButton())
                  : const SizedBox(width: 56)),
        ),
      ),
    ),
  );

  @override
  Size get preferredSize => const Size.fromHeight(64);
}

class SettingsSwitch extends StatelessWidget {
  const SettingsSwitch({
    required this.value,
    required this.onChanged,
    super.key,
  });

  final bool value;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) => Transform.scale(
    scale: 0.7,
    child: CupertinoSwitch(
      activeTrackColor: context.mixinTheme.accent,
      value: value,
      onChanged: onChanged,
    ),
  );
}

class RadioItem<T> extends StatelessWidget {
  const RadioItem({
    required this.title,
    required this.value,
    required this.onChanged,
    super.key,
    this.groupValue,
  });

  final Widget title;
  final T? groupValue;
  final T value;
  final ValueChanged<T> onChanged;

  @override
  Widget build(BuildContext context) => GestureDetector(
    behavior: HitTestBehavior.opaque,
    onTap: () => onChanged(value),
    child: Padding(
      padding: const EdgeInsets.symmetric(vertical: 10),
      child: Row(
        children: [
          ClipOval(
            child: Container(
              color: groupValue == value
                  ? context.mixinTheme.accent
                  : context.mixinTheme.secondaryText,
              height: 16,
              width: 16,
              alignment: const Alignment(0, -0.2),
              child: SvgPicture.asset(
                MixinAssets.selected,
                height: 10,
                width: 10,
              ),
            ),
          ),
          const SizedBox(width: 30),
          DefaultTextStyle.merge(
            style: TextStyle(color: context.mixinTheme.text, fontSize: 16),
            child: title,
          ),
        ],
      ),
    ),
  );
}
