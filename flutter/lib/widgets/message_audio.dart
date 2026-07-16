import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;

import 'package:audio_session/audio_session.dart';
import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/message_style.dart';
import 'package:ogg_opus_player/ogg_opus_player.dart';

typedef AudioMessageCallback = void Function(MessageListEntry message);

class AudioMessageWidget extends StatefulWidget {
  const AudioMessageWidget({
    required this.message,
    super.key,
    this.onMarkRead,
    this.onDownloadAttachment,
    this.onCancelAttachment,
  });

  final MessageListEntry message;
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
        final started = await _coordinator.play(widget.message.id, path);
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
    final localPath = _localMediaPath(widget.message.mediaUrl);
    final playable =
        (status == 'DONE' || status == 'READ') && localPath != null;
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
    final callbackAvailable = switch (status) {
      'CANCELED' => widget.onDownloadAttachment != null,
      'PENDING' => widget.onCancelAttachment != null,
      'DONE' || 'READ' => playable,
      _ => false,
    };

    Widget child = ConstrainedBox(
      key: Key('message-media-audio-${widget.message.id}'),
      constraints: const BoxConstraints(maxWidth: 286, minWidth: 180),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          _AudioStatusButton(status: status, playing: playing),
          const SizedBox(width: 8),
          Flexible(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                SizedBox(
                  width: 238,
                  height: 12,
                  child: CustomPaint(
                    painter: AudioWaveformPainter(
                      waveform: waveform,
                      progress: progress,
                      backgroundColor: context.theme.waveformBackground,
                      foregroundColor: context.theme.waveformForeground,
                    ),
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
    if (!callbackAvailable) return child;
    child = GestureDetector(onTap: _onTap, child: child);
    return MouseRegion(cursor: SystemMouseCursors.click, child: child);
  }
}

class _AudioStatusButton extends StatelessWidget {
  const _AudioStatusButton({required this.status, required this.playing});

  final String status;
  final bool playing;

  @override
  Widget build(BuildContext context) {
    final icon = switch (status) {
      'CANCELED' => Icons.download,
      'PENDING' => Icons.close,
      'EXPIRED' => Icons.warning_amber_rounded,
      _ => playing ? Icons.stop : Icons.play_arrow,
    };
    return Container(
      width: 38,
      height: 38,
      decoration: BoxDecoration(
        color: context.theme.statusBackground,
        shape: BoxShape.circle,
      ),
      alignment: Alignment.center,
      child: status == 'PENDING'
          ? Stack(
              alignment: Alignment.center,
              children: [
                const SizedBox.square(
                  dimension: 38,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
                Icon(icon, size: 18, color: context.theme.secondaryText),
              ],
            )
          : Icon(icon, size: 24, color: context.theme.secondaryText),
    );
  }
}

class AudioMessagePlaybackCoordinator extends ChangeNotifier {
  AudioMessagePlaybackCoordinator._();

  static final instance = AudioMessagePlaybackCoordinator._();

  OggOpusPlayer? _player;
  Timer? _positionTimer;
  int _listenerOwners = 0;
  String? currentMessageId;
  Duration position = Duration.zero;
  bool isPlaying = false;

  void attach() => _listenerOwners++;

  void detach() {
    _listenerOwners = math.max(0, _listenerOwners - 1);
    if (_listenerOwners == 0) stop();
  }

  Future<bool> play(String messageId, String path) async {
    _stop(deactivateSession: false);
    try {
      final session = await AudioSession.instance;
      await session.configure(const AudioSessionConfiguration.speech());
      await session.setActive(true);
      final player = OggOpusPlayer(path);
      _player = player;
      currentMessageId = messageId;
      player.state.addListener(_handlePlayerState);
      player.play();
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

  void _handlePlayerState() {
    final state = _player?.state.value;
    if (state == PlayerState.ended || state == PlayerState.error) {
      stop();
      return;
    }
    isPlaying = state == PlayerState.playing;
    notifyListeners();
  }

  void stop() => _stop(deactivateSession: true);

  void _stop({required bool deactivateSession}) {
    _positionTimer?.cancel();
    _positionTimer = null;
    final player = _player;
    _player = null;
    player?.state.removeListener(_handlePlayerState);
    player?.dispose();
    currentMessageId = null;
    position = Duration.zero;
    isPlaying = false;
    if (deactivateSession) unawaited(_deactivateAudioSession());
    notifyListeners();
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

class AudioWaveformPainter extends CustomPainter {
  const AudioWaveformPainter({
    required this.waveform,
    required this.progress,
    required this.backgroundColor,
    required this.foregroundColor,
  });

  final List<int> waveform;
  final double progress;
  final Color backgroundColor;
  final Color foregroundColor;

  @override
  void paint(Canvas canvas, Size size) {
    final count = math.max(1, (size.width / 4).floor());
    final backgroundPaint = Paint()
      ..color = backgroundColor
      ..strokeWidth = 2
      ..strokeCap = StrokeCap.round;
    final foregroundPaint = Paint()
      ..color = foregroundColor
      ..strokeWidth = 2
      ..strokeCap = StrokeCap.round;
    for (var index = 0; index < count; index++) {
      final sampleIndex = waveform.isEmpty
          ? index
          : (index * waveform.length / count).floor().clamp(
              0,
              waveform.length - 1,
            );
      final sample = waveform.isEmpty
          ? const [0.35, 0.65, 0.9, 0.5, 0.75, 0.4][index % 6]
          : waveform[sampleIndex] / 255;
      final height = (size.height * sample.clamp(0.2, 1.0)).toDouble();
      final x = index * size.width / count + 1;
      final paint = index / count <= progress
          ? foregroundPaint
          : backgroundPaint;
      canvas.drawLine(
        Offset(x, (size.height - height) / 2),
        Offset(x, (size.height + height) / 2),
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(covariant AudioWaveformPainter oldDelegate) =>
      oldDelegate.waveform != waveform ||
      oldDelegate.progress != progress ||
      oldDelegate.backgroundColor != backgroundColor ||
      oldDelegate.foregroundColor != foregroundColor;
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
  final minutes = duration.inMinutes;
  final seconds = duration.inSeconds.remainder(60);
  return '$minutes:${seconds.toString().padLeft(2, '0')}';
}
