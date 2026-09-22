// JavaScript numbers cannot represent every Rust i64. Reject values outside
// the exact integer range instead of silently rounding IDs or timestamps.
const _maxSafeInteger = 9007199254740991;
final _maxSafeBigInt = BigInt.from(_maxSafeInteger);

extension RustIntToDart on BigInt {
  int toDartInt() {
    if (this < -_maxSafeBigInt || this > _maxSafeBigInt) {
      throw RangeError(
        'Rust integer is outside the exact JavaScript range: $this',
      );
    }
    return toInt();
  }
}

extension DartIntToRust on int {
  BigInt toRustInt() {
    if (this < -_maxSafeInteger || this > _maxSafeInteger) {
      throw RangeError(
        'Dart integer is outside the exact JavaScript range: $this',
      );
    }
    return BigInt.from(this);
  }
}
