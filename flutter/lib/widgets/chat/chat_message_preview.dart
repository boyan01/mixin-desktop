import 'package:flutter/widgets.dart';

import '../../l10n/l10n.dart';
import '../../models/message_list_entry.dart';

String messagePreview(BuildContext context, MessageListEntry message) {
  if (message.isText || message.isPost) return message.content;
  if (message.isImage) {
    return message.caption?.trim().isNotEmpty == true
        ? message.caption!
        : context.l10n.image;
  }
  if (message.isVideo) return context.l10n.video;
  if (message.isAudio) return context.l10n.audio;
  if (message.isSticker) return context.l10n.sticker;
  if (message.category.endsWith('_DATA')) {
    return message.mediaName ?? context.l10n.file;
  }
  return message.content.isEmpty
      ? context.l10n.messageNotSupport
      : message.content;
}
