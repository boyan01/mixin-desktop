import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:pretty_qr_code/pretty_qr_code.dart';

Future<void> showMessageQrDialog(
  BuildContext context, {
  required String content,
}) => showDialog<void>(
  context: context,
  builder: (context) => Dialog(
    child: ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 360),
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              context.l10n.generateQrcode,
              style: TextStyle(
                color: context.theme.text,
                fontSize: 18,
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 20),
            Container(
              width: 260,
              height: 260,
              padding: const EdgeInsets.all(12),
              color: Colors.white,
              child: PrettyQrView.data(data: content),
            ),
            const SizedBox(height: 16),
            Text(
              content,
              maxLines: 3,
              overflow: TextOverflow.ellipsis,
              textAlign: TextAlign.center,
              style: TextStyle(
                color: context.theme.secondaryText,
                fontSize: 13,
              ),
            ),
            const SizedBox(height: 16),
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: Text(context.l10n.close),
            ),
          ],
        ),
      ),
    ),
  ),
);
