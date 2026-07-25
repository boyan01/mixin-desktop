import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/sticker_controller.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';

import 'test_settings_store.dart';

void main() {
  test('shows local stickers before remote refresh completes', () async {
    final remoteRefresh = Completer<void>();
    final account = _FakeAccount(
      id: 'local-first',
      remoteRefresh: remoteRefresh,
    );
    final controller = StickerController(
      account: account,
      settings: TestSettingsStore(),
    );
    final localReady = Completer<void>();
    controller.addListener(() {
      if (controller.initialized && !localReady.isCompleted) {
        localReady.complete();
      }
    });

    final loading = controller.load();
    await localReady.future;

    expect(controller.personalStickers, [_sticker]);
    expect(remoteRefresh.isCompleted, isFalse);

    remoteRefresh.complete();
    await loading;
    expect(account.refreshCalls, 1);
    controller.dispose();
  });

  test('limits background refresh to once every 24 hours', () async {
    final settings = TestSettingsStore({
      'sticker_refresh_at_rate-limited': DateTime.now().millisecondsSinceEpoch,
    });
    final account = _FakeAccount(id: 'rate-limited');

    expect(
      await StickerController.refreshRemote(account, settings: settings),
      isFalse,
    );
    expect(account.refreshCalls, 0);

    expect(
      await StickerController.refreshRemote(
        account,
        settings: settings,
        force: true,
      ),
      isTrue,
    );
    expect(account.refreshCalls, 1);
  });
}

const _sticker = StickerItem(
  stickerId: 'favorite',
  name: 'Favorite',
  assetUrl: '',
  assetWidth: 128,
  assetHeight: 128,
  assetType: 'png',
  createdAtMillis: 0,
);

class _FakeAccount implements AccountHandle, StickerAccess {
  _FakeAccount({required this.id, this.remoteRefresh});

  final String id;
  final Completer<void>? remoteRefresh;
  int refreshCalls = 0;

  @override
  String accountId() => id;

  @override
  StickerAccess sticker() => this;

  @override
  Future<bool> refreshStickers() async {
    refreshCalls++;
    await remoteRefresh?.future;
    return false;
  }

  @override
  Future<List<StickerItem>> recentStickers() async => const [];

  @override
  Future<List<StickerItem>> personalStickers() async => const [_sticker];

  @override
  Future<List<StickerAlbumItem>> stickerAlbums() async => const [];

  @override
  Future<List<StickerItem>> albumStickers({required String albumId}) async =>
      const [];

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}
