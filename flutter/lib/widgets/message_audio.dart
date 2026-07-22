import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;

import 'package:audio_session/audio_session.dart';
import 'package:flutter/material.dart';
import 'package:ogg_opus_player/ogg_opus_player.dart';

import '../models/message_list_entry.dart';
import '../theme.dart';
import 'attachment_status.dart';
import 'interactive_decorated_box.dart';
import 'message_style.dart';
import 'waveform_widget.dart';

typedef AudioMessageCallback = void Function(MessageListEntry message);

class AudioMessageWidget extends StatefulWidget {
  const AudioMessageWidget({
    required this.message,
    super.key,
    this.playlist = const [],
    this.sentByCurrentUser,
    this.onMarkRead,
    this.onDownloadAttachment,
    this.onCancelAttachment,
  });

  final MessageListEntry message;
  final List<MessageListEntry> playlist;
  final bool? sentByCurrentUser;
  final AudioMessageCallback? onMarkRead;
  final AudioMessageCallback? onDownloadAttachment;
  final AudioMessageCallback? onCancelAttachment;

  @override
  State<AudioMessageWidget> createState() => _AudioMessageWidgetState();
}

class _AudioMessageWidgetState extends State<AudioMessageWidget> {
  final _coordinator = AudioMessagePlaybackCoordinator.instance;
  bool _markedRead = false;

  @override
  void initState() {
    super.initState();
    _coordinator
      ..attach()
      ..addListener(_onPlaybackChanged);
  }

  @override
  void didUpdateWidget(covariant AudioMessageWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.message.id != widget.message.id) {
      if (_coordinator.currentMessageId == oldWidget.message.id) {
        _coordinator.stop();
      }
      _markedRead = false;
    }
  }

  @override
  void dispose() {
    _coordinator
      ..removeListener(_onPlaybackChanged)
      ..detach();
    super.dispose();
  }

  void _onPlaybackChanged() {
    if (mounted) setState(() {});
  }

  Future<void> _onTap() async {
    final status = widget.message.mediaStatus.toUpperCase();
    switch (status) {
      case 'CANCELED':
        widget.onDownloadAttachment?.call(widget.message);
        return;
      case 'PENDING':
        widget.onCancelAttachment?.call(widget.message);
        return;
      case 'DONE':
      case 'READ':
        final path = _localMediaPath(widget.message.mediaUrl);
        if (path == null) return;
        if (_coordinator.currentMessageId == widget.message.id &&
            _coordinator.isPlaying) {
          _coordinator.stop();
          return;
        }
        final started = await _coordinator.play(
          widget.message,
          path,
          playlist: widget.playlist,
          onMarkRead: widget.onMarkRead,
        );
        if (started && status == 'DONE' && !_markedRead) {
          _markedRead = true;
          widget.onMarkRead?.call(widget.message);
        }
        return;
      default:
        return;
    }
  }

  @override
  Widget build(BuildContext context) {
    final status = widget.message.mediaStatus.toUpperCase();
    final sentByCurrentUser =
        widget.sentByCurrentUser ??
        widget.message.senderRelationship.toUpperCase() == 'ME';
    final isCurrentUser =
        widget.message.senderRelationship.toUpperCase() == 'ME';
    final playing =
        _coordinator.currentMessageId == widget.message.id &&
        _coordinator.isPlaying;
    final duration = Duration(
      milliseconds: int.tryParse(widget.message.mediaDuration) ?? 0,
    );
    final progress = playing && duration.inMilliseconds > 0
        ? (_coordinator.position.inMilliseconds / duration.inMilliseconds)
              .clamp(0.0, 1.0)
        : 0.0;
    final waveform = _decodeWaveform(widget.message.mediaWaveform);
    final useReadWaveform = isCurrentUser || status == 'READ';
    return InteractiveDecoratedBox(
      onTap: _onTap,
      child: Row(
        key: Key('message-media-audio-${widget.message.id}'),
        mainAxisSize: MainAxisSize.min,
        children: [
          _AudioStatusButton(
            messageId: widget.message.id,
            status: status,
            playing: playing,
            upload:
                status == 'CANCELED' &&
                sentByCurrentUser &&
                (widget.message.mediaUrl?.isNotEmpty ?? false),
          ),
          const SizedBox(width: 8),
          Flexible(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                SizedBox(
                  key: Key('audio-waveform-${widget.message.id}'),
                  width: 238,
                  height: 12,
                  child: WaveformWidget(
                    value: progress,
                    waveform: waveform,
                    backgroundColor: useReadWaveform
                        ? context.theme.waveformBackground
                        : context.theme.accent,
                    foregroundColor: useReadWaveform
                        ? context.theme.waveformForeground
                        : context.theme.accent,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  _formatDuration(duration),
                  style: TextStyle(
                    fontSize: context.messageStyle.tertiaryFontSize,
                    color: context.theme.secondaryText,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _AudioStatusButton extends StatelessWidget {
  const _AudioStatusButton({
    required this.messageId,
    required this.status,
    required this.playing,
    required this.upload,
  });

  final String messageId;
  final String status;
  final bool playing;
  final bool upload;

  @override
  Widget build(BuildContext context) => KeyedSubtree(
    key: Key('audio-status-$messageId-$status-$playing-$upload'),
    child: switch (status) {
      'CANCELED' =>
        upload
            ? const AttachmentStatusUpload()
            : const AttachmentStatusDownload(),
      'PENDING' => AttachmentStatusPending(messageId: messageId),
      'EXPIRED' => const AttachmentStatusWarning(),
      _ =>
        playing
            ? const AttachmentStatusAudioStop()
            : const AttachmentStatusAudioPlay(),
    },
  );
}

class AudioMessagePlaybackCoordinator extends ChangeNotifier {
  AudioMessagePlaybackCoordinator._();

  static final instance = AudioMessagePlaybackCoordinator._();

  OggOpusPlayer? _player;
  Timer? _positionTimer;
  int _listenerOwners = 0;
  String? _currentMessageId;
  MessageListEntry? currentMessage;
  Duration position = Duration.zero;
  bool isPlaying = false;
  double speed = 1;
  List<MessageListEntry> _playlist = const [];
  int _playlistIndex = -1;
  AudioMessageCallback? _onMarkRead;

  String? get currentMessageId => _currentMessageId;

  void attach() => _listenerOwners++;

  void detach() {
    _listenerOwners = math.max(0, _listenerOwners - 1);
    if (_listenerOwners == 0) stop();
  }

  Future<bool> play(
    MessageListEntry message,
    String path, {
    List<MessageListEntry> playlist = const [],
    AudioMessageCallback? onMarkRead,
  }) async {
    _stop(deactivateSession: false);
    _playlist = playlist.where((item) => item.isAudio).toList(growable: false);
    _playlistIndex = _playlist.indexWhere((item) => item.id == message.id);
    if (_playlistIndex < 0) {
      _playlist = [message];
      _playlistIndex = 0;
    }
    _onMarkRead = onMarkRead;
    return _start(message.id, path, message);
  }

  Future<bool> playPreview(String previewId, String path) async {
    _stop(deactivateSession: false);
    return _start(previewId, path, null);
  }

  Future<bool> _start(
    String messageId,
    String path,
    MessageListEntry? message,
  ) async {
    _disposePlayer();
    try {
      final session = await AudioSession.instance;
      await session.configure(const AudioSessionConfiguration.speech());
      await session.setActive(true);
      final player = OggOpusPlayer(path);
      _player = player;
      _currentMessageId = messageId;
      currentMessage = message;
      player.state.addListener(_handlePlayerState);
      player
        ..setPlaybackRate(speed)
        ..play();
      isPlaying = true;
      _positionTimer = Timer.periodic(const Duration(milliseconds: 50), (_) {
        final current = _player;
        if (current == null) return;
        position = Duration(
          milliseconds: (current.currentPosition * 1000).round(),
        );
        notifyListeners();
      });
      notifyListeners();
      return true;
    } catch (_) {
      stop();
      return false;
    }
  }

  void pause() => _player?.pause();

  void resume() => _player?.play();

  void setPlaybackRate(double value) {
    speed = value;
    _player?.setPlaybackRate(value);
    notifyListeners();
  }

  void _handlePlayerState() {
    final state = _player?.state.value;
    if (state == PlayerState.ended) {
      unawaited(_playNext());
      return;
    }
    if (state == PlayerState.error) {
      stop();
      return;
    }
    isPlaying = state == PlayerState.playing;
    notifyListeners();
  }

  void stop() => _stop(deactivateSession: true);

  void _stop({required bool deactivateSession}) {
    _disposePlayer();
    _playlist = const [];
    _playlistIndex = -1;
    _onMarkRead = null;
    if (deactivateSession) unawaited(_deactivateAudioSession());
    notifyListeners();
  }

  void _disposePlayer() {
    _positionTimer?.cancel();
    _positionTimer = null;
    final player = _player;
    _player = null;
    player?.state.removeListener(_handlePlayerState);
    player?.dispose();
    _currentMessageId = null;
    currentMessage = null;
    position = Duration.zero;
    isPlaying = false;
  }

  Future<void> _playNext() async {
    final nextIndex = _playlistIndex + 1;
    if (nextIndex >= _playlist.length) {
      stop();
      return;
    }
    final next = _playlist[nextIndex];
    final path = _localMediaPath(next.mediaUrl);
    if (path == null ||
        !const {'DONE', 'READ'}.contains(next.mediaStatus.toUpperCase())) {
      _playlistIndex = nextIndex;
      await _playNext();
      return;
    }
    _playlistIndex = nextIndex;
    if (next.mediaStatus.toUpperCase() == 'DONE') _onMarkRead?.call(next);
    await _start(next.id, path, next);
  }

  Future<void> _deactivateAudioSession() async {
    try {
      final session = await AudioSession.instance;
      await session.setActive(
        false,
        avAudioSessionSetActiveOptions:
            AVAudioSessionSetActiveOptions.notifyOthersOnDeactivation,
      );
    } catch (_) {}
  }
}

String? _localMediaPath(String? source) {
  final value = source?.trim();
  if (value == null || value.isEmpty) return null;
  final uri = Uri.tryParse(value);
  if (uri != null && uri.scheme == 'file') {
    final path = File.fromUri(uri).path;
    return File(path).existsSync() ? path : null;
  }
  if (uri != null && uri.hasScheme) return null;
  return File(value).existsSync() ? value : null;
}

List<int> _decodeWaveform(String? value) {
  if (value == null || value.isEmpty) return const [];
  try {
    return base64Decode(value);
  } on FormatException {
    return const [];
  }
}

String _formatDuration(Duration duration) {
  final value = duration < const Duration(seconds: 1)
      ? const Duration(seconds: 1)
      : duration;
  return '${value.inMinutes.toString().padLeft(2, '0')}:'
      '${value.inSeconds.remainder(60).toString().padLeft(2, '0')}';
}
