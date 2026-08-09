import 'package:flutter/material.dart';

import '../../l10n/l10n.dart';
import '../../theme.dart';
import '../app_protocol_handler.dart';
import '../message_style.dart';

class SecretMessageItem extends StatelessWidget {
  const SecretMessageItem({super.key});

  @override
  Widget build(BuildContext context) {
    final uri = Uri.tryParse(context.l10n.secretUrl);
    return Center(
      child: Padding(
        padding: const EdgeInsets.only(left: 8, right: 8, bottom: 4),
        child: MouseRegion(
          cursor: SystemMouseCursors.click,
          child: GestureDetector(
            onTap: uri == null ? null : () => context.openUri(uri),
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: context.theme.encrypt,
                borderRadius: const BorderRadius.all(Radius.circular(10)),
              ),
              child: Padding(
                padding: const EdgeInsets.all(10),
                child: Text(
                  context.l10n.messageE2ee,
                  style: TextStyle(
                    fontSize: context.messageStyle.secondaryFontSize,
                    color: context.dynamicColor(
                      const Color.fromRGBO(0, 0, 0, 1),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
