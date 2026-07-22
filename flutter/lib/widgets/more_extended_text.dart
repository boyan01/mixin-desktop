import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import '../l10n/l10n.dart';
import '../theme.dart';
import 'high_light_text.dart';

class MoreExtendedText extends StatefulWidget {
  const MoreExtendedText(this.text, {this.style, super.key});

  final String text;
  final TextStyle? style;

  @override
  State<MoreExtendedText> createState() => _MoreExtendedTextState();
}

class _MoreExtendedTextState extends State<MoreExtendedText> {
  bool expanded = false;

  @override
  void didUpdateWidget(MoreExtendedText oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.text != widget.text) expanded = false;
  }

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final style = widget.style?.merge(const TextStyle(height: 1));
      final overflow = TextSpan(
        text: '...${context.l10n.more}',
        style: style?.merge(TextStyle(color: context.theme.accent)),
        recognizer: TapGestureRecognizer()
          ..onTap = () => setState(() => expanded = true),
      );
      var endIndex = -1;
      if (!expanded) {
        final painter =
            TextPainter(
              text: overflow,
              textDirection: TextDirection.rtl,
              maxLines: 6,
            )..layout(
              minWidth: constraints.minWidth,
              maxWidth: constraints.maxWidth,
            );
        final overflowSize = painter.size;
        painter
          ..text = TextSpan(text: widget.text, style: style)
          ..layout(
            minWidth: constraints.minWidth,
            maxWidth: constraints.maxWidth,
          );
        if (painter.didExceedMaxLines) {
          final position = painter.getPositionForOffset(
            Offset(
              painter.size.width - overflowSize.width,
              painter.size.height,
            ),
          );
          endIndex = painter.getOffsetBefore(position.offset) ?? -1;
        }
      }
      final text = endIndex == -1
          ? widget.text
          : widget.text.substring(0, endIndex);
      return CustomSelectableText.rich(
        TextSpan(
          children: [
            TextSpan(text: text, style: style),
            if (endIndex != -1) overflow,
          ],
        ),
        textMatchers: [UrlTextMatcher(context), EmojiTextMatcher()],
        textAlign: TextAlign.center,
      );
    },
  );
}
