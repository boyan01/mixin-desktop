import 'dart:async';
import 'dart:io';

import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:uuid/uuid.dart';

import '../constants/assets.dart';
import '../l10n/l10n.dart';
import '../models/message_list_entry.dart';
import '../src/rust/desktop_api.dart' show ProxyItem;
import '../theme.dart';
import '../utils/local_notification_center.dart';
import '../widgets/action_button.dart';
import '../widgets/adaptive_selection_toolbar.dart';
import '../widgets/animated_visibility.dart';
import '../widgets/message_bubble.dart';
import '../widgets/message_content.dart';
import '../widgets/message_datetime_and_status.dart';
import '../widgets/message_day_time.dart';
import '../widgets/mixin_dialog.dart';
import '../widgets/settings_widgets.dart';

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
      child: Container(
        padding: const EdgeInsets.only(top: 20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _SectionTitle(context.l10n.theme),
            CellGroup(
              cellBackgroundColor:
                  context.mixinTheme.settingCellBackgroundColor,
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
              cellBackgroundColor:
                  context.mixinTheme.settingCellBackgroundColor,
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
            ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 600),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  _SectionTitle(context.l10n.chatTextSize, top: 22),
                  _ChatTextSizePreview(fontSizeDelta: _chatFontSizeDelta),
                  const SizedBox(height: 10),
                  Row(
                    children: [
                      const SizedBox(width: 10),
                      Text(
                        'A',
                        style: TextStyle(
                          fontSize: 12,
                          color: context.mixinTheme.text,
                        ),
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: SliderTheme(
                          data: const SliderThemeData(
                            trackHeight: 4,
                            trackShape: RoundedRectSliderTrackShape(),
                            overlayShape: RoundSliderOverlayShape(
                              overlayRadius: 10,
                            ),
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
                ],
              ),
            ),
          ],
        ),
      ),
    ),
  );
}

class NotificationSettingsPage extends StatefulWidget {
  const NotificationSettingsPage({
    required this.messagePreview,
    required this.onMessagePreviewChanged,
    super.key,
  });

  final bool messagePreview;
  final ValueChanged<bool> onMessagePreviewChanged;

  @override
  State<NotificationSettingsPage> createState() =>
      _NotificationSettingsPageState();
}

class _NotificationSettingsPageState extends State<NotificationSettingsPage>
    with WidgetsBindingObserver {
  late bool _messagePreview = widget.messagePreview;
  bool? _hasNotificationPermission;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    unawaited(_refreshPermission());
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) unawaited(_refreshPermission());
  }

  Future<void> _refreshPermission() async {
    final value = await requestNotificationPermission();
    if (mounted) setState(() => _hasNotificationPermission = value);
  }

  @override
  void didUpdateWidget(NotificationSettingsPage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.messagePreview != widget.messagePreview) {
      _messagePreview = widget.messagePreview;
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => _PreferenceScaffold(
    title: context.l10n.notifications,
    child: Container(
      alignment: Alignment.topCenter,
      padding: const EdgeInsets.only(top: 40),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          CellGroup(
            padding: const EdgeInsets.only(right: 10, left: 10),
            cellBackgroundColor: context.mixinTheme.settingCellBackgroundColor,
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
            padding: const EdgeInsets.only(left: 20, bottom: 14, top: 10),
            child: Text(
              context.l10n.messagePreviewDescription,
              style: TextStyle(
                color: context.mixinTheme.secondaryText,
                fontSize: 14,
              ),
            ),
          ),
          if (Platform.isMacOS)
            AnimatedVisibility(
              visible: _hasNotificationPermission == false,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  CellGroup(
                    padding: const EdgeInsets.only(right: 10, left: 10),
                    cellBackgroundColor:
                        context.mixinTheme.settingCellBackgroundColor,
                    child: CellItem(
                      title: Text(context.l10n.enablePushNotification),
                      onTap: () => launchUrl(
                        Uri.parse(
                          'x-apple.systempreferences:com.apple.preference.notifications',
                        ),
                      ),
                    ),
                  ),
                  Padding(
                    padding: const EdgeInsets.only(
                      left: 20,
                      bottom: 14,
                      top: 10,
                    ),
                    child: Text(
                      context.l10n.notificationContent,
                      style: TextStyle(
                        color: context.mixinTheme.secondaryText,
                        fontSize: 14,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          if (!Platform.isMacOS)
            AnimatedVisibility(
              visible: _hasNotificationPermission == false,
              child: Padding(
                padding: const EdgeInsets.only(left: 20, bottom: 14),
                child: Text(
                  '${context.l10n.notificationPermissionManually}${context.l10n.notificationContent}',
                  style: TextStyle(
                    color: context.mixinTheme.secondaryText,
                    fontSize: 14,
                  ),
                ),
              ),
            ),
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
    return Scaffold(
      backgroundColor: context.mixinTheme.background,
      appBar: MixinAppBar(title: Text(context.l10n.proxy)),
      body: ConstrainedBox(
        constraints: const BoxConstraints.expand(),
        child: SingleChildScrollView(
          child: Column(
            children: [
              const SizedBox(height: 40),
              CellGroup(
                cellBackgroundColor:
                    context.mixinTheme.settingCellBackgroundColor,
                child: _SwitchCell(
                  title: context.l10n.proxy,
                  value: hasProxy && enabled,
                  onChanged: hasProxy
                      ? (value) => unawaited(onEnabledChanged(value))
                      : null,
                ),
              ),
              CellGroup(
                cellBackgroundColor:
                    context.mixinTheme.settingCellBackgroundColor,
                child: Column(
                  children: [
                    CellItem(
                      title: Text(context.l10n.addProxy),
                      leading: Icon(Icons.add, color: context.mixinTheme.icon),
                      trailing: null,
                      onTap: () => _showAddProxyDialog(context),
                    ),
                    Divider(
                      height: 0.5,
                      indent: 56,
                      color: context.mixinTheme.divider,
                    ),
                    for (final proxy in proxies)
                      _ProxyCell(
                        proxy: proxy,
                        selected: effectiveSelectedProxyId == proxy.id,
                        onSelected: () => unawaited(onProxySelected(proxy.id)),
                        onDeleted: () => unawaited(onProxyDeleted(proxy.id)),
                      ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Future<void> _showAddProxyDialog(BuildContext context) async {
    final proxy = await showMixinDialog<ProxyItem>(
      context: context,
      child: const _AddProxyDialog(),
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

  @override
  void dispose() {
    _hostController.dispose();
    _portController.dispose();
    _usernameController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  void _submit() {
    final host = _hostController.text;
    final port = int.tryParse(_portController.text);
    if (port == null) return;
    Navigator.pop(
      context,
      ProxyItem(
        id: const Uuid().v4(),
        kind: 'http',
        host: host,
        port: port,
      ),
    );
  }

  @override
  Widget build(BuildContext context) => AlertDialogLayout(
    title: Text(context.l10n.addProxy),
    titleMarginBottom: 24,
    content: DefaultTextStyle.merge(
      style: TextStyle(
        fontWeight: FontWeight.normal,
        fontSize: 14,
        color: context.mixinTheme.text,
      ),
      child: Column(
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
          ),
          const SizedBox(height: 16),
          _DialogLabel(context.l10n.proxyAuth),
          const SizedBox(height: 8),
          _ProxyInputGroup(
            firstController: _usernameController,
            secondController: _passwordController,
            firstHint: context.l10n.username,
            secondHint: context.l10n.password,
          ),
        ],
      ),
    ),
    actions: [
      MixinButton(
        backgroundTransparent: true,
        onTap: () => Navigator.pop(context),
        child: Text(context.l10n.cancel),
      ),
      MixinButton(onTap: _submit, child: Text(context.l10n.add)),
    ],
  );
}

class _ProxyTypeTile extends StatelessWidget {
  @override
  Widget build(BuildContext context) => Material(
    color: context.mixinTheme.listSelected,
    borderRadius: const BorderRadius.all(Radius.circular(8)),
    child: ListTileTheme(
      data: ListTileThemeData(dense: true, textColor: context.mixinTheme.text),
      child: Column(
        children: [
          ListTile(
            title: const Text('HTTP'),
            trailing: SvgPicture.asset(
              MixinAssets.checked,
              width: 24,
              height: 24,
            ),
            onTap: () {},
          ),
        ],
      ),
    ),
  );
}

class _ProxyInputGroup extends StatelessWidget {
  const _ProxyInputGroup({
    required this.firstController,
    required this.secondController,
    required this.firstHint,
    required this.secondHint,
  });

  final TextEditingController firstController;
  final TextEditingController secondController;
  final String firstHint;
  final String secondHint;

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
  });

  final TextEditingController controller;
  final String hint;
  final BorderRadius borderRadius;

  @override
  Widget build(BuildContext context) => TextField(
    controller: controller,
    style: TextStyle(fontSize: 14, color: context.mixinTheme.text),
    inputFormatters: [LengthLimitingTextInputFormatter(200)],
    contextMenuBuilder: (context, state) =>
        MixinAdaptiveSelectionToolbar(editableTextState: state),
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
  Widget build(BuildContext context) => Material(
    color: context.mixinTheme.settingCellBackgroundColor,
    child: ListTile(
      leading: SizedBox(
        height: double.infinity,
        width: 20,
        child: selected
            ? Icon(Icons.check, color: context.mixinTheme.icon, size: 20)
            : null,
      ),
      minLeadingWidth: 0,
      title: Text(
        '${proxy.host}:${proxy.port}',
        style: TextStyle(fontSize: 16, color: context.mixinTheme.text),
      ),
      subtitle: Text(
        proxy.kind,
        style: TextStyle(fontSize: 14, color: context.mixinTheme.secondaryText),
      ),
      trailing: ActionButton(
        name: MixinAssets.delete,
        color: context.mixinTheme.icon,
        onTap: onDeleted,
      ),
      onTap: () {
        if (selected) return;
        onSelected();
      },
    ),
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
      padding: EdgeInsets.only(left: 10, bottom: 14, top: top),
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
  Widget build(BuildContext context) {
    final createdAt = DateTime(2023, 1, 1, 8, 25);
    final messageHi = _previewMessage(
      id: 'preview-hi',
      content: context.l10n.sayHi,
      createdAt: createdAt,
    );
    final messageAnswer = _previewMessage(
      id: 'preview-answer',
      content: context.l10n.iAmGood,
      createdAt: createdAt,
    );
    return IgnorePointer(
      child: Container(
        margin: const EdgeInsets.symmetric(horizontal: 10),
        padding: const EdgeInsets.only(
          left: 20,
          right: 20,
          top: 10,
          bottom: 20,
        ),
        decoration: BoxDecoration(
          color: context.mixinTheme.chatBackground,
          borderRadius: const BorderRadius.all(Radius.circular(8)),
          image: DecorationImage(
            image: const ExactAssetImage(MixinAssets.chatBackground),
            fit: BoxFit.none,
            colorFilter: ColorFilter.mode(
              Theme.of(context).brightness == Brightness.dark
                  ? Colors.white.withValues(alpha: 0.02)
                  : Colors.black.withValues(alpha: 0.03),
              BlendMode.srcIn,
            ),
          ),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            MessageDayTime(dateTime: DateTime(2023)),
            _PreviewMessage(message: messageHi, isCurrentUser: true),
            _PreviewMessage(message: messageAnswer, isCurrentUser: false),
          ],
        ),
      ),
    );
  }
}

MessageListEntry _previewMessage({
  required String id,
  required String content,
  required DateTime createdAt,
}) => MessageListEntry(
  id: id,
  conversationId: 'fake',
  senderId: 'fake',
  senderName: '',
  senderAvatarUrl: '',
  senderIsVerified: false,
  category: 'PLAIN_TEXT',
  content: content,
  status: 'READ',
  createdAt: createdAt,
  mediaDuration: '',
  mediaStatus: '',
);

class _PreviewMessage extends StatelessWidget {
  const _PreviewMessage({required this.message, required this.isCurrentUser});

  final MessageListEntry message;
  final bool isCurrentUser;

  @override
  Widget build(BuildContext context) {
    final metadata = MessageDatetimeAndStatus(
      message: message,
      isCurrentUser: isCurrentUser,
    );
    return MessageBubble(
      isCurrentUser: isCurrentUser,
      showNip: true,
      child: MessageContent(
        message: message,
        isCurrentUser: isCurrentUser,
        dateAndStatus: metadata,
        overlayDateAndStatus: metadata,
      ),
    );
  }
}
