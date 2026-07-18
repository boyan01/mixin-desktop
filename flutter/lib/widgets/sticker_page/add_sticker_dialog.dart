import 'dart:io';
import 'dart:math' as math;

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:image/image.dart' as image;
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/buttons.dart';
import 'package:mixin_desktop_ui/widgets/dash_path_border.dart';
import 'package:mixin_desktop_ui/widgets/mixin_dialog.dart';
import 'package:mixin_desktop_ui/widgets/settings_widgets.dart';
import 'package:mixin_desktop_ui/widgets/toast.dart';
import 'package:mime/mime.dart';

const _minStickerFileSize = 1024;
const _maxStickerFileSize = 1024 * 1024;
const _minStickerDimension = 128;
const _maxStickerDimension = 1024;

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
  await showMixinDialog<void>(
    context: context,
    child: ConstrainedBox(
      constraints: BoxConstraints.tight(const Size(480, 600)),
      child: _AddStickerDialog(filepath: image.path, onSave: onSave),
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
    if (!await _validateSticker()) return;
    _saving = true;
    showToastLoading();
    try {
      await widget.onSave(widget.filepath);
      showToastSuccessful();
      if (mounted) Navigator.pop(context);
    } on Object catch (error) {
      showToastFailed(error);
    } finally {
      _saving = false;
    }
  }

  Future<bool> _validateSticker() async {
    final invalidFormat = context.l10n.invalidStickerFormat;
    final invalidSize = context.l10n.stickerAddInvalidSize;
    final file = File(widget.filepath);
    final mimeType = lookupMimeType(widget.filepath)?.toLowerCase();
    if (!const {
      'image/gif',
      'image/png',
      'image/webp',
      'image/jpeg',
    }.contains(mimeType)) {
      showToastFailed(ToastError(invalidFormat));
      return false;
    }
    final length = await file.length();
    if (length < _minStickerFileSize || length > _maxStickerFileSize) {
      showToastFailed(ToastError(invalidSize));
      return false;
    }
    final decoded = image.decodeImage(await file.readAsBytes());
    if (decoded == null ||
        math.min(decoded.width, decoded.height) < _minStickerDimension ||
        math.max(decoded.width, decoded.height) > _maxStickerDimension) {
      showToastFailed(ToastError(invalidSize));
      return false;
    }
    return true;
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: MixinAppBar(
      backgroundColor: Colors.transparent,
      title: Text(context.l10n.addSticker),
      leading: const SizedBox(),
      actions: [
        MixinCloseButton(
          onTap: () => Navigator.maybeOf(context, rootNavigator: true)?.pop(),
        ),
      ],
    ),
    body: ConstrainedBox(
      constraints: const BoxConstraints(minWidth: double.infinity),
      child: Column(
        children: [
          const Spacer(),
          SizedBox.square(
            dimension: 400,
            child: Container(
              decoration: BoxDecoration(
                borderRadius: const BorderRadius.all(Radius.circular(8)),
                border: DashPathBorder.all(
                  borderSide: BorderSide(
                    color: context.theme.divider,
                    width: 2,
                  ),
                  dashArray: CircularIntervalList([8, 2]),
                ),
              ),
              child: Padding(
                padding: const EdgeInsets.all(30),
                child: Image.file(File(widget.filepath), fit: BoxFit.scaleDown),
              ),
            ),
          ),
          const Spacer(),
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 30),
            child: MixinButton(onTap: _save, child: Text(context.l10n.save)),
          ),
        ],
      ),
    ),
  );
}
