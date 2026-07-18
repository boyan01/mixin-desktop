import 'dart:io';

import 'package:desktop_drop/desktop_drop.dart';
import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/dash_path_border.dart';

class ChatDropOverlay extends StatefulWidget {
  const ChatDropOverlay({
    required this.child,
    required this.enabled,
    required this.onFilesDropped,
    super.key,
  });

  final Widget child;
  final bool enabled;
  final Future<void> Function(List<XFile> files) onFilesDropped;

  @override
  State<ChatDropOverlay> createState() => _ChatDropOverlayState();
}

class _ChatDropOverlayState extends State<ChatDropOverlay> {
  bool dragging = false;
  bool dialogEnabled = true;

  @override
  Widget build(BuildContext context) => DropTarget(
    onDragEntered: (_) => setState(() => dragging = true),
    onDragExited: (_) => setState(() => dragging = false),
    onDragDone: (details) async {
      final files = details.files
          .where((file) => File(file.path).existsSync())
          .toList(growable: false);
      setState(() => dragging = false);
      if (files.isEmpty) return;
      setState(() => dialogEnabled = false);
      await widget.onFilesDropped(files);
      if (mounted) setState(() => dialogEnabled = true);
    },
    enable: widget.enabled && dialogEnabled,
    child: Stack(
      children: [widget.child, if (dragging) const _ChatDragIndicator()],
    ),
  );
}

class _ChatDragIndicator extends StatelessWidget {
  const _ChatDragIndicator();

  @override
  Widget build(BuildContext context) => DecoratedBox(
    decoration: BoxDecoration(color: context.mixinTheme.popUp),
    child: Container(
      margin: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: context.mixinTheme.listSelected,
        borderRadius: const BorderRadius.all(Radius.circular(8)),
        border: DashPathBorder.all(
          borderSide: BorderSide(color: context.mixinTheme.accent),
          dashArray: CircularIntervalList([4, 4]),
        ),
      ),
      child: Center(
        child: Text(
          context.l10n.addFile,
          style: TextStyle(fontSize: 14, color: context.mixinTheme.text),
        ),
      ),
    ),
  );
}
