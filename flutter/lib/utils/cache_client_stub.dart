import 'dart:typed_data';

import 'package:http/http.dart';

class CacheClient extends BaseClient {
  CacheClient(this.folderName, {Client? client, this.rootDirectory})
    : _client = client ?? Client(),
      _ownsClient = client == null;

  final String folderName;
  final Client _client;
  final bool _ownsClient;
  final Object? rootDirectory;

  Client get client => _client;

  @override
  Future<StreamedResponse> send(BaseRequest request) => _client.send(request);

  Future<CachedBytes> getOrFetchBytes(
    Uri url,
    Future<Uint8List> Function() fetch,
  ) async => CachedBytes(await fetch(), fromCache: false);

  Future<void> evict(Uri url) async {}

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
