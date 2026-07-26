import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models/message_list_entry.dart';
import '../src/rust/api/media.dart';
import '../theme.dart';
import '../utils/app_logger.dart';
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
  late final AudioMessagePlaybackCoordinator _coordinator;
  bool _markedRead = false;

  @override
  void initState() {
    super.initState();
    _coordinator = context.read<AudioMessagePlaybackCoordinator>();
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

abstract interface class AudioPlaybackBackend {
  Stream<MediaPlaybackEvent> events();

  Future<void> play({
    required List<MediaAudioItem> playlist,
    required BigInt startIndex,
  });

  Future<void> pause();

  Future<void> resume();

  void stop();

  Future<void> setSpeed(double speed);
}

class RustAudioPlaybackBackend implements AudioPlaybackBackend {
  RustAudioPlaybackBackend(this._media);

  final MediaHandle _media;

  @override
  Stream<MediaPlaybackEvent> events() => _media.audioPlaybackEvents();

  @override
  Future<void> play({
    required List<MediaAudioItem> playlist,
    required BigInt startIndex,
  }) => _media.playAudio(playlist: playlist, startIndex: startIndex);

  @override
  Future<void> pause() => _media.pauseAudio();

  @override
  Future<void> resume() => _media.resumeAudio();

  @override
  void stop() => _media.stopAudio();

  @override
  Future<void> setSpeed(double speed) => _media.setAudioSpeed(speed: speed);
}

class AudioMessagePlaybackCoordinator extends ChangeNotifier {
  AudioMessagePlaybackCoordinator({required this._backend}) {
    _events = _backend.events().listen(
      _handleEvent,
      onError: (Object error, StackTrace stackTrace) {
        e('Observe message audio playback failed', error, stackTrace);
        _clearPlayback();
      },
    );
    _positionTimer = Timer.periodic(
      const Duration(milliseconds: 50),
      (_) => _updatePosition(),
    );
  }

  final AudioPlaybackBackend _backend;
  late final StreamSubscription<MediaPlaybackEvent> _events;
  late final Timer _positionTimer;
  int _listenerOwners = 0;
  String? _currentMessageId;
  MessageListEntry? currentMessage;
  Duration position = Duration.zero;
  bool isPlaying = false;
  double speed = 1;
  Map<String, MessageListEntry> _messagesById = const {};
  AudioMessageCallback? _onMarkRead;
  Duration _anchorPosition = Duration.zero;
  Duration _duration = Duration.zero;
  Stopwatch? _positionClock;

  String? get currentMessageId => _currentMessageId;

  void attach() => _listenerOwners++;

  void detach() {
    _listenerOwners = math.max(0, _listenerOwners - 1);
    if (_listenerOwners == 0) {
      _backend.stop();
      _messagesById = const {};
      _onMarkRead = null;
      _clearPlayback(notify: false);
    }
  }

  Future<bool> play(
    MessageListEntry message,
    String path, {
    List<MessageListEntry> playlist = const [],
    AudioMessageCallback? onMarkRead,
  }) async {
    final candidates = playlist.where(_isPlayable).toList(growable: true);
    var startIndex = candidates.indexWhere((item) => item.id == message.id);
    if (startIndex < 0) {
      candidates.insert(0, message);
      startIndex = 0;
    }
    _messagesById = {for (final item in candidates) item.id: item};
    _onMarkRead = onMarkRead;
    return _start(
      playlist: [
        for (final item in candidates)
          MediaAudioItem(
            id: item.id,
            path: item.id == message.id
                ? path
                : _localMediaPath(item.mediaUrl)!,
            durationMillis: BigInt.from(
              int.tryParse(item.mediaDuration) ?? 0,
            ),
          ),
      ],
      startIndex: startIndex,
    );
  }

  Future<bool> playPreview(String previewId, String path) async {
    _messagesById = const {};
    _onMarkRead = null;
    return _start(
      playlist: [
        MediaAudioItem(
          id: previewId,
          path: path,
          durationMillis: BigInt.zero,
        ),
      ],
      startIndex: 0,
    );
  }

  Future<bool> _start({
    required List<MediaAudioItem> playlist,
    required int startIndex,
  }) async {
    try {
      await _backend.play(
        playlist: playlist,
        startIndex: BigInt.from(startIndex),
      );
      return true;
    } catch (error, stackTrace) {
      e('Start message audio playback failed', error, stackTrace);
      stop();
      return false;
    }
  }

  Future<void> pause() => _runPlaybackCommand(
    _backend.pause,
    'Pause message audio playback failed',
  );

  Future<void> resume() => _runPlaybackCommand(
    _backend.resume,
    'Resume message audio playback failed',
  );

  void setPlaybackRate(double value) {
    unawaited(
      _runPlaybackCommand(
        () => _backend.setSpeed(value),
        'Set message audio playback speed failed',
      ),
    );
  }

  void stop() {
    _backend.stop();
    _messagesById = const {};
    _onMarkRead = null;
    _clearPlayback();
  }

  void _handleEvent(MediaPlaybackEvent event) {
    switch (event) {
      case MediaPlaybackEvent_Changed(:final snapshot):
        final item = snapshot.item;
        final nextMessage = item == null ? null : _messagesById[item.id];
        if (nextMessage != null &&
            nextMessage.id != currentMessage?.id &&
            nextMessage.mediaStatus.toUpperCase() == 'DONE') {
          _onMarkRead?.call(nextMessage);
        }
        _currentMessageId = item?.id;
        currentMessage = nextMessage;
        _anchorPosition = Duration(
          milliseconds: snapshot.positionMillis.toInt(),
        );
        _duration = Duration(milliseconds: snapshot.durationMillis.toInt());
        position = _anchorPosition;
        speed = snapshot.speed;
        isPlaying = snapshot.status == MediaPlaybackStatus.playing;
        _resetPositionClock();
        notifyListeners();
      case MediaPlaybackEvent_Finished():
        break;
      case MediaPlaybackEvent_Failed(:final message):
        e('Message audio playback failed: $message');
        _clearPlayback();
    }
  }

  void _updatePosition() {
    final clock = _positionClock;
    if (!isPlaying || clock == null) return;
    final elapsedMillis = (clock.elapsedMilliseconds * speed).round();
    final next = _anchorPosition + Duration(milliseconds: elapsedMillis);
    position = next > _duration ? _duration : next;
    notifyListeners();
  }

  void _resetPositionClock() {
    _positionClock = isPlaying ? (Stopwatch()..start()) : null;
  }

  void _clearPlayback({bool notify = true}) {
    _currentMessageId = null;
    currentMessage = null;
    position = Duration.zero;
    _anchorPosition = Duration.zero;
    _duration = Duration.zero;
    _positionClock = null;
    isPlaying = false;
    if (notify) notifyListeners();
  }

  Future<void> _runPlaybackCommand(
    Future<void> Function() command,
    String failureMessage,
  ) async {
    try {
      await command();
    } catch (error, stackTrace) {
      e(failureMessage, error, stackTrace);
    }
  }

  bool _isPlayable(MessageListEntry message) =>
      message.isAudio &&
      const {'DONE', 'READ'}.contains(message.mediaStatus.toUpperCase()) &&
      _localMediaPath(message.mediaUrl) != null;

  @override
  void dispose() {
    _positionTimer.cancel();
    unawaited(_events.cancel());
    _backend.stop();
    super.dispose();
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
