import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import '../constants/assets.dart';
import '../theme.dart';
import 'adaptive_selection_toolbar.dart';
import 'interactive_decorated_box.dart';

class SearchTextField extends StatefulWidget {
  const SearchTextField({
    required this.controller,
    super.key,
    this.focusNode,
    this.onChanged,
    this.fontSize = 14,
    this.hintText,
    this.autofocus = false,
    this.showClear = false,
    this.onTapClear,
    this.leading,
  });

  final FocusNode? focusNode;
  final TextEditingController controller;
  final ValueChanged<String>? onChanged;
  final double fontSize;
  final String? hintText;
  final bool autofocus;
  final bool showClear;
  final VoidCallback? onTapClear;
  final Widget? leading;

  @override
  State<SearchTextField> createState() => _SearchTextFieldState();
}

class _SearchTextFieldState extends State<SearchTextField> {
  late FocusNode _focusNode;
  late bool _ownsFocusNode;

  @override
  void initState() {
    super.initState();
    _setFocusNode();
    widget.controller.addListener(_changed);
  }

  @override
  void didUpdateWidget(covariant SearchTextField oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      oldWidget.controller.removeListener(_changed);
      widget.controller.addListener(_changed);
    }
    if (oldWidget.focusNode != widget.focusNode) {
      if (_ownsFocusNode) _focusNode.dispose();
      _setFocusNode();
    }
  }

  void _setFocusNode() {
    _ownsFocusNode = widget.focusNode == null;
    _focusNode = widget.focusNode ?? FocusNode();
  }

  void _changed() {
    if (mounted) setState(() {});
    if (!widget.controller.value.composing.isCollapsed) return;
    widget.onChanged?.call(widget.controller.text);
  }

  @override
  void dispose() {
    widget.controller.removeListener(_changed);
    if (_ownsFocusNode) _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final backgroundColor = context.dynamicColor(
      const Color.fromRGBO(245, 247, 250, 1),
      darkColor: const Color.fromRGBO(255, 255, 255, 0.08),
    );
    final hintColor = context.theme.secondaryText;
    final hasText = widget.controller.text.isNotEmpty;
    return InteractiveDecoratedBox(
      decoration: ShapeDecoration(
        color: backgroundColor,
        shape: const StadiumBorder(),
      ),
      cursor: SystemMouseCursors.text,
      onTap: _focusNode.requestFocus,
      child: SizedBox(
        height: 36,
        child: Row(
          children: [
            Padding(
              padding: const EdgeInsets.only(left: 16, right: 8),
              child: SvgPicture.asset(
                MixinAssets.search,
                colorFilter: ColorFilter.mode(hintColor, BlendMode.srcIn),
              ),
            ),
            if (widget.leading != null) widget.leading!,
            Expanded(
              child: Stack(
                fit: StackFit.expand,
                children: [
                  Align(
                    alignment: Alignment.centerLeft,
                    child: TextField(
                      focusNode: _focusNode,
                      autofocus: widget.autofocus,
                      controller: widget.controller,
                      style: TextStyle(
                        color: context.theme.text,
                        fontSize: widget.fontSize,
                      ),
                      scrollPadding: EdgeInsets.zero,
                      decoration: const InputDecoration(
                        isDense: true,
                        filled: true,
                        border: InputBorder.none,
                        fillColor: Colors.transparent,
                        hoverColor: Colors.transparent,
                        focusColor: Colors.transparent,
                        contentPadding: EdgeInsets.zero,
                      ),
                      inputFormatters: [LengthLimitingTextInputFormatter(200)],
                      contextMenuBuilder: (context, state) =>
                          MixinAdaptiveSelectionToolbar(
                            editableTextState: state,
                          ),
                    ),
                  ),
                  if (widget.hintText != null && !hasText)
                    Align(
                      alignment: Alignment.centerLeft,
                      child: IgnorePointer(
                        child: Text(
                          widget.hintText!,
                          style: TextStyle(
                            color: hintColor,
                            fontSize: widget.fontSize,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ),
                ],
              ),
            ),
            if (widget.showClear || hasText)
              InteractiveDecoratedBox(
                cursor: SystemMouseCursors.basic,
                onTap: () {
                  widget.controller.clear();
                  widget.onTapClear?.call();
                },
                child: Padding(
                  padding: const EdgeInsets.only(right: 16, left: 8),
                  child: Icon(
                    Icons.close,
                    color: context.theme.secondaryText,
                    size: 16,
                  ),
                ),
              )
            else
              const SizedBox(width: 40),
          ],
        ),
      ),
    );
  }
}
