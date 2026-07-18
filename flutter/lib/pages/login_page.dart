import 'package:flutter/material.dart';
import 'package:flutter_portal/flutter_portal.dart';
import 'package:mixin_desktop_ui/controllers/login_controller.dart';
import 'package:mixin_desktop_ui/pages/settings_storage_about_pages.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/utils/app_logger.dart';
import 'package:mixin_desktop_ui/widgets/buttons.dart';
import 'package:mixin_desktop_ui/widgets/qr_login_card.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:provider/provider.dart';

class LoginPage extends StatefulWidget {
  const LoginPage({
    required this.desktop,
    required this.onAuthenticated,
    required this.onFailure,
    super.key,
  });

  final DesktopHandle desktop;
  final ValueChanged<AccountHandle> onAuthenticated;
  final void Function(Object error, StackTrace stackTrace) onFailure;

  @override
  State<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> {
  late final LoginController controller;

  @override
  void initState() {
    super.initState();
    controller = LoginController(
      widget.desktop,
      widget.onAuthenticated,
      widget.onFailure,
    );
  }

  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => ChangeNotifierProvider.value(
    value: controller,
    child: const _LoginBody(),
  );
}

class _LoginBody extends StatelessWidget {
  const _LoginBody();

  @override
  Widget build(BuildContext context) {
    final controller = context.watch<LoginController>();
    return LandingScaffold(
      child: QrLoginCard(
        authUrl: controller.authUrl,
        loading: controller.status == LoginStatus.loading,
        provisioning: controller.status == LoginStatus.provisioning,
        error: controller.status == LoginStatus.failed
            ? controller.error ?? 'QR code expired. Please try again.'
            : null,
        onRetry: controller.refresh,
      ),
    );
  }
}

class LandingScaffold extends StatelessWidget {
  const LandingScaffold({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) => Portal(
    child: Scaffold(
      backgroundColor: context.dynamicColor(
        const Color(0xFFE5E5E5),
        darkColor: const Color.fromRGBO(35, 39, 43, 1),
      ),
      resizeToAvoidBottomInset: false,
      body: Stack(
        children: [
          Center(
            child: SizedBox(
              width: 520,
              height: 418,
              child: Material(
                color: context.mixinTheme.popUp,
                borderRadius: const BorderRadius.all(Radius.circular(13)),
                elevation: 10,
                child: ClipRRect(
                  borderRadius: const BorderRadius.all(Radius.circular(13)),
                  child: child,
                ),
              ),
            ),
          ),
          const Positioned(bottom: 16, right: 16, child: _VersionInfoWidget()),
        ],
      ),
    ),
  );
}

class _VersionInfoWidget extends StatefulWidget {
  const _VersionInfoWidget();

  @override
  State<_VersionInfoWidget> createState() => _VersionInfoWidgetState();
}

class _VersionInfoWidgetState extends State<_VersionInfoWidget> {
  late final Future<PackageInfo> _packageInfo = PackageInfo.fromPlatform();

  void _showLogs() {
    showGeneralDialog<void>(
      context: context,
      barrierColor: Colors.transparent,
      barrierDismissible: true,
      barrierLabel: MaterialLocalizations.of(context).modalBarrierDismissLabel,
      pageBuilder: (buildContext, animation, secondaryAnimation) =>
          InheritedTheme.capture(
            from: context,
            to: Navigator.of(context, rootNavigator: true).context,
          ).wrap(
            SettingsLogPage(
              onOpenDirectory: openAppLogDirectory,
              onLoadLogs: readAppLogLines,
            ),
          ),
    );
  }

  @override
  Widget build(BuildContext context) => FutureBuilder<PackageInfo>(
    future: _packageInfo,
    builder: (context, snapshot) {
      final info = snapshot.data;
      final version = info == null
          ? ''
          : '${info.version}${info.buildNumber.isEmpty ? '' : '(${info.buildNumber})'}';
      return NTapGestureDetector(
        n: 5,
        onTap: _showLogs,
        child: Text(
          version,
          style: TextStyle(
            fontSize: 14,
            color: context.mixinTheme.secondaryText,
          ),
        ),
      );
    },
  );
}
