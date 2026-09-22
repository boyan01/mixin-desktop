import 'package:data_detector/data_detector.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'url_text_ranges.dart' as common;

Iterable<TextRange> urlTextRanges(String text) {
  if (defaultTargetPlatform == TargetPlatform.macOS) {
    return DataDetector(
      NSTextCheckingType.NSTextCheckingTypeLink,
    ).matchesInString(text).map((match) => match.range);
  }
  return common.urlTextRanges(text);
}
