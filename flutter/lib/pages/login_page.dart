import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/controllers/login_controller.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart';
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
      backgroundColor: const Color(0xFFE5E5E5),
      body: LayoutBuilder(
        builder: (context, constraints) => Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(16),
            child: ConstrainedBox(
              constraints: BoxConstraints(
                maxWidth: 520,
                minHeight: constraints.maxHeight > 450 ? 418 : 0,
              ),
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
        ),
      ),
    );
  }
}
