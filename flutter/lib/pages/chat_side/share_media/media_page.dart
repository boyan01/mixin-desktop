import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:provider/provider.dart';

import '../../../constants/assets.dart';
import '../../../controllers/message_attachment_controller.dart';
import '../../../l10n/l10n.dart';
import '../../../models/message_list_entry.dart';
import '../../../widgets/message_content.dart';
import '../chat_side_scope.dart';
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
        messages: messages,
      );
    } else {
      child = MessageVideo(
        message: message,
        isCurrentUser: isCurrentUser,
        sentByCurrentUser: isCurrentUser,
        overlayDateAndStatus: const SizedBox(),
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
    return Provider<MessageAttachmentController>(
      create: (_) => MessageAttachmentController.conversation(
        account: account,
        onChanged: (_) async {},
      ),
      child: ShareMediaItemMenuWrapper(messageId: message.id, child: child),
    );
  }
}
