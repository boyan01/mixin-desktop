import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/controllers/login_controller.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/qr_login_card.dart';
import 'package:provider/provider.dart';

class LoginPage extends StatefulWidget {
  const LoginPage({
    required this.desktop,
    required this.onAuthenticated,
    super.key,
  });

  final DesktopHandle desktop;
  final ValueChanged<AccountHandle> onAuthenticated;

  @override
  State<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> {
  late final LoginController controller;

  @override
  void initState() {
    super.initState();
    controller = LoginController(widget.desktop, widget.onAuthenticated);
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
    return Scaffold(
      backgroundColor: context.dynamicColor(
        const Color(0xFFE5E5E5),
        darkColor: const Color.fromRGBO(35, 39, 43, 1),
      ),
      resizeToAvoidBottomInset: false,
      body: Center(
        child: SizedBox(
          width: 520,
          height: 418,
          child: QrLoginCard(
            authUrl: controller.authUrl,
            loading: controller.status == LoginStatus.loading,
            provisioning: controller.status == LoginStatus.provisioning,
            error: controller.status == LoginStatus.failed
                ? controller.error ?? 'QR code expired. Please try again.'
                : null,
            onRetry: controller.refresh,
          ),
        ),
      ),
    );
  }
}
