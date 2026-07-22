import 'package:flutter/material.dart';
import 'package:pretty_qr_code/pretty_qr_code.dart';

import '../l10n/l10n.dart';
import 'mixin_dialog.dart';

Future<void> showMessageQrDialog(
  BuildContext context, {
  required String content,
}) => showMixinDialog<void>(
  context: context,
  child: Material(
    color: Colors.transparent,
    child: Container(
      constraints: const BoxConstraints(minWidth: 320, minHeight: 210),
      padding: const EdgeInsets.symmetric(horizontal: 20),
      child: IntrinsicWidth(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const SizedBox(height: 36),
            Container(
              width: 240,
              height: 240,
              color: Colors.white,
              padding: const EdgeInsets.all(8),
              child: PrettyQrView.data(
                data: content,
                errorCorrectLevel: QrErrorCorrectLevel.Q,
              ),
            ),
            const SizedBox(height: 20),
            MixinButton(
              onTap: () => Navigator.pop(context, true),
              child: Text(context.l10n.confirm),
            ),
            const SizedBox(height: 20),
          ],
        ),
      ),
    ),
  ),
);
