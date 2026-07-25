import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:mixin_desktop_ui/controllers/network_controller.dart';
import 'package:mixin_desktop_ui/network/core_http_client.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';

void main() {
  test('persists proxy mutations through DesktopHandle', () async {
    final desktop = _FakeDesktopHandle();
    final controller = NetworkController(desktop);
    await controller.initialize();
    const proxy = ProxyItem(
      id: 'proxy-id',
      kind: 'http',
      host: '127.0.0.1',
      port: 7890,
    );

    await controller.addProxy(proxy);
    await controller.setEnabled(true);

    expect(controller.proxies, [proxy]);
    expect(controller.selectedProxyId, proxy.id);
    expect(controller.enabled, isTrue);
    expect(desktop.savedSettings, hasLength(2));
    expect(desktop.savedSettings.last.enabled, isTrue);
  });

  test('maps package:http requests to the Rust Core request API', () async {
    final desktop = _FakeDesktopHandle();
    final client = CoreHttpClient(desktop);
    final request = http.Request('POST', Uri.parse('https://example.com/image'))
      ..headers['content-type'] = 'application/json'
      ..body = '{"hello":"world"}';

    final response = await client.send(request);

    expect(desktop.lastMethod, 'POST');
    expect(desktop.lastUrl, 'https://example.com/image');
    expect(desktop.lastHeaders?['content-type'], 'application/json');
    expect(String.fromCharCodes(desktop.lastBody!), '{"hello":"world"}');
    expect(response.statusCode, 206);
    expect(await response.stream.bytesToString(), 'response');
  });
}

class _FakeDesktopHandle implements DesktopHandle {
  final settingsHandle = _FakeSettingsHandle();
  String? lastMethod;
  String? lastUrl;
  Map<String, String>? lastHeaders;
  Uint8List? lastBody;

  List<ProxySettingsItem> get savedSettings => settingsHandle.savedSettings;

  @override
  SettingsHandle get settings => settingsHandle;

  @override
  Future<HttpResponseItem> httpRequest({
    required String method,
    required String url,
    required Map<String, String> headers,
    Uint8List? body,
    BigInt? timeoutMillis,
    BigInt? maxResponseBytes,
  }) async {
    lastMethod = method;
    lastUrl = url;
    lastHeaders = headers;
    lastBody = body;
    return HttpResponseItem(
      statusCode: 206,
      headers: const {'content-type': 'text/plain'},
      body: Uint8List.fromList('response'.codeUnits),
    );
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _FakeSettingsHandle implements SettingsHandle {
  ProxySettingsItem proxy = const ProxySettingsItem(
    enabled: false,
    proxies: [],
  );
  final savedSettings = <ProxySettingsItem>[];

  @override
  Future<ProxySettingsItem> proxySettings() async => proxy;

  @override
  Future<void> setProxySettings({required ProxySettingsItem settings}) async {
    proxy = settings;
    savedSettings.add(settings);
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}
