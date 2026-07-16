import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/dash_path_border.dart';

Future<void> pickAndShowAddStickerDialog(
  BuildContext context, {
  required Future<void> Function(String path) onSave,
}) async {
  final image = await openFile(
    acceptedTypeGroups: const [
      XTypeGroup(
        label: 'Images',
        extensions: ['jpg', 'jpeg', 'png', 'gif', 'webp'],
      ),
    ],
  );
  if (image == null || !context.mounted) return;
  await showDialog<void>(
    context: context,
    builder: (context) => Dialog(
      backgroundColor: context.theme.popUp,
      child: SizedBox(
        width: 480,
        height: 600,
        child: _AddStickerDialog(filepath: image.path, onSave: onSave),
      ),
    ),
  );
}

class _AddStickerDialog extends StatefulWidget {
  const _AddStickerDialog({required this.filepath, required this.onSave});

  final String filepath;
  final Future<void> Function(String path) onSave;

  @override
  State<_AddStickerDialog> createState() => _AddStickerDialogState();
}

class _AddStickerDialogState extends State<_AddStickerDialog> {
  bool _saving = false;

  Future<void> _save() async {
    if (_saving) return;
    setState(() => _saving = true);
    try {
      await widget.onSave(widget.filepath);
      if (mounted) Navigator.pop(context);
    } on Object catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(error.toString())));
      setState(() => _saving = false);
    }
  }

  @override
  Widget build(BuildContext context) => Column(
    children: [
      SizedBox(
        height: 56,
        child: Stack(
          alignment: Alignment.center,
          children: [
            Text(
              context.l10n.addSticker,
              style: TextStyle(
                color: context.theme.text,
                fontSize: 16,
                fontWeight: FontWeight.w600,
              ),
            ),
            Align(
              alignment: Alignment.centerRight,
              child: IconButton(
                onPressed: () => Navigator.pop(context),
                icon: Icon(Icons.close, color: context.theme.icon),
              ),
            ),
          ],
        ),
      ),
      const Spacer(),
      SizedBox.square(
        dimension: 400,
        child: Container(
          decoration: BoxDecoration(
            borderRadius: const BorderRadius.all(Radius.circular(8)),
            border: DashPathBorder.all(
              borderSide: BorderSide(color: context.theme.divider, width: 2),
              dashArray: CircularIntervalList([8, 2]),
            ),
          ),
          padding: const EdgeInsets.all(30),
          child: Image.file(File(widget.filepath), fit: BoxFit.scaleDown),
        ),
      ),
      const Spacer(),
      Padding(
        padding: const EdgeInsets.symmetric(vertical: 30),
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
    ],
  );
}
