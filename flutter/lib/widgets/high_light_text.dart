import 'dart:async';
import 'dart:ui' as ui;

import 'package:data_detector/data_detector.dart';
import 'package:emojis/emoji.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:url_launcher/url_launcher.dart';

import '../theme.dart';
import '../utils/emoji.dart';
import 'adaptive_selection_toolbar.dart';
import 'app_protocol_handler.dart';

class CustomText extends StatelessWidget {
  const CustomText(
    String this.text, {
    super.key,
    this.style,
    this.textMatchers,
    this.maxLines,
    this.overflow,
    this.textAlign,
  }) : textSpan = null;

  const CustomText.rich(
    InlineSpan this.textSpan, {
    super.key,
    this.style,
    this.textMatchers,
    this.maxLines,
    this.overflow,
    this.textAlign,
  }) : text = null;

  final String? text;
  final InlineSpan? textSpan;
  final TextStyle? style;
  final Iterable<TextMatcher>? textMatchers;
  final int? maxLines;
  final TextOverflow? overflow;
  final TextAlign? textAlign;

  @override
  Widget build(BuildContext context) {
    final spans = TextMatcher.applyTextMatchers([
      textSpan ?? TextSpan(text: text, style: style),
    ], textMatchers ?? [EmojiTextMatcher()]).toList();
    final defaultTextStyle = DefaultTextStyle.of(context);
    var effectiveTextStyle = style;
    if (style == null || style!.inherit) {
      effectiveTextStyle = defaultTextStyle.style.merge(style);
    }
    if (MediaQuery.boldTextOf(context)) {
      effectiveTextStyle = effectiveTextStyle!.merge(
        const TextStyle(fontWeight: FontWeight.bold),
      );
    }
    final registrar = SelectionContainer.maybeOf(context);
    Widget result = _CustomRichText(
      text: TextSpan(children: spans, style: effectiveTextStyle),
      textAlign: textAlign ?? defaultTextStyle.textAlign ?? TextAlign.start,
      softWrap: defaultTextStyle.softWrap,
      overflow:
          overflow ?? effectiveTextStyle?.overflow ?? defaultTextStyle.overflow,
      textScaler: MediaQuery.textScalerOf(context),
      maxLines: maxLines ?? defaultTextStyle.maxLines,
      textWidthBasis: defaultTextStyle.textWidthBasis,
      textHeightBehavior:
          defaultTextStyle.textHeightBehavior ??
          DefaultTextHeightBehavior.maybeOf(context),
      selectionRegistrar: registrar,
      selectionColor:
          DefaultSelectionStyle.of(context).selectionColor ??
          DefaultSelectionStyle.defaultColor,
    );
    if (registrar != null) {
      result = MouseRegion(
        cursor:
            DefaultSelectionStyle.of(context).mouseCursor ??
            SystemMouseCursors.text,
        child: result,
      );
    }
    return result;
  }
}

class CustomSelectableText extends StatelessWidget {
  const CustomSelectableText(
    String this.text, {
    this.enableInteractiveSelection = true,
    super.key,
    this.textAlign,
    this.style,
    this.textMatchers,
    this.maxLines,
  }) : textSpan = null;

  const CustomSelectableText.rich(
    TextSpan this.textSpan, {
    this.enableInteractiveSelection = true,
    super.key,
    this.textAlign,
    this.style,
    this.textMatchers,
    this.maxLines,
  }) : text = null;

  final String? text;
  final TextSpan? textSpan;
  final TextStyle? style;
  final Iterable<TextMatcher>? textMatchers;
  final int? maxLines;
  final TextAlign? textAlign;
  final bool enableInteractiveSelection;

  @override
  Widget build(BuildContext context) {
    final spans = TextMatcher.applyTextMatchers([
      textSpan ?? TextSpan(text: text, style: style),
    ], textMatchers ?? [EmojiTextMatcher()]).toList();
    return SelectableText.rich(
      TextSpan(children: spans, style: style),
      style: style,
      textAlign: textAlign,
      maxLines: maxLines,
      selectionHeightStyle: ui.BoxHeightStyle.max,
      contextMenuBuilder: (context, selectableState) =>
          MixinAdaptiveSelectionToolbar(editableTextState: selectableState),
    );
  }
}

class CustomSelectableArea extends StatelessWidget {
  const CustomSelectableArea({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) => SelectionArea(
    contextMenuBuilder: (context, state) =>
        MixinAdaptiveSelectionAreaToolbar(state: state),
    child: child,
  );
}

typedef InlineMatchBuilder =
    InlineSpan Function(
      TextSpan originalSpan,
      String displayString,
      String linkString,
    );

class TextMatcher {
  TextMatcher.regExp({required this.regExp, required this.matchBuilder})
    : textRangesFromText = null;

  TextMatcher.textRangesFromText({
    required this.textRangesFromText,
    required this.matchBuilder,
  }) : regExp = null;

  final RegExp? regExp;
  final Iterable<TextRange> Function(String text)? textRangesFromText;
  final InlineMatchBuilder matchBuilder;

  static Iterable<InlineSpan> applyTextMatchers(
    Iterable<InlineSpan> spans,
    Iterable<TextMatcher> textMatchers,
  ) => textMatchers.fold(
    spans,
    (previous, matcher) => previous
        .map((span) => _applyMatcher(span, matcher))
        .toList(growable: false),
  );

  static InlineSpan _applyMatcher(InlineSpan span, TextMatcher matcher) {
    if (span is! TextSpan) return span;
    final children = <InlineSpan>[];
    final value = span.text;
    if (value != null && value.isNotEmpty) {
      var offset = 0;
      final ranges =
          matcher.textRangesFromText?.call(value) ??
          matcher.regExp!
              .allMatches(value)
              .map(
                (match) => TextRange(start: match.start, end: match.end),
              );
      for (final range in ranges) {
        if (range.start == range.end) continue;
        if (range.start > offset) {
          children.add(
            _copyTextSpan(span, value.substring(offset, range.start)),
          );
        }
        final matched = value.substring(range.start, range.end);
        children.add(matcher.matchBuilder(span, matched, matched));
        offset = range.end;
      }
      if (offset < value.length) {
        children.add(_copyTextSpan(span, value.substring(offset)));
      }
    }
    if (span.children != null) {
      children.addAll(
        span.children!.map((child) => _applyMatcher(child, matcher)),
      );
    }
    return TextSpan(
      style: span.style,
      children: children,
      recognizer: span.recognizer,
      mouseCursor: span.mouseCursor,
      onEnter: span.onEnter,
      onExit: span.onExit,
      semanticsLabel: span.semanticsLabel,
      locale: span.locale,
      spellOut: span.spellOut,
    );
  }

  static TextSpan _copyTextSpan(TextSpan source, String text) => TextSpan(
    text: text,
    style: source.style,
    recognizer: source.recognizer,
    mouseCursor: source.mouseCursor,
    onEnter: source.onEnter,
    onExit: source.onExit,
    semanticsLabel: source.semanticsLabel,
    locale: source.locale,
    spellOut: source.spellOut,
  );
}

class EmojiTextMatcher extends TextMatcher {
  EmojiTextMatcher()
    : super.regExp(
        regExp: emojiRegex,
        matchBuilder: (span, displayString, linkString) => TextSpan(
          text: displayString,
          style: TextStyle(fontFamily: kEmojiFontFamily),
          recognizer: span.recognizer,
          mouseCursor: span.mouseCursor,
          onEnter: span.onEnter,
          onExit: span.onExit,
          semanticsLabel: span.semanticsLabel,
          locale: span.locale,
          spellOut: span.spellOut,
        ),
      );
}

class UrlTextMatcher extends TextMatcher {
  UrlTextMatcher(BuildContext context)
    : super.textRangesFromText(
        textRangesFromText: (text) {
          if (defaultTargetPlatform == TargetPlatform.macOS) {
            return DataDetector(
              NSTextCheckingType.NSTextCheckingTypeLink,
            ).matchesInString(text).map((match) => match.range);
          }
          return _textRangesFromRegExp(text, _uriRegExp);
        },
        matchBuilder: (span, displayString, linkString) => TextSpan(
          text: displayString,
          style: TextStyle(color: context.theme.accent),
          mouseCursor: SystemMouseCursors.click,
          recognizer: TapGestureRecognizer()
            ..onTap = () => openUrl(context, linkString),
        ),
      );
}

final _uriRegExp = RegExp(
  r'\b[a-zA-z+]+:(?://)?[\w-]+(?:\.[\w-]+)*(?:[\w.,@?^=%&:/~+#-]*[\w@?^=%&/~+#-])?\b/?',
);

Iterable<TextRange> _textRangesFromRegExp(String text, RegExp regExp) => regExp
    .allMatches(text)
    .map(
      (match) => TextRange(start: match.start, end: match.end),
    );

void openUrl(BuildContext context, String value) {
  final uri = Uri.tryParse(value);
  if (uri == null || uri.scheme.isEmpty) return;
  if (!AppProtocolHandler.maybeOpen(context, uri)) unawaited(launchUrl(uri));
}

class MailTextMatcher extends TextMatcher {
  MailTextMatcher(BuildContext context)
    : super.regExp(
        regExp: RegExp(
          r'\b[\w.%+-]+@[\w.-]+\.[A-Za-z]{2,}\b',
        ),
        matchBuilder: (span, displayString, linkString) => TextSpan(
          text: displayString,
          style: TextStyle(color: context.theme.accent),
          mouseCursor: SystemMouseCursors.click,
          recognizer: TapGestureRecognizer()
            ..onTap = () =>
                unawaited(launchUrl(Uri(scheme: 'mailto', path: linkString))),
        ),
      );
}

class KeyWordTextMatcher extends TextMatcher {
  KeyWordTextMatcher(this.keyword, {this.style, this.caseSensitive = true})
    : super.regExp(
        regExp: RegExp(RegExp.escape(keyword), caseSensitive: caseSensitive),
        matchBuilder: (span, displayString, linkString) => TextSpan(
          text: displayString,
          style: style,
          recognizer: span.recognizer,
          mouseCursor: span.mouseCursor,
          onEnter: span.onEnter,
          onExit: span.onExit,
          semanticsLabel: span.semanticsLabel,
          locale: span.locale,
          spellOut: span.spellOut,
        ),
      );

  final String keyword;
  final TextStyle? style;
  final bool caseSensitive;
}

class MultiKeyWordTextMatcher extends TextMatcher {
  MultiKeyWordTextMatcher(
    this.keywords, {
    this.style,
    this.caseSensitive = true,
  }) : super.regExp(
         regExp: _createMultiKeywordRegExp(keywords, caseSensitive),
         matchBuilder: (span, displayString, linkString) => TextSpan(
           text: displayString,
           style: style,
           recognizer: span.recognizer,
           mouseCursor: span.mouseCursor,
           onEnter: span.onEnter,
           onExit: span.onExit,
           semanticsLabel: span.semanticsLabel,
           locale: span.locale,
           spellOut: span.spellOut,
         ),
       );

  final List<String> keywords;
  final TextStyle? style;
  final bool caseSensitive;

  static RegExp _createMultiKeywordRegExp(
    List<String> keywords,
    bool caseSensitive,
  ) {
    final escapedKeywords = keywords
        .where((keyword) => keyword.trim().isNotEmpty)
        .map((keyword) => RegExp.escape(keyword.trim()))
        .toList();
    if (escapedKeywords.isEmpty) return RegExp('(?!)');
    return RegExp(
      '(${escapedKeywords.join('|')})',
      caseSensitive: caseSensitive,
    );
  }

  static TextMatcher createKeywordMatcher({
    required String keyword,
    TextStyle? style,
    bool caseSensitive = true,
  }) {
    if (keyword.trim().isEmpty) {
      throw ArgumentError('Keyword cannot be empty');
    }
    return keyword.trim().contains(' ')
        ? MultiKeyWordTextMatcher(
            keyword.trim().split(RegExp(r'\s+')),
            style: style,
            caseSensitive: caseSensitive,
          )
        : KeyWordTextMatcher(
            keyword,
            style: style,
            caseSensitive: caseSensitive,
          );
  }
}

class _CustomRichText extends RichText {
  _CustomRichText({
    required super.text,
    super.textAlign = TextAlign.start,
    super.softWrap = true,
    super.overflow = TextOverflow.clip,
    super.textScaler = TextScaler.noScaling,
    super.maxLines,
    super.textWidthBasis = TextWidthBasis.parent,
    super.textHeightBehavior,
    super.selectionRegistrar,
    super.selectionColor,
  });

  @override
  RenderParagraph createRenderObject(BuildContext context) =>
      _CustomRenderParagraph(
        text,
        textAlign: textAlign,
        textDirection: textDirection ?? Directionality.of(context),
        softWrap: softWrap,
        overflow: overflow,
        textScaler: textScaler,
        maxLines: maxLines,
        strutStyle: strutStyle,
        textWidthBasis: textWidthBasis,
        textHeightBehavior: textHeightBehavior,
        locale: locale ?? Localizations.maybeLocaleOf(context),
        registrar: selectionRegistrar,
        selectionColor: selectionColor,
      );
}

class _CustomRenderParagraph extends RenderParagraph {
  _CustomRenderParagraph(
    super.text, {
    required super.textDirection,
    super.softWrap = true,
    super.textAlign,
    super.overflow = TextOverflow.clip,
    super.maxLines,
    super.strutStyle,
    super.textScaler,
    super.textHeightBehavior,
    super.textWidthBasis,
    super.locale,
    super.registrar,
    super.selectionColor,
  });

  @override
  List<TextBox> getBoxesForSelection(
    TextSelection selection, {
    ui.BoxHeightStyle boxHeightStyle = ui.BoxHeightStyle.max,
    ui.BoxWidthStyle boxWidthStyle = ui.BoxWidthStyle.tight,
  }) => super.getBoxesForSelection(
    selection,
    boxHeightStyle: boxHeightStyle,
    boxWidthStyle: boxWidthStyle,
  );
}
