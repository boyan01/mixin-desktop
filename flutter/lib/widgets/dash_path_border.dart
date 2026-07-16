import 'package:flutter/painting.dart';

Path dashPath(
  Path source, {
  required CircularIntervalList<double> dashArray,
  DashOffset? dashOffset,
}) {
  final offset = dashOffset ?? const DashOffset.absolute(0);
  final dest = Path();
  for (final metric in source.computeMetrics()) {
    var distance = offset._calculate(metric.length);
    var draw = true;
    dashArray.reset();
    while (distance < metric.length) {
      final len = dashArray.next;
      if (draw) {
        dest.addPath(metric.extractPath(distance, distance + len), Offset.zero);
      }
      distance += len;
      draw = !draw;
    }
  }
  return dest;
}

enum _DashOffsetType { absolute, percentage }

class DashOffset {
  DashOffset.percentage(double percentage)
    : _rawVal = percentage.clamp(0.0, 1.0),
      _dashOffsetType = _DashOffsetType.percentage;

  const DashOffset.absolute(double start)
    : _rawVal = start,
      _dashOffsetType = _DashOffsetType.absolute;

  final double _rawVal;
  final _DashOffsetType _dashOffsetType;

  double _calculate(double length) =>
      _dashOffsetType == _DashOffsetType.absolute ? _rawVal : length * _rawVal;
}

class CircularIntervalList<T> {
  CircularIntervalList(this._vals);

  final List<T> _vals;
  int _idx = 0;

  void reset() => _idx = 0;

  T get next {
    if (_idx >= _vals.length) _idx = 0;
    return _vals[_idx++];
  }
}

class DashPathBorder extends Border {
  const DashPathBorder({
    required this.dashArray,
    super.top,
    super.left,
    super.right,
    super.bottom,
  });

  factory DashPathBorder.all({
    required CircularIntervalList<double> dashArray,
    BorderSide borderSide = const BorderSide(),
  }) => DashPathBorder(
    dashArray: dashArray,
    top: borderSide,
    right: borderSide,
    left: borderSide,
    bottom: borderSide,
  );

  final CircularIntervalList<double> dashArray;

  @override
  void paint(
    Canvas canvas,
    Rect rect, {
    TextDirection? textDirection,
    BoxShape shape = BoxShape.rectangle,
    BorderRadius? borderRadius,
  }) {
    if (!isUniform || top.style == BorderStyle.none) return;
    final path = switch (shape) {
      BoxShape.circle => Path()..addOval(rect),
      BoxShape.rectangle when borderRadius != null =>
        Path()..addRRect(RRect.fromRectAndRadius(rect, borderRadius.topLeft)),
      BoxShape.rectangle => Path()..addRect(rect),
    };
    canvas.drawPath(dashPath(path, dashArray: dashArray), top.toPaint());
  }
}
