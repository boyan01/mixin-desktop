import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:path/path.dart' as path;
import 'package:path_provider/path_provider.dart';
import 'package:url_launcher/url_launcher.dart';

final appLogs = ValueNotifier<List<String>>(const []);
Directory? _logDirectory;
File? _logFile;

String? get appLogFilePath => _logFile?.path;

Future<void> initAppLogger() async {
  final support = await getApplicationSupportDirectory();
  final directory = Directory(path.join(support.path, 'logs'));
  await directory.create(recursive: true);
  _logDirectory = directory;
  final now = DateTime.now();
  _logFile = File(
    path.join(
      directory.path,
      '${now.year.toString().padLeft(4, '0')}-'
      '${now.month.toString().padLeft(2, '0')}-'
      '${now.day.toString().padLeft(2, '0')}.log',
    ),
  );
}

void writeAppLog(String message) {
  final line = '${DateTime.now().toIso8601String()} $message';
  _appendLogLine(line);
  final file = _logFile;
  if (file != null) {
    unawaited(
      file.writeAsString('$line\n', mode: FileMode.append, flush: true),
    );
  }
}

void appendRustLogLine(String line) {
  _appendLogLine(line);
}

void _appendLogLine(String line) {
  final next = [...appLogs.value, line];
  if (next.length > 1000) next.removeRange(0, next.length - 1000);
  appLogs.value = List.unmodifiable(next);
}

Future<void> openAppLogDirectory() async {
  final directory = _logDirectory;
  if (directory == null) return;
  await launchUrl(Uri.directory(directory.path));
}
