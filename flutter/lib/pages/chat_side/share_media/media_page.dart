import 'dart:io';

import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/pages/chat_side/chat_side_scope.dart';
import 'package:mixin_desktop_ui/widgets/message_media_preview_pages.dart';
import 'package:mixin_desktop_ui/utils/system_clipboard.dart';
import 'package:mixin_desktop_ui/widgets/mixin_image.dart';

import '../shared_media_page.dart';
import 'shared_media_list.dart';

class MediaPage extends StatelessWidget {
  const MediaPage({super.key});

  @override
  Widget build(BuildContext context) {
    final routeMode = ChatSideScope.of(context).routeMode;
    final columnCount = routeMode ? 4 : 3;
    return LayoutBuilder(
      builder: (context, constraints) => SharedMediaList(
        kind: 'media',
        pageSize: (constraints.maxHeight / 90 * 2).toInt() * columnCount,
        emptyAsset: MixinAssets.emptyImage,
        emptyText: context.l10n.noMedia,
        gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
          crossAxisCount: columnCount,
          mainAxisSpacing: 5,
          crossAxisSpacing: 5,
        ),
        itemBuilder: (context, message, messages) =>
            _MediaItem(message: message, messages: messages),
      ),
    );
  }
}

class _MediaItem extends StatelessWidget {
  const _MediaItem({required this.message, required this.messages});

  final MessageListEntry message;
  final List<MessageListEntry> messages;

  Future<void> _open(BuildContext context) async {
    try {
      final status = message.mediaStatus.toUpperCase();
      if (status == 'CANCELED') {
        await ChatSideScope.of(
          context,
        ).account.attachment().downloadAttachment(messageId: message.id);
        return;
      }
      if (status == 'PENDING') {
        await ChatSideScope.of(
          context,
        ).account.attachment().cancelAttachment(messageId: message.id);
        return;
      }
      final source = message.mediaUrl?.trim() ?? '';
      if (source.isEmpty) return;
      if (message.isVideo) {
        await Navigator.of(context, rootNavigator: true).push(
          MaterialPageRoute<void>(
            builder: (_) =>
                VideoPreviewPage(source: source, title: message.mediaName),
          ),
        );
        return;
      }
      final images = messages
          .where((item) => item.isImage && (item.mediaUrl?.isNotEmpty ?? false))
          .map(
            (item) => ImagePreviewEntry(
              id: item.id,
              source: item.mediaUrl!,
              name: item.mediaName,
            ),
          )
          .toList();
      final index = images.indexWhere((item) => item.id == message.id);
      if (index < 0) return;
      await Navigator.of(context, rootNavigator: true).push(
        MaterialPageRoute<void>(
          builder: (_) => ImagePreviewPage(
            images: images,
            initialIndex: index,
            onSave: (image) =>
                saveMessageFileAs(image.source, suggestedName: image.name),
          ),
        ),
      );
    } on Object catch (error) {
      if (context.mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(error.toString())));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final source = message.thumbUrl ?? message.mediaUrl ?? '';
    final file = existingLocalFile(source);
    final image = source.isEmpty
        ? const ColoredBox(
            color: Color(0xFFE8E8E8),
            child: Icon(Icons.broken_image_outlined),
          )
        : file != null
        ? MixinImage.file(File(file.path), fit: BoxFit.cover)
        : MixinImage.network(source, fit: BoxFit.cover);
    return ShareMediaItemMenuWrapper(
      messageId: message.id,
      child: InkWell(
        onTap: () => _open(context),
        child: Stack(
          fit: StackFit.expand,
          children: [
            image,
            if (message.isVideo)
              const Center(
                child: Icon(
                  Icons.play_circle_fill,
                  color: Colors.white,
                  size: 34,
                ),
              ),
            if (message.mediaStatus.toUpperCase() == 'PENDING')
              const Center(child: CircularProgressIndicator()),
            if (message.mediaStatus.toUpperCase() == 'CANCELED')
              const Center(
                child: Icon(Icons.download, color: Colors.white, size: 32),
              ),
          ],
        ),
      ),
    );
  }
}
