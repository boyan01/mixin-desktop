import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:pretty_qr_code/pretty_qr_code.dart';

import '../constants/assets.dart';
import '../l10n/generated/app_localizations.dart';
import '../theme.dart';

class QrLoginCard extends StatelessWidget {
  const QrLoginCard({
    required this.authUrl,
    required this.loading,
    required this.provisioning,
    required this.onRetry,
    super.key,
    this.error,
  });

  final String? authUrl;
  final bool loading;
  final bool provisioning;
  final String? error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);

    Widget child;
    if (loading) {
      child = Center(
        child: _Loading(
          title: l10n?.initializing ?? 'Initializing…',
          message: l10n?.chatHintE2e ?? 'End-to-end encrypted',
        ),
      );
    } else if (provisioning) {
      child = Center(
        child: _Loading(
          title: l10n?.loading ?? 'Loading...',
          message: l10n?.chatHintE2e ?? 'End-to-end encrypted',
        ),
      );
    } else {
      child = Stack(
        fit: StackFit.expand,
        children: [
          Column(
            children: [
              const Spacer(),
              _QrContent(
                authUrl: authUrl,
                error: error,
                onRetry: onRetry,
                title:
                    l10n?.loginByQrcode ??
                    'Log in to Mixin Messenger with a QR code',
                firstTip:
                    l10n?.loginByQrcodeTips1 ??
                    'Open Mixin Messenger on your phone.',
                secondTip:
                    l10n?.loginByQrcodeTips2 ??
                    'Scan the QR code on the screen and confirm your sign-in.',
                retryText:
                    l10n?.clickToReloadQrcode ?? 'Click to reload the QR code',
              ),
              const Spacer(),
            ],
          ),
        ],
      );
    }

    return child;
  }
}

class _QrContent extends StatelessWidget {
  const _QrContent({
    required this.authUrl,
    required this.error,
    required this.onRetry,
    required this.title,
    required this.firstTip,
    required this.secondTip,
    required this.retryText,
  });

  final String? authUrl;
  final String? error;
  final VoidCallback onRetry;
  final String title;
  final String firstTip;
  final String secondTip;
  final String retryText;

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    children: [
      ClipRRect(
        borderRadius: const BorderRadius.all(Radius.circular(11)),
        clipBehavior: Clip.antiAliasWithSaveLayer,
        child: SizedBox.square(
          dimension: 160,
          child: Stack(
            fit: StackFit.expand,
            children: [
              if (authUrl == null)
                const SizedBox()
              else
                Container(
                  color: Colors.white,
                  padding: const EdgeInsets.all(8),
                  child: PrettyQrView.data(
                    data: authUrl!,
                    errorCorrectLevel: QrErrorCorrectLevel.Q,
                    decoration: const PrettyQrDecoration(
                      image: PrettyQrDecorationImage(
                        image: AssetImage(MixinAssets.logo),
                      ),
                    ),
                  ),
                ),
              Visibility(
                visible: error != null,
                child: DecoratedBox(
                  decoration: const BoxDecoration(
                    color: Color.fromRGBO(0, 0, 0, 0.86),
                  ),
                  child: GestureDetector(
                    onTap: onRetry,
                    behavior: HitTestBehavior.opaque,
                    child: Tooltip(
                      message: error ?? '',
                      excludeFromSemantics: true,
                      child: Center(
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            SvgPicture.asset(
                              MixinAssets.retry,
                              width: 40,
                              height: 40,
                            ),
                            const SizedBox(height: 14),
                            Padding(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 8,
                              ),
                              child: Text(
                                retryText,
                                textAlign: TextAlign.center,
                                style: const TextStyle(
                                  color: Color.fromRGBO(255, 255, 255, 0.9),
                                  fontSize: 14,
                                ),
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
          ),
        ),
      ),
      const SizedBox(height: 16),
      Text(
        title,
        style: TextStyle(fontSize: 16, color: context.mixinTheme.text),
      ),
      const SizedBox(height: 16),
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 20),
        child: DefaultTextStyle.merge(
          style: TextStyle(
            fontSize: 14,
            color: context.dynamicColor(
              const Color.fromRGBO(187, 190, 195, 1),
              darkColor: const Color.fromRGBO(255, 255, 255, 0.4),
            ),
          ),
          textAlign: TextAlign.left,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('1. $firstTip'),
              const SizedBox(height: 4),
              Text('2. $secondTip'),
            ],
          ),
        ),
      ),
    ],
  );
}

class _Loading extends StatelessWidget {
  const _Loading({required this.title, required this.message});

  final String title;
  final String message;

  @override
  Widget build(BuildContext context) {
    final primaryColor = context.mixinTheme.text;
    return SizedBox(
      width: 375,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          CircularProgressIndicator(
            valueColor: AlwaysStoppedAnimation(primaryColor),
          ),
          const SizedBox(height: 24),
          Text(
            title,
            style: TextStyle(
              color: primaryColor,
              fontSize: 18,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            message,
            textAlign: TextAlign.center,
            style: TextStyle(
              color: context.dynamicColor(
                const Color.fromRGBO(188, 190, 195, 1),
                darkColor: const Color.fromRGBO(255, 255, 255, 0.4),
              ),
              fontSize: 16,
            ),
          ),
        ],
      ),
    );
  }
}
