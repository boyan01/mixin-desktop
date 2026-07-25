import 'package:flutter/foundation.dart';
import '../network/core_http_client.dart';
import '../src/rust/desktop_api.dart';

class NetworkController extends ChangeNotifier {
  NetworkController(DesktopHandle desktop)
    : settings = desktop.settings,
      httpClient = CoreHttpClient(desktop);

  final SettingsHandle settings;
  final CoreHttpClient httpClient;

  ProxySettingsItem _settings = const ProxySettingsItem(
    enabled: false,
    proxies: [],
  );
  Object? error;
  bool loading = true;
  int revision = 0;
  Future<void> _pendingMutation = Future.value();

  bool get enabled => _settings.enabled;
  String? get selectedProxyId => _settings.selectedProxyId;
  List<ProxyItem> get proxies => List.unmodifiable(_settings.proxies);

  Future<void> initialize() async {
    loading = true;
    error = null;
    notifyListeners();
    try {
      _settings = await settings.proxySettings();
    } catch (exception) {
      error = exception;
    } finally {
      loading = false;
      notifyListeners();
    }
  }

  Future<void> setEnabled(bool enabled) => _mutate((current) {
    final selected = current.selectedProxyId ?? current.proxies.firstOrNull?.id;
    return ProxySettingsItem(
      enabled: enabled && selected != null,
      selectedProxyId: selected,
      proxies: current.proxies,
    );
  });

  Future<void> selectProxy(String id) => _mutate(
    (current) => ProxySettingsItem(
      enabled: current.enabled,
      selectedProxyId: id,
      proxies: current.proxies,
    ),
  );

  Future<void> addProxy(ProxyItem proxy) => _mutate(
    (current) => ProxySettingsItem(
      enabled: current.enabled,
      selectedProxyId: current.selectedProxyId,
      proxies: [...current.proxies, proxy],
    ),
  );

  Future<void> deleteProxy(String id) => _mutate((current) {
    final proxies = current.proxies.where((proxy) => proxy.id != id).toList();
    final selected = current.selectedProxyId ?? current.proxies.firstOrNull?.id;
    final deletedSelected = selected == id;
    return ProxySettingsItem(
      enabled: !deletedSelected && proxies.isNotEmpty && current.enabled,
      selectedProxyId: deletedSelected ? null : current.selectedProxyId,
      proxies: proxies,
    );
  });

  Future<void> _mutate(
    ProxySettingsItem Function(ProxySettingsItem current) update,
  ) {
    final operation = _pendingMutation.catchError((_) {}).then((_) async {
      final next = update(_settings);
      error = null;
      try {
        await settings.setProxySettings(settings: next);
        _settings = next;
        revision++;
      } catch (exception) {
        error = exception;
      } finally {
        notifyListeners();
      }
    });
    _pendingMutation = operation;
    return operation;
  }

  @override
  void dispose() {
    httpClient.close();
    super.dispose();
  }
}
