import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:super_context_menu/super_context_menu.dart';

import '../../constants/assets.dart';
import '../../l10n/l10n.dart';
import '../../models/conversation_list_entry.dart';
import '../../src/rust/desktop_api.dart' as rust;
import '../../theme.dart';
import '../action_button.dart';
import '../animated_visibility.dart';
import '../avatar_view.dart';
import '../badges_widget.dart';
import '../custom_context_menu.dart';
import '../high_light_text.dart';
import '../interactive_decorated_box.dart';
import '../message_bubble.dart';
import '../message_items/pin_message_item.dart';
import '../pin_message_bubble.dart';

class GroupCannotSendBar extends StatelessWidget {
  const GroupCannotSendBar({super.key});

  @override
  Widget build(BuildContext context) => Container(
    height: 56,
    alignment: Alignment.center,
    color: context.theme.primary,
    child: Text(
      context.l10n.groupCantSend,
      style: TextStyle(color: context.theme.secondaryText),
    ),
  );
}

class ChatHeader extends StatelessWidget {
  const ChatHeader({
    required this.conversation,
    required this.onBack,
    required this.onSearch,
    required this.onInfo,
    required this.showInfoAction,
    required this.selectionMode,
    required this.onCancelSelection,
    required this.onBot,
    super.key,
  });

  final ConversationListEntry conversation;
  final VoidCallback? onBack;
  final VoidCallback? onSearch;
  final VoidCallback? onInfo;
  final bool showInfoAction;
  final bool selectionMode;
  final VoidCallback onCancelSelection;
  final VoidCallback? onBot;

  @override
  Widget build(BuildContext context) {
    final subtitle = conversation.isGroup
        ? context.l10n.participantsCount(conversation.participantCount)
        : conversation.identityNumber;
    return Container(
      key: const Key('chat-header'),
      height: 64,
      decoration: BoxDecoration(
        color: context.theme.primary,
        border: Border(bottom: BorderSide(color: context.theme.divider)),
      ),
      child: Row(
        children: [
          if (onBack == null)
            const SizedBox(width: 16)
          else
            _HeaderAction(
              key: const Key('chat-back'),
              asset: MixinAssets.back,
              onPressed: onBack,
            ),
          GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: onInfo,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                ConversationAvatarView(conversation: conversation, size: 36),
                const SizedBox(width: 10),
              ],
            ),
          ),
          Expanded(
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: onInfo,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Flexible(
                        child: CustomSelectableArea(
                          child: CustomText(
                            conversation.name,
                            maxLines: 1,
                            style: TextStyle(
                              color: context.theme.text,
                              fontSize: 16,
                              height: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                            textAlign: TextAlign.center,
                          ),
                        ),
                      ),
                      BadgesWidget(
                        verified: conversation.isVerified,
                        isBot: conversation.isBot,
                        membership: conversation.membership,
                      ),
                    ],
                  ),
                  if (subtitle.isNotEmpty) ...[
                    const SizedBox(height: 4),
                    CustomSelectableText(
                      subtitle,
                      maxLines: 1,
                      style: TextStyle(
                        color: context.theme.secondaryText,
                        fontSize: 14,
                        height: 1,
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
          AnimatedSize(
            duration: const Duration(milliseconds: 200),
            alignment: Alignment.centerLeft,
            child: onBot == null
                ? const SizedBox()
                : _HeaderAction(asset: MixinAssets.bots, onPressed: onBot),
          ),
          if (selectionMode)
            TextButton(
              onPressed: onCancelSelection,
              child: Text(context.l10n.cancel),
            )
          else ...[
            _HeaderAction(
              key: const Key('chat-search'),
              asset: MixinAssets.chatSearch,
              onPressed: onSearch,
            ),
            AnimatedSize(
              duration: const Duration(milliseconds: 200),
              alignment: Alignment.centerLeft,
              child: showInfoAction
                  ? _HeaderAction(
                      key: const Key('chat-info'),
                      asset: MixinAssets.chatInfo,
                      onPressed: onInfo,
                    )
                  : const SizedBox(),
            ),
          ],
          const SizedBox(width: 16),
        ],
      ),
    );
  }
}

class _HeaderAction extends StatelessWidget {
  const _HeaderAction({
    required this.asset,
    required this.onPressed,
    super.key,
  });

  final String asset;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    final color = onPressed == null
        ? context.theme.secondaryText.withValues(alpha: 0.5)
        : context.theme.icon;
    final icon = Center(
      child: SvgPicture.asset(
        asset,
        width: 20,
        height: 20,
        colorFilter: ColorFilter.mode(color, BlendMode.srcIn),
      ),
    );
    return SizedBox.square(
      dimension: 40,
      child: onPressed == null
          ? ExcludeSemantics(child: icon)
          : InkResponse(onTap: onPressed, radius: 20, child: icon),
    );
  }
}

class ScamWarningBanner extends StatelessWidget {
  const ScamWarningBanner({
    required this.visible,
    required this.onDismiss,
    super.key,
  });

  final bool visible;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) => AnimatedVisibility(
    visible: visible,
    child: Container(
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        borderRadius: const BorderRadius.all(Radius.circular(4)),
        color: context.messageBubbleColor(false),
        boxShadow: const [
          BoxShadow(
            color: Color.fromRGBO(0, 0, 0, 0.15),
            offset: Offset(0, 2),
            blurRadius: 10,
          ),
        ],
      ),
      child: Row(
        children: [
          Padding(
            padding: const EdgeInsets.only(
              left: 8,
              right: 16,
              top: 8,
              bottom: 8,
            ),
            child: SvgPicture.asset(
              MixinAssets.warning,
              width: 26,
              height: 26,
              colorFilter: ColorFilter.mode(context.theme.red, BlendMode.srcIn),
            ),
          ),
          Expanded(
            child: Text(
              context.l10n.scamWarning,
              style: TextStyle(color: context.theme.text, fontSize: 14),
            ),
          ),
          ActionButton(
            name: MixinAssets.close,
            color: context.theme.icon,
            size: 20,
            onTap: onDismiss,
          ),
        ],
      ),
    ),
  );
}

class JumpCurrentButton extends StatelessWidget {
  const JumpCurrentButton({
    required this.visible,
    required this.onTap,
    super.key,
  });

  final bool visible;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    if (!visible) return const SizedBox();
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: InteractiveDecoratedBox(
        onTap: onTap,
        child: Container(
          height: 40,
          width: 40,
          decoration: BoxDecoration(
            color: context.messageBubbleColor(false),
            boxShadow: const [
              BoxShadow(
                color: Color.fromRGBO(0, 0, 0, 0.15),
                offset: Offset(0, 2),
                blurRadius: 10,
              ),
            ],
            shape: BoxShape.circle,
          ),
          alignment: Alignment.center,
          child: SvgPicture.asset(
            MixinAssets.jumpCurrentArrow,
            colorFilter: ColorFilter.mode(context.theme.text, BlendMode.srcIn),
          ),
        ),
      ),
    );
  }
}

class JumpMentionButton extends StatelessWidget {
  const JumpMentionButton({
    required this.messageIds,
    required this.onJump,
    required this.onClear,
    super.key,
  });

  final List<String> messageIds;
  final ValueChanged<String> onJump;
  final VoidCallback onClear;

  @override
  Widget build(BuildContext context) {
    if (messageIds.isEmpty) return const SizedBox();
    return ContextMenuWidget(
      desktopMenuWidgetBuilder: CustomDesktopMenuWidgetBuilder(),
      menuProvider: (_) => Menu(
        children: [MenuAction(title: context.l10n.clear, callback: onClear)],
      ),
      child: InteractiveDecoratedBox(
        onTap: () => onJump(messageIds.first),
        child: SizedBox(
          height: 52,
          width: 40,
          child: Stack(
            children: [
              Positioned(
                top: 12,
                child: Container(
                  height: 40,
                  width: 40,
                  decoration: BoxDecoration(
                    color: context.messageBubbleColor(false),
                    boxShadow: const [
                      BoxShadow(
                        color: Color.fromRGBO(0, 0, 0, 0.15),
                        offset: Offset(0, 2),
                        blurRadius: 10,
                      ),
                    ],
                    shape: BoxShape.circle,
                  ),
                  alignment: Alignment.center,
                  child: Text(
                    '@',
                    style: TextStyle(
                      fontSize: 17,
                      height: 1,
                      color: context.theme.text,
                    ),
                  ),
                ),
              ),
              Container(
                width: 40,
                alignment: Alignment.topCenter,
                child: Container(
                  constraints: const BoxConstraints(
                    minWidth: 20,
                    maxWidth: 40,
                    minHeight: 20,
                    maxHeight: 20,
                  ),
                  padding: const EdgeInsets.symmetric(horizontal: 4),
                  decoration: BoxDecoration(
                    color: context.theme.accent,
                    borderRadius: const BorderRadius.all(Radius.circular(10)),
                  ),
                  child: Column(
                    children: [
                      const Spacer(),
                      Text(
                        messageIds.length > 99 ? '99+' : '${messageIds.length}',
                        maxLines: 1,
                        textAlign: TextAlign.center,
                        style: const TextStyle(
                          fontSize: 14,
                          height: 1,
                          color: Colors.white,
                        ),
                      ),
                      const Spacer(),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class PinMessagesBanner extends StatelessWidget {
  const PinMessagesBanner({
    required this.messageIds,
    required this.preview,
    required this.visible,
    required this.onDismiss,
    required this.onLocate,
    super.key,
    this.onOpenAll,
  });

  final List<String> messageIds;
  final rust.PinMessagePreviewItem? preview;
  final bool visible;
  final VoidCallback onDismiss;
  final ValueChanged<String> onLocate;
  final VoidCallback? onOpenAll;

  @override
  Widget build(BuildContext context) {
    final pinnedMessageId = messageIds.firstOrNull;
    final showPreview = visible && pinnedMessageId != null;
    final preview = pinnedMessageId == null
        ? ''
        : context.l10n.chatPinMessage(
            this.preview?.senderName ?? '',
            this.preview == null
                ? context.l10n.aMessage
                : pinMessagePreview(context.l10n, this.preview!.content),
          );
    return Positioned(
      top: 12,
      right: 16,
      left: 10,
      height: 64,
      child: AnimatedVisibility(
        visible: showPreview || messageIds.isNotEmpty,
        child: Row(
          children: [
            Expanded(
              child: AnimatedVisibility(
                visible: showPreview,
                child: PinMessageBubble(
                  child: Row(
                    children: [
                      ActionButton(
                        name: MixinAssets.close,
                        color: context.theme.icon,
                        size: 20,
                        onTap: onDismiss,
                      ),
                      const SizedBox(width: 4),
                      Expanded(
                        child: InteractiveDecoratedBox(
                          cursor: SystemMouseCursors.click,
                          onTap: pinnedMessageId == null
                              ? null
                              : () => onLocate(pinnedMessageId),
                          child: CustomText(
                            preview,
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
            const SizedBox(width: 8),
            AnimatedVisibility(
              visible: messageIds.isNotEmpty,
              child: InteractiveDecoratedBox(
                onTap: onOpenAll,
                child: Container(
                  height: 34,
                  width: 34,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: context.messageBubbleColor(false),
                    boxShadow: const [
                      BoxShadow(
                        color: Color.fromRGBO(0, 0, 0, 0.15),
                        offset: Offset(0, 2),
                        blurRadius: 10,
                      ),
                    ],
                  ),
                  alignment: Alignment.center,
                  child: SvgPicture.asset(
                    MixinAssets.chatPin,
                    width: 34,
                    height: 34,
                    colorFilter: ColorFilter.mode(
                      context.theme.text,
                      BlendMode.srcIn,
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
