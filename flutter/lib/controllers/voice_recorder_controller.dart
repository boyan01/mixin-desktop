import 'dart:async';
import 'dart:io';

import 'package:audio_session/audio_session.dart';
import 'package:flutter/foundation.dart';
import 'package:ogg_opus_player/ogg_opus_player.dart';

import '../utils/app_logger.dart';

const maxVoiceRecordingDuration = Duration(seconds: 60);

enum VoiceRecorderStatus { idle, recording, recorded, sending }

@immutable
class VoiceRecording {
  const VoiceRecording({
    required this.path,
    required this.duration,
    required this.waveform,
  });

  final String path;
  final Duration duration;
  final List<int> waveform;
}

@immutable
class VoiceRecorderState {
  const VoiceRecorderState({
    required this.status,
    this.elapsed = Duration.zero,
    this.recording,
    this.error,
  });

  const VoiceRecorderState.idle() : this(status: VoiceRecorderStatus.idle);

  final VoiceRecorderStatus status;
  final Duration elapsed;
  final VoiceRecording? recording;
  final Object? error;
}

typedef VoiceRecordingSender = Future<void> Function(VoiceRecording recording);
typedef VoiceRecordingPathFactory = Future<String> Function();

abstract interface class VoiceRecorderBackend {
  Future<void> start(String path);

  Future<VoiceRecording> stop();

  Future<void> cancel();
}

class VoiceRecorderController extends ValueNotifier<VoiceRecorderState> {
  VoiceRecorderController({
    VoiceRecorderBackend? backend,
    VoiceRecordingPathFactory? pathFactory,
  }) : _backend = backend ?? OggOpusRecorderBackend(),
       _pathFactory = pathFactory ?? _temporaryRecordingPath,
       super(const VoiceRecorderState.idle());

  final VoiceRecorderBackend _backend;
  final VoiceRecordingPathFactory _pathFactory;
  Timer? _elapsedTimer;
  DateTime? _startedAt;
  bool _transitioning = false;
  bool _disposed = false;

  Future<bool> start() async {
    if (_transitioning || value.status == VoiceRecorderStatus.sending) {
      return false;
    }
    if (value.status == VoiceRecorderStatus.recording) return true;
    _transitioning = true;
    final previousPath = value.recording?.path;
    try {
      if (previousPath != null) await _deleteFile(previousPath);
      final path = await _pathFactory();
      await _backend.start(path);
      _startedAt = DateTime.now();
      _setState(
        const VoiceRecorderState(status: VoiceRecorderStatus.recording),
      );
      _elapsedTimer = Timer.periodic(const Duration(milliseconds: 100), (_) {
        final startedAt = _startedAt;
        if (startedAt == null || _disposed) return;
        final elapsed = DateTime.now().difference(startedAt);
        if (elapsed >= maxVoiceRecordingDuration) {
          unawaited(stop());
          return;
        }
        _setState(
          VoiceRecorderState(
            status: VoiceRecorderStatus.recording,
            elapsed: elapsed,
          ),
        );
      });
      return true;
    } catch (error) {
      _setState(
        VoiceRecorderState(status: VoiceRecorderStatus.idle, error: error),
      );
      return false;
    } finally {
      _transitioning = false;
    }
  }

  Future<VoiceRecording?> stop() async {
    if (_transitioning || value.status != VoiceRecorderStatus.recording) {
      return value.recording;
    }
    _transitioning = true;
    _stopElapsedTimer();
    try {
      final recording = await _backend.stop();
      final file = File(recording.path);
      if (!await file.exists() || await file.length() == 0) {
        throw StateError('Recorded audio is empty');
      }
      if (recording.duration <= Duration.zero || recording.waveform.isEmpty) {
        throw StateError('Recorded audio metadata is invalid');
      }
      _setState(
        VoiceRecorderState(
          status: VoiceRecorderStatus.recorded,
          elapsed: recording.duration,
          recording: recording,
        ),
      );
      return recording;
    } catch (error) {
      _setState(
        VoiceRecorderState(status: VoiceRecorderStatus.idle, error: error),
      );
      return null;
    } finally {
      _transitioning = false;
    }
  }

  Future<void> cancel() async {
    if (_transitioning || value.status == VoiceRecorderStatus.sending) return;
    _transitioning = true;
    _stopElapsedTimer();
    final path = value.recording?.path;
    try {
      if (value.status == VoiceRecorderStatus.recording) {
        await _backend.cancel();
      }
      if (path != null) await _deleteFile(path);
      _setState(const VoiceRecorderState.idle());
    } catch (error) {
      _setState(
        VoiceRecorderState(status: VoiceRecorderStatus.idle, error: error),
      );
    } finally {
      _transitioning = false;
    }
  }

  Future<bool> send(VoiceRecordingSender sender) async {
    if (_transitioning || value.status == VoiceRecorderStatus.sending) {
      return false;
    }
    var recording = value.recording;
    if (value.status == VoiceRecorderStatus.recording) {
      recording = await stop();
    }
    if (recording == null) return false;
    _transitioning = true;
    _setState(const VoiceRecorderState.idle());
    try {
      await sender(recording);
      await _deleteFile(recording.path);
      return true;
    } catch (error) {
      _setState(
        VoiceRecorderState(
          status: VoiceRecorderStatus.recorded,
          elapsed: recording.duration,
          recording: recording,
          error: error,
        ),
      );
      return false;
    } finally {
      _transitioning = false;
    }
  }

  void clearError() {
    final state = value;
    if (state.error == null) return;
    _setState(
      VoiceRecorderState(
        status: state.status,
        elapsed: state.elapsed,
        recording: state.recording,
      ),
    );
  }

  void _stopElapsedTimer() {
    _elapsedTimer?.cancel();
    _elapsedTimer = null;
    _startedAt = null;
  }

  void _setState(VoiceRecorderState state) {
    if (!_disposed) value = state;
  }

  @override
  void dispose() {
    _disposed = true;
    _stopElapsedTimer();
    if (value.status == VoiceRecorderStatus.recording) {
      unawaited(_backend.cancel());
    }
    final path = value.recording?.path;
    if (path != null) unawaited(_deleteFile(path));
    super.dispose();
  }
}

class OggOpusRecorderBackend implements VoiceRecorderBackend {
  OggOpusRecorder? _recorder;
  String? _path;

  @override
  Future<void> start(String path) async {
    if (_recorder != null) throw StateError('Recorder is already active');
    final file = File(path);
    if (await file.exists()) await file.delete();
    await file.create(recursive: true);
    final session = await AudioSession.instance;
    await session.configure(const AudioSessionConfiguration.speech());
    await session.setActive(true);
    final recorder = OggOpusRecorder(path);
    try {
      recorder.start();
    } catch (error, stackTrace) {
      e('Start voice recorder failed', error, stackTrace);
      recorder.dispose();
      await _deactivateAudioSession();
      rethrow;
    }
    _path = path;
    _recorder = recorder;
  }

  @override
  Future<VoiceRecording> stop() async {
    final recorder = _recorder;
    final path = _path;
    if (recorder == null || path == null) {
      throw StateError('Recorder is not active');
    }
    _recorder = null;
    _path = null;
    try {
      await recorder.stop();
      final waveform = await recorder.getWaveformData();
      final duration = await recorder.duration();
      return VoiceRecording(
        path: path,
        duration: Duration(milliseconds: (duration * 1000).round()),
        waveform: waveform,
      );
    } finally {
      recorder.dispose();
      await _deactivateAudioSession();
    }
  }

  @override
  Future<void> cancel() async {
    final recorder = _recorder;
    final path = _path;
    _recorder = null;
    _path = null;
    try {
      await recorder?.stop();
    } finally {
      recorder?.dispose();
      await _deactivateAudioSession();
      if (path != null) await _deleteFile(path);
    }
  }
}

Future<String> _temporaryRecordingPath() async {
  final directory = Directory(
    '${Directory.systemTemp.path}${Platform.pathSeparator}mixin-desktop-voice',
  );
  await directory.create(recursive: true);
  return '${directory.path}${Platform.pathSeparator}'
      '${DateTime.now().microsecondsSinceEpoch}-$pid.ogg';
}

Future<void> _deleteFile(String path) async {
  try {
    final file = File(path);
    if (await file.exists()) await file.delete();
  } catch (error, stackTrace) {
    e('Delete temporary voice recording failed: $path', error, stackTrace);
  }
}

Future<void> _deactivateAudioSession() async {
  try {
    final session = await AudioSession.instance;
    await session.setActive(
      false,
      avAudioSessionSetActiveOptions:
          AVAudioSessionSetActiveOptions.notifyOthersOnDeactivation,
    );
  } catch (error, stackTrace) {
    e('Deactivate audio session failed', error, stackTrace);
  }
}
