import 'package:flutter/material.dart';

import '../theme.dart';
import '../widgets/conversation_list_view.dart';

class ConversationListPane extends StatelessWidget {
  const ConversationListPane({super.key});

  @override
  Widget build(BuildContext context) => RepaintBoundary(
    child: DecoratedBox(
      decoration: BoxDecoration(
        color: context.mixinTheme.primary,
        border: Border(right: BorderSide(color: context.mixinTheme.divider)),
      ),
      child: const ConversationListView(),
    ),
  );
}
