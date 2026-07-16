import 'dart:io';
import 'dart:typed_data';

import 'package:http/http.dart' as http;

const _kMinGifFrameDelayCentiseconds = 2;
const _kDefaultGifFrameDelayCentiseconds = 10;
const _kMaxDownloadedImageBytes = 32 * 1024 * 1024;

enum ImageType { gif, jpeg, png }

extension ImageTypeExtension on ImageType {
  String get mimeType => switch (this) {
    ImageType.gif => 'image/gif',
    ImageType.jpeg => 'image/jpeg',
    ImageType.png => 'image/png',
  };

  String get extension => switch (this) {
    ImageType.gif => 'gif',
    ImageType.jpeg => 'jpg',
    ImageType.png => 'png',
  };
}

Future<Uint8List> downloadImageBytes(
  String url, {
  http.Client? client,
  int maxBytes = _kMaxDownloadedImageBytes,
}) async {
  final resolved = Uri.base.resolve(url);
  final ownedClient = client == null ? http.Client() : null;
  try {
    final response = await (client ?? ownedClient!).send(
      http.Request('GET', resolved),
    );
    if (response.statusCode != HttpStatus.ok) {
      throw HttpException(
        'NetworkImage HTTP ${response.statusCode}',
        uri: resolved,
      );
    }
    final builder = BytesBuilder(copy: false);
    final received = await _readImageStream(
      response,
      resolved,
      maxBytes,
      builder.add,
    );
    if (received == 0) {
      throw StateError('NetworkImage is an empty file: $resolved');
    }
    return builder.takeBytes();
  } finally {
    ownedClient?.close();
  }
}

Future<int> _readImageStream(
  http.StreamedResponse response,
  Uri resolved,
  int maxBytes,
  void Function(List<int> chunk) add,
) async {
  final contentLength = response.contentLength;
  if (contentLength != null && contentLength > maxBytes) {
    throw StateError('NetworkImage is too large: $resolved');
  }

  var received = 0;
  await for (final chunk in response.stream) {
    received += chunk.length;
    if (received > maxBytes) {
      throw StateError('NetworkImage is too large: $resolved');
    }
    add(chunk);
  }
  return received;
}

Uint8List normalizeGifBytesIfNeeded(Uint8List data) {
  if (!_isGifBytes(data)) return data;
  return normalizeGifFrameDurations(data);
}

Uint8List normalizeGifFrameDurations(Uint8List data) {
  if (!_isGifBytes(data)) return data;

  Uint8List? normalized;
  for (var index = 0; index <= data.length - 8; index++) {
    if (data[index] != 0x21 ||
        data[index + 1] != 0xF9 ||
        data[index + 2] != 0x04 ||
        data[index + 7] != 0x00) {
      continue;
    }
    final delay = data[index + 4] | (data[index + 5] << 8);
    if (delay >= _kMinGifFrameDelayCentiseconds) continue;
    normalized ??= Uint8List.fromList(data);
    normalized[index + 4] = _kDefaultGifFrameDelayCentiseconds;
    normalized[index + 5] = 0;
  }
  return normalized ?? data;
}

Future<void> normalizeGifFileIfNeeded(File file, String? mimeType) async {
  final normalizedMimeType = mimeType?.toLowerCase();
  if (normalizedMimeType != null &&
      normalizedMimeType != ImageType.gif.mimeType) {
    return;
  }
  if (!file.existsSync()) return;

  final opened = await file.open();
  late final Uint8List data;
  try {
    if (normalizedMimeType == null) {
      final header = await opened.read(6);
      if (!_isGifBytes(Uint8List.fromList(header))) return;
      await opened.setPosition(0);
    }
    data = await opened.read(await file.length());
  } finally {
    await opened.close();
  }

  final normalized = normalizeGifFrameDurations(data);
  if (!identical(normalized, data)) {
    await file.writeAsBytes(normalized, flush: true);
  }
}

bool _isGifBytes(Uint8List data) =>
    data.length >= 6 &&
    data[0] == 0x47 &&
    data[1] == 0x49 &&
    data[2] == 0x46 &&
    data[3] == 0x38 &&
    (data[4] == 0x37 || data[4] == 0x39) &&
    data[5] == 0x61;
