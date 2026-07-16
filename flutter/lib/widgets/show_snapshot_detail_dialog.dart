import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';

Future<void> showSnapshotDetailDialog(
  BuildContext context, {
  required MessageListEntry message,
}) => showDialog<void>(
  context: context,
  builder: (context) => _SnapshotDetailDialog(message: message),
);

class _SnapshotDetailDialog extends StatelessWidget {
  const _SnapshotDetailDialog({required this.message});

  final MessageListEntry message;

  @override
  Widget build(BuildContext context) {
    final inscription = message.category == 'SYSTEM_SAFE_INSCRIPTION';
    final memo = _decodeMemo(message.snapshotMemo ?? '');
    final rows = <(String, String?)>[
      ('Amount', message.snapshotAmount),
      ('Asset', message.snapshotAssetSymbol),
      (context.l10n.memo, memo),
      ('Type', message.snapshotType),
      ('Opponent', message.snapshotOpponentId),
      (context.l10n.transactionHash, message.snapshotTransactionHash),
      ('Snapshot ID', message.snapshotId),
      ('Created at', _formatCreatedAt(message.snapshotCreatedAt)),
      if (inscription) ...[
        ('Inscription', message.inscriptionHash),
        ('Collection', message.inscriptionCollectionHash),
        ('Sequence', message.inscriptionSequence?.toString()),
        ('Content type', message.inscriptionContentType),
        ('Content URL', message.inscriptionContentUrl),
      ],
    ].where((row) => row.$2?.isNotEmpty == true).toList(growable: false);

    return AlertDialog(
      title: Text(inscription ? 'Inscription' : context.l10n.transfer),
      content: SizedBox(
        width: 440,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (inscription) _InscriptionPreview(message: message),
              for (final row in rows) _DetailRow(label: row.$1, value: row.$2!),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(context.l10n.close),
        ),
      ],
    );
  }
}

class _InscriptionPreview extends StatelessWidget {
  const _InscriptionPreview({required this.message});

  final MessageListEntry message;

  @override
  Widget build(BuildContext context) {
    final contentUrl = message.inscriptionContentUrl;
    final isImage =
        message.inscriptionContentType?.startsWith('image/') == true;
    return Padding(
      padding: const EdgeInsets.only(bottom: 20),
      child: Column(
        children: [
          ClipRRect(
            borderRadius: const BorderRadius.all(Radius.circular(8)),
            child: contentUrl?.isNotEmpty == true && isImage
                ? Image.network(
                    contentUrl!,
                    width: 240,
                    height: 240,
                    fit: BoxFit.cover,
                    errorBuilder: (_, _, _) => _placeholder(context),
                  )
                : _placeholder(context),
          ),
          const SizedBox(height: 10),
          Text(
            message.inscriptionName ?? '',
            style: TextStyle(
              color: context.theme.text,
              fontSize: 16,
              fontWeight: FontWeight.w500,
            ),
          ),
        ],
      ),
    );
  }

  Widget _placeholder(BuildContext context) => Container(
    width: 240,
    height: 160,
    color: context.theme.statusBackground,
    child: Icon(Icons.hexagon_outlined, size: 64, color: context.theme.accent),
  );
}

class _DetailRow extends StatelessWidget {
  const _DetailRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 7),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 110,
          child: Text(
            label,
            style: TextStyle(color: context.theme.secondaryText, fontSize: 13),
          ),
        ),
        Expanded(
          child: SelectableText(
            value,
            style: TextStyle(color: context.theme.text, fontSize: 13),
          ),
        ),
      ],
    ),
  );
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

String? _formatCreatedAt(String? raw) {
  if (raw == null || raw.isEmpty) return null;
  DateTime? date;
  final timestamp = int.tryParse(raw);
  if (timestamp != null) {
    date = DateTime.fromMillisecondsSinceEpoch(
      timestamp > 100000000000000
          ? timestamp ~/ 1000
          : timestamp < 100000000000
          ? timestamp * 1000
          : timestamp,
    );
  } else {
    date = DateTime.tryParse(raw)?.toLocal();
  }
  return date?.toString() ?? raw;
}
