import 'package:flutter/foundation.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart' as rust;

@immutable
class MessageListEntry {
  const MessageListEntry({
    required this.id,
    required this.conversationId,
    required this.senderId,
    required this.senderName,
    this.senderIdentityNumber = '',
    required this.senderAvatarUrl,
    required this.senderIsVerified,
    required this.category,
    required this.content,
    required this.status,
    required this.createdAt,
    required this.mediaDuration,
    required this.mediaStatus,
    this.mediaUrl,
    this.mediaMimeType,
    this.mediaSize,
    this.mediaWidth,
    this.mediaHeight,
    this.thumbImage,
    this.quoteMessageId,
    this.quoteContent,
    this.caption,
    this.action,
    this.participantId,
    this.participantFullName,
    this.snapshotId,
    this.snapshotType,
    this.snapshotAmount,
    this.snapshotMemo,
    this.snapshotAssetId,
    this.snapshotAssetSymbol,
    this.snapshotAssetIconUrl,
    this.snapshotChainIconUrl,
    this.snapshotOpponentId,
    this.snapshotTransactionHash,
    this.snapshotCreatedAt,
    this.inscriptionHash,
    this.inscriptionCollectionHash,
    this.inscriptionSequence,
    this.inscriptionContentType,
    this.inscriptionContentUrl,
    this.inscriptionName,
    this.inscriptionIconUrl,
    this.hyperlink,
    this.mediaName,
    this.albumId,
    this.stickerId,
    this.sharedUserId,
    this.mediaWaveform,
    this.thumbUrl,
    this.senderRelationship = '',
    this.senderAppId,
    this.senderIsScam = false,
    this.senderIsBot = false,
    this.conversationOwnerId,
    this.conversationCategory,
    this.sharedUserFullName,
    this.sharedUserIdentityNumber,
    this.sharedUserAvatarUrl,
    this.sharedUserIsVerified = false,
    this.sharedUserAppId,
    this.stickerAssetUrl,
    this.stickerAssetWidth,
    this.stickerAssetHeight,
    this.stickerAssetName,
    this.stickerAssetType,
    this.mentionRead,
    this.pinned = false,
    this.expireIn,
  });

  factory MessageListEntry.fromRust(rust.MessageListItem item) =>
      MessageListEntry(
        id: item.messageId,
        conversationId: item.conversationId,
        senderId: item.senderId,
        senderName: item.senderName,
        senderIdentityNumber: item.senderIdentityNumber ?? '',
        senderAvatarUrl: item.senderAvatarUrl,
        senderIsVerified: item.senderIsVerified,
        category: item.category,
        content: item.content,
        status: item.status,
        createdAt: DateTime.fromMicrosecondsSinceEpoch(
          item.createdAtMicros.toInt(),
        ),
        mediaUrl: item.mediaUrl,
        mediaMimeType: item.mediaMimeType,
        mediaSize: item.mediaSize?.toInt(),
        mediaDuration: item.mediaDuration,
        mediaWidth: item.mediaWidth,
        mediaHeight: item.mediaHeight,
        thumbImage: item.thumbImage,
        mediaStatus: item.mediaStatus,
        quoteMessageId: item.quoteMessageId,
        quoteContent: item.quoteContent,
        caption: item.caption,
        action: item.action,
        participantId: item.participantId,
        participantFullName: item.participantFullName,
        snapshotId: item.snapshotId,
        snapshotType: item.snapshotType,
        snapshotAmount: item.snapshotAmount,
        snapshotMemo: item.snapshotMemo,
        snapshotAssetId: item.snapshotAssetId,
        snapshotAssetSymbol: item.snapshotAssetSymbol,
        snapshotAssetIconUrl: item.snapshotAssetIconUrl,
        snapshotChainIconUrl: item.snapshotChainIconUrl,
        snapshotOpponentId: item.snapshotOpponentId,
        snapshotTransactionHash: item.snapshotTransactionHash,
        snapshotCreatedAt: item.snapshotCreatedAt,
        inscriptionHash: item.inscriptionHash,
        inscriptionCollectionHash: item.inscriptionCollectionHash,
        inscriptionSequence: item.inscriptionSequence?.toInt(),
        inscriptionContentType: item.inscriptionContentType,
        inscriptionContentUrl: item.inscriptionContentUrl,
        inscriptionName: item.inscriptionName,
        inscriptionIconUrl: item.inscriptionIconUrl,
        hyperlink: item.hyperlink,
        mediaName: item.mediaName,
        albumId: item.albumId,
        stickerId: item.stickerId,
        sharedUserId: item.sharedUserId,
        mediaWaveform: item.mediaWaveform,
        thumbUrl: item.thumbUrl,
        senderRelationship: item.senderRelationship,
        senderAppId: item.senderAppId,
        senderIsScam: item.senderIsScam,
        senderIsBot: item.senderIsBot,
        conversationOwnerId: item.conversationOwnerId,
        conversationCategory: item.conversationCategory,
        sharedUserFullName: item.sharedUserFullName,
        sharedUserIdentityNumber: item.sharedUserIdentityNumber,
        sharedUserAvatarUrl: item.sharedUserAvatarUrl,
        sharedUserIsVerified: item.sharedUserIsVerified,
        sharedUserAppId: item.sharedUserAppId,
        stickerAssetUrl: item.stickerAssetUrl,
        stickerAssetWidth: item.stickerAssetWidth,
        stickerAssetHeight: item.stickerAssetHeight,
        stickerAssetName: item.stickerAssetName,
        stickerAssetType: item.stickerAssetType,
        mentionRead: item.mentionRead,
        pinned: item.pinned,
        expireIn: item.expireIn?.toInt(),
      );

  final String id;
  final String conversationId;
  final String senderId;
  final String senderName;
  final String senderIdentityNumber;
  final String senderAvatarUrl;
  final bool senderIsVerified;
  final String category;
  final String content;
  final String status;
  final DateTime createdAt;
  final String? mediaUrl;
  final String? mediaMimeType;
  final int? mediaSize;
  final String mediaDuration;
  final int? mediaWidth;
  final int? mediaHeight;
  final String? thumbImage;
  final String mediaStatus;
  final String? quoteMessageId;
  final String? quoteContent;
  final String? caption;
  final String? action;
  final String? participantId;
  final String? participantFullName;
  final String? snapshotId;
  final String? snapshotType;
  final String? snapshotAmount;
  final String? snapshotMemo;
  final String? snapshotAssetId;
  final String? snapshotAssetSymbol;
  final String? snapshotAssetIconUrl;
  final String? snapshotChainIconUrl;
  final String? snapshotOpponentId;
  final String? snapshotTransactionHash;
  final String? snapshotCreatedAt;
  final String? inscriptionHash;
  final String? inscriptionCollectionHash;
  final int? inscriptionSequence;
  final String? inscriptionContentType;
  final String? inscriptionContentUrl;
  final String? inscriptionName;
  final String? inscriptionIconUrl;
  final String? hyperlink;
  final String? mediaName;
  final String? albumId;
  final String? stickerId;
  final String? sharedUserId;
  final String? mediaWaveform;
  final String? thumbUrl;
  final String senderRelationship;
  final String? senderAppId;
  final bool senderIsScam;
  final bool senderIsBot;
  final String? conversationOwnerId;
  final String? conversationCategory;
  final String? sharedUserFullName;
  final String? sharedUserIdentityNumber;
  final String? sharedUserAvatarUrl;
  final bool sharedUserIsVerified;
  final String? sharedUserAppId;
  final String? stickerAssetUrl;
  final int? stickerAssetWidth;
  final int? stickerAssetHeight;
  final String? stickerAssetName;
  final String? stickerAssetType;
  final bool? mentionRead;
  final bool pinned;
  final int? expireIn;

  bool get isText => category.endsWith('_TEXT');
  bool get isImage => category.endsWith('_IMAGE');
  bool get isVideo => category.endsWith('_VIDEO') || category.endsWith('_LIVE');
  bool get isAudio => category.endsWith('_AUDIO');
  bool get isSticker => category.endsWith('_STICKER');
  bool get isPost => category.endsWith('_POST');
  bool get isRecall => category == 'MESSAGE_RECALL';
}
