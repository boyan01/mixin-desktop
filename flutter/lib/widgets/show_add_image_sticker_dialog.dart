import 'dart:io';

import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/buttons.dart';
import 'package:mixin_desktop_ui/widgets/dash_path_border.dart';
import 'package:mixin_desktop_ui/widgets/mixin_dialog.dart';
import 'package:mixin_desktop_ui/widgets/settings_widgets.dart';
import 'package:mixin_desktop_ui/widgets/toast.dart';

Future<void> showAddImageStickerDialog(
  BuildContext context, {
  required File file,
  required Future<void> Function() onConfirm,
}) => showMixinDialog<void>(
  context: context,
  child: ConstrainedBox(
    constraints: BoxConstraints.tight(const Size(480, 600)),
    child: _AddImageStickerDialog(file: file, onConfirm: onConfirm),
  ),
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
    _saving = true;
    showToastLoading();
    try {
      await widget.onConfirm();
      if (!mounted) return;
      showToastSuccessful();
      Navigator.pop(context);
    } on Object catch (error) {
      showToastFailed(error);
    } finally {
      _saving = false;
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: MixinAppBar(
      backgroundColor: Colors.transparent,
      title: Text(context.l10n.addSticker),
      leading: const SizedBox(),
      actions: [MixinCloseButton(onTap: () => Navigator.pop(context))],
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
                child: Image.file(widget.file, fit: BoxFit.scaleDown),
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
