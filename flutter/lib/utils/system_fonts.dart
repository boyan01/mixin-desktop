import 'dart:convert';
import 'dart:io';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'app_logger.dart';

bool _fallbackFontsLoaded = false;
String? loadedFallbackFonts;

Future<void> loadFallbackFonts() async {
  if (!Platform.isLinux ||
      PlatformDispatcher.instance.locale.languageCode == 'en' ||
      _fallbackFontsLoaded) {
    return;
  }
  _fallbackFontsLoaded = true;
  try {
    final matched = Process.runSync('fc-match', ['-f', '%{family}']);
    if (matched.exitCode != 0) {
      e('Failed to get best match font family: ${matched.stderr}');
      return;
    }
    final result = Process.runSync('fc-list', [
      '-f',
      '%{family}:%{file}\n',
      matched.stdout as String,
    ]);
    final lines = const LineSplitter().convert(result.stdout as String);
    String? fontFamily;
    final fontPaths = <String>[];
    for (final line in lines) {
      final separator = line.indexOf(':');
      if (separator <= 0 || separator == line.length - 1) continue;
      final family = line.substring(0, separator);
      if (fontFamily == null) {
        fontFamily = family;
      } else if (fontFamily != family) {
        continue;
      }
      fontPaths.add(line.substring(separator + 1));
    }
    if (fontPaths.isEmpty || fontFamily == null) return;
    loadedFallbackFonts = fontFamily;
    for (final fontPath in fontPaths) {
      try {
        await loadFontFromList(
          File(fontPath.trim()).readAsBytesSync(),
          fontFamily: fontFamily,
        );
      } on Object catch (error, stackTrace) {
        e('Failed to load fallback font', error, stackTrace);
      }
    }
  } on Object catch (error, stackTrace) {
    e('Failed to load system fonts', error, stackTrace);
  }
}

extension ApplyFontsExtension on ThemeData {
  ThemeData withFallbackFonts() {
    if (loadedFallbackFonts == null) {
      if (Platform.isWindows) {
        return copyWith(
          textTheme: textTheme.applyFonts(null, ['Microsoft Yahei']),
          primaryTextTheme: primaryTextTheme.applyFonts(null, [
            'Microsoft Yahei',
          ]),
        );
      }
      return this;
    }
    return copyWith(
      textTheme: textTheme.applyFonts(loadedFallbackFonts, null),
      primaryTextTheme: primaryTextTheme.applyFonts(loadedFallbackFonts, null),
    );
  }
}

extension _TextThemeFonts on TextTheme {
  TextTheme applyFonts(String? fontFamily, List<String>? fontFamilyFallback) =>
      copyWith(
        displayLarge: displayLarge?.copyWith(
          fontFamily: fontFamily,
          fontFamilyFallback: fontFamilyFallback,
        ),
        displayMedium: displayMedium?.copyWith(
          fontFamily: fontFamily,
          fontFamilyFallback: fontFamilyFallback,
        ),
        displaySmall: displaySmall?.copyWith(
          fontFamily: fontFamily,
          fontFamilyFallback: fontFamilyFallback,
        ),
        headlineLarge: headlineLarge?.copyWith(
          fontFamily: fontFamily,
          fontFamilyFallback: fontFamilyFallback,
        ),
        headlineMedium: headlineMedium?.copyWith(
          fontFamily: fontFamily,
          fontFamilyFallback: fontFamilyFallback,
        ),
        headlineSmall: headlineSmall?.copyWith(
          fontFamily: fontFamily,
          fontFamilyFallback: fontFamilyFallback,
        ),
        titleLarge: titleLarge?.copyWith(
          fontFamily: fontFamily,
          fontFamilyFallback: fontFamilyFallback,
        ),
        titleMedium: titleMedium?.copyWith(
          fontFamily: fontFamily,
          fontFamilyFallback: fontFamilyFallback,
        ),
        titleSmall: titleSmall?.copyWith(
          fontFamily: fontFamily,
          fontFamilyFallback: fontFamilyFallback,
        ),
        bodyLarge: bodyLarge?.copyWith(
          fontFamily: fontFamily,
          fontFamilyFallback: fontFamilyFallback,
        ),
        bodyMedium: bodyMedium?.copyWith(
          fontFamily: fontFamily,
          fontFamilyFallback: fontFamilyFallback,
        ),
        bodySmall: bodySmall?.copyWith(
          fontFamily: fontFamily,
          fontFamilyFallback: fontFamilyFallback,
        ),
        labelLarge: labelLarge?.copyWith(
          fontFamily: fontFamily,
          fontFamilyFallback: fontFamilyFallback,
        ),
        labelMedium: labelMedium?.copyWith(
          fontFamily: fontFamily,
          fontFamilyFallback: fontFamilyFallback,
        ),
        labelSmall: labelSmall?.copyWith(
          fontFamily: fontFamily,
          fontFamilyFallback: fontFamilyFallback,
        ),
      );
}
