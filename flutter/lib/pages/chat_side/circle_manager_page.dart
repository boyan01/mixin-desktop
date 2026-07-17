import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_svg/svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';

import 'chat_side_scope.dart';

class CircleManagerPage extends StatefulWidget {
  const CircleManagerPage({super.key});

  @override
  State<CircleManagerPage> createState() => _CircleManagerPageState();
}

class _CircleManagerPageState extends State<CircleManagerPage> {
  List<rust.CircleItem> circles = const [];
  late Set<String> selected;
  bool loading = true;
  Object? error;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (loading && circles.isEmpty && error == null) {
      selected = ChatSideScope.of(context).conversation.circleIds.toSet();
      unawaited(_load());
    }
  }

  Future<void> _load() async {
    final scope = ChatSideScope.of(context);
    try {
      final values = await scope.account.conversation().circles();
      if (!mounted) return;
      setState(() {
        circles = values;
        loading = false;
        error = null;
      });
    } on Object catch (exception) {
      if (!mounted) return;
      setState(() {
        loading = false;
        error = exception;
      });
    }
  }

  Future<void> _toggle(rust.CircleItem circle) async {
    final scope = ChatSideScope.of(context);
    final add = !selected.contains(circle.circleId);
    setState(() {
      if (add) {
        selected.add(circle.circleId);
      } else {
        selected.remove(circle.circleId);
      }
    });
    try {
      await scope.account.conversation().editCircleConversation(
        circleId: circle.circleId,
        conversationId: scope.conversation.id,
        ownerId: scope.conversation.ownerId,
        isGroup: scope.conversation.isGroup,
        add: add,
      );
    } on Object catch (exception) {
      if (!mounted) return;
      setState(() {
        if (add) {
          selected.remove(circle.circleId);
        } else {
          selected.add(circle.circleId);
        }
        error = exception;
      });
    }
  }

  Future<void> _create() async {
    final name = await showDialog<String>(
      context: context,
      builder: (context) => const _CircleNameDialog(),
    );
    if (name == null || name.isEmpty || !mounted) return;
    final scope = ChatSideScope.of(context);
    try {
      final circle = await scope.account.conversation().createCircle(
        name: name,
      );
      await scope.account.conversation().editCircleConversation(
        circleId: circle.circleId,
        conversationId: scope.conversation.id,
        ownerId: scope.conversation.ownerId,
        isGroup: scope.conversation.isGroup,
        add: true,
      );
      if (!mounted) return;
      selected.add(circle.circleId);
      await _load();
    } on Object catch (exception) {
      if (mounted) setState(() => error = exception);
    }
  }

  @override
  Widget build(BuildContext context) => ChatSidePageScaffold(
    title: context.l10n.circles,
    actions: [IconButton(onPressed: _create, icon: const Icon(Icons.add))],
    body: loading
        ? const Center(child: CircularProgressIndicator())
        : error != null && circles.isEmpty
        ? ChatSideError(error: error!, onRetry: _load)
        : ListView.builder(
            itemCount: circles.length,
            itemBuilder: (context, index) {
              final circle = circles[index];
              final checked = selected.contains(circle.circleId);
              return SizedBox(
                height: 80,
                child: Material(
                  color: context.theme.primary,
                  child: Row(
                    children: [
                      GestureDetector(
                        onTap: () => unawaited(_toggle(circle)),
                        child: SizedBox(
                          width: 52,
                          height: 80,
                          child: Center(
                            child: SvgPicture.asset(
                              checked
                                  ? MixinAssets.circleRemove
                                  : MixinAssets.circleAdd,
                              width: 16,
                              height: 16,
                            ),
                          ),
                        ),
                      ),
                      const SizedBox(width: 4),
                      ClipOval(
                        child: Container(
                          color: const Color(0xFFF6F7FA),
                          width: 50,
                          height: 50,
                          alignment: Alignment.center,
                          child: SvgPicture.asset(
                            MixinAssets.circle,
                            width: 24,
                            height: 24,
                            colorFilter: ColorFilter.mode(
                              _circleColor(circle.circleId),
                              BlendMode.srcIn,
                            ),
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              circle.name,
                              style: TextStyle(
                                color: context.theme.text,
                                fontSize: 16,
                              ),
                            ),
                            const SizedBox(height: 6),
                            Text(
                              context.l10n.circleSubtitle(
                                circle.conversationCount,
                                circle.conversationCount,
                              ),
                              style: TextStyle(
                                color: context.theme.secondaryText,
                                fontSize: 14,
                              ),
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(width: 16),
                    ],
                  ),
                ),
              );
            },
          ),
  );
}

Color _circleColor(String id) {
  const colors = [
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
  final components = id.trim().split('-');
  var hash = id.hashCode;
  if (components.length == 5) {
    try {
      final high =
          (int.parse(components[0], radix: 16) << 32) |
          (int.parse(components[1], radix: 16) << 16) |
          int.parse(components[2], radix: 16);
      final low =
          (int.parse(components[3], radix: 16) << 48) |
          int.parse(components[4], radix: 16);
      final hilo = high ^ low;
      hash = (hilo >> 32) ^ hilo.toSigned(32);
    } on FormatException {
      // Keep the Dart string hash for non-UUID identifiers.
    }
  }
  return colors[hash.abs() % colors.length];
}

class _CircleNameDialog extends StatefulWidget {
  const _CircleNameDialog();

  @override
  State<_CircleNameDialog> createState() => _CircleNameDialogState();
}

class _CircleNameDialogState extends State<_CircleNameDialog> {
  final controller = TextEditingController();

  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text(context.l10n.circles),
    content: TextField(
      controller: controller,
      autofocus: true,
      decoration: InputDecoration(hintText: context.l10n.editCircleName),
      onSubmitted: (value) => Navigator.pop(context, value.trim()),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: Text(context.l10n.cancel),
      ),
      FilledButton(
        onPressed: () => Navigator.pop(context, controller.text.trim()),
        child: Text(context.l10n.confirm),
      ),
    ],
  );
}
