import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../src/rust/desktop_api.dart';
import '../theme.dart';
import '../utils/app_logger.dart';
import '../widgets/settings_widgets.dart';
import '../widgets/toast.dart';

class McpSettingsPage extends StatefulWidget {
  const McpSettingsPage({required this.desktop, super.key});

  final DesktopHandle desktop;

  @override
  State<McpSettingsPage> createState() => _McpSettingsPageState();
}

class _McpSettingsPageState extends State<McpSettingsPage> {
  McpSettingsItem? _settings;
  McpServerStatusItem? _status;
  Object? _error;
  var _updating = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final settings = await widget.desktop.settings.mcpSettings();
      final status = await widget.desktop.settings.mcpServerStatus();
      if (mounted) {
        setState(() {
          _settings = settings;
          _status = status;
          _error = null;
        });
      }
    } catch (error, stackTrace) {
      e('load MCP settings failed', error, stackTrace);
      if (mounted) {
        setState(() => _error = error);
      }
    }
  }

  Future<void> _update(McpSettingsItem next) async {
    if (_updating) return;
    setState(() => _updating = true);
    try {
      final status = await widget.desktop.settings.updateMcpSettings(
        settings: next,
      );
      if (mounted) {
        setState(() {
          _settings = next;
          _status = status;
          _error = null;
        });
      }
    } catch (error, stackTrace) {
      e('update MCP settings failed', error, stackTrace);
      if (mounted) {
        setState(() => _error = error);
        showToastFailed(error, context: context);
      }
    } finally {
      if (mounted) {
        setState(() => _updating = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final settings = _settings;
    return Scaffold(
      backgroundColor: context.mixinTheme.background,
      appBar: const MixinAppBar(title: Text('Local MCP Server')),
      body: settings == null
          ? Center(
              child: _error == null
                  ? const CircularProgressIndicator()
                  : Text('Failed to load MCP settings: $_error'),
            )
          : ListView(
              padding: const EdgeInsets.symmetric(vertical: 20),
              children: [
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 20),
                  child: Text(
                    _status?.running == true
                        ? 'Running on localhost'
                        : _status?.lastError ?? 'Stopped',
                    style: TextStyle(color: context.mixinTheme.secondaryText),
                  ),
                ),
                const SizedBox(height: 12),
                CellGroup(
                  cellBackgroundColor:
                      context.mixinTheme.settingCellBackgroundColor,
                  child: Column(
                    children: [
                      _switchCell(
                        context,
                        'Server',
                        settings.enabled,
                        (value) => _update(
                          McpSettingsItem(
                            enabled: value,
                            token: settings.token,
                            draftToolsEnabled: settings.draftToolsEnabled,
                            circleManagementEnabled:
                                settings.circleManagementEnabled,
                          ),
                        ),
                      ),
                      if (settings.enabled) ...[
                        const Divider(height: 1),
                        CellItem(
                          title: const Text('Endpoint'),
                          description: Text(_status?.endpoint ?? 'Not running'),
                          trailing: IconButton(
                            onPressed: _status?.endpoint == null
                                ? null
                                : () => _copy(_status!.endpoint!),
                            icon: const Icon(Icons.copy_rounded),
                          ),
                        ),
                        const Divider(height: 1),
                        CellItem(
                          title: const Text('Access Token'),
                          description: Text(_masked(settings.token)),
                          trailing: IconButton(
                            onPressed: () => _copy(settings.token),
                            icon: const Icon(Icons.copy_rounded),
                          ),
                        ),
                        const Divider(height: 1),
                        _switchCell(
                          context,
                          'Draft Editing',
                          settings.draftToolsEnabled,
                          (value) => _update(
                            McpSettingsItem(
                              enabled: settings.enabled,
                              token: settings.token,
                              draftToolsEnabled: value,
                              circleManagementEnabled:
                                  settings.circleManagementEnabled,
                            ),
                          ),
                        ),
                        const Divider(height: 1),
                        _switchCell(
                          context,
                          'Circle Management',
                          settings.circleManagementEnabled,
                          (value) => _update(
                            McpSettingsItem(
                              enabled: settings.enabled,
                              token: settings.token,
                              draftToolsEnabled: settings.draftToolsEnabled,
                              circleManagementEnabled: value,
                            ),
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
                const Padding(
                  padding: EdgeInsets.fromLTRB(20, 12, 20, 0),
                  child: Text(
                    'Local clients must use the bearer token. MCP never sends messages or changes account data.',
                  ),
                ),
              ],
            ),
    );
  }

  Widget _switchCell(
    BuildContext context,
    String title,
    bool value,
    ValueChanged<bool> onChanged,
  ) => CellItem(
    title: Text(title),
    trailing: Transform.scale(
      scale: .75,
      child: CupertinoSwitch(
        value: value,
        onChanged: _updating ? null : onChanged,
      ),
    ),
  );

  Future<void> _copy(String text) async {
    await Clipboard.setData(ClipboardData(text: text));
    if (mounted) showToastSuccessful();
  }

  String _masked(String token) => token.length < 8
      ? '••••••••'
      : '${token.substring(0, 4)}••••${token.substring(token.length - 4)}';
}
