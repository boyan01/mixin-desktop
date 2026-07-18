// ignore_for_file: implementation_imports

import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:super_context_menu/src/default_builder/group_intrinsic_width.dart';
import 'package:super_context_menu/src/scaffold/desktop/menu_widget_builder.dart';
import 'package:super_context_menu/super_context_menu.dart';

class CustomDesktopMenuWidgetBuilder extends DefaultDesktopMenuWidgetBuilder {
  CustomDesktopMenuWidgetBuilder();

  static DefaultDesktopMenuTheme _themeForContext(BuildContext context) =>
      DefaultDesktopMenuTheme.themeForBrightness(Theme.of(context).brightness);

  @override
  Widget buildMenuContainer(
    BuildContext context,
    DesktopMenuInfo menuInfo,
    Widget child,
  ) {
    final pixelRatio = MediaQuery.of(context).devicePixelRatio;
    final theme = _themeForContext(context);
    return Container(
      decoration: theme.decorationOuter.copyWith(
        borderRadius: BorderRadius.circular(6.0 + 1.0 / pixelRatio),
      ),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(6),
        child: Padding(
          padding: EdgeInsets.all(1.0 / pixelRatio),
          child: Container(
            decoration: theme.decorationInner,
            padding: const EdgeInsets.symmetric(vertical: 6),
            child: DefaultTextStyle.merge(
              style: TextStyle(
                color: Colors.black,
                fontSize: 14,
                decoration: TextDecoration.none,
                fontWeight: FontWeight.w500,
                fontFamilyFallback: Platform.isWindows
                    ? ['Microsoft Yahei']
                    : null,
              ),
              child: ConstrainedBox(
                constraints: BoxConstraints(maxWidth: maxWidth),
                child: GroupIntrinsicWidthContainer(child: child),
              ),
            ),
          ),
        ),
      ),
    );
  }

  @override
  Widget buildSeparator(
    BuildContext context,
    DesktopMenuInfo menuInfo,
    MenuSeparator separator,
  ) {
    final theme = _themeForContext(context);
    final paddingLeft = 10.0 + (menuInfo.hasAnyCheckedItems ? (16 + 6) : 0);
    const paddingRight = 10.0;
    return Container(
      height: 1,
      margin: EdgeInsets.only(
        left: paddingLeft,
        right: paddingRight,
        top: 5,
        bottom: 6,
      ),
      color: theme.separatorColor,
    );
  }

  IconData? _stateToIcon(MenuActionState state) {
    switch (state) {
      case MenuActionState.none:
        return null;
      case MenuActionState.checkOn:
        return Icons.check;
      case MenuActionState.checkOff:
        return null;
      case MenuActionState.checkMixed:
        return Icons.remove;
      case MenuActionState.radioOn:
        return Icons.radio_button_on;
      case MenuActionState.radioOff:
        return Icons.radio_button_off;
    }
  }

  @override
  Widget buildMenuItem(
    BuildContext context,
    DesktopMenuInfo menuInfo,
    Key innerKey,
    DesktopMenuButtonState state,
    MenuElement element,
  ) {
    final theme = _themeForContext(context);
    final itemInfo = DesktopMenuItemInfo(
      destructive: element is MenuAction && element.attributes.destructive,
      disabled: element is MenuAction && element.attributes.disabled,
      menuFocused: menuInfo.focused,
      selected: state.selected,
    );
    final textStyle = theme.textStyleForItem(itemInfo);
    final iconTheme = menuInfo.iconTheme.copyWith(
      size: 16,
      color: textStyle.color,
    );
    final stateIcon = element is MenuAction
        ? _stateToIcon(element.state)
        : null;
    final Widget? prefix;
    if (stateIcon != null) {
      prefix = Icon(stateIcon, size: 16, color: iconTheme.color);
    } else if (menuInfo.hasAnyCheckedItems) {
      prefix = const SizedBox(width: 16);
    } else {
      prefix = null;
    }
    final image = element.image?.asWidget(iconTheme);

    final Widget? suffix;
    if (element is Menu) {
      suffix = Icon(
        Icons.chevron_right_outlined,
        size: 18,
        color: iconTheme.color,
      );
    } else if (element is MenuAction) {
      final activator = element.activator?.stringRepresentation();
      if (activator != null) {
        suffix = Padding(
          padding: const EdgeInsetsDirectional.only(end: 6),
          child: Text(
            activator,
            style: theme.textStyleForItemActivator(itemInfo, textStyle),
          ),
        );
      } else {
        suffix = null;
      }
    } else {
      suffix = null;
    }

    final child = element is DeferredMenuElement
        ? const Align(
            alignment: Alignment.centerLeft,
            child: SizedBox(
              height: 16,
              width: 16,
              child: CircularProgressIndicator(
                strokeWidth: 2,
                color: Colors.grey,
              ),
            ),
          )
        : Text(
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            element.title ?? '',
            style: textStyle,
          );

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 6),
      child: Container(
        key: innerKey,
        padding: const EdgeInsets.all(5),
        decoration: theme.decorationForItem(itemInfo),
        child: Row(
          children: [
            ?prefix,
            if (prefix != null) const SizedBox(width: 6),
            ?image,
            if (image != null) const SizedBox(width: 4),
            Expanded(
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 2),
                child: child,
              ),
            ),
            GroupIntrinsicWidth(
              child: Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  if (suffix != null) const SizedBox(width: 6),
                  ?suffix,
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

extension on DesktopMenuInfo {
  bool get hasAnyCheckedItems => resolvedChildren.any(
    (element) => element is MenuAction && element.state != MenuActionState.none,
  );
}

extension on SingleActivator {
  String stringRepresentation() => [
    if (control) 'Ctrl',
    if (alt) 'Alt',
    if (meta) defaultTargetPlatform == TargetPlatform.macOS ? 'Cmd' : 'Meta',
    if (shift) 'Shift',
    trigger.keyLabel,
  ].join('+');
}
