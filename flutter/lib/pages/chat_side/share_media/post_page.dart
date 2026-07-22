import 'package:flutter/material.dart';
import '../../../constants/assets.dart';
import '../../../l10n/l10n.dart';
import '../../../models/message_list_entry.dart';
import '../../../theme.dart';
import '../../../widgets/message_content.dart';
import '../../../widgets/message_media_preview_pages.dart';

import '../shared_media_page.dart';
import 'shared_media_list.dart';

class PostPage extends StatelessWidget {
  const PostPage({super.key});

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) => SharedMediaList(
      kind: 'post',
      pageSize: (constraints.maxHeight / 90 * 2).toInt(),
      emptyAsset: MixinAssets.empty,
      emptyText: context.l10n.noPosts,
      itemBuilder: (context, message, _) => _PostItem(message: message),
    ),
  );
}

class _PostItem extends StatelessWidget {
  const _PostItem({required this.message});

  final MessageListEntry message;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(10, 0, 10, 10),
    child: ShareMediaItemMenuWrapper(
      messageId: message.id,
      child: MessagePost(
        message: message,
        overlayDateAndStatus: const SizedBox(),
        showStatus: false,
        padding: const EdgeInsets.all(8),
        decoration: BoxDecoration(
          color: context.theme.sidebarSelected,
          borderRadius: const BorderRadius.all(Radius.circular(8)),
        ),
        onOpen: (_) => PostPreviewPage.show(
          context,
          PostPreviewPage(content: message.content, title: message.senderName),
        ),
      ),
    ),
  );
}
