import 'package:mixin_desktop_ui/src/rust/api/media.dart';
import 'package:mixin_desktop_ui/widgets/message_audio.dart';

class TestAudioPlaybackBackend implements AudioPlaybackBackend {
  @override
  Stream<MediaPlaybackEvent> events() => const Stream.empty();

  @override
  Future<void> pause() async {}

  @override
  Future<void> play({
    required List<MediaAudioItem> playlist,
    required BigInt startIndex,
  }) async {}

  @override
  Future<void> resume() async {}

  @override
  Future<void> setSpeed(double speed) async {}

  @override
  void stop() {}
}
