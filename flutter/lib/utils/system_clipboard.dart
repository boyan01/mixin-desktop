import 'dart:io';

import 'package:super_clipboard/super_clipboard.dart';

File? existingLocalFile(String? source) {
  final value = source?.trim();
  if (value == null || value.isEmpty) return null;
  final uri = Uri.tryParse(value);
  if (uri != null && uri.hasScheme && uri.scheme != 'file') return null;
  final file = uri?.scheme == 'file' ? File.fromUri(uri!) : File(value);
  return file.existsSync() ? file : null;
}

Future<bool> copyLocalFileToClipboard(File file) async {
  if (!file.existsSync()) return false;
  final clipboard = SystemClipboard.instance;
  if (clipboard == null) return false;
  final item = DataWriterItem()..add(Formats.fileUri(file.uri));
  await clipboard.write([item]);
  return true;
}
