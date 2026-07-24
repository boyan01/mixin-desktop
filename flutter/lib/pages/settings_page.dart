import 'dart:async';

import 'package:auto_size_text/auto_size_text.dart';
import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:provider/provider.dart';

import '../constants/assets.dart';
import '../controllers/app_controller.dart';
import '../controllers/network_controller.dart';
import '../controllers/security_controller.dart';
import '../controllers/settings_controller.dart';
import '../l10n/l10n.dart';
import '../src/rust/desktop_api.dart' show AccountHandle, AccountProfile;
import '../theme.dart';
import '../utils/app_logger.dart';
import '../utils/local_notification_center.dart';
import '../widgets/avatar_view.dart';
import '../widgets/badges_widget.dart';
import '../widgets/high_light_text.dart';
import '../widgets/settings_widgets.dart';
import '../widgets/toast.dart';
import 'mcp_settings_page.dart';
import 'settings_account_pages.dart';
import 'settings_preference_pages.dart';
import 'settings_storage_about_pages.dart';

class SettingsPage extends StatefulWidget {
  const SettingsPage({
    required this.profile,
    required this.onSignOut,
    required this.onProfileUpdated,
    required this.onProfileRefresh,
    required this.onClose,
    super.key,
  });

  final AccountProfile profile;
  final Future<void> Function() onSignOut;
  final Future<void> Function(String fullName, String biography)
  onProfileUpdated;
  final Future<AccountProfile> Function() onProfileRefresh;
  final VoidCallback onClose;

  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

enum _SettingsDestination {
  editProfile,
  notifications,
  storage,
  security,
  proxy,
  appearance,
  mcp,
  about,
}

class _SettingsPageState extends State<SettingsPage>
    with WidgetsBindingObserver {
  final _routeNavigatorKey = GlobalKey<NavigatorState>();
  final _detailNavigatorKey = GlobalKey<NavigatorState>();
  var _wideMode = false;
  var _wideDetailOpened = false;
  var _activeDestination = _SettingsDestination.editProfile;
  String? _fullName;
  String? _biography;
  bool? _hasNotificationPermission;

  String get _displayName => _fullName ?? widget.profile.fullName;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    unawaited(_refreshNotificationPermission());
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      unawaited(_refreshNotificationPermission());
    }
  }

  Future<void> _refreshNotificationPermission() async {
    try {
      final value = await requestNotificationPermission();
      if (mounted) setState(() => _hasNotificationPermission = value);
    } catch (exception, stackTrace) {
      e('Refresh notification permission failed', exception, stackTrace);
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  Future<void> _signOut() async {
    await runFutureWithToast(widget.onSignOut());
  }

  void _push(Widget page, {bool nested = false}) {
    final route = MaterialPageRoute<void>(builder: (_) => page);
    final navigator =
        (_wideMode ? _detailNavigatorKey : _routeNavigatorKey).currentState!;
    if (_wideMode && !nested) {
      navigator.pushReplacement(route);
    } else {
      navigator.push(route);
    }
  }

  VoidCallback _popCallback(BuildContext context) =>
      () => Navigator.of(context).pop();

  void _openEditProfile() {
    setState(() => _activeDestination = _SettingsDestination.editProfile);
    _push(
      Builder(
        builder: (context) => EditProfileSettingsPage(
          userId: widget.profile.userId,
          fullName: _displayName,
          biography: _biography ?? widget.profile.biography,
          identityNumber: widget.profile.identityNumber,
          phone: widget.profile.phone,
          avatarUrl: widget.profile.avatarUrl,
          createdAt: DateTime.tryParse(widget.profile.createdAt),
          onBack: _wideMode ? null : _popCallback(context),
          onSave: (fullName, biography) async {
            await widget.onProfileUpdated(fullName, biography);
            if (!mounted) return;
            setState(() {
              _fullName = fullName;
              _biography = biography;
            });
          },
          onRefresh: widget.onProfileRefresh,
        ),
      ),
    );
  }

  void _openNotifications() {
    setState(() => _activeDestination = _SettingsDestination.notifications);
    final settings = context.read<SettingsController>();
    _push(
      NotificationSettingsPage(
        messagePreview: settings.messagePreview,
        onMessagePreviewChanged: settings.setMessagePreview,
      ),
    );
  }

  void _openStorage() {
    setState(() => _activeDestination = _SettingsDestination.storage);
    final account = context.read<AccountHandle>();
    _push(
      StoragePage(
        onOpenStorageUsage: () => _push(
          StorageUsageListPage(
            account: account,
            onSelected: (entry) => _push(
              StorageUsageDetailPage(
                account: account,
                name: entry.conversation.name,
                conversationId: entry.conversation.id,
              ),
              nested: true,
            ),
          ),
          nested: true,
        ),
      ),
    );
  }

  void _openSecurity() {
    setState(() => _activeDestination = _SettingsDestination.security);
    final security = context.read<SecurityController>();
    _push(
      Builder(
        builder: (context) => SecuritySettingsPage(
          onBack: _wideMode ? null : _popCallback(context),
          hasPasscode: security.hasPasscode,
          biometricEnabled: security.biometric,
          autoLockDuration: security.lockDuration,
          onPasscodeChanged: security.setPasscode,
          onBiometricChanged: (enabled) async {
            if (!await security.canAuthenticate()) return false;
            await security.setBiometric(enabled);
            return true;
          },
          onAutoLockChanged: security.setLockDuration,
        ),
      ),
    );
  }

  void _openProxy() {
    setState(() => _activeDestination = _SettingsDestination.proxy);
    _push(
      Consumer<NetworkController>(
        builder: (context, network, child) => ProxySettingsPage(
          enabled: network.enabled,
          proxies: network.proxies,
          selectedProxyId: network.selectedProxyId,
          onEnabledChanged: network.setEnabled,
          onProxySelected: network.selectProxy,
          onProxyAdded: network.addProxy,
          onProxyDeleted: network.deleteProxy,
        ),
      ),
    );
  }

  void _openAppearance() {
    setState(() => _activeDestination = _SettingsDestination.appearance);
    final settings = context.read<SettingsController>();
    _push(
      AppearanceSettingsPage(
        brightness: switch (settings.themeMode) {
          ThemeMode.light => Brightness.light,
          ThemeMode.dark => Brightness.dark,
          ThemeMode.system => null,
        },
        showAvatar: settings.messageShowAvatar,
        showIdentityNumber: settings.messageShowIdentityNumber,
        chatFontSizeDelta: settings.chatFontSizeDelta,
        onBrightnessChanged: (brightness) =>
            settings.setThemeMode(switch (brightness) {
              Brightness.light => ThemeMode.light,
              Brightness.dark => ThemeMode.dark,
              null => ThemeMode.system,
            }),
        onShowAvatarChanged: settings.setMessageShowAvatar,
        onShowIdentityNumberChanged: settings.setMessageShowIdentityNumber,
        onChatFontSizeDeltaChanged: settings.setChatFontSizeDelta,
      ),
    );
  }

  void _openMcp() {
    setState(() => _activeDestination = _SettingsDestination.mcp);
    _push(McpSettingsPage(desktop: context.read<AppController>().desktop));
  }

  Future<void> _openAbout() async {
    setState(() => _activeDestination = _SettingsDestination.about);
    final version = await PackageInfo.fromPlatform()
        .then(
          (info) =>
              '${info.version}${info.buildNumber.isEmpty ? '' : '(${info.buildNumber})'}',
        )
        .onError((_, _) => '1.0.0');
    if (!mounted) return;
    _push(
      AboutPage(
        version: version,
        onOpenLogDirectory: openAppLogDirectory,
        onLoadLogs: readAppLogLines,
      ),
    );
  }

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      _wideMode = constraints.maxWidth >= 620;
      if (!_wideMode) {
        _wideDetailOpened = false;
        return Navigator(
          key: _routeNavigatorKey,
          onGenerateInitialRoutes: (_, _) => [
            MaterialPageRoute<void>(builder: (_) => _SettingsHome(this)),
          ],
        );
      }
      if (!_wideDetailOpened) {
        _wideDetailOpened = true;
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted && _wideMode) _openEditProfile();
        });
      }
      return Row(
        children: [
          SizedBox(
            width: 300,
            child: DecoratedBox(
              decoration: BoxDecoration(
                border: Border(
                  right: BorderSide(color: context.mixinTheme.divider),
                ),
              ),
              child: _SettingsHome(this),
            ),
          ),
          Expanded(
            child: Navigator(
              key: _detailNavigatorKey,
              onGenerateInitialRoutes: (_, _) => [
                MaterialPageRoute<void>(
                  builder: (_) =>
                      ColoredBox(color: context.mixinTheme.chatBackground),
                ),
              ],
            ),
          ),
        ],
      );
    },
  );
}

class _SettingsHome extends StatelessWidget {
  const _SettingsHome(this.state);

  final _SettingsPageState state;

  Widget _icon(BuildContext context, String asset, {Color? color}) =>
      SvgPicture.asset(
        asset,
        width: 24,
        height: 24,
        colorFilter: ColorFilter.mode(
          color ?? context.mixinTheme.text,
          BlendMode.srcIn,
        ),
      );

  @override
  Widget build(BuildContext context) {
    final colors = context.mixinTheme;
    return ColoredBox(
      color: colors.background,
      child: Column(
        children: [
          const SizedBox(height: 64),
          Expanded(
            child: SingleChildScrollView(
              child: Column(
                children: [
                  _UserProfile(
                    profile: state.widget.profile,
                    fullName: state._displayName,
                  ),
                  const SizedBox(height: 24),
                  CellGroup(
                    child: CellItem(
                      key: const ValueKey('settings-edit-profile'),
                      leading: _icon(context, MixinAssets.profile),
                      title: AutoSizeText(
                        context.l10n.editProfile,
                        maxLines: 1,
                      ),
                      selected:
                          state._wideMode &&
                          state._activeDestination ==
                              _SettingsDestination.editProfile,
                      onTap: state._openEditProfile,
                    ),
                  ),
                  CellGroup(
                    child: Column(
                      children: [
                        CellItem(
                          key: const ValueKey('settings-notifications'),
                          leading: _icon(
                            context,
                            MixinAssets.notification,
                            color: state._hasNotificationPermission == false
                                ? colors.red
                                : null,
                          ),
                          title: AutoSizeText(
                            context.l10n.notifications,
                            maxLines: 1,
                          ),
                          color: state._hasNotificationPermission == false
                              ? colors.red
                              : colors.text,
                          trailing: state._hasNotificationPermission == false
                              ? Padding(
                                  padding: const EdgeInsets.all(4),
                                  child: SvgPicture.asset(
                                    MixinAssets.warning,
                                    colorFilter: ColorFilter.mode(
                                      colors.red,
                                      BlendMode.srcIn,
                                    ),
                                    width: 22,
                                    height: 22,
                                  ),
                                )
                              : const Arrow(),
                          selected:
                              state._wideMode &&
                              state._activeDestination ==
                                  _SettingsDestination.notifications,
                          onTap: state._openNotifications,
                        ),
                        CellItem(
                          key: const ValueKey('settings-storage'),
                          leading: _icon(context, MixinAssets.storageUsage),
                          title: AutoSizeText(
                            context.l10n.dataAndStorageUsage,
                            maxLines: 1,
                          ),
                          selected:
                              state._wideMode &&
                              state._activeDestination ==
                                  _SettingsDestination.storage,
                          onTap: state._openStorage,
                        ),
                        CellItem(
                          key: const ValueKey('settings-security'),
                          leading: _icon(context, MixinAssets.shield),
                          title: AutoSizeText(
                            context.l10n.security,
                            maxLines: 1,
                          ),
                          selected:
                              state._wideMode &&
                              state._activeDestination ==
                                  _SettingsDestination.security,
                          onTap: state._openSecurity,
                        ),
                        CellItem(
                          key: const ValueKey('settings-proxy'),
                          leading: _icon(context, MixinAssets.proxy),
                          title: AutoSizeText(context.l10n.proxy, maxLines: 1),
                          selected:
                              state._wideMode &&
                              state._activeDestination ==
                                  _SettingsDestination.proxy,
                          onTap: state._openProxy,
                        ),
                        CellItem(
                          key: const ValueKey('settings-appearance'),
                          leading: _icon(context, MixinAssets.appearance),
                          title: AutoSizeText(
                            context.l10n.appearance,
                            maxLines: 1,
                          ),
                          selected:
                              state._wideMode &&
                              state._activeDestination ==
                                  _SettingsDestination.appearance,
                          onTap: state._openAppearance,
                        ),
                        CellItem(
                          key: const ValueKey('settings-mcp'),
                          leading: const Icon(Icons.hub_outlined),
                          title: const AutoSizeText(
                            'Local MCP Server',
                            maxLines: 1,
                          ),
                          selected:
                              state._wideMode &&
                              state._activeDestination ==
                                  _SettingsDestination.mcp,
                          onTap: state._openMcp,
                        ),
                        CellItem(
                          key: const ValueKey('settings-about'),
                          leading: _icon(context, MixinAssets.about),
                          title: AutoSizeText(context.l10n.about, maxLines: 1),
                          selected:
                              state._wideMode &&
                              state._activeDestination ==
                                  _SettingsDestination.about,
                          onTap: state._openAbout,
                        ),
                      ],
                    ),
                  ),
                  CellGroup(
                    child: CellItem(
                      key: const ValueKey('settings-sign-out'),
                      leading: _icon(
                        context,
                        MixinAssets.signOut,
                        color: colors.red,
                      ),
                      title: AutoSizeText(context.l10n.signOut, maxLines: 1),
                      color: colors.red,
                      trailing: const SizedBox(),
                      onTap: state._signOut,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _UserProfile extends StatelessWidget {
  const _UserProfile({required this.profile, required this.fullName});

  final AccountProfile profile;
  final String fullName;

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    children: [
      AvatarView(
        userId: profile.userId,
        name: fullName,
        avatarUrl: profile.avatarUrl,
        size: 90,
      ),
      const SizedBox(height: 10),
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 20),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            CustomText(
              fullName,
              maxLines: 2,
              textAlign: TextAlign.center,
              style: TextStyle(
                fontWeight: FontWeight.w600,
                fontSize: 18,
                color: context.mixinTheme.text,
              ),
            ),
            BadgesWidget(
              verified: profile.isVerified,
              isBot: false,
              membership: profile.membership,
            ),
          ],
        ),
      ),
      const SizedBox(height: 4),
      SelectableText(
        'Mixin ID: ${profile.identityNumber}',
        style: TextStyle(
          fontSize: 14,
          color: context.dynamicColor(
            const Color.fromRGBO(188, 190, 195, 1),
            darkColor: const Color.fromRGBO(255, 255, 255, 0.4),
          ),
        ),
      ),
    ],
  );
}
