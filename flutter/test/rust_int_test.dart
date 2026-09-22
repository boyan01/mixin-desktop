import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/utils/rust_int.dart' as bridge;
import 'package:mixin_desktop_ui/utils/rust_int_web.dart' as web;

void main() {
  test('bridge integers preserve exact UI timestamps and signed bounds', () {
    for (final value in [
      0,
      -1,
      1,
      1790000000000000,
      9007199254740991,
      -9007199254740991,
    ]) {
      expect(
        bridge.RustIntToDart(
          bridge.DartIntToRust(value).toRustInt(),
        ).toDartInt(),
        value,
      );
      expect(web.RustIntToDart(BigInt.from(value)).toDartInt(), value);
    }
  });

  test(
    'web conversion rejects integers outside the exact JavaScript range',
    () {
      for (final value in [
        BigInt.parse('9007199254740992'),
        BigInt.parse('-9007199254740992'),
        BigInt.parse('9223372036854775807'),
      ]) {
        expect(() => web.RustIntToDart(value).toDartInt(), throwsRangeError);
      }
      expect(
        () => web.DartIntToRust(9007199254740992).toRustInt(),
        throwsRangeError,
      );
    },
  );
}
