import 'dart:typed_data';

import 'package:http/http.dart' as http;
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart';

class CoreHttpClient extends http.BaseClient {
  CoreHttpClient(this._desktop);

  final DesktopHandle _desktop;
  var _closed = false;

  @override
  Future<http.StreamedResponse> send(http.BaseRequest request) async {
    if (_closed) throw StateError('HTTP client is closed');

    final bytes = BytesBuilder(copy: false);
    await for (final chunk in request.finalize()) {
      bytes.add(chunk);
    }
    final body = bytes.takeBytes();
    final response = await _desktop.httpRequest(
      method: request.method,
      url: request.url.toString(),
      headers: request.headers,
      body: body.isEmpty ? null : body,
    );
    return http.StreamedResponse(
      Stream.value(response.body),
      response.statusCode,
      contentLength: response.body.length,
      headers: response.headers,
      request: request,
    );
  }

  @override
  void close() => _closed = true;
}
