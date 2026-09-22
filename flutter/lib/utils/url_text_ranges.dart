import 'package:flutter/services.dart';

Iterable<TextRange> urlTextRanges(String text) => _uriRegExp
    .allMatches(text)
    .map((match) => TextRange(start: match.start, end: match.end));

final _uriRegExp = RegExp(
  r'\b[a-zA-z+]+:(?://)?[\w-]+(?:\.[\w-]+)*(?:[\w.,@?^=%&:/~+#-]*[\w@?^=%&/~+#-])?\b/?',
);
