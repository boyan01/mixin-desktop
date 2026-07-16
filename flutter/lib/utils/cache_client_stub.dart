import 'package:http/http.dart';

class CacheClient extends BaseClient {
  CacheClient(this.folderName, {Client? client})
    : _client = client ?? Client(),
      _ownsClient = client == null;

  final String folderName;
  final Client _client;
  final bool _ownsClient;

  @override
  Future<StreamedResponse> send(BaseRequest request) => _client.send(request);

  @override
  void close() {
    if (_ownsClient) _client.close();
  }
}
