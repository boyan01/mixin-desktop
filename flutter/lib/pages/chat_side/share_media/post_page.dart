import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/message_media_preview_pages.dart';

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
      child: InkWell(
        onTap: () => Navigator.of(context, rootNavigator: true).push(
          MaterialPageRoute<void>(
            builder: (_) => PostPreviewPage(
              content: message.content,
              title: message.senderName,
            ),
          ),
        ),
        child: Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: context.theme.sidebarSelected,
            borderRadius: BorderRadius.circular(8),
          ),
          child: Text(
            message.content,
            maxLines: 6,
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ),
    ),
  );
}
