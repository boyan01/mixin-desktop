import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:pretty_qr_code/pretty_qr_code.dart';
import 'package:mixin_desktop_ui/widgets/mixin_dialog.dart';

Future<void> showUnknownMixinUrlDialog(BuildContext context, Uri uri) =>
    showMixinDialog<void>(
      context: context,
      child: _UnknownMixinUri(uri: uri),
    );

class _UnknownMixinUri extends StatelessWidget {
  const _UnknownMixinUri({required this.uri});

  final Uri uri;

  @override
  Widget build(BuildContext context) => Material(
    color: Colors.transparent,
    child: Container(
      constraints: const BoxConstraints(minWidth: 320, minHeight: 210),
      padding: const EdgeInsets.symmetric(horizontal: 20),
      child: IntrinsicWidth(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const SizedBox(height: 36),
            Text(
              context.l10n.chatNotSupportUriOnPhone,
              style: TextStyle(fontSize: 16, color: context.mixinTheme.text),
            ),
            const SizedBox(height: 36),
            ClipRRect(
              borderRadius: const BorderRadius.all(Radius.circular(8)),
              child: Container(
                width: 240,
                height: 240,
                color: Colors.white,
                padding: const EdgeInsets.all(8),
                child: PrettyQrView.data(
                  errorCorrectLevel: QrErrorCorrectLevel.Q,
                  data: uri.toString(),
                ),
              ),
            ),
            const SizedBox(height: 36),
            MixinButton(
              onTap: () => Navigator.pop(context, true),
              child: Text(context.l10n.confirm),
            ),
            const SizedBox(height: 36),
          ],
        ),
      ),
    ),
  );
}
