import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/theme.dart';

typedef OpenMessageUri = void Function(Uri uri);
typedef OpenIdentityNumber = void Function(String identityNumber);

class SelectableMessageText extends StatefulWidget {
  const SelectableMessageText({
    required this.content,
    required this.style,
    super.key,
    this.onOpenUri,
    this.onOpenIdentityNumber,
    this.mentionNames = const {},
  });

  final String content;
  final TextStyle style;
  final OpenMessageUri? onOpenUri;
  final OpenIdentityNumber? onOpenIdentityNumber;
  final Map<String, String> mentionNames;

  @override
  State<SelectableMessageText> createState() => _SelectableMessageTextState();
}

class _SelectableMessageTextState extends State<SelectableMessageText> {
  late List<_TextSegment> _segments;

  @override
  void initState() {
    super.initState();
    _segments = _parseSegments();
  }

  @override
  void didUpdateWidget(covariant SelectableMessageText oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.content == widget.content &&
        oldWidget.onOpenUri == widget.onOpenUri &&
        oldWidget.onOpenIdentityNumber == widget.onOpenIdentityNumber &&
        identical(oldWidget.mentionNames, widget.mentionNames)) {
      return;
    }
    _disposeSegments();
    _segments = _parseSegments();
  }

  @override
  void dispose() {
    _disposeSegments();
    super.dispose();
  }

  void _disposeSegments() {
    for (final segment in _segments) {
      segment.recognizer?.dispose();
    }
  }

  List<_TextSegment> _parseSegments() {
    final segments = <_TextSegment>[];
    var offset = 0;
    for (final match in _interactivePattern.allMatches(widget.content)) {
      if (match.start > offset) {
        segments.add(
          _TextSegment(widget.content.substring(offset, match.start)),
        );
      }
      final raw = match.group(0)!;
      final type = _segmentType(raw);
      final interactiveText = type == _SegmentType.identity
          ? raw
          : raw.replaceFirst(_trailingPunctuation, '');
      final trailing = raw.substring(interactiveText.length);
      segments.add(_interactiveSegment(interactiveText, type));
      if (trailing.isNotEmpty) segments.add(_TextSegment(trailing));
      offset = match.end;
    }
    if (offset < widget.content.length) {
      segments.add(_TextSegment(widget.content.substring(offset)));
    }
    return segments;
  }

  _TextSegment _interactiveSegment(String text, _SegmentType type) {
    VoidCallback? onTap;
    switch (type) {
      case _SegmentType.uri:
        final uri = Uri.tryParse(text);
        if (uri != null && widget.onOpenUri != null) {
          onTap = () => widget.onOpenUri!(uri);
        }
        break;
      case _SegmentType.email:
        final uri = Uri(scheme: 'mailto', path: text);
        if (widget.onOpenUri != null) onTap = () => widget.onOpenUri!(uri);
        break;
      case _SegmentType.identity:
        final identityNumber = text.substring(1);
        if (widget.onOpenIdentityNumber != null) {
          onTap = () => widget.onOpenIdentityNumber!(identityNumber);
        }
        text = '@${widget.mentionNames[identityNumber] ?? identityNumber}';
        break;
    }
    final recognizer = onTap == null
        ? null
        : (TapGestureRecognizer()..onTap = onTap);
    return _TextSegment(text, interactive: true, recognizer: recognizer);
  }

  @override
  Widget build(BuildContext context) => SelectableText.rich(
    TextSpan(
      style: widget.style,
      children: [
        for (final segment in _segments)
          TextSpan(
            text: segment.text,
            recognizer: segment.recognizer,
            mouseCursor: segment.recognizer == null
                ? MouseCursor.defer
                : SystemMouseCursors.click,
            style: segment.interactive
                ? widget.style.copyWith(color: context.theme.accent)
                : widget.style,
          ),
      ],
    ),
  );
}

class _TextSegment {
  const _TextSegment(this.text, {this.interactive = false, this.recognizer});

  final String text;
  final bool interactive;
  final TapGestureRecognizer? recognizer;
}

enum _SegmentType { uri, email, identity }

final _interactivePattern = RegExp(
  r'https?://[^\s<]+|mixin:(?://)?[^\s<]+|'
  r'[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}|@\d+',
  caseSensitive: false,
);

final _trailingPunctuation = RegExp(r'[.,!?;:)\]\}]+$');

_SegmentType _segmentType(String value) {
  if (value.startsWith('@')) return _SegmentType.identity;
  if (value.toLowerCase().startsWith('http') ||
      value.toLowerCase().startsWith('mixin:')) {
    return _SegmentType.uri;
  }
  return _SegmentType.email;
}
