import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/voice_recorder_controller.dart';

void main() {
  late Directory directory;
  late String path;
  late _FakeVoiceRecorderBackend backend;
  late VoiceRecorderController controller;

  setUp(() async {
    directory = await Directory.systemTemp.createTemp('voice-controller-test');
    path = '${directory.path}/recording.ogg';
    backend = _FakeVoiceRecorderBackend();
    controller = VoiceRecorderController(
      backend: backend,
      pathFactory: () async => path,
    );
  });

  tearDown(() async {
    controller.dispose();
    if (await directory.exists()) await directory.delete(recursive: true);
  });

  test('records, sends, and removes the temporary file', () async {
    expect(await controller.start(), isTrue);
    expect(controller.value.status, VoiceRecorderStatus.recording);

    final recording = await controller.stop();
    expect(recording?.duration, const Duration(milliseconds: 1400));
    expect(controller.value.status, VoiceRecorderStatus.recorded);
    expect(await File(path).exists(), isTrue);

    VoiceRecording? sent;
    expect(await controller.send((value) async => sent = value), isTrue);
    expect(sent?.waveform, [10, 20, 30]);
    expect(controller.value.status, VoiceRecorderStatus.idle);
    expect(await File(path).exists(), isFalse);
  });

  test('cancel stops an active recorder and removes its file', () async {
    await controller.start();
    await controller.cancel();

    expect(backend.canceled, isTrue);
    expect(controller.value.status, VoiceRecorderStatus.idle);
    expect(await File(path).exists(), isFalse);
  });

  test('send failure keeps the recording available for retry', () async {
    await controller.start();
    await controller.stop();

    final sent = await controller.send((_) async => throw Exception('upload'));

    expect(sent, isFalse);
    expect(controller.value.status, VoiceRecorderStatus.recorded);
    expect(controller.value.recording?.path, path);
    expect(controller.value.error, isA<Exception>());
    expect(await File(path).exists(), isTrue);
  });
}

class _FakeVoiceRecorderBackend implements VoiceRecorderBackend {
  String? path;
  bool canceled = false;

  @override
  Future<void> start(String path) async {
    this.path = path;
    await File(path).writeAsBytes([1, 2, 3]);
  }

  @override
  Future<VoiceRecording> stop() async => VoiceRecording(
    path: path!,
    duration: const Duration(milliseconds: 1400),
    waveform: const [10, 20, 30],
  );

  @override
  Future<void> cancel() async {
    canceled = true;
    final current = path;
    if (current != null && await File(current).exists()) {
      await File(current).delete();
    }
  }
}
