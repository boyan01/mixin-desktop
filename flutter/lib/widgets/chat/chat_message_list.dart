import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:provider/provider.dart';
import 'package:super_context_menu/super_context_menu.dart';
import 'package:visibility_detector/visibility_detector.dart';

import '../../constants/assets.dart';
import '../../controllers/chat_side_notifier.dart';
import '../../controllers/message_action_controller.dart';
import '../../controllers/message_controller.dart';
import '../../controllers/settings_controller.dart';
import '../../l10n/l10n.dart';
import '../../models/message_list_entry.dart';
import '../../theme.dart';
import '../../utils/system_clipboard.dart';
import '../avatar_view.dart';
import '../custom_context_menu.dart';
import '../interactive_decorated_box.dart';
import '../message_action_policy.dart';
import '../message_actions_menu.dart';
import '../message_bubble.dart';
import '../message_content.dart';
import '../message_datetime_and_status.dart';
import '../message_day_time.dart';
import '../message_items/quote_message_preview.dart';
import '../message_name.dart';
import '../message_presentation.dart';
import '../message_rows.dart';
import '../message_selectable_text.dart';
import '../show_add_image_sticker_dialog.dart';
import '../show_forward_conversation_selector.dart';
import '../show_message_user_dialog.dart';
import '../toast.dart';
import 'chat_history_viewport.dart';
import 'chat_scroll_coordinator.dart';

@visibleForTesting
bool shouldMarkMentionRead(bool? mentionRead, double visibleFraction) =>
    mentionRead == false && visibleFraction >= 1;

class ChatMessageList extends StatelessWidget {
  const ChatMessageList({
    required this.messageController,
    required this.scrollCoordinator,
    required this.selectedMessageIds,
    required this.highlightedMessageId,
    required this.highlightOpacity,
    required this.onReply,
    required this.onSelect,
    required this.onToggleSelection,
    super.key,
  });

  final MessageController messageController;
  final ChatScrollCoordinator scrollCoordinator;
  final Set<String> selectedMessageIds;
  final String? highlightedMessageId;
  final double highlightOpacity;
  final ValueChanged<MessageListEntry> onReply;
  final ValueChanged<MessageListEntry> onSelect;
  final ValueChanged<MessageListEntry> onToggleSelection;

  @override
  Widget build(BuildContext context) {
    final actions = context.watch<MessageActionController>();
    return ChatHistoryViewport(
      messageController: messageController,
      presentationListenable: actions,
      scrollCoordinator: scrollCoordinator,
      unreadBar: const _UnreadMessageBar(),
      messageBuilder: (row, dayTimeKey) {
        final message = row.message;
        return MessageItemWidget(
          dayTimeKey: dayTimeKey,
          row: row,
          selected: selectedMessageIds.contains(message.id),
          highlightOpacity: highlightedMessageId == message.id
              ? highlightOpacity
              : 0,
          inSelectionMode: selectedMessageIds.isNotEmpty,
          onReply: () => onReply(message),
          onSelect: () => onSelect(message),
          onToggleSelection: () => onToggleSelection(message),
        );
      },
    );
  }
}

class _UnreadMessageBar extends StatelessWidget {
  const _UnreadMessageBar();

  @override
  Widget build(BuildContext context) => Container(
    key: const Key('unread-message-bar'),
    color: context.theme.background,
    padding: const EdgeInsets.symmetric(vertical: 4),
    margin: const EdgeInsets.symmetric(vertical: 6),
    alignment: Alignment.center,
    child: Text(
      context.l10n.unreadMessages,
      style: TextStyle(color: context.theme.secondaryText, fontSize: 14),
    ),
  );
}

class MessageItemWidget extends StatefulHookWidget {
  const MessageItemWidget({
    required this.dayTimeKey,
    required this.row,
    required this.selected,
    required this.highlightOpacity,
    required this.inSelectionMode,
    required this.onReply,
    required this.onSelect,
    required this.onToggleSelection,
    super.key,
  });

  final GlobalKey? dayTimeKey;
  final MessageRowModel row;
  final bool selected;
  final double highlightOpacity;
  final bool inSelectionMode;
  final VoidCallback onReply;
  final VoidCallback onSelect;
  final VoidCallback onToggleSelection;

  @override
  State<MessageItemWidget> createState() => _MessageItemWidgetState();
}

class _MessageItemWidgetState extends State<MessageItemWidget> {
  @override
  Widget build(BuildContext context) {
    final actions = context.watch<MessageActionController>();
    final account = actions.account;
    final conversation = actions.conversation;
    final currentUserId = account.accountId();
    final message = widget.row.message;
    final menuHighlighted = useState(false);
    final focusNode = useFocusScopeNode(
      debugLabel: 'message_item_${message.id}',
    );
    final mentionNames = useMessageMentionNames(
      account,
      [message.content, message.caption, message.quoteContent],
      revision: (message, actions.changeRevision),
    );
    final settings = context.watch<SettingsController>();
    final row = widget.row;
    final presentation = MessagePresentation.fromRow(
      row: row,
      isGroupOrBotGroupConversation: resolveGroupOrBotGroupConversation(
        message: message,
        isGroup: conversation.isGroup,
        isBotGroup: actions.isBotGroup,
        conversationOwnerId: conversation.ownerId,
      ),
      enableShowAvatar: settings.messageShowAvatar,
    );
    final showIdentityNumber =
        settings.messageShowIdentityNumber &&
        message.senderIdentityNumber.isNotEmpty &&
        message.senderIdentityNumber != '0';
    final search = context.watch<SearchConversationKeywordNotifier?>()?.value;
    final (selectedUserId, searchKeyword) = search ?? (null, '');
    final messageKeyword =
        selectedUserId == null || selectedUserId == message.senderId
        ? searchKeyword
        : '';
    final messageContent = MessageContent(
      message: message,
      isCurrentUser: presentation.isCurrentUser,
      currentUserId: currentUserId,
      recalledText: actions.recalledText(message.id),
      mentionNames: mentionNames,
      keyword: messageKeyword,
      audioPlaylist: actions.messages,
      showNip: presentation.showNip,
      highlighted: menuHighlighted.value,
      highlightOpacity: widget.highlightOpacity,
      dateAndStatus: MessageDatetimeAndStatus(
        message: message,
        isCurrentUser: presentation.isCurrentUser,
        isRepresentative:
            conversation.isBot &&
            message.senderId != conversation.ownerId &&
            !presentation.isCurrentUser,
      ),
      overlayDateAndStatus: MessageDatetimeAndStatus(
        message: message,
        isCurrentUser: presentation.isCurrentUser,
        color: Colors.white,
        isRepresentative:
            conversation.isBot &&
            message.senderId != conversation.ownerId &&
            !presentation.isCurrentUser,
      ),
    );
    final bypassActions = const {
      'SYSTEM_CONVERSATION',
      'MESSAGE_PIN',
      'SECRET',
      'STRANGER',
    }.contains(message.category);
    final bypassBubble =
        !message.isUnresolvedMessage &&
        !message.isInvalidSpecialMessage &&
        (bypassActions ||
            message.category == 'APP_BUTTON_GROUP' ||
            message.category == 'APP_CARD');
    final quote = buildMessageQuotePreview(
      message,
      mentionNames: mentionNames,
    );
    final renderedMessage = bypassActions
        ? messageContent
        : bypassBubble
        ? Align(
            key: Key('message-bubble-${message.id}'),
            alignment: presentation.isCurrentUser
                ? Alignment.centerRight
                : Alignment.centerLeft,
            child: messageContent,
          )
        : MessageBubble(
            key: Key('message-bubble-${message.id}'),
            isCurrentUser: presentation.isCurrentUser,
            showNip: presentation.showNip,
            showBubble: message.showMessageBubble,
            includeNip: message.includeMessageBubbleNip,
            clip: message.clipMessageBubble,
            highlighted: menuHighlighted.value,
            highlightOpacity: widget.highlightOpacity,
            padding: message.messageBubblePadding,
            forceIsCurrentUserColor: message.forceCurrentMessageBubbleColor,
            isDisappearingMessage: (message.expireIn ?? 0) > 0,
            quote: quote,
            constrainQuoteWidth:
                message.isImage || message.isVideo || message.isLive,
            highlightMedia:
                !message.showMessageBubble &&
                quote == null &&
                (message.isImage ||
                    message.isVideo ||
                    message.isLive ||
                    message.isSticker),
            outerTimeAndStatusWidget: message.useOuterMessageDateAndStatus
                ? MessageDatetimeAndStatus(
                    message: message,
                    isCurrentUser: presentation.isCurrentUser,
                    hideStatus: message.hideOuterMessageStatus,
                    isRepresentative:
                        conversation.isBot &&
                        message.senderId != conversation.ownerId &&
                        !presentation.isCurrentUser,
                  )
                : null,
            child: messageContent,
          );
    final policy = MessageActionPolicy(
      message: message,
      currentUserId: currentUserId,
      currentUserRole: actions.currentUserRole,
      now: DateTime.now(),
    );
    final copyableImage =
        message.isImage &&
            const {
              'DONE',
              'READ',
            }.contains(message.mediaStatus.toUpperCase())
        ? existingLocalFile(message.mediaUrl)
        : null;
    final contextMenuMessage = ContextMenuWidget(
      desktopMenuWidgetBuilder: CustomDesktopMenuWidgetBuilder(),
      menuProvider: (request) {
        request.onShowMenu.addListener(() {
          menuHighlighted.value = true;
        });
        request.onHideMenu.addListener(() {
          menuHighlighted.value = false;
        });
        return buildMessageActionsMenu(
          context: context,
          message: message,
          policy: policy,
          onReply: widget.onReply,
          onForward: () => unawaited(_forward(actions, message)),
          onAddSticker: message.stickerId?.trim().isNotEmpty == true
              ? () => unawaited(_addSticker(actions, message))
              : copyableImage == null
              ? null
              : () => unawaited(
                  showAddImageStickerDialog(
                    context,
                    file: copyableImage,
                    onConfirm: () =>
                        actions.account.sticker().addStickerFromFile(
                          messageId: message.id,
                        ),
                  ),
                ),
          onSelect: widget.onSelect,
          onTogglePin: () => unawaited(
            _ignoreMutation(
              actions.setMessagePinned(message, !message.pinned),
            ),
          ),
          onRecall: () => unawaited(
            _ignoreMutation(actions.recallMessages([message])),
          ),
          onDelete: () => unawaited(
            _ignoreMutation(actions.deleteMessages([message])),
          ),
          selectedText: _findSelectedText(context),
          hasSelectedMessages: widget.inSelectionMode,
        );
      },
      child: _MessageQuickReplyDetector(
        onReply: policy.canReply ? widget.onReply : null,
        child: renderedMessage,
      ),
    );
    final interactiveMessage = bypassActions
        ? renderedMessage
        : contextMenuMessage;
    final messageColumn = Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (presentation.showSender)
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Flexible(
                child: MessageName(
                  userName: message.senderName,
                  userId: message.senderId,
                  userIdentityNumber: message.senderIdentityNumber,
                  verified: message.senderIsVerified,
                  isBot: message.senderIsBot,
                  membership: message.senderMembership,
                  showIdentityNumber: showIdentityNumber,
                  onTap: () => openMessageUserDialog(
                    context,
                    userId: message.senderId,
                  ),
                ),
              ),
            ],
          ),
        interactiveMessage,
      ],
    );

    Widget child;
    if (bypassActions) {
      child = interactiveMessage;
    } else if (presentation.showSender && presentation.showAvatar) {
      child = Row(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const SizedBox(width: 8),
          InteractiveDecoratedBox(
            cursor: SystemMouseCursors.click,
            onTap: () => openMessageUserDialog(
              context,
              userId: message.senderId,
            ),
            child: AvatarView(
              userId: message.senderId,
              name: message.senderName,
              avatarUrl: message.senderAvatarUrl,
              size: 32,
            ),
          ),
          Flexible(
            child: Padding(
              padding: const EdgeInsets.only(top: 2, bottom: 2),
              child: messageColumn,
            ),
          ),
          const SizedBox(width: 65),
        ],
      );
    } else {
      child = Padding(
        padding: EdgeInsets.only(
          left: presentation.isCurrentUser
              ? 65
              : (presentation.showAvatar ? 40 : 16),
          right: presentation.isCurrentUser ? 16 : 65,
          top: 2,
          bottom: 2,
        ),
        child: messageColumn,
      );
    }

    final selectionWrapped = bypassActions
        ? child
        : GestureDetector(
            key: Key('message-selection-${message.id}'),
            behavior: HitTestBehavior.translucent,
            onTap: widget.inSelectionMode ? widget.onToggleSelection : null,
            onLongPress: widget.inSelectionMode || !policy.canSelect
                ? null
                : widget.onSelect,
            child: Row(
              children: [
                _AnimatedSelectionIcon(
                  selected: widget.selected,
                  inSelectionMode: widget.inSelectionMode,
                ),
                Expanded(
                  child: IgnorePointer(
                    ignoring: widget.inSelectionMode,
                    child: child,
                  ),
                ),
              ],
            ),
          );

    return FocusScope(
      node: focusNode,
      child: VisibilityDetector(
        key: ValueKey('message_visibility_${message.id}'),
        onVisibilityChanged: (info) {
          if (shouldMarkMentionRead(
            message.mentionRead,
            info.visibleFraction,
          )) {
            unawaited(actions.markMentionRead(message));
          }
        },
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (row.dateTime != null)
              MessageDayTime(key: widget.dayTimeKey, dateTime: row.dateTime!),
            Padding(
              padding: row.sameUserPrevious
                  ? EdgeInsets.zero
                  : const EdgeInsets.only(top: 8),
              child: selectionWrapped,
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _forward(
    MessageActionController actions,
    MessageListEntry message,
  ) async {
    final target = await showConversationSelector(
      context,
      account: actions.account,
      title: context.l10n.forward,
    );
    if (target == null || !mounted) return;
    await runWithLoading(() => actions.forwardMessages([message], target.id));
  }

  Future<void> _addSticker(
    MessageActionController actions,
    MessageListEntry message,
  ) async {
    try {
      final stickerId = message.stickerId?.trim();
      if (stickerId == null || stickerId.isEmpty) return;
      await actions.account.sticker().addSticker(stickerId: stickerId);
      if (!mounted) return;
      showToastSuccessful();
    } on Object {
      if (!mounted) return;
      showToastFailed(ToastError(context.l10n.addStickerFailed));
    }
  }
}

class _AnimatedSelectionIcon extends HookWidget {
  const _AnimatedSelectionIcon({
    required this.selected,
    required this.inSelectionMode,
  });

  final bool selected;
  final bool inSelectionMode;

  @override
  Widget build(BuildContext context) {
    final controller = useAnimationController(
      duration: const Duration(milliseconds: 300),
    );
    final width = useMemoized(
      () => controller.drive(
        Tween<double>(
          begin: 0,
          end: 48,
        ).chain(CurveTween(curve: Curves.easeInOut)),
      ),
      [controller],
    );

    useEffect(() {
      if (inSelectionMode) {
        controller.forward();
      } else {
        controller.reverse();
      }
      return null;
    }, [inSelectionMode, controller]);

    useListenable(width);
    if (width.isDismissed) return const SizedBox();
    return SizedBox(
      width: width.value,
      height: 20,
      child: Center(
        child: ClipOval(
          child: Container(
            width: 16,
            height: 16,
            alignment: const Alignment(0, -0.2),
            color: selected
                ? context.theme.accent
                : context.theme.secondaryText,
            child: SvgPicture.asset(
              MixinAssets.selected,
              width: 10,
              height: 10,
            ),
          ),
        ),
      ),
    );
  }
}

class _MessageQuickReplyDetector extends HookWidget {
  const _MessageQuickReplyDetector({
    required this.child,
    required this.onReply,
  });

  final Widget child;
  final VoidCallback? onReply;

  @override
  Widget build(BuildContext context) {
    final lastPrimaryDownTime = useRef<Duration?>(null);
    final lastPrimaryDownPosition = useRef<Offset?>(null);

    return Listener(
      behavior: HitTestBehavior.translucent,
      onPointerDown: (event) {
        if ((event.buttons & kPrimaryButton) == 0 ||
            _hitsSelectableText(context, event.position)) {
          lastPrimaryDownTime.value = null;
          lastPrimaryDownPosition.value = null;
          return;
        }

        final previousTime = lastPrimaryDownTime.value;
        final previousPosition = lastPrimaryDownPosition.value;
        lastPrimaryDownTime.value = event.timeStamp;
        lastPrimaryDownPosition.value = event.position;
        if (previousTime == null || previousPosition == null) return;

        final withinTimeout =
            event.timeStamp - previousTime <= kDoubleTapTimeout;
        final withinSlop =
            (event.position - previousPosition).distance <= kDoubleTapSlop;
        if (!withinTimeout || !withinSlop) return;

        lastPrimaryDownTime.value = null;
        lastPrimaryDownPosition.value = null;
        onReply?.call();
      },
      child: child,
    );
  }
}

bool _hitsSelectableText(BuildContext context, Offset globalPosition) {
  var hit = false;
  void visit(Element element) {
    if (hit) return;
    final renderObject = element.renderObject;
    if (renderObject is RenderParagraph &&
        renderObject.registrar != null &&
        renderObject.attached &&
        _paragraphHitsText(renderObject, globalPosition)) {
      hit = true;
      return;
    }
    element.visitChildren(visit);
  }

  context.visitChildElements(visit);
  return hit;
}

String? _findSelectedText(BuildContext context) {
  String? selectedText;
  void visit(Element element) {
    if (selectedText != null) return;
    if (element is StatefulElement && element.state is EditableTextState) {
      final state = element.state as EditableTextState;
      final value = state.textEditingValue;
      final selection = value.selection;
      if (selection.isValid && !selection.isCollapsed) {
        selectedText = selection.textInside(value.text);
        return;
      }
    }
    element.visitChildren(visit);
  }

  if (context is Element) visit(context);
  return selectedText;
}

bool _paragraphHitsText(RenderParagraph paragraph, Offset globalPosition) {
  final localPosition = paragraph.globalToLocal(globalPosition);
  if (!paragraph.paintBounds.contains(localPosition)) return false;
  final text = paragraph.text.toPlainText(includeSemanticsLabels: false);
  if (text.isEmpty) return false;
  final offset = paragraph.getPositionForOffset(localPosition).offset;
  for (final index in {offset, offset - 1}) {
    if (index < 0 || index >= text.length) continue;
    if (text.substring(index, index + 1).trim().isEmpty) continue;
    final boxes = paragraph.getBoxesForSelection(
      TextSelection(baseOffset: index, extentOffset: index + 1),
    );
    if (boxes.any((box) => box.toRect().contains(localPosition))) return true;
  }
  return false;
}

Future<void> _ignoreMutation(Future<void> mutation) async {
  try {
    await mutation;
  } on Object {
    // MessageActionController already logs mutation failures with context.
  }
}
