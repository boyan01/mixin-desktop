import 'package:flutter/foundation.dart';

@immutable
class StickerData {
  const StickerData({
    required this.stickerId,
    required this.name,
    required this.assetUrl,
    required this.assetWidth,
    required this.assetHeight,
    required this.assetType,
    this.albumId,
  });

  final String stickerId;
  final String? albumId;
  final String name;
  final String assetUrl;
  final int assetWidth;
  final int assetHeight;
  final String assetType;
}

@immutable
class StickerAlbumData {
  const StickerAlbumData({
    required this.albumId,
    required this.name,
    required this.iconUrl,
    required this.added,
  });

  final String albumId;
  final String name;
  final String iconUrl;
  final bool added;
}
