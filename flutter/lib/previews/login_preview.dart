import 'package:flutter/material.dart';
import 'package:flutter/widget_previews.dart';
import '../widgets/qr_login_card.dart';

@Preview(name: 'QR Login', group: 'Authentication', size: Size(560, 460))
Widget qrLoginPreview() => const MaterialApp(
  home: Scaffold(
    backgroundColor: Color(0xFFE5E5E5),
    body: Center(
      child: SizedBox(
        width: 520,
        child: QrLoginCard(
          authUrl: 'mixin://device/auth?id=preview&pub_key=preview',
          loading: false,
          provisioning: false,
          onRetry: _noop,
        ),
      ),
    ),
  ),
);

void _noop() {}
