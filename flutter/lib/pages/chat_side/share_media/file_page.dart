import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/pages/chat_side/chat_side_scope.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/message_media_preview_pages.dart';

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

  String get size {
    final bytes = message.mediaSize;
    if (bytes == null || bytes <= 0) return '';
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    return '${(bytes / 1024 / 1024).toStringAsFixed(1)} MB';
  }

  @override
  Widget build(BuildContext context) => ShareMediaItemMenuWrapper(
    messageId: message.id,
    child: ListTile(
      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      leading: Icon(Icons.insert_drive_file, color: context.theme.accent),
      title: Text(
        message.mediaName ?? message.content,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: size.isEmpty ? null : Text(size),
      trailing: switch (message.mediaStatus.toUpperCase()) {
        'PENDING' => const SizedBox.square(
          dimension: 20,
          child: CircularProgressIndicator(strokeWidth: 2),
        ),
        'CANCELED' => const Icon(Icons.download),
        _ => null,
      },
      onTap: () async {
        try {
          final status = message.mediaStatus.toUpperCase();
          final account = ChatSideScope.of(context).account;
          if (status == 'CANCELED') {
            await account.downloadAttachment(messageId: message.id);
            return;
          }
          if (status == 'PENDING') {
            await account.cancelAttachment(messageId: message.id);
            return;
          }
          final source = message.mediaUrl?.trim() ?? '';
          if (source.isEmpty) return;
          final result = await openMessageFile(source);
          if (!context.mounted || result.type.name == 'done') return;
          ScaffoldMessenger.of(
            context,
          ).showSnackBar(SnackBar(content: Text(result.message)));
        } on Object catch (error) {
          if (context.mounted) {
            ScaffoldMessenger.of(
              context,
            ).showSnackBar(SnackBar(content: Text(error.toString())));
          }
        }
      },
    ),
  );
}
