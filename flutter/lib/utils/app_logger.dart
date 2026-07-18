import 'dart:io';

import 'package:mixin_desktop_ui/src/rust/api/desktop.dart';
import 'package:url_launcher/url_launcher.dart';

String? _logDirectoryPath;
var _isLogReady = false;

void initializeLogUtil() {
  _logDirectoryPath = rustLogDirectory();
  _isLogReady = true;
}

void v(String message) {
  _print(message, _LogLevel.verbose);
}

void d(String message) {
  _print(message, _LogLevel.debug);
}

void i(String message) {
  _print(message, _LogLevel.info);
}

void w(String message) {
  _print(message, _LogLevel.warning);
}

void e(String message, [Object? error, StackTrace? stackTrace]) {
  var messageWithStack = message;
  if (error != null) {
    messageWithStack += ' ($error)';
  }
  if (stackTrace != null) {
    messageWithStack += ':\n$stackTrace';
  }
  _print(messageWithStack, _LogLevel.error);
}

void wtf(String message) {
  _print(message, _LogLevel.wtf);
}

void _print(String message, _LogLevel level) {
  if (!_isLogReady) return;
  logFlutter(level: level.name, message: message);
}

enum _LogLevel { verbose, debug, info, warning, error, wtf }

Future<void> openAppLogDirectory() async {
  final directory = _logDirectoryPath;
  if (directory == null) return;
  await launchUrl(Uri.directory(directory));
}

Future<List<String>> readAppLogLines() async {
  final directory = _logDirectoryPath;
  if (directory == null) return const [];
  final logFiles = await Directory(directory)
      .list()
      .where((entity) => entity is File && entity.path.endsWith('.log'))
      .map((entity) => entity as File)
      .toList();
  if (logFiles.isEmpty) return const [];
  logFiles.sort((first, second) => first.path.compareTo(second.path));
  return logFiles.last.readAsLines();
}
