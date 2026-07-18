import 'dart:io';

import 'package:flutter/widgets.dart';

class TextInputActionHandler extends StatefulWidget {
  const TextInputActionHandler({required this.child, super.key});

  final Widget child;

  @override
  State<TextInputActionHandler> createState() => _TextInputActionHandlerState();
}

class _TextInputActionHandlerState extends State<TextInputActionHandler> {
  late final actions = <Type, Action<Intent>>{
    DeleteCharacterIntent: _makeAction<DeleteCharacterIntent>(),
    ExtendSelectionByCharacterIntent:
        _makeAction<ExtendSelectionByCharacterIntent>(),
    ExtendSelectionVerticallyToAdjacentLineIntent:
        _makeAction<ExtendSelectionVerticallyToAdjacentLineIntent>(),
    SelectAllTextIntent: _makeAction<SelectAllTextIntent>(),
    PasteTextIntent: _makeAction<PasteTextIntent>(),
    RedoTextIntent: _makeAction<RedoTextIntent>(),
    UndoTextIntent: _makeAction<UndoTextIntent>(),
  };

  Action<T> _makeAction<T extends Intent>() => Action<T>.overridable(
    defaultAction: _CallbackContextAction<T>(),
    context: context,
  );

  @override
  Widget build(BuildContext context) => Platform.isMacOS
      ? Actions(actions: actions, child: widget.child)
      : widget.child;
}

class _CallbackContextAction<T extends Intent> extends ContextAction<T> {
  bool? consumeKey;

  @override
  bool consumesKey(T intent) {
    final value = consumeKey;
    consumeKey = null;
    return value ?? callingAction?.consumesKey(intent) ?? true;
  }

  @override
  Object? invoke(T intent, [BuildContext? context]) {
    if (context == null) return callingAction?.invoke(intent);
    final state = context.findAncestorStateOfType<EditableTextState>();
    if (state == null) return callingAction?.invoke(intent);
    final composingRange = state.textEditingValue.composing;
    if (composingRange.isValid && !composingRange.isCollapsed) {
      consumeKey = false;
      return null;
    }
    return callingAction?.invoke(intent);
  }
}
