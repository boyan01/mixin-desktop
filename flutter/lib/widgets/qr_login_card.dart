import 'package:flutter/material.dart';
import 'package:pretty_qr_code/pretty_qr_code.dart';

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
  Widget build(BuildContext context) => Material(
    color: Colors.white,
    elevation: 10,
    shadowColor: Colors.black26,
    borderRadius: const BorderRadius.all(Radius.circular(13)),
    clipBehavior: Clip.antiAlias,
    child: Padding(
      padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 38),
      child: AnimatedSwitcher(
        duration: const Duration(milliseconds: 180),
        child: loading || provisioning
            ? _LoginProgress(provisioning: provisioning)
            : _QrContent(authUrl: authUrl, error: error, onRetry: onRetry),
      ),
    ),
  );
}

class _LoginProgress extends StatelessWidget {
  const _LoginProgress({required this.provisioning});

  final bool provisioning;

  @override
  Widget build(BuildContext context) => SizedBox(
    height: 342,
    child: Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        const CircularProgressIndicator(strokeWidth: 2),
        const SizedBox(height: 24),
        Text(
          provisioning ? 'Loading' : 'Initializing',
          style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
        ),
        const SizedBox(height: 8),
        const Text(
          'Messages are end-to-end encrypted.',
          style: TextStyle(fontSize: 16, color: Color(0xFFBBBEC3)),
        ),
      ],
    ),
  );
}

class _QrContent extends StatelessWidget {
  const _QrContent({
    required this.authUrl,
    required this.error,
    required this.onRetry,
  });

  final String? authUrl;
  final String? error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    children: [
      SizedBox.square(
        dimension: 160,
        child: ClipRRect(
          borderRadius: const BorderRadius.all(Radius.circular(11)),
          child: Stack(
            fit: StackFit.expand,
            children: [
              if (authUrl case final url?)
                ColoredBox(
                  color: Colors.white,
                  child: Padding(
                    padding: const EdgeInsets.all(4),
                    child: PrettyQrView.data(
                      data: url,
                      errorCorrectLevel: QrErrorCorrectLevel.Q,
                    ),
                  ),
                ),
              if (error != null)
                Material(
                  color: Colors.black.withValues(alpha: 0.86),
                  child: InkWell(
                    onTap: onRetry,
                    child: const Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(Icons.refresh, color: Colors.white, size: 40),
                        SizedBox(height: 12),
                        Text(
                          'Click to reload the QR code',
                          textAlign: TextAlign.center,
                          style: TextStyle(color: Colors.white, fontSize: 14),
                        ),
                      ],
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
      const SizedBox(height: 16),
      const Text(
        'Log in to Mixin Messenger with a QR code',
        textAlign: TextAlign.center,
        style: TextStyle(fontSize: 16),
      ),
      const SizedBox(height: 16),
      const Text(
        '1. Open Mixin Messenger on your phone.\n'
        '2. Scan the QR code on the screen and confirm your sign-in.',
        style: TextStyle(fontSize: 14, height: 1.6, color: Color(0xFFBBBEC3)),
      ),
    ],
  );
}
