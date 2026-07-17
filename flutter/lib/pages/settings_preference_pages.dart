import 'dart:async';

import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/settings_widgets.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' show ProxyItem;
import 'package:uuid/uuid.dart';

class AppearanceSettingsPage extends StatefulWidget {
  const AppearanceSettingsPage({
    required this.brightness,
    required this.showAvatar,
    required this.showIdentityNumber,
    required this.chatFontSizeDelta,
    required this.onBrightnessChanged,
    required this.onShowAvatarChanged,
    required this.onShowIdentityNumberChanged,
    required this.onChatFontSizeDeltaChanged,
    super.key,
  });

  final Brightness? brightness;
  final bool showAvatar;
  final bool showIdentityNumber;
  final double chatFontSizeDelta;
  final ValueChanged<Brightness?> onBrightnessChanged;
  final ValueChanged<bool> onShowAvatarChanged;
  final ValueChanged<bool> onShowIdentityNumberChanged;
  final ValueChanged<double> onChatFontSizeDeltaChanged;

  @override
  State<AppearanceSettingsPage> createState() => _AppearanceSettingsPageState();
}

class _AppearanceSettingsPageState extends State<AppearanceSettingsPage> {
  late Brightness? _brightness = widget.brightness;
  late bool _showAvatar = widget.showAvatar;
  late bool _showIdentityNumber = widget.showIdentityNumber;
  late double _chatFontSizeDelta = widget.chatFontSizeDelta;

  @override
  void didUpdateWidget(AppearanceSettingsPage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.brightness != widget.brightness) {
      _brightness = widget.brightness;
    }
    if (oldWidget.showAvatar != widget.showAvatar) {
      _showAvatar = widget.showAvatar;
    }
    if (oldWidget.showIdentityNumber != widget.showIdentityNumber) {
      _showIdentityNumber = widget.showIdentityNumber;
    }
    if (oldWidget.chatFontSizeDelta != widget.chatFontSizeDelta) {
      _chatFontSizeDelta = widget.chatFontSizeDelta;
    }
  }

  void _setBrightness(Brightness? value) {
    setState(() => _brightness = value);
    widget.onBrightnessChanged(value);
  }

  @override
  Widget build(BuildContext context) => _PreferenceScaffold(
    title: context.l10n.appearance,
    child: SingleChildScrollView(
      padding: const EdgeInsets.only(top: 20, bottom: 28),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _SectionTitle(context.l10n.theme),
          CellGroup(
            child: Column(
              children: [
                CellItem(
                  title: RadioItem<Brightness?>(
                    title: Text(context.l10n.followSystem),
                    value: null,
                    groupValue: _brightness,
                    onChanged: _setBrightness,
                  ),
                  trailing: null,
                ),
                CellItem(
                  title: RadioItem<Brightness?>(
                    title: Text(context.l10n.light),
                    value: Brightness.light,
                    groupValue: _brightness,
                    onChanged: _setBrightness,
                  ),
                  trailing: null,
                ),
                CellItem(
                  title: RadioItem<Brightness?>(
                    title: Text(context.l10n.dark),
                    value: Brightness.dark,
                    groupValue: _brightness,
                    onChanged: _setBrightness,
                  ),
                  trailing: null,
                ),
              ],
            ),
          ),
          _SectionTitle(context.l10n.chats, top: 22),
          CellGroup(
            child: Column(
              children: [
                _SwitchCell(
                  title: context.l10n.showAvatar,
                  value: _showAvatar,
                  onChanged: (value) {
                    setState(() => _showAvatar = value);
                    widget.onShowAvatarChanged(value);
                  },
                ),
                _SwitchCell(
                  title: context.l10n.showIdentityNumber,
                  value: _showIdentityNumber,
                  onChanged: (value) {
                    setState(() => _showIdentityNumber = value);
                    widget.onShowIdentityNumberChanged(value);
                  },
                ),
              ],
            ),
          ),
          _SectionTitle(context.l10n.chatTextSize, top: 22),
          _ChatTextSizePreview(fontSizeDelta: _chatFontSizeDelta),
          const SizedBox(height: 10),
          ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 600),
            child: Row(
              children: [
                const SizedBox(width: 10),
                Text('A', style: TextStyle(color: context.mixinTheme.text)),
                const SizedBox(width: 10),
                Expanded(
                  child: SliderTheme(
                    data: const SliderThemeData(
                      trackHeight: 4,
                      trackShape: RoundedRectSliderTrackShape(),
                      overlayShape: RoundSliderOverlayShape(overlayRadius: 10),
                    ),
                    child: Slider(
                      value: _chatFontSizeDelta.clamp(-2, 4).toDouble(),
                      min: -2,
                      max: 4,
                      divisions: 6,
                      onChanged: (value) {
                        setState(() => _chatFontSizeDelta = value);
                        widget.onChatFontSizeDeltaChanged(value);
                      },
                    ),
                  ),
                ),
                const SizedBox(width: 10),
                Text(
                  'A',
                  style: TextStyle(
                    fontSize: 24,
                    color: context.mixinTheme.text,
                  ),
                ),
                const SizedBox(width: 10),
              ],
            ),
          ),
        ],
      ),
    ),
  );
}

class NotificationSettingsPage extends StatefulWidget {
  const NotificationSettingsPage({
    required this.messagePreview,
    required this.onMessagePreviewChanged,
    this.hasNotificationPermission,
    this.onOpenSystemNotificationSettings,
    super.key,
  });

  final bool messagePreview;
  final ValueChanged<bool> onMessagePreviewChanged;
  final bool? hasNotificationPermission;
  final VoidCallback? onOpenSystemNotificationSettings;

  @override
  State<NotificationSettingsPage> createState() =>
      _NotificationSettingsPageState();
}

class _NotificationSettingsPageState extends State<NotificationSettingsPage> {
  late bool _messagePreview = widget.messagePreview;

  @override
  void didUpdateWidget(NotificationSettingsPage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.messagePreview != widget.messagePreview) {
      _messagePreview = widget.messagePreview;
    }
  }

  @override
  Widget build(BuildContext context) => _PreferenceScaffold(
    title: context.l10n.notifications,
    child: SingleChildScrollView(
      padding: const EdgeInsets.only(top: 40, bottom: 28),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          CellGroup(
            child: _SwitchCell(
              title: context.l10n.messagePreview,
              value: _messagePreview,
              onChanged: (value) {
                setState(() => _messagePreview = value);
                widget.onMessagePreviewChanged(value);
              },
            ),
          ),
          Padding(
            padding: const EdgeInsets.only(left: 20, right: 20, top: 10),
            child: Text(
              context.l10n.messagePreviewDescription,
              style: TextStyle(
                color: context.mixinTheme.secondaryText,
                fontSize: 14,
              ),
            ),
          ),
          if (widget.hasNotificationPermission == false) ...[
            const SizedBox(height: 14),
            CellGroup(
              child: CellItem(
                title: Text(context.l10n.enablePushNotification),
                onTap: widget.onOpenSystemNotificationSettings,
              ),
            ),
            Padding(
              padding: const EdgeInsets.only(left: 20, right: 20, top: 10),
              child: Text(
                context.l10n.notificationContent,
                style: TextStyle(
                  color: context.mixinTheme.secondaryText,
                  fontSize: 14,
                ),
              ),
            ),
          ],
        ],
      ),
    ),
  );
}

class ProxySettingsPage extends StatelessWidget {
  const ProxySettingsPage({
    required this.enabled,
    required this.proxies,
    required this.selectedProxyId,
    required this.onEnabledChanged,
    required this.onProxySelected,
    required this.onProxyAdded,
    required this.onProxyDeleted,
    super.key,
  });

  final bool enabled;
  final List<ProxyItem> proxies;
  final String? selectedProxyId;
  final Future<void> Function(bool) onEnabledChanged;
  final Future<void> Function(String) onProxySelected;
  final Future<void> Function(ProxyItem) onProxyAdded;
  final Future<void> Function(String) onProxyDeleted;

  @override
  Widget build(BuildContext context) {
    final hasProxy = proxies.isNotEmpty;
    final effectiveSelectedProxyId =
        selectedProxyId ?? (hasProxy ? proxies.first.id : null);
    return _PreferenceScaffold(
      title: context.l10n.proxy,
      child: SingleChildScrollView(
        padding: const EdgeInsets.only(top: 40, bottom: 28),
        child: Column(
          children: [
            CellGroup(
              child: _SwitchCell(
                title: context.l10n.proxy,
                value: hasProxy && enabled,
                onChanged: hasProxy
                    ? (value) => unawaited(onEnabledChanged(value))
                    : null,
              ),
            ),
            CellGroup(
              child: Column(
                children: [
                  CellItem(
                    title: Text(context.l10n.addProxy),
                    leading: Icon(Icons.add, color: context.mixinTheme.icon),
                    trailing: null,
                    onTap: () => _showAddProxyDialog(context),
                  ),
                  for (final proxy in proxies) ...[
                    Divider(
                      height: 0.5,
                      indent: 56,
                      color: context.mixinTheme.divider,
                    ),
                    _ProxyCell(
                      proxy: proxy,
                      selected: effectiveSelectedProxyId == proxy.id,
                      onSelected: () => unawaited(onProxySelected(proxy.id)),
                      onDeleted: () => unawaited(onProxyDeleted(proxy.id)),
                    ),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _showAddProxyDialog(BuildContext context) async {
    final proxy = await showDialog<ProxyItem>(
      context: context,
      builder: (context) => const _AddProxyDialog(),
    );
    if (proxy != null) await onProxyAdded(proxy);
  }
}

class _AddProxyDialog extends StatefulWidget {
  const _AddProxyDialog();

  @override
  State<_AddProxyDialog> createState() => _AddProxyDialogState();
}

class _AddProxyDialogState extends State<_AddProxyDialog> {
  final _hostController = TextEditingController();
  final _portController = TextEditingController();
  final _usernameController = TextEditingController();
  final _passwordController = TextEditingController();
  String? _error;

  @override
  void dispose() {
    _hostController.dispose();
    _portController.dispose();
    _usernameController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  void _submit() {
    final host = _hostController.text.trim();
    final port = int.tryParse(_portController.text.trim());
    if (host.isEmpty || port == null || port < 1 || port > 65535) {
      setState(() => _error = '${context.l10n.host} / ${context.l10n.port}');
      return;
    }
    Navigator.pop(
      context,
      ProxyItem(
        id: const Uuid().v4(),
        kind: 'http',
        host: host,
        port: port,
        username: _usernameController.text.isEmpty
            ? null
            : _usernameController.text,
        password: _passwordController.text.isEmpty
            ? null
            : _passwordController.text,
      ),
    );
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    backgroundColor: context.mixinTheme.popUp,
    title: Text(context.l10n.addProxy),
    content: SizedBox(
      width: 420,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _DialogLabel(context.l10n.proxyType),
          const SizedBox(height: 8),
          _ProxyTypeTile(),
          const SizedBox(height: 16),
          _DialogLabel(context.l10n.proxyConnection),
          const SizedBox(height: 8),
          _ProxyInputGroup(
            firstController: _hostController,
            secondController: _portController,
            firstHint: context.l10n.host,
            secondHint: context.l10n.port,
            secondKeyboardType: TextInputType.number,
          ),
          const SizedBox(height: 16),
          _DialogLabel(context.l10n.proxyAuth),
          const SizedBox(height: 8),
          _ProxyInputGroup(
            firstController: _usernameController,
            secondController: _passwordController,
            firstHint: context.l10n.username,
            secondHint: context.l10n.password,
            obscureSecond: true,
          ),
          if (_error != null) ...[
            const SizedBox(height: 10),
            Text(
              _error!,
              style: TextStyle(color: context.mixinTheme.red, fontSize: 13),
            ),
          ],
        ],
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: Text(context.l10n.cancel),
      ),
      FilledButton(onPressed: _submit, child: Text(context.l10n.add)),
    ],
  );
}

class _ProxyTypeTile extends StatelessWidget {
  @override
  Widget build(BuildContext context) => Container(
    height: 52,
    padding: const EdgeInsets.symmetric(horizontal: 16),
    decoration: BoxDecoration(
      color: context.mixinTheme.listSelected,
      borderRadius: const BorderRadius.all(Radius.circular(8)),
    ),
    child: Row(
      children: [
        Expanded(
          child: Text('HTTP', style: TextStyle(color: context.mixinTheme.text)),
        ),
        Icon(Icons.check, color: context.mixinTheme.accent, size: 22),
      ],
    ),
  );
}

class _ProxyInputGroup extends StatelessWidget {
  const _ProxyInputGroup({
    required this.firstController,
    required this.secondController,
    required this.firstHint,
    required this.secondHint,
    this.secondKeyboardType,
    this.obscureSecond = false,
  });

  final TextEditingController firstController;
  final TextEditingController secondController;
  final String firstHint;
  final String secondHint;
  final TextInputType? secondKeyboardType;
  final bool obscureSecond;

  @override
  Widget build(BuildContext context) => Column(
    children: [
      _ProxyTextField(
        controller: firstController,
        hint: firstHint,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(8)),
      ),
      Divider(height: 1, color: context.mixinTheme.divider),
      _ProxyTextField(
        controller: secondController,
        hint: secondHint,
        keyboardType: secondKeyboardType,
        obscureText: obscureSecond,
        borderRadius: const BorderRadius.vertical(bottom: Radius.circular(8)),
      ),
    ],
  );
}

class _ProxyTextField extends StatelessWidget {
  const _ProxyTextField({
    required this.controller,
    required this.hint,
    required this.borderRadius,
    this.keyboardType,
    this.obscureText = false,
  });

  final TextEditingController controller;
  final String hint;
  final BorderRadius borderRadius;
  final TextInputType? keyboardType;
  final bool obscureText;

  @override
  Widget build(BuildContext context) => TextField(
    controller: controller,
    keyboardType: keyboardType,
    obscureText: obscureText,
    style: TextStyle(fontSize: 14, color: context.mixinTheme.text),
    inputFormatters: [LengthLimitingTextInputFormatter(1024)],
    decoration: InputDecoration(
      isDense: true,
      hintText: hint,
      hintStyle: TextStyle(
        color: context.mixinTheme.secondaryText,
        fontSize: 14,
      ),
      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 18),
      border: OutlineInputBorder(
        borderRadius: borderRadius,
        borderSide: BorderSide.none,
      ),
      filled: true,
      fillColor: context.mixinTheme.listSelected,
    ),
  );
}

class _ProxyCell extends StatelessWidget {
  const _ProxyCell({
    required this.proxy,
    required this.selected,
    required this.onSelected,
    required this.onDeleted,
  });

  final ProxyItem proxy;
  final bool selected;
  final VoidCallback onSelected;
  final VoidCallback onDeleted;

  @override
  Widget build(BuildContext context) => ListTile(
    minLeadingWidth: 20,
    leading: SizedBox(
      width: 20,
      child: selected
          ? Icon(Icons.check, color: context.mixinTheme.icon, size: 20)
          : null,
    ),
    title: Text(
      '${proxy.host}:${proxy.port}',
      style: TextStyle(fontSize: 16, color: context.mixinTheme.text),
    ),
    subtitle: Text(
      proxy.kind.toUpperCase(),
      style: TextStyle(fontSize: 14, color: context.mixinTheme.secondaryText),
    ),
    trailing: IconButton(
      tooltip: MaterialLocalizations.of(context).deleteButtonTooltip,
      onPressed: onDeleted,
      icon: Icon(Icons.delete_outline, color: context.mixinTheme.icon),
    ),
    onTap: selected ? null : onSelected,
  );
}

class _PreferenceScaffold extends StatelessWidget {
  const _PreferenceScaffold({required this.title, required this.child});

  final String title;
  final Widget child;

  @override
  Widget build(BuildContext context) => Scaffold(
    backgroundColor: context.mixinTheme.background,
    appBar: MixinAppBar(title: Text(title)),
    body: Align(alignment: Alignment.topCenter, child: child),
  );
}

class _SwitchCell extends StatelessWidget {
  const _SwitchCell({
    required this.title,
    required this.value,
    required this.onChanged,
  });

  final String title;
  final bool value;
  final ValueChanged<bool>? onChanged;

  @override
  Widget build(BuildContext context) => CellItem(
    title: Text(title),
    trailing: Transform.scale(
      scale: 0.7,
      child: CupertinoSwitch(
        activeTrackColor: context.mixinTheme.accent,
        value: value,
        onChanged: onChanged,
      ),
    ),
    onTap: onChanged == null ? null : () => onChanged!(!value),
  );
}

class _SectionTitle extends StatelessWidget {
  const _SectionTitle(this.text, {this.top = 0});

  final String text;
  final double top;

  @override
  Widget build(BuildContext context) => ConstrainedBox(
    constraints: const BoxConstraints(maxWidth: 600),
    child: Padding(
      padding: EdgeInsets.only(left: 10, right: 10, bottom: 14, top: top),
      child: Text(
        text,
        style: TextStyle(color: context.mixinTheme.secondaryText, fontSize: 14),
      ),
    ),
  );
}

class _DialogLabel extends StatelessWidget {
  const _DialogLabel(this.text);

  final String text;

  @override
  Widget build(BuildContext context) => Text(
    text,
    style: TextStyle(color: context.mixinTheme.secondaryText, fontSize: 14),
  );
}

class _ChatTextSizePreview extends StatelessWidget {
  const _ChatTextSizePreview({required this.fontSizeDelta});

  final double fontSizeDelta;

  @override
  Widget build(BuildContext context) => ConstrainedBox(
    constraints: const BoxConstraints(maxWidth: 600),
    child: IgnorePointer(
      child: Container(
        margin: const EdgeInsets.symmetric(horizontal: 10),
        padding: const EdgeInsets.fromLTRB(20, 10, 20, 20),
        decoration: BoxDecoration(
          color: context.mixinTheme.chatBackground,
          image: const DecorationImage(
            image: ExactAssetImage(MixinAssets.chatBackground),
            repeat: ImageRepeat.repeat,
          ),
          borderRadius: const BorderRadius.all(Radius.circular(8)),
        ),
        child: Column(
          children: [
            Text(
              '08:25',
              style: TextStyle(
                color: context.mixinTheme.secondaryText,
                fontSize: 12,
              ),
            ),
            const SizedBox(height: 10),
            Align(
              alignment: Alignment.centerRight,
              child: _PreviewBubble(
                text: context.l10n.sayHi,
                fontSize: 14 + fontSizeDelta,
                color: context.mixinTheme.accent,
                textColor: Colors.white,
              ),
            ),
            const SizedBox(height: 8),
            Align(
              alignment: Alignment.centerLeft,
              child: _PreviewBubble(
                text: context.l10n.iAmGood,
                fontSize: 14 + fontSizeDelta,
                color: context.mixinTheme.primary,
                textColor: context.mixinTheme.text,
              ),
            ),
          ],
        ),
      ),
    ),
  );
}

class _PreviewBubble extends StatelessWidget {
  const _PreviewBubble({
    required this.text,
    required this.fontSize,
    required this.color,
    required this.textColor,
  });

  final String text;
  final double fontSize;
  final Color color;
  final Color textColor;

  @override
  Widget build(BuildContext context) => Container(
    constraints: const BoxConstraints(maxWidth: 360),
    padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
    decoration: BoxDecoration(
      color: color,
      borderRadius: const BorderRadius.all(Radius.circular(12)),
    ),
    child: Text(
      text,
      style: TextStyle(fontSize: fontSize, color: textColor),
    ),
  );
}
