import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:image/image.dart' as image;
import 'package:markdown_widget/markdown_widget.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/controllers/conversation_list_controller.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/network/core_http_scope.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/buttons.dart';
import 'package:mixin_desktop_ui/widgets/message_bubble.dart';
import 'package:mixin_desktop_ui/widgets/message_items/special_message_items.dart';
import 'package:mixin_desktop_ui/widgets/message_style.dart';
import 'package:mixin_desktop_ui/widgets/mixin_dialog.dart';
import 'package:mixin_desktop_ui/widgets/mixin_image.dart';
import 'package:mixin_desktop_ui/widgets/post_markdown.dart';
import 'package:mixin_desktop_ui/widgets/settings_widgets.dart';
import 'package:mixin_desktop_ui/widgets/show_forward_conversation_selector.dart';
import 'package:mixin_desktop_ui/widgets/sticker_page/sticker_item.dart';
import 'package:path_provider/path_provider.dart';
import 'package:provider/provider.dart';
import 'package:uuid/uuid.dart';

enum _SendCategory { text, image, sticker, contact, post, appCard }

extension on String {
  _SendCategory? get sendCategory => switch (toLowerCase()) {
    'text' => _SendCategory.text,
    'image' => _SendCategory.image,
    'sticker' => _SendCategory.sticker,
    'contact' => _SendCategory.contact,
    'post' => _SendCategory.post,
    'app_card' => _SendCategory.appCard,
    _ => null,
  };
}

final class _SendPayload {
  const _SendPayload({required this.category, required this.data});

  final _SendCategory category;
  final Object data;

  static _SendPayload? parse(String? category, String? encoded) {
    final parsedCategory = category?.sendCategory;
    if (parsedCategory == null || encoded == null || encoded.isEmpty) {
      return null;
    }
    try {
      final decoded = utf8.decode(base64.decode(base64.normalize(encoded)));
      final Object data = switch (parsedCategory) {
        _SendCategory.image =>
          (jsonDecode(decoded) as Map<String, dynamic>)['url'] as String,
        _SendCategory.contact =>
          (jsonDecode(decoded) as Map<String, dynamic>)['user_id'] as String,
        _SendCategory.appCard => jsonEncode(
          jsonDecode(decoded) as Map<String, dynamic>,
        ),
        _ => decoded,
      };
      if (data is String && data.isEmpty) return null;
      if (parsedCategory == _SendCategory.sticker &&
          !Uuid.isValidUUID(fromString: data as String)) {
        return null;
      }
      if (parsedCategory == _SendCategory.contact &&
          !Uuid.isValidUUID(fromString: data as String)) {
        return null;
      }
      return _SendPayload(category: parsedCategory, data: data);
    } on Object {
      return null;
    }
  }
}

Future<bool> showSendMessageDialog(
  BuildContext context, {
  required rust.AccountHandle account,
  required String? category,
  required String? conversationId,
  required String? data,
  required String? userId,
  required ConversationListEntry? currentConversation,
  required ValueChanged<ConversationListEntry> onSelectConversation,
}) async {
  final payload = _SendPayload.parse(category, data);
  if (payload == null) return false;

  if (userId?.isNotEmpty == true) {
    if (!Uuid.isValidUUID(fromString: userId!)) return false;
    try {
      final targetId = await account.conversation().openUserConversation(
        userId: userId,
      );
      if (!context.mounted) return false;
      final controller = context.read<ConversationListController>();
      await controller.refresh();
      final conversation = await controller.findConversation(targetId);
      if (!context.mounted || conversation == null) return false;
      onSelectConversation(conversation);
      await _sendPayload(context, account, targetId, payload);
      return true;
    } on Object {
      return false;
    }
  }

  if (conversationId == null &&
      payload.category == _SendCategory.text &&
      currentConversation != null) {
    await _sendPayload(context, account, currentConversation.id, payload);
    return true;
  }

  await showMixinDialog<void>(
    context: context,
    child: _SendPage(
      account: account,
      payload: payload,
      conversationId: conversationId,
    ),
  );
  return true;
}

class _SendPage extends StatefulWidget {
  const _SendPage({
    required this.account,
    required this.payload,
    required this.conversationId,
  });

  final rust.AccountHandle account;
  final _SendPayload payload;
  final String? conversationId;

  @override
  State<_SendPage> createState() => _SendPageState();
}

class _SendPageState extends State<_SendPage> {
  var _sending = false;

  Future<void> _send() async {
    if (_sending) return;
    var conversationId = widget.conversationId;
    if (conversationId == null) {
      final conversation = await showConversationSelector(
        context,
        account: widget.account,
        title: context.l10n.forward,
      );
      conversationId = conversation?.id;
    } else {
      final conversation = await context
          .read<ConversationListController>()
          .findConversation(conversationId);
      if (conversation == null) return;
    }
    if (!mounted || conversationId == null) return;
    _sending = true;
    try {
      await _sendPayload(
        context,
        widget.account,
        conversationId,
        widget.payload,
      );
      if (mounted) Navigator.pop(context);
    } finally {
      _sending = false;
    }
  }

  @override
  Widget build(BuildContext context) => SizedBox(
    width: 480,
    child: Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        MixinAppBar(
          title: Text(
            context.l10n.shareMessageDescriptionEmpty(
              switch (widget.payload.category) {
                _SendCategory.text => context.l10n.text,
                _SendCategory.image => context.l10n.image,
                _SendCategory.sticker => context.l10n.sticker,
                _SendCategory.contact => context.l10n.contact,
                _SendCategory.post => context.l10n.post,
                _SendCategory.appCard => context.l10n.card,
              },
            ),
          ),
          actions: const [MixinCloseButton()],
          leading: const SizedBox(),
          backgroundColor: context.theme.popUp,
        ),
        const SizedBox(height: 12),
        Container(
          width: 340,
          height: 340,
          decoration: BoxDecoration(
            color: context.dynamicColor(
              const Color.fromRGBO(245, 247, 250, 1),
              darkColor: const Color.fromRGBO(255, 255, 255, 0.08),
            ),
            borderRadius: const BorderRadius.all(Radius.circular(8)),
          ),
          alignment: Alignment.center,
          padding: widget.payload.category == _SendCategory.appCard
              ? null
              : const EdgeInsets.all(34),
          child: _PayloadPreview(
            account: widget.account,
            payload: widget.payload,
          ),
        ),
        const SizedBox(height: 54),
        MixinButton(
          onTap: _send,
          child: Text(
            widget.conversationId != null
                ? context.l10n.send
                : context.l10n.forward,
          ),
        ),
        const SizedBox(height: 56),
      ],
    ),
  );
}

Future<void> _sendPayload(
  BuildContext context,
  rust.AccountHandle account,
  String conversationId,
  _SendPayload payload,
) async {
  switch (payload.category) {
    case _SendCategory.text:
      await account.message().sendText(
        conversationId: conversationId,
        content: payload.data as String,
        quoteMessageId: null,
        silent: false,
      );
    case _SendCategory.post:
      await account.message().sendPost(
        conversationId: conversationId,
        content: payload.data as String,
      );
    case _SendCategory.contact:
      await account.message().sendContact(
        conversationId: conversationId,
        sharedUserId: payload.data as String,
        quoteMessageId: null,
        silent: false,
      );
    case _SendCategory.sticker:
      final stickerId = payload.data as String;
      await account.sticker().stickerDetail(stickerId: stickerId);
      await account.message().sendSticker(
        conversationId: conversationId,
        stickerId: stickerId,
      );
    case _SendCategory.appCard:
      await account.message().sendAppCard(
        conversationId: conversationId,
        content: payload.data as String,
      );
    case _SendCategory.image:
      await _sendImage(
        context,
        account,
        conversationId,
        payload.data as String,
      );
  }
}

Future<void> _sendImage(
  BuildContext context,
  rust.AccountHandle account,
  String conversationId,
  String url,
) async {
  final client = CoreHttpScope.maybeOf(context)?.client;
  if (client == null) throw StateError('network client unavailable');
  final response = await client.get(Uri.parse(url));
  if (response.statusCode < 200 || response.statusCode >= 300) {
    throw HttpException('image download failed: ${response.statusCode}');
  }
  final decoded = image.decodeImage(response.bodyBytes);
  if (decoded == null) throw const FormatException('invalid image');
  final directory = await getTemporaryDirectory();
  final file = File('${directory.path}/${const Uuid().v4()}');
  await file.writeAsBytes(response.bodyBytes, flush: true);
  try {
    await account.message().sendAttachment(
      conversationId: conversationId,
      path: file.path,
      kind: 'IMAGE',
      mimeType:
          response.headers['content-type']?.split(';').first.trim() ??
          'image/jpeg',
      width: decoded.width,
      height: decoded.height,
      silent: false,
    );
  } finally {
    if (await file.exists()) await file.delete();
  }
}

class _PayloadPreview extends StatelessWidget {
  const _PayloadPreview({required this.account, required this.payload});

  final rust.AccountHandle account;
  final _SendPayload payload;

  @override
  Widget build(BuildContext context) => switch (payload.category) {
    _SendCategory.text => _TextPreview(payload.data as String),
    _SendCategory.image => _ImagePreview(payload.data as String),
    _SendCategory.sticker => _StickerPreview(
      account: account,
      stickerId: payload.data as String,
    ),
    _SendCategory.contact => _ContactPreview(
      account: account,
      userId: payload.data as String,
    ),
    _SendCategory.post => _PostPreview(payload.data as String),
    _SendCategory.appCard => _AppCardPreview(payload.data as String),
  };
}

final _bubbleClipper = BubbleClipper(
  currentUser: false,
  showNip: false,
  nipPadding: false,
);

class _MessageBubble extends StatelessWidget {
  const _MessageBubble({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final child = Padding(padding: const EdgeInsets.all(8), child: this.child);
    return CustomPaint(
      painter: BubblePainter(
        color: context.dynamicColor(
          lightOtherBubble,
          darkColor: darkOtherBubble,
        ),
        clipper: _bubbleClipper,
      ),
      child: child,
    );
  }
}

class _TextPreview extends StatelessWidget {
  const _TextPreview(this.text);

  final String text;

  @override
  Widget build(BuildContext context) => _MessageBubble(
    child: Text(
      text,
      style: TextStyle(
        fontSize: context.messageStyle.primaryFontSize,
        color: context.theme.text,
      ),
    ),
  );
}

class _ImagePreview extends StatelessWidget {
  const _ImagePreview(this.url);

  final String url;

  @override
  Widget build(BuildContext context) => MixinImage.network(
    url,
    placeholder: () => ColoredBox(color: context.theme.secondaryText),
  );
}

class _StickerPreview extends StatelessWidget {
  const _StickerPreview({required this.account, required this.stickerId});

  final rust.AccountHandle account;
  final String stickerId;

  @override
  Widget build(BuildContext context) => FutureBuilder<rust.StickerDetailItem>(
    future: account.sticker().stickerDetail(stickerId: stickerId),
    builder: (context, snapshot) {
      final sticker = snapshot.data?.sticker;
      if (sticker == null) return const SizedBox();
      return Padding(
        padding: const EdgeInsets.all(45),
        child: StickerItem(
          stickerId: sticker.stickerId,
          assetUrl: sticker.assetUrl,
          assetType: sticker.assetType,
        ),
      );
    },
  );
}

class _ContactPreview extends StatelessWidget {
  const _ContactPreview({required this.account, required this.userId});

  final rust.AccountHandle account;
  final String userId;

  @override
  Widget build(BuildContext context) => FutureBuilder<rust.UserProfileItem?>(
    future: account.user().userProfile(userId: userId),
    builder: (context, snapshot) {
      final user = snapshot.data;
      if (user == null) return const SizedBox();
      return _MessageBubble(
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            AvatarView(
              size: 40,
              avatarUrl: user.avatarUrl,
              userId: user.userId,
              name: user.fullName,
            ),
            const SizedBox(width: 8),
            Flexible(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Flexible(
                        child: Text(
                          user.fullName,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            color: context.theme.text,
                            fontSize: context.messageStyle.primaryFontSize,
                          ),
                        ),
                      ),
                      if (user.isVerified || user.isBot)
                        Padding(
                          padding: const EdgeInsets.only(left: 3),
                          child: SvgPicture.asset(
                            user.isVerified
                                ? MixinAssets.verified
                                : MixinAssets.botBadge,
                            width: 14,
                            height: 14,
                          ),
                        ),
                    ],
                  ),
                  Text(
                    user.identityNumber,
                    style: TextStyle(
                      color: context.theme.secondaryText,
                      fontSize: context.messageStyle.secondaryFontSize,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      );
    },
  );
}

class _PostPreview extends StatelessWidget {
  const _PostPreview(this.content);

  final String content;

  @override
  Widget build(BuildContext context) => SizedBox.expand(
    child: Padding(
      padding: const EdgeInsets.all(36),
      child: _MessageBubble(
        child: Stack(
          children: [
            ConstrainedBox(
              constraints: const BoxConstraints(minWidth: 128, maxHeight: 400),
              child: MarkdownWidget(
                data: content,
                config: postMarkdownConfig(
                  context,
                  fontSize: context.messageStyle.primaryFontSize,
                ),
                shrinkWrap: true,
                physics: const NeverScrollableScrollPhysics(),
              ),
            ),
            Positioned(
              right: 0,
              top: 0,
              child: Container(
                decoration: const BoxDecoration(
                  color: Color.fromRGBO(0, 0, 0, 0.2),
                  borderRadius: BorderRadius.all(Radius.circular(8)),
                ),
                child: SvgPicture.asset(
                  MixinAssets.postDetail,
                  width: 20,
                  height: 20,
                ),
              ),
            ),
          ],
        ),
      ),
    ),
  );
}

class _AppCardPreview extends StatelessWidget {
  const _AppCardPreview(this.content);

  final String content;

  @override
  Widget build(BuildContext context) {
    final data = jsonDecode(content) as Map<String, dynamic>;
    final action = data['action']?.toString() ?? '';
    if (action.isNotEmpty) {
      final iconUrl = data['icon_url']?.toString() ?? '';
      final description = const LineSplitter()
          .convert(data['description']?.toString() ?? '')
          .firstOrNull;
      return _MessageBubble(
        child: Padding(
          padding: const EdgeInsets.all(34),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              ClipRRect(
                borderRadius: const BorderRadius.all(Radius.circular(4)),
                child: MixinImage.network(iconUrl, height: 40, width: 40),
              ),
              const SizedBox(width: 8),
              Flexible(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      data['title']?.toString() ?? '',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        color: context.theme.text,
                        fontSize: context.messageStyle.secondaryFontSize,
                      ),
                    ),
                    Text(
                      description ?? '',
                      maxLines: 1,
                      style: TextStyle(
                        color: context.theme.secondaryText,
                        fontSize: context.messageStyle.tertiaryFontSize,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      );
    }
    return AppCardMessageItem(
      message: MessageListEntry(
        id: 'send-preview',
        conversationId: '',
        senderId: '',
        senderName: '',
        senderAvatarUrl: '',
        senderIsVerified: false,
        category: 'APP_CARD',
        content: content,
        status: 'SENT',
        createdAt: DateTime.fromMillisecondsSinceEpoch(0),
        mediaDuration: '',
        mediaStatus: '',
      ),
      onAction: null,
      isCurrentUser: false,
      showNip: false,
      highlighted: false,
      dateAndStatus: const SizedBox(),
    );
  }
}
