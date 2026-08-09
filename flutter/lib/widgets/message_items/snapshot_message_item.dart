import 'dart:convert';

import 'package:auto_size_text/auto_size_text.dart';
import 'package:decimal/decimal.dart';
import 'package:decimal/intl.dart';
import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:hexagon/hexagon.dart';
import 'package:intl/intl.dart' as intl;

import '../../constants/assets.dart';
import '../../l10n/l10n.dart';
import '../../models/message_list_entry.dart';
import '../../theme.dart';
import '../high_light_text.dart';
import '../interactive_decorated_box.dart';
import '../message_style.dart';
import '../show_snapshot_detail_dialog.dart';
import '../toast.dart';
import 'inscription_content.dart';
import 'message_preview_utils.dart';

enum SnapshotKind { account, safe, safeInscription }

class SnapshotMessageItem extends StatelessWidget {
  const SnapshotMessageItem({
    required this.message,
    required this.kind,
    super.key,
  });

  final MessageListEntry message;
  final SnapshotKind kind;

  @override
  Widget build(BuildContext context) {
    final data = _SnapshotData.parse(message.content);
    final id = message.snapshotId ?? message.inscriptionHash ?? data.id;
    final amount = message.snapshotAmount ?? data.amount;
    final symbol = message.snapshotAssetSymbol ?? data.symbol;
    final memo = _decodeSafeMemo(message.snapshotMemo ?? '');
    return InteractiveDecoratedBox(
      onTap: () {
        if (id != null) {
          showSnapshotDetailDialog(context, message: message);
        } else if (kind == SnapshotKind.safeInscription) {
          showToastFailed(
            ToastError(context.l10n.dataLoading),
            context: context,
          );
        }
      },
      child: kind == SnapshotKind.safeInscription
          ? _InscriptionCard(message: message, data: data)
          : kind == SnapshotKind.safe
          ? _SafeSnapshotCard(
              amount: amount,
              symbol: symbol,
              iconUrl: message.snapshotAssetIconUrl,
              memo: memo,
            )
          : Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                if (message.snapshotAssetIconUrl?.isNotEmpty == true)
                  _AssetIcon(
                    iconUrl: message.snapshotAssetIconUrl,
                    chainIconUrl: message.snapshotChainIconUrl,
                    size: 40,
                  )
                else
                  const SizedBox(width: 40, height: 40),
                const SizedBox(width: 8),
                Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Flexible(
                          child: amount.isEmpty
                              ? const SizedBox()
                              : Text(
                                  _numberFormat(amount),
                                  style: TextStyle(
                                    color: context.theme.text,
                                    fontSize:
                                        context.messageStyle.secondaryFontSize,
                                  ),
                                ),
                        ),
                      ],
                    ),
                    Text(
                      symbol,
                      style: TextStyle(
                        color: context.theme.secondaryText,
                        fontSize: context.messageStyle.tertiaryFontSize,
                      ),
                    ),
                  ],
                ),
              ],
            ),
    );
  }
}

class _SafeSnapshotCard extends StatelessWidget {
  const _SafeSnapshotCard({
    required this.amount,
    required this.symbol,
    required this.iconUrl,
    required this.memo,
  });

  final String amount;
  final String symbol;
  final String? iconUrl;
  final String memo;

  @override
  Widget build(BuildContext context) => SizedBox(
    width: switch (amount.length) {
      < 10 => 174,
      < 15 => 190,
      < 25 => 216,
      _ => 232,
    },
    child: Stack(
      children: [
        Align(
          alignment: Alignment.topRight,
          child: SvgPicture.asset(MixinAssets.bgSnapshot),
        ),
        Padding(
          padding: const EdgeInsets.all(4),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  if (iconUrl?.isNotEmpty != true)
                    const SizedBox.square(dimension: 16)
                  else
                    ClipOval(
                      child: Image.network(
                        iconUrl!,
                        width: 16,
                        height: 16,
                        fit: BoxFit.cover,
                        errorBuilder: (_, _, _) =>
                            const SizedBox.square(dimension: 16),
                      ),
                    ),
                  const SizedBox(width: 4),
                  Text(
                    symbol,
                    style: TextStyle(color: context.theme.text, fontSize: 13),
                  ),
                ],
              ),
              const SizedBox(height: 16),
              AutoSizeText(
                _numberFormat(amount),
                maxFontSize: 36,
                minFontSize: 24,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  color: context.theme.text,
                  fontFamily: 'MixinCondensed',
                  fontSize: 36,
                  height: 1,
                ),
              ),
              if (memo.isNotEmpty) ...[
                const SizedBox(height: 10),
                CustomText(
                  memo,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: context.theme.secondaryText,
                    fontSize: 12,
                  ),
                ),
                const SizedBox(height: 2),
              ] else
                const SizedBox(height: 6),
            ],
          ),
        ),
      ],
    ),
  );
}

class _SnapshotData {
  const _SnapshotData({
    required this.id,
    required this.amount,
    required this.symbol,
    required this.name,
    required this.contentType,
    required this.contentUrl,
  });
  final String? id;
  final String amount;
  final String symbol;
  final String name;
  final String? contentType;
  final String? contentUrl;

  static _SnapshotData parse(String raw) {
    try {
      final json = jsonDecode(raw) as Map<String, dynamic>;
      return _SnapshotData(
        id: nonEmpty(json['snapshot_id']) ?? nonEmpty(json['inscription_hash']),
        amount: json['amount']?.toString() ?? '',
        symbol:
            json['symbol']?.toString() ??
            json['asset_symbol']?.toString() ??
            '',
        name: json['name']?.toString() ?? '',
        contentType: nonEmpty(json['content_type']),
        contentUrl: nonEmpty(json['content_url']),
      );
    } on Object {
      return const _SnapshotData(
        id: null,
        amount: '',
        symbol: '',
        name: '',
        contentType: null,
        contentUrl: null,
      );
    }
  }
}

String _decodeSafeMemo(String raw) {
  if (raw.isEmpty ||
      raw.length.isOdd ||
      !RegExp(r'^[0-9a-fA-F]+$').hasMatch(raw)) {
    return raw;
  }
  try {
    final bytes = <int>[
      for (var index = 0; index < raw.length; index += 2)
        int.parse(raw.substring(index, index + 2), radix: 16),
    ];
    return utf8.decode(bytes);
  } on Object {
    return raw;
  }
}

String _numberFormat(String value) {
  if (value.isEmpty) return value;
  try {
    return DecimalFormatter(
      intl.NumberFormat('#,###.########'),
    ).format(Decimal.parse(value));
  } on Object {
    return value;
  }
}

class _InscriptionCard extends StatelessWidget {
  const _InscriptionCard({required this.message, required this.data});
  final MessageListEntry message;
  final _SnapshotData data;

  @override
  Widget build(BuildContext context) => SizedBox(
    width: 260,
    height: 112,
    child: Row(
      children: [
        InscriptionContent(
          contentType: message.inscriptionContentType ?? data.contentType,
          contentUrl: message.inscriptionContentUrl ?? data.contentUrl,
          iconUrl: message.inscriptionIconUrl,
          mode: InscriptionContentMode.small,
        ),
        Expanded(
          child: Padding(
            padding: const EdgeInsets.all(10),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  message.inscriptionName ?? data.name,
                  style: TextStyle(color: context.theme.text, fontSize: 14),
                ),
                const SizedBox(height: 4),
                Text(
                  message.inscriptionSequence == null
                      ? ''
                      : '#${message.inscriptionSequence}',
                  style: TextStyle(
                    color: context.theme.secondaryText,
                    fontSize: 12,
                  ),
                ),
                const Spacer(),
                Row(
                  children: [
                    ColoredHashWidget(inscriptionHex: message.inscriptionHash),
                    const SizedBox(width: 10),
                    HexagonWidget(
                      type: HexagonType.FLAT,
                      cornerRadius: 4,
                      height: 22,
                      width: 22,
                      child: SizedBox.square(
                        dimension: 22,
                        child: Image.network(
                          message.inscriptionIconUrl ?? '',
                          fit: BoxFit.cover,
                          errorBuilder: (_, _, _) => SvgPicture.asset(
                            MixinAssets.collectionPlaceholder,
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(width: 2),
                  ],
                ),
                const SizedBox(height: 2),
              ],
            ),
          ),
        ),
      ],
    ),
  );
}

class _AssetIcon extends StatelessWidget {
  const _AssetIcon({required this.size, this.iconUrl, this.chainIconUrl});

  final String? iconUrl;
  final String? chainIconUrl;
  final double size;

  @override
  Widget build(BuildContext context) => SizedBox.square(
    dimension: size,
    child: Stack(
      children: [
        Positioned.fill(
          child: ClipPath(
            clipper: _SymbolCustomClipper(
              chainPlaceholderSize: chainIconUrl?.isNotEmpty == true ? 14 : 0,
            ),
            clipBehavior: Clip.antiAliasWithSaveLayer,
            child: iconUrl?.isNotEmpty == true
                ? Image.network(
                    iconUrl!,
                    fit: BoxFit.cover,
                    errorBuilder: (_, _, _) => const SizedBox.expand(),
                  )
                : const SizedBox.expand(),
          ),
        ),
        if (chainIconUrl?.isNotEmpty == true)
          Positioned(
            right: 1,
            bottom: 1,
            child: Image.network(
              chainIconUrl!,
              width: 12,
              height: 12,
              fit: BoxFit.cover,
              errorBuilder: (_, _, _) => const SizedBox.square(dimension: 12),
            ),
          ),
      ],
    ),
  );
}

class _SymbolCustomClipper extends CustomClipper<Path> {
  const _SymbolCustomClipper({required this.chainPlaceholderSize});

  final double chainPlaceholderSize;

  @override
  Path getClip(Size size) {
    final symbol = Path()..addOval(Offset.zero & size);
    if (chainPlaceholderSize <= 0) return symbol;
    final chain = Path()
      ..addOval(
        Offset(
              size.width - chainPlaceholderSize,
              size.height - chainPlaceholderSize,
            ) &
            Size.square(chainPlaceholderSize),
      );
    return Path.combine(PathOperation.difference, symbol, chain);
  }

  @override
  bool shouldReclip(covariant _SymbolCustomClipper oldClipper) =>
      oldClipper.chainPlaceholderSize != chainPlaceholderSize;
}
