import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../../constants/assets.dart';
import '../../../controllers/message_attachment_controller.dart';
import '../../../l10n/l10n.dart';
import '../../../models/message_list_entry.dart';
import '../../../widgets/message_content.dart';
import '../chat_side_scope.dart';
import '../shared_media_page.dart';
import 'shared_media_list.dart';

class FilePage extends StatelessWidget {
  const FilePage({super.key});

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) => SharedMediaList(
      kind: 'file',
      pageSize: (constraints.maxHeight / 90 * 2).toInt(),
      emptyAsset: MixinAssets.empty,
      emptyText: context.l10n.noFiles,
      itemBuilder: (context, message, _) => _FileItem(message: message),
    ),
  );
}

class _FileItem extends StatelessWidget {
  const _FileItem({required this.message});

  final MessageListEntry message;

  @override
  Widget build(BuildContext context) {
    final account = ChatSideScope.of(context).account;
    return Provider<MessageAttachmentController>(
      create: (_) => MessageAttachmentController.conversation(
        account: account,
        onChanged: (_) async {},
      ),
      child: ShareMediaItemMenuWrapper(
        messageId: message.id,
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 8, horizontal: 16),
          child: MessageFile(message: message),
        ),
      ),
    );
  }
}
