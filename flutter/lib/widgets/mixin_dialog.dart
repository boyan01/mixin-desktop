import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/adaptive_selection_toolbar.dart';
import 'package:mixin_desktop_ui/widgets/interactive_decorated_box.dart';

Future<T?> showMixinDialog<T>({
  required BuildContext context,
  required Widget child,
  RouteSettings? routeSettings,
  EdgeInsets? padding = const EdgeInsets.all(32),
  BoxConstraints? constraints = const BoxConstraints(maxWidth: 600),
  Color? backgroundColor,
  bool barrierDismissible = true,
}) => showGeneralDialog<T>(
  context: context,
  barrierDismissible: barrierDismissible,
  barrierLabel: MaterialLocalizations.of(context).modalBarrierDismissLabel,
  barrierColor: const Color(0x80000000),
  transitionDuration: const Duration(milliseconds: 80),
  routeSettings: routeSettings,
  pageBuilder: (buildContext, animation, secondaryAnimation) =>
      InheritedTheme.capture(
        from: context,
        to: Navigator.of(context, rootNavigator: true).context,
      ).wrap(
        Center(
          child: _DialogPage(
            padding: padding,
            constraints: constraints,
            backgroundColor: backgroundColor,
            child: child,
          ),
        ),
      ),
);

class _DialogPage extends StatelessWidget {
  const _DialogPage({
    required this.child,
    this.padding,
    this.constraints,
    this.backgroundColor,
  });

  final Widget child;
  final EdgeInsets? padding;
  final BoxConstraints? constraints;
  final Color? backgroundColor;

  @override
  Widget build(BuildContext context) {
    final effectivePadding =
        MediaQuery.viewInsetsOf(context) + (padding ?? EdgeInsets.zero);
    return Padding(
      padding: effectivePadding,
      child: ConstrainedBox(
        constraints: constraints ?? const BoxConstraints(),
        child: DecoratedBox(
          decoration: BoxDecoration(
            borderRadius: const BorderRadius.all(Radius.circular(11)),
            boxShadow: [
              const BoxShadow(
                color: Color.fromRGBO(0, 0, 0, 0.15),
                offset: Offset(0, 8),
                blurRadius: 40,
              ),
              BoxShadow(
                color: const Color.fromRGBO(0, 0, 0, 0.07),
                offset: const Offset(0, 4),
                blurRadius: lerpDouble(
                  16,
                  6,
                  Theme.of(context).brightness == Brightness.dark ? 1 : 0,
                )!,
              ),
            ],
          ),
          child: Material(
            color: backgroundColor ?? context.mixinTheme.popUp,
            borderRadius: const BorderRadius.all(Radius.circular(11)),
            child: ClipRRect(
              borderRadius: const BorderRadius.all(Radius.circular(11)),
              child: child,
            ),
          ),
        ),
      ),
    );
  }
}

class AlertDialogLayout extends StatelessWidget {
  const AlertDialogLayout({
    required this.content,
    super.key,
    this.title,
    this.titleMarginBottom = 48,
    this.actions = const [],
    this.minWidth = 400,
    this.minHeight = 210,
    this.padding = const EdgeInsets.all(30),
    this.maxWidth,
  });

  final Widget? title;
  final double titleMarginBottom;
  final Widget content;
  final List<Widget> actions;
  final double minWidth;
  final double minHeight;
  final EdgeInsetsGeometry padding;
  final double? maxWidth;

  @override
  Widget build(BuildContext context) => Material(
    color: Colors.transparent,
    child: ConstrainedBox(
      constraints: BoxConstraints(
        minWidth: minWidth,
        minHeight: minHeight,
        maxWidth: maxWidth ?? double.infinity,
      ),
      child: Padding(
        padding: padding,
        child: IntrinsicWidth(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              if (title != null)
                DefaultTextStyle.merge(
                  style: TextStyle(
                    fontSize: 16,
                    color: context.mixinTheme.text,
                  ),
                  child: title!,
                ),
              if (title != null) SizedBox(height: titleMarginBottom),
              DefaultTextStyle.merge(
                style: TextStyle(
                  fontSize: 18,
                  fontWeight: FontWeight.w600,
                  color: context.mixinTheme.text,
                ),
                child: content,
              ),
              const SizedBox(height: 30),
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  for (var index = 0; index < actions.length; index++) ...[
                    if (index > 0) const SizedBox(width: 4),
                    actions[index],
                  ],
                ],
              ),
            ],
          ),
        ),
      ),
    ),
  );
}

class MixinButton<T> extends StatelessWidget {
  const MixinButton({
    required this.child,
    super.key,
    this.value,
    this.backgroundTransparent = false,
    this.onTap,
    this.padding = const EdgeInsets.symmetric(vertical: 8, horizontal: 16),
    this.disable = false,
    this.backgroundColor,
  });

  final T? value;
  final bool backgroundTransparent;
  final Widget child;
  final VoidCallback? onTap;
  final EdgeInsetsGeometry padding;
  final bool disable;
  final Color? backgroundColor;

  @override
  Widget build(BuildContext context) {
    final decoration = backgroundTransparent
        ? const BoxDecoration()
        : BoxDecoration(
            borderRadius: const BorderRadius.all(Radius.circular(5)),
            color: backgroundColor ?? context.mixinTheme.accent,
          );
    final textColor = backgroundTransparent
        ? context.mixinTheme.accent
        : Colors.white;
    return IgnorePointer(
      ignoring: disable,
      child: AnimatedOpacity(
        opacity: disable ? 0.4 : 1,
        duration: const Duration(milliseconds: 200),
        child: InteractiveDecoratedBox.color(
          decoration: decoration,
          onTap: () =>
              onTap != null ? onTap!() : Navigator.pop<T>(context, value),
          child: DefaultTextStyle.merge(
            style: TextStyle(
              fontWeight: FontWeight.w500,
              fontSize: 16,
              color: textColor,
            ),
            child: Padding(padding: padding, child: child),
          ),
        ),
      ),
    );
  }
}

enum DialogEvent { positive, neutral }

Future<DialogEvent?> showConfirmMixinDialog(
  BuildContext context,
  String content, {
  String? description,
  double? maxWidth,
  bool barrierDismissible = true,
  String? positiveText,
  String? negativeText,
  String? neutralText,
}) => showMixinDialog<DialogEvent>(
  context: context,
  barrierDismissible: barrierDismissible,
  child: Builder(
    builder: (context) => AlertDialogLayout(
      maxWidth: maxWidth,
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(content),
          if (description != null)
            Padding(
              padding: const EdgeInsets.only(top: 20),
              child: Text(
                description,
                style: TextStyle(
                  color: context.mixinTheme.text,
                  fontSize: 14,
                  fontWeight: FontWeight.normal,
                ),
              ),
            ),
        ],
      ),
      actions: [
        if (neutralText != null) ...[
          MixinButton(
            onTap: () => Navigator.pop(context, DialogEvent.neutral),
            child: Text(neutralText),
          ),
          const Spacer(),
        ],
        MixinButton(
          backgroundTransparent: true,
          onTap: () => Navigator.pop(context),
          child: Text(negativeText ?? context.l10n.cancel),
        ),
        MixinButton(
          onTap: () => Navigator.pop(context, DialogEvent.positive),
          child: Text(positiveText ?? context.l10n.confirm),
        ),
      ],
    ),
  ),
);

class EditDialog extends StatefulWidget {
  const EditDialog({
    required this.title,
    super.key,
    this.editText = '',
    this.hintText = '',
    this.positiveAction,
    this.maxLines,
    this.maxLength,
    this.inputFormatters,
  });

  final Widget title;
  final String editText;
  final String hintText;
  final String? positiveAction;
  final int? maxLines;
  final int? maxLength;
  final List<TextInputFormatter>? inputFormatters;

  @override
  State<EditDialog> createState() => _EditDialogState();
}

class _EditDialogState extends State<EditDialog> {
  late final TextEditingController controller = TextEditingController(
    text: widget.editText,
  )..addListener(_changed);

  void _changed() => setState(() {});

  @override
  void dispose() {
    controller
      ..removeListener(_changed)
      ..dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AlertDialogLayout(
    title: widget.title,
    content: DialogTextField(
      textEditingController: controller,
      hintText: widget.hintText,
      maxLines: widget.maxLines,
      maxLength: widget.maxLength,
      inputFormatters: widget.inputFormatters,
    ),
    actions: [
      MixinButton(
        backgroundTransparent: true,
        onTap: () => Navigator.pop(context),
        child: Text(context.l10n.cancel),
      ),
      MixinButton(
        disable: controller.text.isEmpty,
        onTap: () => Navigator.pop(context, controller.text),
        child: Text(widget.positiveAction ?? context.l10n.create),
      ),
    ],
  );
}

class DialogTextField extends StatefulWidget {
  const DialogTextField({
    required this.textEditingController,
    required this.hintText,
    super.key,
    this.inputFormatters,
    this.maxLines = 1,
    this.maxLength,
  });

  final TextEditingController textEditingController;
  final String hintText;
  final List<TextInputFormatter>? inputFormatters;
  final int? maxLines;
  final int? maxLength;

  @override
  State<DialogTextField> createState() => _DialogTextFieldState();
}

class _DialogTextFieldState extends State<DialogTextField> {
  @override
  void initState() {
    super.initState();
    widget.textEditingController.addListener(_changed);
  }

  @override
  void didUpdateWidget(covariant DialogTextField oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.textEditingController != widget.textEditingController) {
      oldWidget.textEditingController.removeListener(_changed);
      widget.textEditingController.addListener(_changed);
    }
  }

  void _changed() => setState(() {});

  @override
  void dispose() {
    widget.textEditingController.removeListener(_changed);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Container(
    constraints: const BoxConstraints(minHeight: 48),
    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
    decoration: BoxDecoration(
      color: context.mixinTheme.background,
      borderRadius: const BorderRadius.all(Radius.circular(5)),
    ),
    alignment: Alignment.center,
    child: Stack(
      children: [
        TextField(
          autofocus: true,
          controller: widget.textEditingController,
          style: TextStyle(color: context.mixinTheme.text),
          maxLines: widget.maxLines ?? 1,
          minLines: 1,
          maxLength: widget.maxLength,
          scrollPadding: EdgeInsets.zero,
          contextMenuBuilder: (context, state) =>
              MixinAdaptiveSelectionToolbar(editableTextState: state),
          decoration: InputDecoration(
            contentPadding: EdgeInsets.zero,
            isDense: true,
            focusedBorder: InputBorder.none,
            enabledBorder: InputBorder.none,
            counterStyle: TextStyle(
              fontSize: 14,
              color: context.mixinTheme.secondaryText,
            ),
          ),
          inputFormatters: widget.inputFormatters,
          selectionHeightStyle: BoxHeightStyle.includeLineSpacingMiddle,
        ),
        if (widget.hintText.isNotEmpty &&
            widget.textEditingController.text.isEmpty)
          IgnorePointer(
            child: Text(
              widget.hintText,
              style: TextStyle(
                color: Colors.white.withValues(alpha: 0.08),
                height: 1,
              ),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
      ],
    ),
  );
}

class DialogAddOrJoinButton extends StatelessWidget {
  const DialogAddOrJoinButton({
    required this.onTap,
    required this.title,
    super.key,
  });

  final VoidCallback onTap;
  final Widget title;

  @override
  Widget build(BuildContext context) => TextButton(
    style: TextButton.styleFrom(
      backgroundColor: context.mixinTheme.statusBackground,
      padding: const EdgeInsets.symmetric(horizontal: 15, vertical: 7),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.all(Radius.circular(15)),
      ),
    ),
    onPressed: onTap,
    child: DefaultTextStyle.merge(
      style: TextStyle(fontSize: 12, color: context.mixinTheme.accent),
      child: title,
    ),
  );
}
