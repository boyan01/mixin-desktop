import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/controllers/network_controller.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/pages/settings_account_pages.dart';
import 'package:mixin_desktop_ui/pages/settings_preference_pages.dart';
import 'package:mixin_desktop_ui/pages/settings_storage_about_pages.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' show AccountProfile;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/settings_widgets.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:provider/provider.dart';
import 'package:url_launcher/url_launcher.dart';

class SettingsPage extends StatefulWidget {
  const SettingsPage({
    required this.profile,
    required this.onSignOut,
    required this.onClose,
    super.key,
  });

  final AccountProfile profile;
  final Future<void> Function() onSignOut;
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
  about,
}

class _SettingsPageState extends State<SettingsPage> {
  final _routeNavigatorKey = GlobalKey<NavigatorState>();
  final _detailNavigatorKey = GlobalKey<NavigatorState>();
  var _wideMode = false;
  var _wideDetailOpened = false;
  var _activeDestination = _SettingsDestination.editProfile;
  var _signingOut = false;
  String? _fullName;
  String? _biography;
  var _hasPasscode = false;
  var _biometricEnabled = false;
  var _autoLockDuration = Duration.zero;

  String get _displayName => _fullName ?? widget.profile.fullName;

  Future<void> _signOut() async {
    if (_signingOut) return;
    setState(() => _signingOut = true);
    try {
      await widget.onSignOut();
    } catch (_) {
      if (mounted) {
        ScaffoldMessenger.maybeOf(
          context,
        )?.showSnackBar(SnackBar(content: Text(context.l10n.failed)));
      }
    } finally {
      if (mounted) setState(() => _signingOut = false);
    }
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
            if (!mounted) return;
            setState(() {
              _fullName = fullName;
              _biography = biography;
            });
          },
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
    _push(
      StoragePage(
        onOpenStorageUsage: () => _push(
          StorageUsageListPage(entries: const [], onSelected: (_) {}),
          nested: true,
        ),
      ),
    );
  }

  void _openSecurity() {
    setState(() => _activeDestination = _SettingsDestination.security);
    _push(
      Builder(
        builder: (context) => SecuritySettingsPage(
          onBack: _wideMode ? null : _popCallback(context),
          hasPasscode: _hasPasscode,
          biometricEnabled: _biometricEnabled,
          autoLockDuration: _autoLockDuration,
          onPasscodeChanged: (passcode) async {
            _hasPasscode = passcode != null;
            if (!_hasPasscode) _biometricEnabled = false;
          },
          onBiometricChanged: (enabled) async {
            _biometricEnabled = enabled;
            return true;
          },
          onAutoLockChanged: (duration) => _autoLockDuration = duration,
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

  Future<void> _openAbout() async {
    setState(() => _activeDestination = _SettingsDestination.about);
    final version = await PackageInfo.fromPlatform()
        .then((info) => '${info.version} (${info.buildNumber})')
        .onError((_, _) => '1.0.0');
    if (!mounted) return;
    _push(
      AboutPage(
        version: version,
        onOpenUri: (uri) =>
            launchUrl(uri, mode: LaunchMode.externalApplication).then((_) {}),
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

  Widget _icon(BuildContext context, String asset) => SvgPicture.asset(
    asset,
    width: 24,
    height: 24,
    colorFilter: ColorFilter.mode(context.mixinTheme.icon, BlendMode.srcIn),
  );

  @override
  Widget build(BuildContext context) {
    final colors = context.mixinTheme;
    return ColoredBox(
      color: colors.background,
      child: Column(
        children: [
          SizedBox(
            height: 64,
            child: Align(
              alignment: AlignmentDirectional.centerStart,
              child: Padding(
                padding: const EdgeInsetsDirectional.only(start: 8),
                child: IconButton(
                  key: const ValueKey('settings-close'),
                  onPressed: state.widget.onClose,
                  tooltip: MaterialLocalizations.of(context).closeButtonTooltip,
                  icon: SvgPicture.asset(
                    MixinAssets.close,
                    width: 20,
                    height: 20,
                    colorFilter: ColorFilter.mode(colors.icon, BlendMode.srcIn),
                  ),
                ),
              ),
            ),
          ),
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
                    cellBackgroundColor: colors.listSelected,
                    child: CellItem(
                      key: const ValueKey('settings-edit-profile'),
                      leading: _icon(context, MixinAssets.profile),
                      title: Text(context.l10n.editProfile),
                      selected:
                          state._wideMode &&
                          state._activeDestination ==
                              _SettingsDestination.editProfile,
                      onTap: state._openEditProfile,
                    ),
                  ),
                  CellGroup(
                    cellBackgroundColor: colors.listSelected,
                    child: Column(
                      children: [
                        CellItem(
                          key: const ValueKey('settings-notifications'),
                          leading: _icon(context, MixinAssets.notification),
                          title: Text(context.l10n.notifications),
                          selected:
                              state._wideMode &&
                              state._activeDestination ==
                                  _SettingsDestination.notifications,
                          onTap: state._openNotifications,
                        ),
                        CellItem(
                          key: const ValueKey('settings-storage'),
                          leading: _icon(context, MixinAssets.storageUsage),
                          title: Text(context.l10n.dataAndStorageUsage),
                          selected:
                              state._wideMode &&
                              state._activeDestination ==
                                  _SettingsDestination.storage,
                          onTap: state._openStorage,
                        ),
                        CellItem(
                          key: const ValueKey('settings-security'),
                          leading: _icon(context, MixinAssets.shield),
                          title: Text(context.l10n.security),
                          selected:
                              state._wideMode &&
                              state._activeDestination ==
                                  _SettingsDestination.security,
                          onTap: state._openSecurity,
                        ),
                        CellItem(
                          key: const ValueKey('settings-proxy'),
                          leading: _icon(context, MixinAssets.proxy),
                          title: Text(context.l10n.proxy),
                          selected:
                              state._wideMode &&
                              state._activeDestination ==
                                  _SettingsDestination.proxy,
                          onTap: state._openProxy,
                        ),
                        CellItem(
                          key: const ValueKey('settings-appearance'),
                          leading: _icon(context, MixinAssets.appearance),
                          title: Text(context.l10n.appearance),
                          selected:
                              state._wideMode &&
                              state._activeDestination ==
                                  _SettingsDestination.appearance,
                          onTap: state._openAppearance,
                        ),
                        CellItem(
                          key: const ValueKey('settings-about'),
                          leading: _icon(context, MixinAssets.about),
                          title: Text(context.l10n.about),
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
                    cellBackgroundColor: colors.listSelected,
                    child: CellItem(
                      key: const ValueKey('settings-sign-out'),
                      leading: _icon(context, MixinAssets.signOut),
                      title: Text(context.l10n.signOut),
                      color: colors.red,
                      trailing: state._signingOut
                          ? SizedBox.square(
                              dimension: 18,
                              child: CircularProgressIndicator(
                                strokeWidth: 2,
                                color: colors.red,
                              ),
                            )
                          : null,
                      onTap: state._signingOut ? null : state._signOut,
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
        child: Text(
          fullName,
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
          textAlign: TextAlign.center,
          style: TextStyle(
            fontWeight: FontWeight.w600,
            fontSize: 18,
            color: context.mixinTheme.text,
          ),
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
