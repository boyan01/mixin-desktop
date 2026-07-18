import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/action_button.dart';
import 'package:mixin_desktop_ui/widgets/interactive_decorated_box.dart';
import 'package:mixin_desktop_ui/widgets/message_audio.dart';

class AudioPlayerBar extends StatefulWidget {
  const AudioPlayerBar({
    required this.selectedConversationId,
    required this.findConversation,
    required this.onConversationSelected,
    super.key,
  });

  final String? selectedConversationId;
  final Future<ConversationListEntry?> Function(String conversationId)
  findConversation;
  final ValueChanged<ConversationListEntry> onConversationSelected;

  @override
  State<AudioPlayerBar> createState() => _AudioPlayerBarState();
}

class _AudioPlayerBarState extends State<AudioPlayerBar> {
  final coordinator = AudioMessagePlaybackCoordinator.instance;
  ConversationListEntry? conversation;
  String? loadingConversationId;

  @override
  void initState() {
    super.initState();
    coordinator
      ..attach()
      ..addListener(_handlePlaybackChanged);
    _loadConversation();
  }

  @override
  void dispose() {
    coordinator
      ..removeListener(_handlePlaybackChanged)
      ..detach();
    super.dispose();
  }

  void _handlePlaybackChanged() {
    _loadConversation();
    if (mounted) setState(() {});
  }

  void _loadConversation() {
    final conversationId = coordinator.currentMessage?.conversationId;
    if (conversationId == null ||
        conversation?.id == conversationId ||
        loadingConversationId == conversationId) {
      return;
    }
    loadingConversationId = conversationId;
    unawaited(
      widget.findConversation(conversationId).then((value) {
        if (!mounted || loadingConversationId != conversationId) return;
        setState(() {
          conversation = value;
          loadingConversationId = null;
        });
      }),
    );
  }

  @override
  Widget build(BuildContext context) {
    final message = coordinator.currentMessage;
    if (message == null ||
        message.conversationId == widget.selectedConversationId) {
      return const SizedBox.shrink();
    }

    return InteractiveDecoratedBox(
      onTap: conversation == null
          ? null
          : () => widget.onConversationSelected(conversation!),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          SizedBox(
            height: 48,
            child: Row(
              children: [
                _PlaybackSpeedButton(coordinator: coordinator),
                ActionButton(
                  name: coordinator.isPlaying
                      ? 'assets/images/player_pause.svg'
                      : 'assets/images/player_play.svg',
                  color: context.mixinTheme.icon,
                  onTap: coordinator.isPlaying
                      ? coordinator.pause
                      : coordinator.resume,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      _ConversationIcon(conversation: conversation),
                      const SizedBox(width: 8),
                      Flexible(
                        child: Text(
                          conversation?.name ?? '',
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            color: context.mixinTheme.text,
                            fontSize: 14,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                ActionButton(
                  name: 'assets/images/ic_close.svg',
                  color: context.mixinTheme.icon,
                  onTap: coordinator.stop,
                ),
              ],
            ),
          ),
          _ProgressBar(message: message, coordinator: coordinator),
        ],
      ),
    );
  }
}

class _PlaybackSpeedButton extends StatelessWidget {
  const _PlaybackSpeedButton({required this.coordinator});

  final AudioMessagePlaybackCoordinator coordinator;

  @override
  Widget build(BuildContext context) => ActionButton(
    child: Center(
      child: Text(
        '2X',
        style: TextStyle(
          color: coordinator.speed == 2
              ? context.mixinTheme.accent
              : context.mixinTheme.secondaryText,
          fontSize: 16,
          fontWeight: FontWeight.w600,
        ),
      ),
    ),
    onTap: () => coordinator.setPlaybackRate(coordinator.speed == 1 ? 2 : 1),
  );
}

class _ConversationIcon extends StatelessWidget {
  const _ConversationIcon({required this.conversation});

  final ConversationListEntry? conversation;

  @override
  Widget build(BuildContext context) => SizedBox(
    height: 32,
    width: 40,
    child: Stack(
      fit: StackFit.expand,
      children: [
        Align(
          alignment: Alignment.centerLeft,
          child: conversation == null
              ? const SizedBox.square(dimension: 32)
              : ConversationAvatarView(conversation: conversation!, size: 32),
        ),
        Align(
          alignment: Alignment.bottomRight,
          child: SvgPicture.asset(
            'assets/images/audio.svg',
            colorFilter: ColorFilter.mode(
              context.mixinTheme.icon,
              BlendMode.srcIn,
            ),
            width: 16,
            height: 16,
          ),
        ),
      ],
    ),
  );
}

class _ProgressBar extends StatelessWidget {
  const _ProgressBar({required this.message, required this.coordinator});

  final MessageListEntry message;
  final AudioMessagePlaybackCoordinator coordinator;

  @override
  Widget build(BuildContext context) {
    final duration = int.tryParse(message.mediaDuration) ?? 0;
    final progress = duration == 0
        ? 0.0
        : (coordinator.position.inMilliseconds / duration).clamp(0.0, 1.0);
    return SizedBox(
      height: 3,
      child: LinearProgressIndicator(
        value: progress,
        backgroundColor: Colors.transparent,
        valueColor: AlwaysStoppedAnimation(context.mixinTheme.accent),
      ),
    );
  }
}
