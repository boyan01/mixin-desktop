import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:http/http.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';

class CacheClient extends BaseClient {
  CacheClient(this.folderName, {Client? client})
    : _client = client ?? Client(),
      _ownsClient = client == null;

  final String folderName;
  final Client _client;
  final bool _ownsClient;

  @override
  Future<Response> get(Uri url, {Map<String, String>? headers}) async {
    final cacheKey = md5.convert(utf8.encode(url.toString())).toString();
    final cache = await _loadCache(cacheKey);
    if (cache != null) return Response.bytes(cache, HttpStatus.ok);

    final response = await super.get(url, headers: headers);
    if (response.statusCode == HttpStatus.ok) {
      await _saveCache(cacheKey, response.bodyBytes);
    }
    return response;
  }

  @override
  Future<StreamedResponse> send(BaseRequest request) => _client.send(request);

  Future<Directory> _getCacheDir() async {
    final appTempDirectory = await getTemporaryDirectory();
    return Directory(p.join(appTempDirectory.path, folderName));
  }

  Future<Uint8List?> _loadCache(String key) async {
    final cacheDir = await _getCacheDir();
    final file = File(p.join(cacheDir.path, key));
    return file.existsSync() ? file.readAsBytes() : null;
  }

  Future<void> _saveCache(String key, Uint8List bytes) async {
    final cacheDir = await _getCacheDir();
    if (!cacheDir.existsSync()) await cacheDir.create(recursive: true);
    await File(p.join(cacheDir.path, key)).writeAsBytes(bytes);
  }

  @override
  void close() {
    if (_ownsClient) _client.close();
  }
}
