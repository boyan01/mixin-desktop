import 'dart:async';
import 'dart:math' as math;
import 'dart:ui' as ui show BoxHeightStyle;

import 'package:file_selector/file_selector.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_portal/flutter_portal.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:provider/provider.dart';
import 'package:super_context_menu/super_context_menu.dart';

import '../../constants/assets.dart';
import '../../constants/icon_fonts.dart';
import '../../controllers/mention_controller.dart';
import '../../l10n/l10n.dart';
import '../../models/conversation_list_entry.dart';
import '../../models/message_list_entry.dart';
import '../../src/rust/desktop_api.dart' as rust;
import '../../theme.dart';
import '../../utils/name_color.dart';
import '../../utils/system_clipboard.dart';
import '../action_button.dart';
import '../adaptive_selection_toolbar.dart';
import '../avatar_view.dart';
import '../badges_widget.dart';
import '../custom_context_menu.dart';
import '../custom_popup_menu.dart';
import '../high_light_text.dart';
import '../interactive_decorated_box.dart';
import '../message_media_preview_pages.dart';
import '../message_style.dart';
import 'chat_message_preview.dart';

const _maxTextLength = 64 * 1024;

class ChatInputBar extends StatefulWidget {
  const ChatInputBar({
    required this.controller,
    required this.focusNode,
    required this.quoteMessage,
    required this.onChanged,
    required this.onCancelQuote,
    required this.onSend,
    this.onSendSilent,
    this.onPasteFiles,
    this.onSendPost,
    this.mentionAccount,
    this.mentionConversation,
    this.onMentionSelected,
    this.onMentionNamesChanged,
    this.onContactPressed,
    this.onFilesPressed,
    this.onPicturesPressed,
    this.onStickerPressed,
    this.stickerAction,
    this.onVoicePressed,
    this.voiceButtonActive = false,
    this.isEncryptConversation = false,
    super.key,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final MessageListEntry? quoteMessage;
  final ValueChanged<String> onChanged;
  final VoidCallback onCancelQuote;
  final Future<void> Function() onSend;
  final Future<void> Function()? onSendSilent;
  final Future<void> Function()? onSendPost;
  final Future<void> Function(List<XFile> files)? onPasteFiles;
  final rust.AccountHandle? mentionAccount;
  final ConversationListEntry? mentionConversation;
  final ValueChanged<String>? onMentionSelected;
  final ValueChanged<Map<String, String>>? onMentionNamesChanged;
  final VoidCallback? onContactPressed;
  final VoidCallback? onFilesPressed;
  final VoidCallback? onPicturesPressed;
  final VoidCallback? onStickerPressed;
  final Widget? stickerAction;
  final VoidCallback? onVoicePressed;
  final bool voiceButtonActive;
  final bool isEncryptConversation;

  @override
  State<ChatInputBar> createState() => _ChatInputBarState();
}

class _ChatInputBarState extends State<ChatInputBar> {
  MessageListEntry? lastQuoteMessage;
  MentionController? _mentionController;

  @override
  void initState() {
    super.initState();
    lastQuoteMessage = widget.quoteMessage;
    widget.controller.addListener(_handleTextChanged);
    _createMentionController();
  }

  @override
  void didUpdateWidget(covariant ChatInputBar oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.quoteMessage != null) lastQuoteMessage = widget.quoteMessage;
    if (!identical(oldWidget.mentionAccount, widget.mentionAccount) ||
        oldWidget.mentionConversation != widget.mentionConversation ||
        oldWidget.controller != widget.controller) {
      _mentionController?.dispose();
      _createMentionController();
    }
    if (oldWidget.controller == widget.controller) return;
    oldWidget.controller.removeListener(_handleTextChanged);
    widget.controller.addListener(_handleTextChanged);
  }

  @override
  void dispose() {
    widget.controller.removeListener(_handleTextChanged);
    _mentionController?.dispose();
    super.dispose();
  }

  void _handleTextChanged() => setState(() {});

  void _createMentionController() {
    _mentionController = null;
    final account = widget.mentionAccount;
    final conversation = widget.mentionConversation;
    if (account == null || conversation == null) return;
    _mentionController = MentionController(
      account: account,
      conversation: conversation,
      inputController: widget.controller,
      onTextChanged: widget.onChanged,
      onMentionSelected: widget.onMentionSelected ?? (_) {},
      onMentionNamesChanged: widget.onMentionNamesChanged ?? (_) {},
    );
  }

  @override
  Widget build(BuildContext context) {
    final sendable =
        widget.controller.text.trim().isNotEmpty &&
        widget.controller.value.composing.isCollapsed;
    final mentions = _mentionController;
    if (mentions == null) {
      return _MentionPanelPortal(
        textEditingController: widget.controller,
        child: _build(context, sendable, false),
      );
    }
    return ChangeNotifierProvider.value(
      value: mentions,
      child: Consumer<MentionController>(
        builder: (context, mentions, _) => _MentionPanelPortal(
          textEditingController: widget.controller,
          mentions: mentions,
          child: _build(context, sendable, mentions.users.isNotEmpty),
        ),
      ),
    );
  }

  Widget _build(
    BuildContext context,
    bool sendable,
    bool mentionVisible,
  ) => ColoredBox(
    key: const Key('chat-input-bar'),
    color: context.theme.primary,
    child: Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        TweenAnimationBuilder<double>(
          tween: Tween(begin: 0, end: widget.quoteMessage == null ? 0 : 1),
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOut,
          builder: (context, progress, child) => ClipRect(
            child: Align(
              alignment: AlignmentDirectional.topCenter,
              heightFactor: progress,
              child: child,
            ),
          ),
          child: lastQuoteMessage == null
              ? const SizedBox()
              : _QuoteInputPreview(
                  message: lastQuoteMessage!,
                  onCancel: widget.onCancelQuote,
                ),
        ),
        Container(
          constraints: const BoxConstraints(minHeight: 56),
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              _SendActionTypeButton(
                onContact: widget.onContactPressed,
                onFiles: widget.onFilesPressed,
                onPictures: widget.onPicturesPressed,
              ),
              const SizedBox(width: 6),
              widget.stickerAction ??
                  ChatInputAction(
                    actionKey: const Key('chat-sticker'),
                    asset: MixinAssets.sticker,
                    onPressed: widget.onStickerPressed,
                  ),
              const SizedBox(width: 16),
              Expanded(
                child: Container(
                  key: const Key('chat-input-surface'),
                  constraints: const BoxConstraints(minHeight: 40),
                  decoration: BoxDecoration(
                    color: Theme.of(context).brightness == Brightness.dark
                        ? const Color.fromRGBO(255, 255, 255, 0.08)
                        : const Color.fromRGBO(245, 247, 250, 1),
                    borderRadius: const BorderRadius.all(Radius.circular(4)),
                  ),
                  alignment: Alignment.center,
                  child: Shortcuts(
                    shortcuts: {
                      if (sendable && !mentionVisible)
                        const SingleActivator(LogicalKeyboardKey.enter):
                            const _SendInputIntent(),
                      if (widget.onSendPost != null)
                        SingleActivator(
                          LogicalKeyboardKey.enter,
                          meta: defaultTargetPlatform == TargetPlatform.macOS,
                          shift: true,
                          alt: defaultTargetPlatform != TargetPlatform.macOS,
                        ): const _SendPostInputIntent(),
                      const SingleActivator(LogicalKeyboardKey.escape):
                          const _CancelQuoteInputIntent(),
                    },
                    child: Actions(
                      actions: {
                        if (widget.onPasteFiles != null)
                          PasteTextIntent: _PasteFilesAction(
                            context,
                            widget.onPasteFiles!,
                          ),
                        _SendInputIntent: CallbackAction<_SendInputIntent>(
                          onInvoke: (_) {
                            unawaited(widget.onSend());
                            return null;
                          },
                        ),
                        _SendPostInputIntent:
                            CallbackAction<_SendPostInputIntent>(
                              onInvoke: (_) {
                                unawaited(widget.onSendPost?.call());
                                return null;
                              },
                            ),
                        _CancelQuoteInputIntent:
                            CallbackAction<_CancelQuoteInputIntent>(
                              onInvoke: (_) {
                                widget.onCancelQuote();
                                return null;
                              },
                            ),
                      },
                      child: Stack(
                        children: [
                          TextField(
                            key: const Key('chat-input'),
                            controller: widget.controller,
                            focusNode: widget.focusNode,
                            onChanged: widget.onChanged,
                            minLines: 1,
                            maxLines: 7,
                            textInputAction: TextInputAction.newline,
                            inputFormatters: [
                              LengthLimitingTextInputFormatter(_maxTextLength),
                            ],
                            textAlignVertical: TextAlignVertical.center,
                            selectionHeightStyle:
                                ui.BoxHeightStyle.includeLineSpacingMiddle,
                            contextMenuBuilder: (context, state) =>
                                MixinAdaptiveSelectionToolbar(
                                  editableTextState: state,
                                ),
                            cursorColor: context.theme.accent,
                            style: TextStyle(
                              fontSize: 14,
                              color: context.theme.text,
                            ),
                            decoration: const InputDecoration(
                              isDense: true,
                              border: InputBorder.none,
                              enabledBorder: InputBorder.none,
                              focusedBorder: InputBorder.none,
                              contentPadding: EdgeInsets.only(
                                left: 8,
                                top: 8,
                                bottom: 8,
                              ),
                            ),
                          ),
                          if (widget.controller.text.isEmpty)
                            Positioned.fill(
                              left: 8,
                              child: Align(
                                alignment: Alignment.centerLeft,
                                child: IgnorePointer(
                                  child: Text(
                                    widget.isEncryptConversation
                                        ? context.l10n.chatHintE2e
                                        : context.l10n.typeMessage,
                                    style: TextStyle(
                                      color: context.theme.secondaryText,
                                      fontSize: 14,
                                    ),
                                    maxLines: 1,
                                    overflow: TextOverflow.ellipsis,
                                  ),
                                ),
                              ),
                            ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 16),
              Stack(
                fit: StackFit.passthrough,
                children: [
                  if (sendable)
                    Transform.scale(
                      scale: 1,
                      child: ContextMenuWidget(
                        desktopMenuWidgetBuilder:
                            CustomDesktopMenuWidgetBuilder(),
                        menuProvider: (_) => Menu(
                          children: [
                            MenuAction(
                              image: MenuImage.icon(IconFonts.mute),
                              title: context.l10n.sendWithoutSound,
                              callback: widget.onSendSilent ?? widget.onSend,
                            ),
                          ],
                        ),
                        child: ChatInputAction(
                          actionKey: const Key('chat-send'),
                          asset: MixinAssets.send,
                          onPressed: widget.onSend,
                        ),
                      ),
                    )
                  else
                    Transform.scale(
                      scale: 1,
                      child: ChatInputAction(
                        actionKey: const Key('chat-voice'),
                        asset: MixinAssets.microphone,
                        active: widget.voiceButtonActive,
                        onPressed: widget.onVoicePressed,
                      ),
                    ),
                ],
              ),
            ],
          ),
        ),
      ],
    ),
  );
}

class MentionTextEditingController extends TextEditingController {
  MentionTextEditingController({super.text});

  final Set<String> _identityNumbers = {};

  void markMention(String identityNumber) {
    _identityNumbers.add(identityNumber);
    notifyListeners();
  }

  void cacheMentionNames(Map<String, String> names) {
    final previousLength = _identityNumbers.length;
    _identityNumbers.addAll(names.keys);
    if (_identityNumbers.length != previousLength) notifyListeners();
  }

  @override
  TextSpan buildTextSpan({
    required BuildContext context,
    required bool withComposing,
    TextStyle? style,
  }) {
    TextSpan build(String text, TextStyle? textStyle) => TextSpan(
      style: textStyle,
      children: TextMatcher.applyTextMatchers(
        [
          TextSpan(text: text, style: textStyle),
        ],
        [
          EmojiTextMatcher(),
          TextMatcher.regExp(
            regExp: RegExp(r'@(\d{4,})'),
            matchBuilder: (span, displayString, linkString) {
              final identityNumber = linkString.substring(1);
              return TextSpan(
                text: displayString,
                style: _identityNumbers.contains(identityNumber)
                    ? (span.style ?? const TextStyle()).merge(
                        TextStyle(color: context.theme.accent),
                      )
                    : span.style,
              );
            },
          ),
        ],
      ).toList(),
    );

    if (!value.isComposingRangeValid || !withComposing) {
      return build(text, style);
    }
    return TextSpan(
      style: style,
      children: [
        build(value.composing.textBefore(value.text), style),
        build(
          value.composing.textInside(value.text),
          style?.merge(const TextStyle(decoration: TextDecoration.underline)),
        ),
        build(value.composing.textAfter(value.text), style),
      ],
    );
  }
}

class _MentionPanelPortal extends HookWidget {
  const _MentionPanelPortal({
    required this.textEditingController,
    required this.child,
    this.mentions,
  });

  final TextEditingController textEditingController;
  final MentionController? mentions;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final visible = mentions?.users.isNotEmpty ?? false;
    final selectable =
        useValueListenable(textEditingController).composing.isCollapsed &&
        visible;
    return LayoutBuilder(
      builder: (context, constraints) => FocusableActionDetector(
        enabled: selectable,
        shortcuts: {
          const SingleActivator(LogicalKeyboardKey.arrowDown):
              const _MoveMentionIntent(1),
          const SingleActivator(LogicalKeyboardKey.arrowUp):
              const _MoveMentionIntent(-1),
          const SingleActivator(LogicalKeyboardKey.tab):
              const _MoveMentionIntent(1),
          const SingleActivator(LogicalKeyboardKey.enter):
              const _SelectMentionIntent(),
          if (defaultTargetPlatform == TargetPlatform.macOS) ...{
            const SingleActivator(LogicalKeyboardKey.keyN, control: true):
                const _MoveMentionIntent(1),
            const SingleActivator(LogicalKeyboardKey.keyP, control: true):
                const _MoveMentionIntent(-1),
          },
        },
        actions: {
          _MoveMentionIntent: CallbackAction<_MoveMentionIntent>(
            onInvoke: (intent) {
              mentions?.move(intent.delta);
              return null;
            },
          ),
          _SelectMentionIntent: CallbackAction<_SelectMentionIntent>(
            onInvoke: (_) {
              mentions?.select();
              return null;
            },
          ),
        },
        child: PortalTarget(
          visible: visible,
          anchor: const Aligned(
            follower: Alignment.bottomCenter,
            target: Alignment.topCenter,
          ),
          closeDuration: const Duration(milliseconds: 150),
          portalFollower: ConstrainedBox(
            constraints: BoxConstraints(
              minWidth: constraints.maxWidth,
              maxWidth: constraints.maxWidth,
              maxHeight: 200,
            ),
            child: ClipRRect(
              child: TweenAnimationBuilder<double>(
                duration: const Duration(milliseconds: 150),
                curve: Curves.easeOut,
                tween: Tween(begin: 0, end: visible ? 1 : 0),
                builder: (context, progress, child) => FractionalTranslation(
                  translation: Offset(0, 1 - progress),
                  child: child,
                ),
                child: _MentionPanel(
                  users: mentions?.users ?? const [],
                  keyword: mentions?.keyword ?? '',
                  selectedIndex: mentions?.index ?? 0,
                  scrollController: mentions?.scrollController,
                  onSelected: mentions?.select,
                ),
              ),
            ),
          ),
          child: child,
        ),
      ),
    );
  }
}

class _MentionPanel extends StatelessWidget {
  const _MentionPanel({
    required this.users,
    required this.keyword,
    required this.selectedIndex,
    required this.onSelected,
    this.scrollController,
  });

  final List<rust.ConversationParticipantItem> users;
  final String keyword;
  final int selectedIndex;
  final ScrollController? scrollController;
  final ValueChanged<int?>? onSelected;

  @override
  Widget build(BuildContext context) => DecoratedBox(
    decoration: BoxDecoration(color: context.theme.popUp),
    child: ListView.builder(
      controller: scrollController,
      itemCount: users.length,
      shrinkWrap: true,
      itemBuilder: (context, index) {
        final user = users[index];
        return InteractiveDecoratedBox.color(
          decoration: selectedIndex == index
              ? BoxDecoration(color: context.theme.listSelected)
              : null,
          onTap: () => onSelected?.call(index),
          child: Container(
            height: 50,
            padding: const EdgeInsets.all(8),
            child: Row(
              children: [
                AvatarView(
                  userId: user.userId,
                  name: user.fullName,
                  avatarUrl: user.avatarUrl,
                  size: 32,
                ),
                const SizedBox(width: 6),
                Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    CustomText(
                      user.fullName,
                      style: TextStyle(
                        fontSize: 14,
                        color: context.theme.text,
                        height: 1,
                      ),
                      textMatchers: [
                        EmojiTextMatcher(),
                        if (keyword.trim().isNotEmpty)
                          MultiKeyWordTextMatcher.createKeywordMatcher(
                            keyword: keyword,
                            style: TextStyle(color: context.theme.accent),
                            caseSensitive: false,
                          ),
                      ],
                      maxLines: 1,
                    ),
                    const SizedBox(height: 2),
                    CustomText(
                      user.identityNumber,
                      style: TextStyle(
                        fontSize: 12,
                        color: context.theme.secondaryText,
                      ),
                      textMatchers: [
                        EmojiTextMatcher(),
                        if (keyword.trim().isNotEmpty)
                          MultiKeyWordTextMatcher.createKeywordMatcher(
                            keyword: keyword,
                            style: TextStyle(color: context.theme.accent),
                            caseSensitive: false,
                          ),
                      ],
                      maxLines: 1,
                    ),
                  ],
                ),
              ],
            ),
          ),
        );
      },
    ),
  );
}

class _MoveMentionIntent extends Intent {
  const _MoveMentionIntent(this.delta);

  final int delta;
}

class _SelectMentionIntent extends Intent {
  const _SelectMentionIntent();
}

class _SendInputIntent extends Intent {
  const _SendInputIntent();
}

class _SendPostInputIntent extends Intent {
  const _SendPostInputIntent();
}

class _CancelQuoteInputIntent extends Intent {
  const _CancelQuoteInputIntent();
}

class _PasteFilesAction extends Action<PasteTextIntent> {
  _PasteFilesAction(this.context, this.onPasteFiles);

  final BuildContext context;
  final Future<void> Function(List<XFile> files) onPasteFiles;

  @override
  Object? invoke(PasteTextIntent intent) {
    final defaultAction = callingAction;
    scheduleMicrotask(() async {
      final files = await readClipboardFiles();
      if (files.isEmpty) {
        defaultAction?.invoke(intent);
        return;
      }
      if (!context.mounted) return;
      await onPasteFiles(
        files.map((file) => XFile(file.path)).toList(growable: false),
      );
    });
    return null;
  }
}

enum _SendActionType { contact, files, picturesAndVideos }

class _SendActionTypeButton extends StatefulWidget {
  const _SendActionTypeButton({
    required this.onContact,
    required this.onFiles,
    required this.onPictures,
  });

  final VoidCallback? onContact;
  final VoidCallback? onFiles;
  final VoidCallback? onPictures;

  @override
  State<_SendActionTypeButton> createState() => _SendActionTypeButtonState();
}

class _SendActionTypeButtonState extends State<_SendActionTypeButton>
    with SingleTickerProviderStateMixin {
  late final AnimationController animationController = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 200),
  );

  @override
  void dispose() {
    animationController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(left: 6),
    child: AnimatedBuilder(
      animation: animationController,
      builder: (context, child) {
        final value = Curves.easeInOut.transform(animationController.value);
        final scale = value < 0.5
            ? 1 - 0.2 * Curves.easeIn.transform(value * 2)
            : 0.8 + 0.2 * Curves.easeOut.transform((value - 0.5) * 2);
        return Transform.scale(
          scale: scale,
          child: Transform.rotate(angle: value * math.pi / 4, child: child),
        );
      },
      child: CustomPopupMenuButton<_SendActionType>(
        key: const Key('chat-add'),
        alignment: Alignment.topCenter,
        icon: MixinAssets.add,
        color: context.theme.icon,
        size: 32,
        padding: const EdgeInsets.all(4),
        onVisibilityChanged: (value) {
          if (!mounted) return;
          if (value) {
            animationController.forward();
          } else {
            animationController.reverse();
          }
        },
        itemBuilder: (context) => [
          CustomPopupMenuItem(
            icon: MixinAssets.contact,
            title: context.l10n.contact,
            value: _SendActionType.contact,
          ),
          CustomPopupMenuItem(
            icon: MixinAssets.file,
            title: context.l10n.files,
            value: _SendActionType.files,
          ),
          CustomPopupMenuItem(
            icon: MixinAssets.filePreviewImages,
            title: context.l10n.picturesAndVideos,
            value: _SendActionType.picturesAndVideos,
          ),
        ],
        onSelected: (value) {
          switch (value) {
            case _SendActionType.contact:
              widget.onContact?.call();
            case _SendActionType.files:
              widget.onFiles?.call();
            case _SendActionType.picturesAndVideos:
              widget.onPictures?.call();
          }
        },
      ),
    ),
  );
}

class ChatInputAction extends StatelessWidget {
  const ChatInputAction({
    required this.asset,
    required this.actionKey,
    this.onPressed,
    this.active = false,
    super.key,
  });

  final String asset;
  final Key actionKey;
  final VoidCallback? onPressed;
  final bool active;

  @override
  Widget build(BuildContext context) => ActionButton(
    key: actionKey,
    name: asset,
    onTap: onPressed,
    color: active ? context.theme.accent : context.theme.icon,
  );
}

class _QuoteInputPreview extends StatelessWidget {
  const _QuoteInputPreview({required this.message, required this.onCancel});

  final MessageListEntry message;
  final VoidCallback onCancel;

  @override
  Widget build(BuildContext context) {
    final color = nameColorForId(message.senderId) ?? context.theme.accent;
    final iconAsset = MixinAssets.messageIcon(message.category);
    final source = message.isSticker
        ? message.stickerAssetUrl
        : message.mediaUrl ?? message.thumbUrl ?? message.thumbImage;
    final imageProvider = source == null
        ? null
        : imageProviderForSource(source);
    Widget? image;
    if (message.category.endsWith('_CONTACT')) {
      image = AvatarView(
        userId: message.sharedUserId ?? '',
        name: message.sharedUserFullName ?? '',
        avatarUrl: message.sharedUserAvatarUrl ?? '',
        size: 48,
      );
    } else if (imageProvider != null) {
      image = Image(image: imageProvider, fit: BoxFit.cover);
    }

    return DecoratedBox(
      key: const Key('chat-quote-preview'),
      decoration: BoxDecoration(color: context.theme.popUp),
      child: Row(
        children: [
          Expanded(
            child: Container(
              constraints: const BoxConstraints(minHeight: 50),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Flexible(
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Container(
                          key: const Key('chat-quote-accent'),
                          width: 6,
                          color: color,
                        ),
                        Flexible(
                          child: Padding(
                            padding: const EdgeInsets.only(
                              top: 6,
                              left: 6,
                              bottom: 6,
                            ),
                            child: Column(
                              mainAxisSize: MainAxisSize.min,
                              crossAxisAlignment: CrossAxisAlignment.start,
                              mainAxisAlignment: MainAxisAlignment.spaceBetween,
                              children: [
                                Padding(
                                  padding: const EdgeInsets.only(bottom: 4),
                                  child: Row(
                                    mainAxisSize: MainAxisSize.min,
                                    children: [
                                      Flexible(
                                        child: Text(
                                          message.senderName,
                                          maxLines: 1,
                                          overflow: TextOverflow.ellipsis,
                                          style: TextStyle(
                                            color: color,
                                            fontSize: context
                                                .messageStyle
                                                .secondaryFontSize,
                                            height: 1,
                                          ),
                                        ),
                                      ),
                                      BadgesWidget(
                                        verified: false,
                                        isBot: false,
                                        membership: message.senderMembership,
                                      ),
                                    ],
                                  ),
                                ),
                                Row(
                                  mainAxisSize: MainAxisSize.min,
                                  children: [
                                    if (iconAsset != null)
                                      Padding(
                                        padding: const EdgeInsets.only(
                                          right: 4,
                                        ),
                                        child: SvgPicture.asset(
                                          iconAsset,
                                          width: 16,
                                          height: 16,
                                          colorFilter: ColorFilter.mode(
                                            context.theme.secondaryText,
                                            BlendMode.srcIn,
                                          ),
                                        ),
                                      ),
                                    Flexible(
                                      child: Text(
                                        messagePreview(context, message),
                                        maxLines: 1,
                                        overflow: TextOverflow.ellipsis,
                                        style: TextStyle(
                                          color: context.theme.secondaryText,
                                          fontSize: context
                                              .messageStyle
                                              .tertiaryFontSize,
                                        ),
                                      ),
                                    ),
                                  ],
                                ),
                              ],
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 8),
                  if (image != null)
                    SizedBox.square(
                      dimension: 48,
                      child: RepaintBoundary(
                        child: ClipRRect(
                          borderRadius: const BorderRadius.all(
                            Radius.circular(6),
                          ),
                          child: image,
                        ),
                      ),
                    ),
                ],
              ),
            ),
          ),
          GestureDetector(
            key: const Key('chat-quote-cancel'),
            onTap: onCancel,
            behavior: HitTestBehavior.opaque,
            child: Container(
              padding: const EdgeInsets.all(14),
              child: SvgPicture.asset(
                MixinAssets.closeOval,
                height: 22,
                width: 22,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
