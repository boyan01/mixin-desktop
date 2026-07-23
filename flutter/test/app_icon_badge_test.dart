import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';
import 'package:mixin_desktop_ui/widgets/app_icon_badge.dart';

void main() {
  testWidgets(
    'updates the macOS app icon badge from unseen message counts',
    (
      tester,
    ) async {
      const channel = MethodChannel('flutter_app_icon_badge');
      final calls = <MethodCall>[];
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
            ..setMockMethodCallHandler(channel, (call) async {
              calls.add(call);
            });
      addTearDown(() => messenger.setMockMethodCallHandler(channel, null));

      final counts = StreamController<PlatformInt64>.broadcast();
      addTearDown(counts.close);
      await tester.pumpWidget(
        MaterialApp(
          home: AppIconBadge(
            account: _FakeAccount(counts.stream),
            child: const SizedBox(),
          ),
        ),
      );

      counts.add(5);
      await tester.pump();
      await tester.pump();

      expect(calls.single.method, 'updateBadge');
      expect(calls.single.arguments, {'count': 5});
    },
    skip: !Platform.isMacOS,
  );
}

class _FakeAccount implements AccountHandle {
  _FakeAccount(this.counts);

  final Stream<PlatformInt64> counts;

  @override
  Stream<PlatformInt64> unseenMessageCountChanges() => counts;

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}
