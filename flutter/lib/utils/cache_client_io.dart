import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:http/http.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';

class CacheClient extends BaseClient {
  CacheClient(this.folderName, {Client? client, this.rootDirectory})
    : _client = client ?? Client(),
      _ownsClient = client == null;

  static final _pendingBytes = <String, Future<CachedBytes>>{};

  final String folderName;
  final Client _client;
  final bool _ownsClient;
  final Directory? rootDirectory;

  Client get client => _client;

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

  Future<CachedBytes> getOrFetchBytes(
    Uri url,
    Future<Uint8List> Function() fetch,
  ) {
    final cacheKey = _cacheKey(url);
    final pendingKey = [
      folderName,
      cacheKey,
      identityHashCode(_client),
      rootDirectory?.absolute.path ?? '',
    ].join('|');
    final pending = _pendingBytes[pendingKey];
    if (pending != null) return pending;

    final operation = _loadOrFetchBytes(cacheKey, fetch, pendingKey);
    _pendingBytes[pendingKey] = operation;
    return operation;
  }

  Future<void> evict(Uri url) async {
    final file = await _cacheFile(_cacheKey(url));
    if (await file.exists()) await file.delete();
  }

  Future<Directory> _getCacheDir() async {
    final appTempDirectory = rootDirectory ?? await getTemporaryDirectory();
    return Directory(p.join(appTempDirectory.path, folderName));
  }

  Future<File> _cacheFile(String key) async {
    final cacheDir = await _getCacheDir();
    return File(p.join(cacheDir.path, key));
  }

  Future<Uint8List?> _loadCache(String key) async {
    final file = await _cacheFile(key);
    if (!await file.exists()) return null;
    try {
      return await file.readAsBytes();
    } on FileSystemException {
      if (await file.exists()) await file.delete();
      return null;
    }
  }

  Future<void> _saveCache(String key, Uint8List bytes) async {
    final cacheDir = await _getCacheDir();
    if (!await cacheDir.exists()) await cacheDir.create(recursive: true);

    final destination = File(p.join(cacheDir.path, key));
    final temporary = File(
      '${destination.path}.${DateTime.now().microsecondsSinceEpoch}.tmp',
    );
    try {
      await temporary.writeAsBytes(bytes, flush: true);
      if (await destination.exists()) {
        await temporary.delete();
      } else {
        try {
          await temporary.rename(destination.path);
        } on FileSystemException {
          if (!await destination.exists()) rethrow;
        }
      }
    } finally {
      if (await temporary.exists()) await temporary.delete();
    }
  }

  Future<CachedBytes> _loadOrFetchBytes(
    String cacheKey,
    Future<Uint8List> Function() fetch,
    String pendingKey,
  ) async {
    try {
      final cached = await _loadCache(cacheKey);
      if (cached != null) return CachedBytes(cached, fromCache: true);

      final bytes = await fetch();
      await _saveCache(cacheKey, bytes);
      return CachedBytes(bytes, fromCache: false);
    } finally {
      final _ = _pendingBytes.remove(pendingKey);
    }
  }

  String _cacheKey(Uri url) =>
      md5.convert(utf8.encode(url.toString())).toString();

  @override
  void close() {
    if (_ownsClient) _client.close();
  }
}

class CachedBytes {
  const CachedBytes(this.bytes, {required this.fromCache});

  final Uint8List bytes;
  final bool fromCache;
}
