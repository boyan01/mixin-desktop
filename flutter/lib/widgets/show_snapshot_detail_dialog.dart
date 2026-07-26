import 'dart:convert';
import 'dart:ui' as ui;

import 'package:decimal/decimal.dart';
import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:provider/provider.dart';

import '../l10n/l10n.dart';
import '../models/message_list_entry.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../src/rust/third_party/mixin_desktop_api/dto.dart';
import '../theme.dart';
import 'buttons.dart';
import 'high_light_text.dart';
import 'message_items/special_message_items.dart';
import 'mixin_dialog.dart';
import 'mixin_image.dart';

Future<void> showSnapshotDetailDialog(
  BuildContext context, {
  required MessageListEntry message,
}) {
  final initial = _SnapshotView.fromMessage(message);
  if (initial.inscription || initial.id.isEmpty) {
    return showMixinDialog<void>(
      context: context,
      child: _SnapshotDetailPage(snapshot: initial),
    );
  }
  final account = context.read<rust.AccountHandle>();
  return showMixinDialog<void>(
    context: context,
    child: _SnapshotDetailLoader(
      account: account,
      initial: initial,
      safe: message.category == 'SYSTEM_SAFE_SNAPSHOT',
    ),
  );
}

Future<void> showSnapshotDetailItemDialog(
  BuildContext context, {
  required SnapshotDetailItem snapshot,
}) {
  final fiatCurrency = context
      .read<rust.AccountHandle>()
      .profile()
      .fiatCurrency;
  return showMixinDialog<void>(
    context: context,
    child: _SnapshotDetailPage(
      snapshot: _SnapshotView.fromItem(snapshot, fiatCurrency: fiatCurrency),
    ),
  );
}

class _SnapshotDetailPage extends StatelessWidget {
  const _SnapshotDetailPage({required this.snapshot});

  final _SnapshotView snapshot;

  @override
  Widget build(BuildContext context) => snapshot.inscription
      ? _InscriptionDetailPage(snapshot: snapshot)
      : SizedBox(
          width: 400,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Row(
                children: [
                  Spacer(),
                  Padding(
                    padding: EdgeInsets.only(right: 12, top: 12),
                    child: MixinCloseButton(),
                  ),
                ],
              ),
              Flexible(
                child: SingleChildScrollView(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      _SnapshotDetailHeader(snapshot: snapshot),
                      if (snapshot.priceUsd != null &&
                          snapshot.fiatRate != null)
                        _SnapshotValuesDescription(snapshot: snapshot),
                      const SizedBox(height: 24),
                      Container(color: context.theme.divider, height: 10),
                      if (snapshot.isSafe)
                        _SafeTransactionDetailInfo(snapshot: snapshot)
                      else
                        _TransactionDetailInfo(snapshot: snapshot),
                    ],
                  ),
                ),
              ),
            ],
          ),
        );
}

class _SnapshotDetailLoader extends StatefulWidget {
  const _SnapshotDetailLoader({
    required this.account,
    required this.initial,
    required this.safe,
  });

  final rust.AccountHandle account;
  final _SnapshotView initial;
  final bool safe;

  @override
  State<_SnapshotDetailLoader> createState() => _SnapshotDetailLoaderState();
}

class _SnapshotDetailLoaderState extends State<_SnapshotDetailLoader> {
  late final Future<SnapshotDetailItem> _future = widget.safe
      ? widget.account.safeSnapshotById(snapshotId: widget.initial.id)
      : widget.account.snapshotById(snapshotId: widget.initial.id);

  @override
  Widget build(BuildContext context) => FutureBuilder<SnapshotDetailItem>(
    future: _future,
    builder: (context, snapshot) => _SnapshotDetailPage(
      snapshot: snapshot.hasData
          ? _SnapshotView.fromItem(
              snapshot.requireData,
              fiatCurrency: widget.account.profile().fiatCurrency,
            )
          : widget.initial,
    ),
  );
}

class _SnapshotValuesDescription extends StatelessWidget {
  const _SnapshotValuesDescription({required this.snapshot});

  final _SnapshotView snapshot;

  @override
  Widget build(BuildContext context) {
    final amount = Decimal.tryParse(snapshot.amount)?.abs();
    final price = Decimal.tryParse(snapshot.priceUsd ?? '');
    final fiatRate = snapshot.fiatRate == null
        ? null
        : Decimal.tryParse(snapshot.fiatRate.toString());
    if (amount == null ||
        amount == Decimal.zero ||
        price == null ||
        fiatRate == null) {
      return const SizedBox();
    }
    final format = NumberFormat.simpleCurrency(name: snapshot.fiatCurrency);
    final current = amount * price * fiatRate;
    final currentUnit = (current / amount).toDouble();
    final currentValue =
        '${context.l10n.valueNow(format.format(current.toDouble()))}'
        '(${format.format(currentUnit)}/${snapshot.symbol})';
    final ticker = Decimal.tryParse(snapshot.tickerPriceUsd ?? '');
    final String? pastValue;
    if (ticker == null) {
      pastValue = null;
    } else if (ticker == Decimal.zero) {
      pastValue = context.l10n.valueThen(context.l10n.na);
    } else {
      final past = amount * ticker * fiatRate;
      pastValue =
          '${context.l10n.valueThen(format.format(past.toDouble()))}'
          '(${format.format((past / amount).toDouble())}/${snapshot.symbol})';
    }
    return DefaultTextStyle.merge(
      style: TextStyle(
        fontSize: 14,
        fontWeight: FontWeight.w400,
        color: context.theme.secondaryText,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          CustomSelectableText(currentValue, enableInteractiveSelection: false),
          if (pastValue != null)
            Padding(
              padding: const EdgeInsets.only(top: 2),
              child: CustomSelectableText(
                pastValue,
                enableInteractiveSelection: false,
              ),
            ),
        ],
      ),
    );
  }
}

class _InscriptionDetailPage extends StatelessWidget {
  const _InscriptionDetailPage({required this.snapshot});

  final _SnapshotView snapshot;

  static const _icon = Color.fromRGBO(255, 255, 255, 0.9);
  static const _text = Color.fromRGBO(255, 255, 255, 0.9);
  static const _secondaryText = Color.fromRGBO(255, 255, 255, 0.4);

  @override
  Widget build(BuildContext context) => SizedBox(
    width: 400,
    child: Stack(
      fit: StackFit.expand,
      children: [
        const Material(color: Colors.black87),
        Opacity(
          opacity: 0.3,
          child: Stack(
            fit: StackFit.expand,
            children: [
              MixinImage.network(snapshot.inscriptionContentUrl),
              BackdropFilter(
                filter: ui.ImageFilter.blur(sigmaX: 25, sigmaY: 25),
                child: const SizedBox.expand(),
              ),
            ],
          ),
        ),
        Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Row(
              children: [
                Spacer(),
                Padding(
                  padding: EdgeInsets.only(right: 12, top: 12),
                  child: MixinCloseButton(color: _icon),
                ),
              ],
            ),
            Flexible(
              child: SingleChildScrollView(
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 30),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const SizedBox(height: 10),
                      ClipRRect(
                        borderRadius: BorderRadius.circular(12),
                        child: InscriptionContent(
                          contentType: snapshot.inscriptionContentType,
                          contentUrl: snapshot.inscriptionContentUrl,
                          iconUrl: snapshot.inscriptionIconUrl,
                          mode: InscriptionContentMode.large,
                        ),
                      ),
                      const SizedBox(height: 20),
                      _InscriptionInfoTile(
                        title: Text(context.l10n.hash),
                        subtitle: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            ColoredHashWidget(
                              inscriptionHex: snapshot.inscriptionHash,
                              blockSize: const ui.Size(7, 24),
                              space: 4,
                            ),
                            const SizedBox(height: 4),
                            CustomSelectableText(
                              snapshot.inscriptionHash,
                              style: const TextStyle(color: _secondaryText),
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(height: 20),
                      _InscriptionInfoTile(
                        title: Text(context.l10n.id),
                        subtitle: Text(
                          '#${snapshot.inscriptionSequence ?? ''}',
                        ),
                      ),
                      const SizedBox(height: 20),
                      _InscriptionInfoTile(
                        title: Text(context.l10n.collection),
                        subtitle: Text(snapshot.inscriptionName),
                      ),
                      const SizedBox(height: 20),
                    ],
                  ),
                ),
              ),
            ),
          ],
        ),
      ],
    ),
  );
}

class _InscriptionInfoTile extends StatelessWidget {
  const _InscriptionInfoTile({required this.title, required this.subtitle});

  final Widget title;
  final Widget subtitle;

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      DefaultTextStyle.merge(
        style: const TextStyle(
          color: _InscriptionDetailPage._secondaryText,
          fontSize: 16,
        ),
        child: title,
      ),
      const SizedBox(height: 8),
      DefaultTextStyle.merge(
        style: const TextStyle(
          color: _InscriptionDetailPage._text,
          fontSize: 16,
        ),
        child: subtitle,
      ),
    ],
  );
}

class _SnapshotDetailHeader extends StatelessWidget {
  const _SnapshotDetailHeader({required this.snapshot});

  final _SnapshotView snapshot;

  @override
  Widget build(BuildContext context) {
    final amount = double.tryParse(snapshot.amount) ?? 0;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        const SizedBox(height: 20),
        SymbolIconWithBorder(
          symbolUrl: snapshot.assetIconUrl,
          chainUrl: snapshot.chainIconUrl,
          size: 58,
          chainSize: 16,
        ),
        const SizedBox(height: 16),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 30),
          child: CustomSelectableText.rich(
            TextSpan(
              children: [
                TextSpan(
                  text: _formatAmount(snapshot.amount),
                  style: TextStyle(
                    fontSize: 32,
                    fontWeight: FontWeight.w700,
                    fontFamily: 'MixinCondensed',
                    color: snapshot.type == 'pending'
                        ? context.theme.text
                        : amount > 0
                        ? context.theme.green
                        : context.theme.red,
                  ),
                ),
                const TextSpan(text: ' '),
                TextSpan(
                  text: snapshot.symbol,
                  style: TextStyle(fontSize: 14, color: context.theme.text),
                ),
              ],
            ),
            textAlign: TextAlign.center,
          ),
        ),
        const SizedBox(height: 4),
      ],
    );
  }
}

class _TransactionDetailInfo extends StatelessWidget {
  const _TransactionDetailInfo({required this.snapshot});

  final _SnapshotView snapshot;

  @override
  Widget build(BuildContext context) {
    final positive = (double.tryParse(snapshot.amount) ?? 0) > 0;
    final type = snapshot.type;
    final rows = <(String, String)>[
      (context.l10n.transactionId, snapshot.id),
      if (snapshot.snapshotHash.isNotEmpty)
        (context.l10n.snapshotHash, snapshot.snapshotHash),
      (context.l10n.assetType, snapshot.assetName),
      (context.l10n.transactionType, _localizedType(context, type)),
      if (type == 'deposit') ...[
        (context.l10n.from, snapshot.sender),
        (context.l10n.transactionHash, snapshot.transactionHash),
      ] else if (type == 'pending') ...[
        (
          context.l10n.status,
          context.l10n.pendingConfirmation(
            snapshot.confirmations,
            snapshot.confirmations,
            snapshot.assetConfirmations,
          ),
        ),
        (context.l10n.from, snapshot.sender),
        (context.l10n.transactionHash, snapshot.transactionHash),
      ] else if (type == 'transfer') ...[
        (
          context.l10n.from,
          positive ? snapshot.opponentName : snapshot.currentUserName,
        ),
        (
          context.l10n.receiver,
          positive ? snapshot.currentUserName : snapshot.opponentName,
        ),
      ] else ...[
        (context.l10n.transactionHash, snapshot.transactionHash),
        (
          snapshot.assetTag.isNotEmpty
              ? context.l10n.address
              : context.l10n.receiver,
          snapshot.receiver,
        ),
      ],
      if (snapshot.memo.isNotEmpty) (context.l10n.memo, snapshot.memo),
      if (snapshot.openingBalance.isNotEmpty && snapshot.symbol.isNotEmpty)
        (
          context.l10n.openingBalance,
          '${snapshot.openingBalance} ${snapshot.symbol}',
        ),
      if (snapshot.closingBalance.isNotEmpty && snapshot.symbol.isNotEmpty)
        (
          context.l10n.closingBalance,
          '${snapshot.closingBalance} ${snapshot.symbol}',
        ),
      if (snapshot.createdAt != null)
        (
          context.l10n.time,
          '${DateFormat.yMMMMd().format(snapshot.createdAt!)}'
              '${DateFormat.Hms().format(snapshot.createdAt!)}',
        ),
      if (type == 'transfer' && snapshot.traceId.isNotEmpty)
        (context.l10n.trace, snapshot.traceId),
      if (snapshot.inscriptionHash.isNotEmpty)
        ('Inscription', snapshot.inscriptionHash),
      if (snapshot.inscriptionCollectionHash.isNotEmpty)
        ('Collection', snapshot.inscriptionCollectionHash),
      if (snapshot.inscriptionSequence != null)
        ('Sequence', snapshot.inscriptionSequence.toString()),
    ].where((row) => row.$2.isNotEmpty).toList(growable: false);

    return Padding(
      padding: const EdgeInsets.only(left: 24, right: 24, top: 20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [for (final row in rows) _TransactionInfoTile(row: row)],
      ),
    );
  }
}

class _SafeTransactionDetailInfo extends StatelessWidget {
  const _SafeTransactionDetailInfo({required this.snapshot});

  final _SnapshotView snapshot;

  @override
  Widget build(BuildContext context) {
    final positive = (double.tryParse(snapshot.amount) ?? 0) > 0;
    final rows = <Widget>[];
    if (snapshot.type == 'pending') {
      rows.add(
        _TransactionInfoTile(
          row: (
            context.l10n.status,
            context.l10n.pendingConfirmation(
              snapshot.confirmations,
              snapshot.confirmations,
              snapshot.assetConfirmations,
            ),
          ),
        ),
      );
      if (snapshot.depositHash.isNotEmpty) {
        rows.add(
          _TransactionInfoTile(
            row: (context.l10n.depositHash, snapshot.depositHash),
          ),
        );
      }
    } else {
      rows
        ..add(
          _TransactionInfoTile(row: (context.l10n.transactionId, snapshot.id)),
        )
        ..add(
          _TransactionInfoTile(
            row: (context.l10n.transactionHash, snapshot.transactionHash),
          ),
        );
    }
    if (snapshot.type == 'transfer') {
      rows.add(
        _TransactionInfoTile(
          row: (
            positive ? context.l10n.from : context.l10n.to,
            snapshot.opponentName.isEmpty
                ? context.l10n.na
                : snapshot.opponentName,
          ),
        ),
      );
      if (snapshot.memo.isNotEmpty) {
        rows.add(_TransactionInfoTile(row: (context.l10n.memo, snapshot.memo)));
      }
    } else if (snapshot.type == 'deposit' && snapshot.depositHash.isNotEmpty) {
      rows.add(
        _TransactionInfoTile(
          row: (context.l10n.depositHash, snapshot.depositHash),
        ),
      );
    } else if (snapshot.type == 'withdrawal') {
      rows
        ..add(
          _TransactionInfoTile(
            row: (context.l10n.to, snapshot.withdrawalReceiver),
          ),
        )
        ..add(
          snapshot.withdrawalHash.isEmpty
              ? _TransactionWidgetInfoTile(
                  title: context.l10n.withdrawalHash,
                  subtitle: SizedBox.square(
                    dimension: 20,
                    child: CircularProgressIndicator(
                      strokeWidth: 2,
                      color: context.theme.secondaryText,
                    ),
                  ),
                )
              : _TransactionInfoTile(
                  row: (context.l10n.withdrawalHash, snapshot.withdrawalHash),
                ),
        );
    }
    if (snapshot.createdAt != null) {
      rows.add(
        _TransactionInfoTile(
          row: (
            context.l10n.time,
            '${DateFormat.yMMMMd().format(snapshot.createdAt!)} '
                '${DateFormat.Hms().format(snapshot.createdAt!)}',
          ),
        ),
      );
    }
    return Padding(
      padding: const EdgeInsets.only(left: 24, right: 24, top: 20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: rows,
      ),
    );
  }
}

class _TransactionWidgetInfoTile extends StatelessWidget {
  const _TransactionWidgetInfoTile({
    required this.title,
    required this.subtitle,
  });

  final String title;
  final Widget subtitle;

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      const SizedBox(height: 12),
      Text(
        title,
        style: TextStyle(
          fontSize: 16,
          height: 1,
          color: context.theme.secondaryText,
        ),
      ),
      const SizedBox(height: 8),
      subtitle,
      const SizedBox(height: 12),
    ],
  );
}

class _TransactionInfoTile extends StatelessWidget {
  const _TransactionInfoTile({required this.row});

  final (String, String) row;

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      const SizedBox(height: 12),
      Text(
        row.$1,
        style: TextStyle(
          fontSize: 16,
          height: 1,
          color: context.theme.secondaryText,
        ),
      ),
      const SizedBox(height: 8),
      CustomSelectableText(
        row.$2,
        style: TextStyle(fontSize: 16, height: 1, color: context.theme.text),
      ),
      const SizedBox(height: 12),
    ],
  );
}

class SymbolIconWithBorder extends StatelessWidget {
  const SymbolIconWithBorder({
    required this.symbolUrl,
    required this.chainUrl,
    required this.size,
    required this.chainSize,
    super.key,
  });

  final String symbolUrl;
  final String chainUrl;
  final double size;
  final double chainSize;

  @override
  Widget build(BuildContext context) => SizedBox.square(
    dimension: size,
    child: Stack(
      children: [
        Positioned.fill(
          child: ClipPath(
            clipper: _SymbolClipper(chainPlaceholderSize: chainSize + 2),
            clipBehavior: Clip.antiAliasWithSaveLayer,
            child: MixinImage.network(symbolUrl),
          ),
        ),
        if (chainUrl.isNotEmpty)
          Positioned(
            right: 1,
            bottom: 1,
            child: ClipOval(
              child: MixinImage.network(
                chainUrl,
                width: chainSize,
                height: chainSize,
              ),
            ),
          ),
      ],
    ),
  );
}

class _SymbolClipper extends CustomClipper<Path> {
  const _SymbolClipper({required this.chainPlaceholderSize});

  final double chainPlaceholderSize;

  @override
  Path getClip(Size size) {
    final symbol = Path()..addOval(Offset.zero & size);
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
  bool shouldReclip(covariant _SymbolClipper oldClipper) =>
      chainPlaceholderSize != oldClipper.chainPlaceholderSize;
}

class _SnapshotView {
  const _SnapshotView({
    required this.id,
    required this.traceId,
    required this.type,
    required this.amount,
    required this.symbol,
    required this.assetName,
    required this.assetIconUrl,
    required this.chainIconUrl,
    required this.opponentName,
    required this.currentUserName,
    required this.transactionHash,
    required this.sender,
    required this.receiver,
    required this.memo,
    required this.confirmations,
    required this.assetConfirmations,
    required this.assetTag,
    required this.snapshotHash,
    required this.openingBalance,
    required this.closingBalance,
    required this.createdAt,
    required this.inscription,
    required this.inscriptionHash,
    required this.inscriptionCollectionHash,
    required this.inscriptionSequence,
    required this.inscriptionContentType,
    required this.inscriptionContentUrl,
    required this.inscriptionName,
    required this.inscriptionIconUrl,
    required this.isSafe,
    required this.priceUsd,
    required this.fiatRate,
    required this.tickerPriceUsd,
    required this.fiatCurrency,
    required this.depositHash,
    required this.withdrawalHash,
    required this.withdrawalReceiver,
  });

  factory _SnapshotView.fromItem(
    SnapshotDetailItem item, {
    required String fiatCurrency,
  }) => _SnapshotView(
    id: item.snapshotId,
    traceId: item.traceId ?? '',
    type: item.snapshotType,
    amount: item.amount,
    symbol: item.symbol,
    assetName: item.assetName,
    assetIconUrl: item.assetIconUrl,
    chainIconUrl: item.chainIconUrl,
    opponentName: item.opponentName ?? '',
    currentUserName: item.currentUserName,
    transactionHash: item.transactionHash ?? '',
    sender: item.sender ?? '',
    receiver: item.receiver ?? '',
    memo: item.memo ?? '',
    confirmations: item.confirmations ?? 0,
    assetConfirmations: item.assetConfirmations,
    assetTag: item.assetTag ?? '',
    snapshotHash: item.snapshotHash ?? '',
    openingBalance: item.openingBalance ?? '',
    closingBalance: item.closingBalance ?? '',
    createdAt: DateTime.fromMillisecondsSinceEpoch(
      item.createdAtMillis,
    ).toLocal(),
    inscription: false,
    inscriptionHash: '',
    inscriptionCollectionHash: '',
    inscriptionSequence: null,
    inscriptionContentType: '',
    inscriptionContentUrl: '',
    inscriptionName: '',
    inscriptionIconUrl: '',
    isSafe: item.isSafe,
    priceUsd: item.priceUsd,
    fiatRate: item.fiatRate,
    tickerPriceUsd: item.tickerPriceUsd,
    fiatCurrency: fiatCurrency,
    depositHash: item.depositHash ?? '',
    withdrawalHash: item.withdrawalHash ?? '',
    withdrawalReceiver: item.withdrawalReceiver ?? '',
  );

  factory _SnapshotView.fromMessage(MessageListEntry message) {
    final inscription = message.category == 'SYSTEM_SAFE_INSCRIPTION';
    return _SnapshotView(
      id: message.snapshotId ?? '',
      traceId: '',
      type: message.snapshotType ?? '',
      amount: message.snapshotAmount ?? '',
      symbol: message.snapshotAssetSymbol ?? '',
      assetName: message.snapshotAssetSymbol ?? '',
      assetIconUrl: message.snapshotAssetIconUrl ?? '',
      chainIconUrl: message.snapshotChainIconUrl ?? '',
      opponentName: message.snapshotOpponentId ?? '',
      currentUserName: '',
      transactionHash: message.snapshotTransactionHash ?? '',
      sender: '',
      receiver: '',
      memo: _decodeMemo(message.snapshotMemo ?? ''),
      confirmations: 0,
      assetConfirmations: 0,
      assetTag: '',
      snapshotHash: '',
      openingBalance: '',
      closingBalance: '',
      createdAt: _parseDate(message.snapshotCreatedAt),
      inscription: inscription,
      inscriptionHash: message.inscriptionHash ?? '',
      inscriptionCollectionHash: message.inscriptionCollectionHash ?? '',
      inscriptionSequence: message.inscriptionSequence,
      inscriptionContentType: message.inscriptionContentType ?? '',
      inscriptionContentUrl: message.inscriptionContentUrl ?? '',
      inscriptionName: message.inscriptionName ?? '',
      inscriptionIconUrl: message.inscriptionIconUrl ?? '',
      isSafe: message.category == 'SYSTEM_SAFE_SNAPSHOT',
      priceUsd: null,
      fiatRate: null,
      tickerPriceUsd: null,
      fiatCurrency: '',
      depositHash: '',
      withdrawalHash: '',
      withdrawalReceiver: '',
    );
  }

  final String id;
  final String traceId;
  final String type;
  final String amount;
  final String symbol;
  final String assetName;
  final String assetIconUrl;
  final String chainIconUrl;
  final String opponentName;
  final String currentUserName;
  final String transactionHash;
  final String sender;
  final String receiver;
  final String memo;
  final int confirmations;
  final int assetConfirmations;
  final String assetTag;
  final String snapshotHash;
  final String openingBalance;
  final String closingBalance;
  final DateTime? createdAt;
  final bool inscription;
  final String inscriptionHash;
  final String inscriptionCollectionHash;
  final int? inscriptionSequence;
  final String inscriptionContentType;
  final String inscriptionContentUrl;
  final String inscriptionName;
  final String inscriptionIconUrl;
  final bool isSafe;
  final String? priceUsd;
  final double? fiatRate;
  final String? tickerPriceUsd;
  final String fiatCurrency;
  final String depositHash;
  final String withdrawalHash;
  final String withdrawalReceiver;
}

String _localizedType(BuildContext context, String type) => switch (type) {
  'deposit' => context.l10n.deposit,
  'withdrawal' => context.l10n.withdrawal,
  'fee' => context.l10n.fee,
  'rebate' => context.l10n.rebate,
  'raw' => context.l10n.raw,
  'transfer' => context.l10n.transfer,
  _ => context.l10n.na,
};

String _formatAmount(String amount) {
  final value = num.tryParse(amount);
  return value == null ? amount : NumberFormat('#,###.########').format(value);
}

String _decodeMemo(String raw) {
  if (raw.isEmpty ||
      raw.length.isOdd ||
      !RegExp(r'^[0-9a-fA-F]+$').hasMatch(raw)) {
    return raw;
  }
  try {
    return utf8.decode([
      for (var index = 0; index < raw.length; index += 2)
        int.parse(raw.substring(index, index + 2), radix: 16),
    ]);
  } on Object {
    return raw;
  }
}

DateTime? _parseDate(String? raw) {
  if (raw == null || raw.isEmpty) return null;
  final timestamp = int.tryParse(raw);
  if (timestamp == null) return DateTime.tryParse(raw)?.toLocal();
  return DateTime.fromMillisecondsSinceEpoch(
    timestamp > 100000000000000
        ? timestamp ~/ 1000
        : timestamp < 100000000000
        ? timestamp * 1000
        : timestamp,
  ).toLocal();
}
