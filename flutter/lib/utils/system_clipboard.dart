import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import 'package:path/path.dart' as path;
import 'package:path_provider/path_provider.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:uuid/uuid.dart';

import 'package:mixin_desktop_ui/widgets/toast.dart';

File? existingLocalFile(String? source) {
  final value = source?.trim();
  if (value == null || value.isEmpty) return null;
  final uri = Uri.tryParse(value);
  if (uri != null && uri.hasScheme && uri.scheme != 'file') return null;
  final file = uri?.scheme == 'file' ? File.fromUri(uri!) : File(value);
  return file.existsSync() ? file : null;
}

Future<bool> copyLocalFileToClipboard(File file) async {
  if (!file.existsSync()) {
    showToastFailed(null);
    return false;
  }
  try {
    final clipboard = SystemClipboard.instance;
    if (clipboard == null) {
      showToastFailed(null);
      return false;
    }
    final item = DataWriterItem()..add(Formats.fileUri(file.uri));
    await clipboard.write([item]);
  } on Object catch (error) {
    showToastFailed(error);
    return false;
  }
  showToastSuccessful();
  return true;
}

Future<List<File>> readClipboardFiles() async {
  final clipboard = SystemClipboard.instance;
  final reader = await clipboard?.read();
  if (reader == null) return const [];

  final fileItems = reader.items.where(
    (item) => item.canProvide(Formats.fileUri),
  );
  if (fileItems.isNotEmpty) {
    final uris = await Future.wait(
      fileItems.map((item) => item.readValue(Formats.fileUri)),
    );
    final files = uris
        .whereType<Uri>()
        .where((uri) => uri.scheme == 'file')
        .map(File.fromUri)
        .where((file) => file.existsSync())
        .toList(growable: false);
    if (files.isNotEmpty) return files;
  }

  const formats = <FileFormat>[
    Formats.jpeg,
    Formats.png,
    Formats.gif,
    Formats.webp,
    Formats.bmp,
  ];
  final files = <File>[];
  for (final item in reader.items) {
    final supported = item.getFormats(formats).whereType<FileFormat>();
    if (supported.isEmpty) continue;
    final bytes = await _readClipboardFile(item, supported.first);
    if (bytes == null) continue;
    final directory = await getTemporaryDirectory();
    final extension = _clipboardImageExtension(supported.first);
    final file = File(
      path.join(directory.path, 'mixin-paste-${const Uuid().v4()}.$extension'),
    );
    await file.writeAsBytes(bytes, flush: true);
    files.add(file);
  }
  return files;
}

Future<Uint8List?> _readClipboardFile(DataReader reader, FileFormat format) {
  final completer = Completer<Uint8List?>();
  final progress = reader.getFile(format, (file) async {
    try {
      completer.complete(await file.readAll());
    } on Object catch (error, stackTrace) {
      completer.completeError(error, stackTrace);
    }
  }, onError: completer.completeError);
  if (progress == null) completer.complete(null);
  return completer.future;
}

String _clipboardImageExtension(FileFormat format) {
  if (format == Formats.jpeg) return 'jpg';
  if (format == Formats.gif) return 'gif';
  if (format == Formats.webp) return 'webp';
  if (format == Formats.bmp) return 'bmp';
  return 'png';
}
