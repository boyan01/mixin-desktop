import 'dart:async';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../constants/assets.dart';
import '../../controllers/voice_recorder_controller.dart';
import '../../l10n/l10n.dart';
import '../../theme.dart';
import '../action_button.dart';
import '../message_audio.dart';
import '../mixin_dialog.dart';
import '../waveform_widget.dart';

class VoiceRecorderBarOverlayComposition extends StatefulWidget {
  const VoiceRecorderBarOverlayComposition({
    required this.controller,
    required this.onCancel,
    required this.onStop,
    required this.onRetry,
    required this.onSend,
    required this.child,
    super.key,
  });

  final VoiceRecorderController controller;
  final Future<void> Function() onCancel;
  final Future<void> Function() onStop;
  final Future<void> Function() onRetry;
  final Future<void> Function() onSend;
  final Widget child;

  @override
  State<VoiceRecorderBarOverlayComposition> createState() =>
      _VoiceRecorderBarOverlayCompositionState();
}

class _VoiceRecorderBarOverlayCompositionState
    extends State<VoiceRecorderBarOverlayComposition> {
  final LayerLink _link = LayerLink();
  OverlayEntry? _entry;
  double _layoutWidth = 0;
  bool _syncScheduled = false;

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_onRecorderChanged);
  }

  @override
  void didUpdateWidget(covariant VoiceRecorderBarOverlayComposition oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.controller, widget.controller)) {
      oldWidget.controller.removeListener(_onRecorderChanged);
      widget.controller.addListener(_onRecorderChanged);
    }
    _scheduleOverlaySync();
  }

  void _onRecorderChanged() {
    _entry?.markNeedsBuild();
    _scheduleOverlaySync();
  }

  void _scheduleOverlaySync() {
    if (_syncScheduled || !mounted) return;
    _syncScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _syncScheduled = false;
      if (mounted) _syncOverlay();
    });
  }

  void _syncOverlay() {
    final recorderMode =
        widget.controller.value.status != VoiceRecorderStatus.idle;
    if (!recorderMode) {
      _entry?.remove();
      _entry = null;
      return;
    }
    if (_entry != null) {
      _entry!.markNeedsBuild();
      return;
    }
    final entry = OverlayEntry(
      builder: (context) => Stack(
        fit: StackFit.expand,
        children: [
          if (widget.controller.value.status == VoiceRecorderStatus.recording)
            GestureDetector(
              behavior: HitTestBehavior.translucent,
              onTap: () => _showDiscardRecordingWarningAlertOverlay(
                context,
                onDiscard: () => unawaited(widget.onCancel()),
              ),
              child: const SizedBox.expand(),
            ),
          UnconstrainedBox(
            child: CompositedTransformFollower(
              link: _link,
              showWhenUnlinked: false,
              targetAnchor: Alignment.bottomCenter,
              followerAnchor: Alignment.bottomCenter,
              child: SizedBox(
                width: _layoutWidth,
                child: Material(
                  child: VoiceRecorderBar(
                    state: widget.controller.value,
                    onCancel: widget.onCancel,
                    onStop: widget.onStop,
                    onRetry: widget.onRetry,
                    onSend: widget.onSend,
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
    _entry = entry;
    (Navigator.of(context).overlay ?? Overlay.of(context)).insert(entry);
  }

  @override
  void dispose() {
    widget.controller.removeListener(_onRecorderChanged);
    _entry?.remove();
    _entry = null;
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      if (_layoutWidth != constraints.maxWidth) {
        _layoutWidth = constraints.maxWidth;
        _scheduleOverlaySync();
      }
      return CompositedTransformTarget(link: _link, child: widget.child);
    },
  );
}

class VoiceRecorderBar extends StatelessWidget {
  const VoiceRecorderBar({
    required this.state,
    required this.onCancel,
    required this.onStop,
    required this.onRetry,
    required this.onSend,
    super.key,
  });

  final VoiceRecorderState state;
  final Future<void> Function() onCancel;
  final Future<void> Function() onStop;
  final Future<void> Function() onRetry;
  final Future<void> Function() onSend;

  @override
  Widget build(BuildContext context) {
    final recording = state.recording;
    final isRecording = state.status == VoiceRecorderStatus.recording;
    return Container(
      key: const Key('voice-recorder-bar'),
      height: 56,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      color: context.theme.primary,
      child: Row(
        children: [
          ActionButton(
            key: const Key('voice-cancel'),
            name: MixinAssets.closeOvalRecord,
            color: context.theme.icon,
            onTap: () => unawaited(onCancel()),
          ),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: recording == null
                  ? _RecordingDuration(duration: state.elapsed)
                  : _VoiceRecordingPreview(recording: recording),
            ),
          ),
          if (isRecording)
            ActionButton(
              key: const Key('voice-stop'),
              name: MixinAssets.recordStop,
              color: context.theme.accent,
              onTap: () => unawaited(onStop()),
            )
          else
            ActionButton(
              key: const Key('voice-retry'),
              name: MixinAssets.recordRetry,
              color: context.theme.icon,
              onTap: () => unawaited(onRetry()),
            ),
          ActionButton(
            key: const Key('voice-send'),
            name: MixinAssets.send,
            color: context.theme.icon,
            onTap: () => unawaited(onSend()),
          ),
        ],
      ),
    );
  }
}

class _RecordingDuration extends StatelessWidget {
  const _RecordingDuration({required this.duration});

  final Duration duration;

  @override
  Widget build(BuildContext context) => Row(
    mainAxisAlignment: MainAxisAlignment.center,
    children: [
      const SizedBox.square(
        dimension: 8,
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: Color(0xFFE57874),
            shape: BoxShape.circle,
          ),
        ),
      ),
      const SizedBox(width: 4),
      Text(
        _formatVoiceDuration(duration),
        style: TextStyle(color: context.theme.text, fontSize: 14),
      ),
    ],
  );
}

class _VoiceRecordingPreview extends StatefulWidget {
  const _VoiceRecordingPreview({required this.recording});

  final VoiceRecording recording;

  @override
  State<_VoiceRecordingPreview> createState() => _VoiceRecordingPreviewState();
}

class _VoiceRecordingPreviewState extends State<_VoiceRecordingPreview> {
  late final AudioMessagePlaybackCoordinator _coordinator;
  late String _previewId;

  @override
  void initState() {
    super.initState();
    _coordinator = context.read<AudioMessagePlaybackCoordinator>();
    _previewId = 'voice-preview-${widget.recording.path.hashCode}';
    _coordinator
      ..attach()
      ..addListener(_onPlaybackChanged);
  }

  @override
  void didUpdateWidget(covariant _VoiceRecordingPreview oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.recording.path == widget.recording.path) return;
    if (_coordinator.currentMessageId == _previewId) _coordinator.stop();
    _previewId = 'voice-preview-${widget.recording.path.hashCode}';
  }

  @override
  void dispose() {
    if (_coordinator.currentMessageId == _previewId) _coordinator.stop();
    _coordinator
      ..removeListener(_onPlaybackChanged)
      ..detach();
    super.dispose();
  }

  void _onPlaybackChanged() {
    if (mounted) setState(() {});
  }

  Future<void> _togglePlayback() async {
    if (_coordinator.currentMessageId == _previewId && _coordinator.isPlaying) {
      _coordinator.stop();
      return;
    }
    await _coordinator.playPreview(_previewId, widget.recording.path);
  }

  @override
  Widget build(BuildContext context) {
    final playing =
        _coordinator.currentMessageId == _previewId && _coordinator.isPlaying;
    final duration = widget.recording.duration;
    final progress = playing && duration.inMilliseconds > 0
        ? (_coordinator.position.inMilliseconds / duration.inMilliseconds)
              .clamp(0.0, 1.0)
        : 0.0;
    return SizedBox(
      height: 32,
      child: Material(
        color: context.theme.listSelected,
        borderRadius: const BorderRadius.all(Radius.circular(15)),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const SizedBox(width: 2),
            ActionButton(
              key: const Key('voice-preview-play'),
              name: playing
                  ? MixinAssets.recordPreviewStop
                  : MixinAssets.recordPreviewPlay,
              onTap: () => unawaited(_togglePlayback()),
            ),
            const SizedBox(width: 2),
            Expanded(
              child: SizedBox(
                height: 20,
                child: WaveformWidget(
                  waveform: widget.recording.waveform,
                  value: progress,
                  backgroundColor: context.theme.waveformBackground,
                  foregroundColor: context.theme.waveformForeground,
                  maxBarCount: null,
                  alignment: WaveBarAlignment.center,
                ),
              ),
            ),
            const SizedBox(width: 10),
            Text(
              _formatVoiceDuration(duration),
              style: TextStyle(color: context.theme.text, fontSize: 14),
            ),
            const SizedBox(width: 12),
          ],
        ),
      ),
    );
  }
}

String _formatVoiceDuration(Duration duration) {
  final minutes = duration.inMinutes;
  final seconds = duration.inSeconds.remainder(60);
  return '$minutes:${seconds.toString().padLeft(2, '0')}';
}

void _showDiscardRecordingWarningAlertOverlay(
  BuildContext context, {
  required VoidCallback onDiscard,
}) {
  final overlay = Overlay.of(context, rootOverlay: true);
  OverlayEntry? entry;

  void dismiss() {
    entry?.remove();
    entry = null;
  }

  entry = OverlayEntry(
    builder: (context) => Stack(
      children: [
        GestureDetector(
          behavior: HitTestBehavior.translucent,
          onTap: dismiss,
          child: const SizedBox.expand(
            child: ColoredBox(color: Color(0x80000000)),
          ),
        ),
        Center(
          child: SizedBox(
            width: 400,
            child: Material(
              borderRadius: const BorderRadius.all(Radius.circular(11)),
              color: context.theme.popUp,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 30),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const SizedBox(height: 40),
                    Text(
                      context.l10n.discardRecordingWarning,
                      style: TextStyle(
                        fontSize: 16,
                        height: 2,
                        color: context.theme.text,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(height: 36),
                    Row(
                      children: [
                        const Spacer(),
                        MixinButton(
                          backgroundTransparent: true,
                          onTap: dismiss,
                          child: Text(context.l10n.cancel),
                        ),
                        MixinButton(
                          onTap: () {
                            dismiss();
                            onDiscard();
                          },
                          child: Text(context.l10n.discard),
                        ),
                      ],
                    ),
                    const SizedBox(height: 30),
                  ],
                ),
              ),
            ),
          ),
        ),
      ],
    ),
  );
  overlay.insert(entry!);
}
