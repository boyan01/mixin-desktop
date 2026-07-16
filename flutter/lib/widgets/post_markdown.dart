import 'dart:async';

import 'package:flutter/material.dart';
import 'package:markdown_widget/markdown_widget.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:url_launcher/url_launcher.dart';

MarkdownConfig postMarkdownConfig(
  BuildContext context, {
  required double fontSize,
}) {
  final colors = context.theme;
  final scale = fontSize / 16;
  TextStyle heading(double size) => TextStyle(
    color: colors.text,
    fontSize: size * scale,
    fontWeight: FontWeight.bold,
  );

  return MarkdownConfig(
    configs: [
      if (Theme.of(context).brightness == Brightness.dark) ...[
        HrConfig.darkConfig,
        PreConfig.darkConfig,
        CodeConfig.darkConfig,
        BlockquoteConfig.darkConfig,
      ],
      PConfig(
        textStyle: TextStyle(color: colors.text, fontSize: fontSize),
      ),
      H1Config(style: heading(32)),
      H2Config(style: heading(24)),
      H3Config(style: heading(20)),
      H4Config(style: heading(16)),
      H5Config(style: heading(16)),
      H6Config(style: heading(16)),
      LinkConfig(
        style: TextStyle(
          color: colors.accent,
          decoration: TextDecoration.underline,
        ),
        onTap: (href) {
          final uri = Uri.tryParse(href);
          if (uri != null) unawaited(launchUrl(uri));
        },
      ),
    ],
  );
}
