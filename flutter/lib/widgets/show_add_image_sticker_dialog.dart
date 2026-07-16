import 'dart:io';

import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';

Future<void> showAddImageStickerDialog(
  BuildContext context, {
  required File file,
  required Future<void> Function() onConfirm,
}) => showDialog<void>(
  context: context,
  builder: (context) =>
      _AddImageStickerDialog(file: file, onConfirm: onConfirm),
);

class _AddImageStickerDialog extends StatefulWidget {
  const _AddImageStickerDialog({required this.file, required this.onConfirm});

  final File file;
  final Future<void> Function() onConfirm;

  @override
  State<_AddImageStickerDialog> createState() => _AddImageStickerDialogState();
}

class _AddImageStickerDialogState extends State<_AddImageStickerDialog> {
  bool _saving = false;

  Future<void> _save() async {
    if (_saving) return;
    setState(() => _saving = true);
    try {
      await widget.onConfirm();
      if (!mounted) return;
      final messenger = ScaffoldMessenger.of(context);
      final message = context.l10n.successful;
      Navigator.pop(context);
      messenger.showSnackBar(SnackBar(content: Text(message)));
    } on Object catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('${context.l10n.addStickerFailed}: $error')),
      );
      setState(() => _saving = false);
    }
  }

  @override
  Widget build(BuildContext context) => Dialog(
    child: SizedBox(
      width: 480,
      height: 600,
      child: Column(
        children: [
          SizedBox(
            height: 56,
            child: Row(
              children: [
                const SizedBox(width: 48),
                Expanded(
                  child: Text(
                    context.l10n.addSticker,
                    textAlign: TextAlign.center,
                    style: TextStyle(
                      color: context.theme.text,
                      fontSize: 18,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ),
                IconButton(
                  onPressed: _saving ? null : () => Navigator.pop(context),
                  icon: const Icon(Icons.close),
                ),
              ],
            ),
          ),
          const Spacer(),
          Container(
            width: 400,
            height: 400,
            padding: const EdgeInsets.all(30),
            decoration: BoxDecoration(
              border: Border.all(color: context.theme.divider, width: 2),
              borderRadius: const BorderRadius.all(Radius.circular(8)),
            ),
            child: Image.file(
              widget.file,
              fit: BoxFit.scaleDown,
              errorBuilder: (_, _, _) => Icon(
                Icons.broken_image_outlined,
                size: 64,
                color: context.theme.secondaryText,
              ),
            ),
          ),
          const Spacer(),
          Padding(
            padding: const EdgeInsets.only(bottom: 30),
            child: SizedBox(
              width: 200,
              height: 44,
              child: FilledButton(
                onPressed: _saving ? null : _save,
                child: _saving
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : Text(context.l10n.save),
              ),
            ),
          ),
        ],
      ),
    ),
  );
}
