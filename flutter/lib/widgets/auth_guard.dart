import 'dart:async';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/controllers/security_controller.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:pin_code_fields/pin_code_fields.dart';
import 'package:provider/provider.dart';

class AuthGuard extends StatefulWidget {
  const AuthGuard({required this.child, super.key});

  final Widget child;

  @override
  State<AuthGuard> createState() => _AuthGuardState();
}

class _AuthGuardState extends State<AuthGuard> with WidgetsBindingObserver {
  final focusNode = FocusNode();
  final textEditingController = TextEditingController();
  Timer? timer;
  bool locked = false;
  bool hasError = false;
  int lockRevision = 0;

  SecurityController get security => context.read<SecurityController>();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    focusNode.addListener(_ensureFocus);
    FocusManager.instance.addListener(_ensureFocus);
    ServicesBinding.instance.keyboard.addHandler(_keyboardHandler);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      setState(() => locked = security.hasPasscode);
      lockRevision = security.lockRevision;
      if (locked) focusNode.requestFocus();
    });
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    timer?.cancel();
    timer = null;
    if (state == AppLifecycleState.resumed) {
      if (locked) focusNode.requestFocus();
      return;
    }
    if (locked || !security.hasPasscode) return;
    final duration = security.lockDuration;
    if (duration.inMinutes == 0) return;
    timer = Timer(duration, () {
      if (!mounted || !security.hasPasscode) return;
      setState(() => locked = true);
    });
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    focusNode.removeListener(_ensureFocus);
    FocusManager.instance.removeListener(_ensureFocus);
    ServicesBinding.instance.keyboard.removeHandler(_keyboardHandler);
    timer?.cancel();
    focusNode.dispose();
    textEditingController.dispose();
    super.dispose();
  }

  void _ensureFocus() {
    if (locked && !focusNode.hasFocus) focusNode.requestFocus();
  }

  bool _keyboardHandler(KeyEvent _) {
    _ensureFocus();
    return false;
  }

  void _verify(String value) {
    textEditingController.clear();
    if (security.verify(value)) {
      setState(() {
        locked = false;
        hasError = false;
      });
    } else {
      setState(() => hasError = true);
    }
  }

  @override
  Widget build(BuildContext context) {
    context.watch<SecurityController>();
    if (security.lockRevision != lockRevision) {
      lockRevision = security.lockRevision;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted || locked) return;
        setState(() => locked = true);
        focusNode.requestFocus();
      });
    }
    return Stack(
      children: [
        widget.child,
        if (locked && security.hasPasscode)
          GestureDetector(
            onTap: focusNode.requestFocus,
            behavior: HitTestBehavior.translucent,
            child: BackdropFilter(
              filter: ImageFilter.blur(sigmaX: 20, sigmaY: 20),
              child: MaterialApp(
                color: Colors.transparent,
                home: Material(
                  color: Colors.transparent,
                  child: Center(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        SvgPicture.asset(
                          MixinAssets.lock,
                          width: 68,
                          height: 68,
                          colorFilter: ColorFilter.mode(
                            context.mixinTheme.icon,
                            BlendMode.srcIn,
                          ),
                        ),
                        const SizedBox(height: 24),
                        Text(
                          context.l10n.unlockWithWasscode,
                          textAlign: TextAlign.center,
                          style: TextStyle(
                            color: context.mixinTheme.text,
                            fontSize: 20,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                        const SizedBox(height: 40),
                        SizedBox(
                          width: 204,
                          child: PinCodeTextField(
                            appContext: context,
                            length: 6,
                            controller: textEditingController,
                            inputFormatters: [
                              FilteringTextInputFormatter.digitsOnly,
                            ],
                            pinTheme: PinTheme(
                              activeColor: context.mixinTheme.text,
                              inactiveColor: context.mixinTheme.text,
                              selectedColor: context.mixinTheme.text,
                              fieldWidth: 15,
                              borderWidth: 1,
                              shape: PinCodeFieldShape.circle,
                            ),
                            obscureText: true,
                            obscuringWidget: Container(
                              decoration: BoxDecoration(
                                color: context.mixinTheme.text,
                                shape: BoxShape.circle,
                              ),
                            ),
                            autoDisposeControllers: false,
                            autoFocus: true,
                            focusNode: focusNode,
                            showCursor: false,
                            onCompleted: _verify,
                            onChanged: (_) {
                              if (hasError) setState(() => hasError = false);
                            },
                          ),
                        ),
                        const SizedBox(height: 28),
                        Visibility(
                          visible: hasError,
                          maintainSize: true,
                          maintainAnimation: true,
                          maintainState: true,
                          child: Text(
                            context.l10n.passcodeIncorrect,
                            textAlign: TextAlign.center,
                            style: TextStyle(
                              color: context.mixinTheme.red,
                              fontSize: 16,
                            ),
                          ),
                        ),
                        if (security.biometric)
                          Padding(
                            padding: const EdgeInsets.only(top: 12),
                            child: TextButton(
                              onPressed: () async {
                                if (await security.authenticate(
                                  context.l10n.unlockMixinMessenger,
                                )) {
                                  setState(() => locked = false);
                                }
                              },
                              style: TextButton.styleFrom(
                                padding: const EdgeInsets.all(24),
                              ),
                              child: Text(context.l10n.useBiometric),
                            ),
                          ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
      ],
    );
  }
}
