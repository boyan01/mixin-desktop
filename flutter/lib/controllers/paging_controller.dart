import 'dart:async';
import 'dart:math';

import 'package:flutter/foundation.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';

class PagingState<T> {
  const PagingState({
    this.map = const {},
    this.count = 0,
    this.initialized = false,
    this.hasData = false,
  });

  final Map<int, T> map;
  final int count;
  final bool initialized;
  final bool hasData;

  PagingState<T> copyWith({
    Map<int, T>? map,
    int? count,
    bool? initialized,
    bool? hasData,
  }) => PagingState(
    map: map ?? this.map,
    count: count ?? this.count,
    initialized: initialized ?? this.initialized,
    hasData: hasData ?? this.hasData,
  );
}

class PagingController<T> extends ValueNotifier<PagingState<T>> {
  PagingController({
    required this.limit,
    required this.queryCount,
    required this.queryRange,
  }) : itemPositionsListener = ItemPositionsListener.create(),
       itemScrollController = ItemScrollController(),
       super(PagingState<T>()) {
    itemPositionsListener.itemPositions.addListener(_onItemPositions);
    _initialize();
  }

  final Future<int> Function() queryCount;
  final Future<List<T>> Function(int limit, int offset) queryRange;
  final ItemPositionsListener itemPositionsListener;
  final ItemScrollController itemScrollController;
  int limit;

  var _generation = 0;
  var _disposed = false;

  void update() {
    final generation = ++_generation;
    unawaited(_update(generation));
  }

  void _initialize() {
    final generation = ++_generation;
    unawaited(_init(generation));
  }

  void _onItemPositions() {
    final positions =
        itemPositionsListener.itemPositions.value
            .map((position) => position.index)
            .toList()
          ..sort();
    if (positions.isEmpty) return;
    final generation = ++_generation;
    unawaited(_loadAround(generation, positions));
  }

  Future<void> _init(int generation) async {
    final count = await queryCount();
    _setState(
      value.copyWith(
        map: await _queryMap(limit, 0),
        count: count,
        hasData: count != 0,
        initialized: true,
      ),
      generation,
    );
  }

  Future<void> _update(int generation) async {
    final count = await queryCount();
    if (count == 0) {
      _setState(
        value.copyWith(map: {}, count: 0, hasData: false, initialized: true),
        generation,
      );
      return;
    }
    final offset = value.map.isEmpty ? 0 : value.map.keys.reduce(min);
    _setState(
      value.copyWith(
        map: await _queryMap(max(value.map.length, limit), offset),
        count: count,
        hasData: true,
        initialized: true,
      ),
      generation,
    );
  }

  Future<void> _loadAround(int generation, List<int> positions) async {
    final prefetchDistance = limit ~/ 2;
    if (!value.initialized ||
        value.map.isEmpty ||
        _containsRange(
          positions.first - prefetchDistance,
          positions.last + prefetchDistance,
        )) {
      return;
    }

    final expectedStart = positions.first - limit;
    final expectedEnd = positions.last + limit;
    final missingRanges = _missingRanges(expectedStart, expectedEnd);
    var map = value.map;

    if (missingRanges.length > 1 &&
        missingRanges.first.$2 - missingRanges.first.$1 > limit) {
      map = await _queryMap(
        positions.length + limit,
        positions.first - prefetchDistance,
      );
    } else {
      for (final (start, end) in missingRanges) {
        map = {...map, ...await _queryMap(end - start, start)};
      }
      if (map.length > expectedEnd - expectedStart) {
        map.removeWhere(
          (index, _) => index < expectedStart || index > expectedEnd,
        );
      }
    }

    _setState(value.copyWith(map: map, initialized: true), generation);
  }

  Future<Map<int, T>> _queryMap(int limit, int offset) async {
    final safeOffset = max(offset, 0);
    final list = await queryRange(limit, safeOffset);
    return Map.fromIterables(
      List.generate(min(limit, list.length), (index) => safeOffset + index),
      list,
    );
  }

  bool _containsRange(int start, int end) {
    final safeStart = max(start, 0);
    final safeEnd = max(min(end, value.count - 1), 0);
    return value.map[safeStart] != null && value.map[safeEnd] != null;
  }

  List<(int, int)> _missingRanges(int start, int end) {
    final safeStart = max(start, 0);
    final safeEnd = min(end, value.count);
    final loadedIndexes = value.map.keys.toList()..sort();
    return [
      if (safeStart < loadedIndexes.first)
        (safeStart, min(loadedIndexes.first, safeEnd)),
      if (loadedIndexes.last < safeEnd)
        (max(loadedIndexes.last, safeStart), safeEnd),
    ];
  }

  void _setState(PagingState<T> state, int generation) {
    if (_disposed || generation != _generation) return;
    value = state;
  }

  @override
  void dispose() {
    _disposed = true;
    itemPositionsListener.itemPositions.removeListener(_onItemPositions);
    super.dispose();
  }
}
