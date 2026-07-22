import 'package:flutter/material.dart';

import '../../../constants/assets.dart';
import '../../../l10n/l10n.dart';
import '../../../models/message_list_entry.dart';
import '../../../widgets/message_content.dart';
import '../../../widgets/message_media_preview_pages.dart';
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
  Widget build(BuildContext context) => ShareMediaItemMenuWrapper(
    messageId: message.id,
    child: Padding(
      padding: const EdgeInsets.symmetric(vertical: 8, horizontal: 16),
      child: MessageFile(
        message: message,
        onOpen: (_) => _open(context),
        onDownloadAttachment: (_) => ChatSideScope.of(
          context,
        ).account.attachment().downloadAttachment(messageId: message.id),
        onCancelAttachment: (_) => ChatSideScope.of(
          context,
        ).account.attachment().cancelAttachment(messageId: message.id),
      ),
    ),
  );

  Future<void> _open(BuildContext context) async {
    final source = message.mediaUrl?.trim() ?? '';
    if (source.isEmpty) return;
    await openOrSaveMessageFile(context, source, mediaName: message.mediaName);
  }
}
