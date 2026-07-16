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
    final isCurrentUser =
        widget.message.senderRelationship.toUpperCase() == 'ME';
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
    final useReadWaveform = isCurrentUser || status == 'READ';
    final callbackAvailable = switch (status) {
      'CANCELED' => widget.onDownloadAttachment != null,
      'PENDING' => widget.onCancelAttachment != null,
      'DONE' || 'READ' => playable,
      _ => false,
    };

    Widget child = Row(
      key: Key('message-media-audio-${widget.message.id}'),
      mainAxisSize: MainAxisSize.min,
      children: [
        _AudioStatusButton(
          messageId: widget.message.id,
          status: status,
          playing: playing,
          upload:
              status == 'CANCELED' &&
              isCurrentUser &&
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
                child: CustomPaint(
                  painter: AudioWaveformPainter(
                    waveform: waveform,
                    progress: progress,
                    backgroundColor: useReadWaveform
                        ? context.theme.waveformBackground
                        : context.theme.accent,
                    foregroundColor: useReadWaveform
                        ? context.theme.waveformForeground
                        : context.theme.accent,
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
    );
    if (!callbackAvailable) return child;
    child = GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: _onTap,
      child: child,
    );
    return MouseRegion(cursor: SystemMouseCursors.click, child: child);
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
  Widget build(BuildContext context) {
    final glyph = switch (status) {
      'CANCELED' => upload ? _AudioGlyph.upload : _AudioGlyph.download,
      'PENDING' => _AudioGlyph.pending,
      'EXPIRED' => _AudioGlyph.warning,
      _ => playing ? _AudioGlyph.pause : _AudioGlyph.play,
    };
    return Container(
      key: Key('audio-status-$messageId-${glyph.name}'),
      width: 38,
      height: 38,
      decoration: BoxDecoration(
        color: context.theme.statusBackground,
        shape: BoxShape.circle,
      ),
      alignment: Alignment.center,
      child: glyph == _AudioGlyph.pending
          ? Stack(
              alignment: Alignment.center,
              children: [
                SizedBox.square(
                  dimension: 38,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    color: context.theme.accent,
                  ),
                ),
                Container(width: 10, height: 10, color: context.theme.accent),
              ],
            )
          : CustomPaint(
              size: _glyphSize(glyph),
              painter: _AudioGlyphPainter(
                glyph: glyph,
                color: glyph == _AudioGlyph.warning
                    ? context.theme.text
                    : context.theme.accent,
              ),
            ),
    );
  }
}

enum _AudioGlyph { play, pause, download, upload, pending, warning }

Size _glyphSize(_AudioGlyph glyph) => switch (glyph) {
  _AudioGlyph.play => const Size(11, 12),
  _AudioGlyph.pause => const Size(12, 14),
  _AudioGlyph.download || _AudioGlyph.upload => const Size(14, 14),
  _AudioGlyph.warning => const Size(18, 18),
  _AudioGlyph.pending => Size.zero,
};

class _AudioGlyphPainter extends CustomPainter {
  const _AudioGlyphPainter({required this.glyph, required this.color});

  final _AudioGlyph glyph;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..strokeWidth = 2
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round;
    switch (glyph) {
      case _AudioGlyph.play:
        canvas.drawPath(
          Path()
            ..moveTo(1, 1)
            ..lineTo(size.width - 1, size.height / 2)
            ..lineTo(1, size.height - 1)
            ..close(),
          paint..style = PaintingStyle.fill,
        );
      case _AudioGlyph.pause:
        paint.style = PaintingStyle.fill;
        canvas
          ..drawRRect(
            RRect.fromRectAndRadius(
              Rect.fromLTWH(0, 0, 4, size.height),
              const Radius.circular(2),
            ),
            paint,
          )
          ..drawRRect(
            RRect.fromRectAndRadius(
              Rect.fromLTWH(8, 0, 4, size.height),
              const Radius.circular(2),
            ),
            paint,
          );
      case _AudioGlyph.download:
      case _AudioGlyph.upload:
        paint.style = PaintingStyle.stroke;
        final isUpload = glyph == _AudioGlyph.upload;
        final startY = isUpload ? size.height - 1 : 1.0;
        final endY = isUpload ? 1.0 : size.height - 1;
        canvas
          ..drawLine(
            Offset(size.width / 2, startY),
            Offset(size.width / 2, endY),
            paint,
          )
          ..drawLine(
            Offset(size.width / 2, endY),
            Offset(1, isUpload ? 6 : 8),
            paint,
          )
          ..drawLine(
            Offset(size.width / 2, endY),
            Offset(size.width - 1, isUpload ? 6 : 8),
            paint,
          );
      case _AudioGlyph.warning:
        paint.style = PaintingStyle.stroke;
        canvas.drawCircle(size.center(Offset.zero), 8, paint);
        paint.style = PaintingStyle.fill;
        canvas
          ..drawRRect(
            RRect.fromRectAndRadius(
              Rect.fromLTWH(8, 4, 2, 7),
              const Radius.circular(1),
            ),
            paint,
          )
          ..drawCircle(const Offset(9, 14), 1, paint);
      case _AudioGlyph.pending:
        break;
    }
  }

  @override
  bool shouldRepaint(covariant _AudioGlyphPainter oldDelegate) =>
      glyph != oldDelegate.glyph || color != oldDelegate.color;
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
    const barWidth = 2.0;
    const barSpacing = 2.0;
    const minBarHeight = 2.0;
    final count = math.min(
      60,
      math.max(
        1,
        ((size.width + barSpacing) / (barWidth + barSpacing)).floor(),
      ),
    );
    final samples = List<int>.filled(count, 0);
    for (var index = 0; index < waveform.length; index++) {
      final target = (index * count / waveform.length).floor().clamp(
        0,
        count - 1,
      );
      samples[target] = math.max(samples[target], waveform[index]);
    }
    final maxSample = samples.reduce(math.max);
    final ratio = maxSample == 0 ? 0.0 : size.height / maxSample;
    final path = Path();
    for (var index = 0; index < count; index++) {
      final height = math.max(minBarHeight, samples[index] * ratio);
      final left = index * (barWidth + barSpacing);
      path.addRRect(
        RRect.fromRectAndCorners(
          Rect.fromLTWH(left, size.height - height, barWidth, height),
          topLeft: const Radius.circular(1),
          topRight: const Radius.circular(1),
        ),
      );
    }
    final progressX = size.width * progress;
    final foregroundPath = Path.combine(
      PathOperation.intersect,
      Path()..addRect(Rect.fromLTRB(0, 0, progressX, size.height)),
      path,
    );
    final backgroundPath = Path.combine(
      PathOperation.intersect,
      Path()..addRect(Rect.fromLTRB(progressX, 0, size.width, size.height)),
      path,
    );
    canvas
      ..drawPath(foregroundPath, Paint()..color = foregroundColor)
      ..drawPath(backgroundPath, Paint()..color = backgroundColor);
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
