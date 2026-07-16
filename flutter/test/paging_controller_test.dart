import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/paging_controller.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';

void main() {
  testWidgets('loads a bounded window around the visible items', (
    tester,
  ) async {
    final requestedRanges = <(int, int)>[];
    final controller = PagingController<int>(
      limit: 15,
      queryCount: () async => 100,
      queryRange: (limit, offset) async {
        requestedRanges.add((limit, offset));
        return List.generate(
          limit,
          (index) => offset + index,
        ).where((index) => index < 100).toList();
      },
    );
    addTearDown(controller.dispose);

    await tester.pumpWidget(
      MaterialApp(
        home: ValueListenableBuilder(
          valueListenable: controller,
          builder: (context, state, child) => ScrollablePositionedList.builder(
            itemCount: state.count,
            itemPositionsListener: controller.itemPositionsListener,
            itemScrollController: controller.itemScrollController,
            itemBuilder: (context, index) => SizedBox(
              height: 78,
              child: Text('${state.map[index] ?? 'loading'}'),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(controller.value.map.keys, containsAll(List.generate(15, (i) => i)));

    controller.itemScrollController.jumpTo(index: 50);
    await tester.pumpAndSettle();

    expect(controller.value.map[50], 50);
    expect(controller.value.map.length, lessThan(100));
    expect(requestedRanges.any((range) => range.$2 > 0), isTrue);
  });
}
