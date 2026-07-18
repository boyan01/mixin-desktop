import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/pages/chat_side/chat_side_scope.dart';
import 'package:mixin_desktop_ui/utils/app_logger.dart';
import 'package:mixin_desktop_ui/widgets/message_media_preview_pages.dart';
import 'package:mixin_desktop_ui/widgets/message_content.dart';

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
      final source = message.mediaUrl?.trim() ?? '';
      if (source.isEmpty) return;
      if (message.isVideo) {
        await VideoPreviewPage.show(
          context,
          VideoPreviewPage(
            source: source,
            title: message.mediaName,
            userId: message.senderId,
            userFullName: message.senderName,
            userIdentityNumber: message.senderIdentityNumber,
            avatarUrl: message.senderAvatarUrl,
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
              thumbImage: item.thumbImage,
              userId: item.senderId,
              userFullName: item.senderName,
              userIdentityNumber: item.senderIdentityNumber,
              avatarUrl: item.senderAvatarUrl,
            ),
          )
          .toList();
      final index = images.indexWhere((item) => item.id == message.id);
      if (index < 0) return;
      await ImagePreviewPage.show(
        context,
        ImagePreviewPage(
          images: images,
          initialIndex: index,
          onSave: (image) =>
              saveMessageFileAs(image.source, suggestedName: image.name),
        ),
      );
    } on Object catch (error) {
      writeAppLog('open shared media failed: $error');
    }
  }

  @override
  Widget build(BuildContext context) {
    final isCurrentUser = message.senderRelationship.toUpperCase() == 'ME';
    final account = ChatSideScope.of(context).account;
    final Widget child;
    if (message.isImage) {
      child = MessageImage(
        message: message,
        showStatus: false,
        isCurrentUser: isCurrentUser,
        sentByCurrentUser: isCurrentUser,
        overlayDateAndStatus: const SizedBox(),
        onOpen: (_) => _open(context),
        onDownloadAttachment: (_) => isCurrentUser
            ? account.attachment().retryAttachment(messageId: message.id)
            : account.attachment().downloadAttachment(messageId: message.id),
        onCancelAttachment: (_) =>
            account.attachment().cancelAttachment(messageId: message.id),
      );
    } else {
      child = MessageVideo(
        message: message,
        isCurrentUser: isCurrentUser,
        sentByCurrentUser: isCurrentUser,
        overlayDateAndStatus: const SizedBox(),
        onOpen: (_) => _open(context),
        onDownloadAttachment: (_) => isCurrentUser
            ? account.attachment().retryAttachment(messageId: message.id)
            : account.attachment().downloadAttachment(messageId: message.id),
        onCancelAttachment: (_) =>
            account.attachment().cancelAttachment(messageId: message.id),
        overlay: Stack(
          fit: StackFit.expand,
          children: [
            Center(
              child: MediaStatusOverlay(
                messageId: message.id,
                status: message.mediaStatus,
                upload: isCurrentUser && message.mediaUrl?.isNotEmpty == true,
                done: const SizedBox(),
              ),
            ),
            Positioned(
              left: 0,
              bottom: 0,
              right: 0,
              height: 20,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  gradient: LinearGradient(
                    begin: Alignment.bottomCenter,
                    end: Alignment.topCenter,
                    colors: [
                      Colors.black.withValues(alpha: 0.5),
                      Colors.transparent,
                    ],
                  ),
                ),
                child: Row(
                  children: [
                    const SizedBox(width: 5),
                    SvgPicture.asset(MixinAssets.videoMessage),
                    const SizedBox(width: 8),
                    Text(
                      formatMessageDuration(message.mediaDuration),
                      textAlign: TextAlign.center,
                      style: const TextStyle(
                        fontSize: 12,
                        height: 1,
                        color: Color(0xFFFFFFFF),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      );
    }
    return ShareMediaItemMenuWrapper(messageId: message.id, child: child);
  }
}
