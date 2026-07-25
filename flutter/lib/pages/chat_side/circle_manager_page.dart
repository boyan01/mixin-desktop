import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_svg/svg.dart';
import '../../constants/assets.dart';
import '../../l10n/l10n.dart';
import '../../src/rust/desktop_api.dart' as rust;
import '../../theme.dart';
import '../../utils/app_logger.dart';
import '../../widgets/action_button.dart';
import '../../widgets/mixin_dialog.dart';

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
    } on Object catch (exception, stackTrace) {
      e('Load circles failed', exception, stackTrace);
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
    } on Object catch (exception, stackTrace) {
      e(
        'Update circle membership failed: ${circle.circleId}',
        exception,
        stackTrace,
      );
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
    final name = await showMixinDialog<String>(
      context: context,
      child: EditDialog(
        title: Text(context.l10n.circles),
        hintText: context.l10n.editCircleName,
      ),
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
    } on Object catch (exception, stackTrace) {
      e('Create circle failed', exception, stackTrace);
      if (mounted) setState(() => error = exception);
    }
  }

  @override
  Widget build(BuildContext context) => ChatSidePageScaffold(
    title: context.l10n.circles,
    backgroundColor: context.theme.background,
    actions: [ActionButton(name: MixinAssets.add, onTap: _create)],
    body: ListView(
      children: [
        ...circles
            .where((circle) => selected.contains(circle.circleId))
            .map(
              (circle) => _CircleManagerItem(
                circle: circle,
                selected: true,
                onTap: () => unawaited(_toggle(circle)),
              ),
            ),
        if (circles.any((circle) => selected.contains(circle.circleId)) &&
            circles.any((circle) => !selected.contains(circle.circleId)))
          const SizedBox(height: 10),
        ...circles
            .where((circle) => !selected.contains(circle.circleId))
            .map(
              (circle) => _CircleManagerItem(
                circle: circle,
                selected: false,
                onTap: () => unawaited(_toggle(circle)),
              ),
            ),
      ],
    ),
  );
}

class _CircleManagerItem extends StatelessWidget {
  const _CircleManagerItem({
    required this.circle,
    required this.selected,
    required this.onTap,
  });

  final rust.CircleItem circle;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => Container(
    height: 80,
    color: context.theme.primary,
    child: Row(
      children: [
        GestureDetector(
          onTap: onTap,
          child: Container(
            height: 80,
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: SvgPicture.asset(
              selected ? MixinAssets.circleRemove : MixinAssets.circleAdd,
              width: 16,
              height: 16,
            ),
          ),
        ),
        const SizedBox(width: 4),
        ClipOval(
          child: Container(
            color: context.dynamicColor(
              const Color.fromRGBO(246, 247, 250, 1),
              darkColor: const Color.fromRGBO(245, 247, 250, 1),
            ),
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
        Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              circle.name,
              style: TextStyle(color: context.theme.text, fontSize: 16),
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
      ],
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
