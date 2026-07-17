import 'package:flutter/foundation.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;
import 'package:shared_preferences/shared_preferences.dart';

class StickerController extends ChangeNotifier {
  StickerController({required this.account});

  static const _recentEmojiKey = 'recent_used_emoji';
  static const _recentEmojiLimit = 35;
  static const _remoteRefreshInterval = Duration(hours: 24);
  static final Map<String, Future<bool>> _remoteRefreshes = {};

  final rust.AccountHandle account;
  List<rust.StickerItem> recentStickers = const [];
  List<rust.StickerItem> personalStickers = const [];
  List<rust.StickerAlbumItem> albums = const [];
  Map<String, List<rust.StickerItem>> albumStickers = const {};
  List<rust.StickerAlbumItem> storeAlbums = const [];
  Map<String, List<rust.StickerItem>> storeAlbumStickers = const {};
  List<String> recentEmojis = const [];
  Object? error;
  bool loading = false;
  bool storeLoading = false;
  bool initialized = false;
  bool _refreshSucceeded = false;
  bool _disposed = false;

  static Future<bool> refreshRemote(
    rust.AccountHandle account, {
    bool force = false,
  }) {
    final accountId = account.accountId();
    final pending = _remoteRefreshes[accountId];
    if (pending != null) return pending;
    final refresh = _refreshRemote(account, accountId, force: force);
    _remoteRefreshes[accountId] = refresh;
    return refresh.whenComplete(() {
      if (identical(_remoteRefreshes[accountId], refresh)) {
        _remoteRefreshes.remove(accountId);
      }
    });
  }

  static Future<bool> _refreshRemote(
    rust.AccountHandle account,
    String accountId, {
    required bool force,
  }) async {
    final preferences = await SharedPreferences.getInstance();
    final refreshKey = 'sticker_refresh_at_$accountId';
    final lastRefreshAt = preferences.getInt(refreshKey);
    final now = DateTime.now().millisecondsSinceEpoch;
    if (!force &&
        lastRefreshAt != null &&
        now - lastRefreshAt < _remoteRefreshInterval.inMilliseconds) {
      return false;
    }
    await account.sticker().refreshStickers();
    await preferences.setInt(refreshKey, now);
    return true;
  }

  Future<void> load() async {
    if (loading || (initialized && _refreshSucceeded)) return;
    loading = true;
    error = null;
    _notify();
    try {
      if (!initialized) {
        await _loadPrimaryLocal();
        final preferences = await SharedPreferences.getInstance();
        recentEmojis = preferences.getStringList(_recentEmojiKey) ?? const [];
        initialized = true;
        _notify();
        await _loadAlbumStickers();
        _notify();
      }
      try {
        final refreshed = await refreshRemote(account);
        _refreshSucceeded = true;
        if (refreshed) await _loadLocal();
      } on Object catch (exception) {
        error = exception;
      }
    } on Object catch (exception) {
      error = exception;
    } finally {
      loading = false;
      _notify();
    }
  }

  Future<void> refreshLocal() async {
    if (loading) return;
    try {
      await _loadLocal();
      error = null;
    } on Object catch (exception) {
      error = exception;
    } finally {
      _notify();
    }
  }

  Future<void> loadStore() async {
    if (storeLoading) return;
    storeLoading = true;
    _notify();
    try {
      await _loadStoreLocal();
      await refreshRemote(account, force: true);
      _refreshSucceeded = true;
      await _loadStoreLocal();
      error = null;
    } on Object catch (exception) {
      error = exception;
      await _loadStoreLocal();
    } finally {
      storeLoading = false;
      _notify();
    }
  }

  Future<void> _loadStoreLocal() async {
    storeAlbums = await account.sticker().stickerStoreAlbums();
    _notify();
    final entries = await Future.wait(
      storeAlbums.map((album) async {
        final stickers = await account.sticker().albumStickers(
          albumId: album.albumId,
        );
        return MapEntry(album.albumId, stickers);
      }),
    );
    storeAlbumStickers = Map.fromEntries(entries);
  }

  Future<void> setAlbumAdded(String albumId, bool added) async {
    await account.sticker().setStickerAlbumAdded(
      albumId: albumId,
      added: added,
    );
    await Future.wait([refreshLocal(), _loadStoreLocal()]);
    _notify();
  }

  Future<void> setAlbumOrder(List<String> albumIds) async {
    await account.sticker().setStickerAlbumOrder(albumIds: albumIds);
    await refreshLocal();
  }

  Future<void> addStickerFromPath(String path) async {
    await account.sticker().addStickerFromPath(path: path);
    personalStickers = await account.sticker().personalStickers();
    _notify();
  }

  Future<void> _loadLocal() async {
    await _loadPrimaryLocal();
    _notify();
    await _loadAlbumStickers();
    _notify();
  }

  Future<void> _loadPrimaryLocal() async {
    final values = await Future.wait<Object>([
      account.sticker().recentStickers(),
      account.sticker().personalStickers(),
      account.sticker().stickerAlbums(),
    ]);
    recentStickers = values[0] as List<rust.StickerItem>;
    personalStickers = values[1] as List<rust.StickerItem>;
    albums = values[2] as List<rust.StickerAlbumItem>;
  }

  Future<void> _loadAlbumStickers() async {
    final entries = await Future.wait(
      albums.map((album) async {
        final stickers = await account.sticker().albumStickers(
          albumId: album.albumId,
        );
        return MapEntry(album.albumId, stickers);
      }),
    );
    albumStickers = Map.fromEntries(entries);
  }

  Future<void> sendSticker({
    required String conversationId,
    required rust.StickerItem sticker,
  }) async {
    await account.message().sendSticker(
      conversationId: conversationId,
      stickerId: sticker.stickerId,
    );
    recentStickers = await account.sticker().recentStickers();
    _notify();
  }

  Future<void> removeSticker(rust.StickerItem sticker) async {
    await account.sticker().removeSticker(stickerId: sticker.stickerId);
    personalStickers = await account.sticker().personalStickers();
    _notify();
  }

  Future<void> recordEmoji(String emoji) async {
    recentEmojis = [
      emoji,
      ...recentEmojis.where((item) => item != emoji),
    ].take(_recentEmojiLimit).toList(growable: false);
    _notify();
    final preferences = await SharedPreferences.getInstance();
    await preferences.setStringList(_recentEmojiKey, recentEmojis);
  }

  void _notify() {
    if (!_disposed) notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }
}
