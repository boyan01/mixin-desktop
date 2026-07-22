import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:provider/provider.dart';

import '../l10n/generated/app_localizations.dart';
import '../theme.dart';
import 'message_style.dart';

class MessageDayTime extends StatelessWidget {
  const MessageDayTime({required this.dateTime, super.key});

  final DateTime dateTime;

  @override
  Widget build(BuildContext context) {
    final hiddenDateTime = context.read<ValueNotifier<DateTime?>?>();
    if (hiddenDateTime == null) {
      return Center(child: MessageDayTimeChip(dateTime: dateTime));
    }
    return ValueListenableBuilder<DateTime?>(
      valueListenable: hiddenDateTime,
      builder: (context, hidden, child) => Center(
        child: Opacity(
          opacity: _isSameDay(hidden, dateTime) ? 0 : 1,
          child: child,
        ),
      ),
      child: MessageDayTimeChip(dateTime: dateTime),
    );
  }
}

class MessageDayTimeChip extends StatelessWidget {
  const MessageDayTimeChip({required this.dateTime, super.key});

  final DateTime dateTime;

  @override
  Widget build(BuildContext context) {
    final localDateTime = dateTime.toLocal();
    final now = DateTime.now();
    final sameDay = _isSameDay(now, localDateTime);
    final text = sameDay
        ? AppLocalizations.of(context).today
        : now.year == localDateTime.year
        ? DateFormat.MMMEd().format(localDateTime)
        : DateFormat.yMMMEd().format(localDateTime);

    return Padding(
      padding: const EdgeInsets.only(top: 16, bottom: 10),
      child: Container(
        decoration: BoxDecoration(
          borderRadius: const BorderRadius.all(Radius.circular(10)),
          color: context.theme.dateTime,
        ),
        padding: const EdgeInsets.symmetric(vertical: 2, horizontal: 10),
        child: ConstrainedBox(
          constraints: const BoxConstraints(minWidth: 64),
          child: Text(
            text,
            textAlign: TextAlign.center,
            style: TextStyle(
              fontSize: context.messageStyle.secondaryFontSize,
              color: Colors.black,
            ),
          ),
        ),
      ),
    );
  }
}

class MessageDayTimeViewportEntry {
  const MessageDayTimeViewportEntry({
    required this.dateTime,
    required this.messageKey,
    this.dayTimeKey,
  });

  final DateTime dateTime;
  final GlobalKey messageKey;
  final GlobalKey? dayTimeKey;
}

class MessageDayTimeViewportWidget extends StatefulWidget {
  const MessageDayTimeViewportWidget({
    required this.child,
    required this.entries,
    this.reTraversalKey,
    super.key,
  });

  final Widget child;
  final List<MessageDayTimeViewportEntry> entries;
  final Object? reTraversalKey;

  @override
  State<MessageDayTimeViewportWidget> createState() =>
      _MessageDayTimeViewportWidgetState();
}

class _MessageDayTimeViewportWidgetState
    extends State<MessageDayTimeViewportWidget> {
  final ValueNotifier<DateTime?> _hiddenDateTime = ValueNotifier(null);
  DateTime? _dateTime;
  double _dateTimeTopOffset = 0;
  BoxConstraints? _constraints;
  bool _traversalScheduled = false;

  @override
  void initState() {
    super.initState();
    _scheduleTraversal();
  }

  @override
  void didUpdateWidget(covariant MessageDayTimeViewportWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.reTraversalKey != widget.reTraversalKey ||
        oldWidget.entries != widget.entries) {
      _scheduleTraversal();
    }
  }

  @override
  void dispose() {
    _hiddenDateTime.dispose();
    super.dispose();
  }

  void _scheduleTraversal() {
    if (_traversalScheduled) return;
    _traversalScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _traversalScheduled = false;
      if (mounted) _traverse();
    });
  }

  void _traverse() {
    final viewport = context.findRenderObject();
    if (viewport is! RenderBox || !viewport.hasSize) return;

    var firstInScreenIndex = -1;
    for (var index = 0; index < widget.entries.length; index++) {
      final render = widget.entries[index].messageKey.currentContext
          ?.findRenderObject();
      if (render is! RenderBox || !render.hasSize) continue;
      final offset = render.localToGlobal(
        Offset(0, render.size.height),
        ancestor: viewport,
      );
      if (offset.dy > 0) {
        firstInScreenIndex = index;
        break;
      }
    }

    if (firstInScreenIndex == -1) {
      _updateDateTime(null, 0, null);
      return;
    }

    var closestDayTimeIndex = -1;
    var closestDayTimeOffset = double.infinity;
    for (var delta = -1; delta <= 1; delta++) {
      final index = firstInScreenIndex + delta;
      if (index < 0 || index >= widget.entries.length) continue;
      final render = widget.entries[index].dayTimeKey?.currentContext
          ?.findRenderObject();
      if (render is! RenderBox || !render.hasSize) continue;
      final offset = render.localToGlobal(
        Offset(0, render.size.height / 2),
        ancestor: viewport,
      );
      final distance = (offset.dy - render.size.height / 2).abs();
      if (distance < closestDayTimeOffset) {
        closestDayTimeIndex = index;
        closestDayTimeOffset = distance;
      }
    }

    DateTime? hidden;
    var topOffset = 0.0;
    if (closestDayTimeIndex != -1) {
      final render =
          widget.entries[closestDayTimeIndex].dayTimeKey!.currentContext!
                  .findRenderObject()!
              as RenderBox;
      final offset = render.localToGlobal(
        Offset(0, render.size.height / 2),
        ancestor: viewport,
      );
      if (offset.dy < render.size.height / 2) {
        firstInScreenIndex = closestDayTimeIndex;
        hidden = widget.entries[closestDayTimeIndex].dateTime;
      } else if (firstInScreenIndex != closestDayTimeIndex) {
        topOffset = offset.dy - render.size.height * 1.5;
      } else {
        firstInScreenIndex = -1;
      }
    }

    _updateDateTime(
      firstInScreenIndex == -1
          ? null
          : widget.entries[firstInScreenIndex].dateTime,
      topOffset,
      hidden,
    );
  }

  void _updateDateTime(DateTime? dateTime, double topOffset, DateTime? hidden) {
    _hiddenDateTime.value = hidden;
    if (_dateTime == dateTime && _dateTimeTopOffset == topOffset) return;
    setState(() {
      _dateTime = dateTime;
      _dateTimeTopOffset = topOffset;
    });
  }

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      if (_constraints != constraints) {
        _constraints = constraints;
        _scheduleTraversal();
      }
      return ChangeNotifierProvider<ValueNotifier<DateTime?>>.value(
        value: _hiddenDateTime,
        child: ClipRect(
          child: Stack(
            fit: StackFit.expand,
            children: [
              NotificationListener<ScrollNotification>(
                onNotification: (_) {
                  _scheduleTraversal();
                  return false;
                },
                child: widget.child,
              ),
              if (_dateTime != null)
                Transform.translate(
                  offset: Offset(0, _dateTimeTopOffset.clamp(-60.0, 0.0)),
                  child: Align(
                    alignment: Alignment.topCenter,
                    child: MessageDayTimeChip(dateTime: _dateTime!),
                  ),
                ),
            ],
          ),
        ),
      );
    },
  );
}

bool _isSameDay(DateTime? first, DateTime? second) =>
    first != null &&
    second != null &&
    first.year == second.year &&
    first.month == second.month &&
    first.day == second.day;
