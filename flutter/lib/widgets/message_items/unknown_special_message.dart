import 'package:flutter/material.dart';

import '../../l10n/l10n.dart';
import '../../theme.dart';

class UnknownSpecialMessage extends StatelessWidget {
  const UnknownSpecialMessage({required this.category, super.key});

  final String category;

  @override
  Widget build(BuildContext context) => Text(
    context.l10n.messageNotSupport,
    style: TextStyle(color: context.theme.text, fontSize: 16),
  );
}
