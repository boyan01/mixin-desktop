import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/mixin_dialog.dart';
import 'package:mixin_desktop_ui/widgets/action_button.dart';
import 'package:mixin_desktop_ui/widgets/custom_popup_menu.dart';
import 'package:path/path.dart' as path;
import 'package:path_provider/path_provider.dart';

Future<String?> showImageEditor(
  BuildContext context, {
  required String imagePath,
}) => showDialog<String>(
  context: context,
  builder: (context) => _ImageEditorDialog(imagePath: imagePath),
);

enum _DrawMode { none, brush, eraser }

class _DrawLine {
  const _DrawLine(this.points, this.color);

  final List<Offset> points;
  final Color color;
}

class _ImageEditorDialog extends StatefulWidget {
  const _ImageEditorDialog({required this.imagePath});

  final String imagePath;

  @override
  State<_ImageEditorDialog> createState() => _ImageEditorDialogState();
}

class _ImageEditorDialogState extends State<_ImageEditorDialog> {
  final boundaryKey = GlobalKey();
  final lines = <_DrawLine>[];
  final redoLines = <_DrawLine>[];
  final backupLines = <_DrawLine>[];
  _DrawLine? currentLine;
  _DrawMode drawMode = _DrawMode.none;
  Color drawColor = const Color.fromRGBO(255, 76, 79, 1);
  int quarterTurns = 0;
  bool flipped = false;
  double? cropRatio;
  bool saving = false;
  late final Future<Size> imageSize = _loadImageSize();

  Future<Size> _loadImageSize() async {
    final codec = await ui.instantiateImageCodec(
      await File(widget.imagePath).readAsBytes(),
    );
    final frame = await codec.getNextFrame();
    final size = Size(
      frame.image.width.toDouble(),
      frame.image.height.toDouble(),
    );
    frame.image.dispose();
    codec.dispose();
    return size;
  }

  bool get canReset =>
      quarterTurns != 0 || flipped || cropRatio != null || lines.isNotEmpty;

  void _reset() => setState(() {
    quarterTurns = 0;
    flipped = false;
    cropRatio = null;
    lines.clear();
    redoLines.clear();
    drawMode = _DrawMode.none;
  });

  Future<void> _cancel() async {
    if (canReset) {
      final confirmed = await showConfirmMixinDialog(
        context,
        context.l10n.editImageClearWarning,
      );
      if (confirmed != DialogEvent.positive || !mounted) return;
    }
    Navigator.pop(context);
  }

  Future<void> _done() async {
    if (saving) return;
    setState(() => saving = true);
    try {
      final boundary =
          boundaryKey.currentContext?.findRenderObject()
              as RenderRepaintBoundary?;
      if (boundary == null) return;
      final rendered = await boundary.toImage(pixelRatio: 2);
      final data = await rendered.toByteData(format: ui.ImageByteFormat.png);
      rendered.dispose();
      if (data == null) return;
      final directory = await getTemporaryDirectory();
      final target = File(
        path.join(
          directory.path,
          'mixin_edited_${DateTime.now().microsecondsSinceEpoch}.png',
        ),
      );
      await target.writeAsBytes(data.buffer.asUint8List(), flush: true);
      if (mounted) Navigator.pop(context, target.path);
    } finally {
      if (mounted) setState(() => saving = false);
    }
  }

  void _startDraw(DragStartDetails details) {
    if (drawMode == _DrawMode.eraser) {
      final index = lines.lastIndexWhere(
        (line) => line.points.any(
          (point) => (point - details.localPosition).distance <= 18,
        ),
      );
      if (index != -1) {
        setState(() => redoLines.add(lines.removeAt(index)));
      }
      return;
    }
    if (drawMode != _DrawMode.brush) return;
    redoLines.clear();
    currentLine = _DrawLine([details.localPosition], drawColor);
    setState(() => lines.add(currentLine!));
  }

  void _updateDraw(DragUpdateDetails details) {
    if (drawMode != _DrawMode.brush || currentLine == null) return;
    setState(() => currentLine!.points.add(details.localPosition));
  }

  @override
  Widget build(BuildContext context) => BackdropFilter(
    filter: ui.ImageFilter.blur(sigmaX: 10, sigmaY: 10),
    child: ColoredBox(
      color: context.mixinTheme.background.withValues(alpha: 0.8),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 20),
        child: Column(
          children: [
            const SizedBox(height: 56),
            Expanded(
              child: FutureBuilder<Size>(
                future: imageSize,
                builder: (context, snapshot) {
                  if (!snapshot.hasData) {
                    return const Center(child: CircularProgressIndicator());
                  }
                  final sourceRatio = snapshot.data!.aspectRatio;
                  final rotatedRatio = quarterTurns.isOdd
                      ? 1 / sourceRatio
                      : sourceRatio;
                  return Center(
                    child: AspectRatio(
                      aspectRatio: cropRatio ?? rotatedRatio,
                      child: RepaintBoundary(
                        key: boundaryKey,
                        child: ClipRect(
                          child: Stack(
                            fit: StackFit.expand,
                            children: [
                              ColoredBox(
                                color: Colors.transparent,
                                child: Transform(
                                  alignment: Alignment.center,
                                  transform: Matrix4.diagonal3Values(
                                    flipped ? -1.0 : 1.0,
                                    1,
                                    1,
                                  ),
                                  child: RotatedBox(
                                    quarterTurns: quarterTurns,
                                    child: Image.file(
                                      File(widget.imagePath),
                                      fit: cropRatio == null
                                          ? BoxFit.contain
                                          : BoxFit.cover,
                                    ),
                                  ),
                                ),
                              ),
                              GestureDetector(
                                behavior: HitTestBehavior.opaque,
                                onPanStart: _startDraw,
                                onPanUpdate: _updateDraw,
                                onPanEnd: (_) => currentLine = null,
                                child: CustomPaint(
                                  painter: _DrawPainter(lines),
                                  size: Size.infinite,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ),
                  );
                },
              ),
            ),
            const SizedBox(height: 20),
            if (drawMode == _DrawMode.none)
              TextButton(
                onPressed: canReset ? _reset : null,
                child: Text(context.l10n.reset),
              )
            else
              _ColorSelector(
                selected: drawColor,
                onSelected: (color) => setState(() => drawColor = color),
              ),
            const SizedBox(height: 8),
            drawMode == _DrawMode.none
                ? _normalOperationBar()
                : _drawOperationBar(),
            const SizedBox(height: 56),
          ],
        ),
      ),
    ),
  );

  Widget _normalOperationBar() => Material(
    borderRadius: const BorderRadius.all(Radius.circular(8)),
    color: context.mixinTheme.stickerPlaceholderColor,
    child: SizedBox(
      height: 40,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          TextButton(
            onPressed: _cancel,
            child: Text(
              context.l10n.cancel,
              style: TextStyle(color: context.mixinTheme.text),
            ),
          ),
          _EditorAction(
            asset: MixinAssets.editImageRotate,
            active: quarterTurns != 0,
            onTap: () => setState(() => quarterTurns = (quarterTurns + 1) % 4),
          ),
          const SizedBox(width: 4),
          _EditorAction(
            asset: MixinAssets.editImageFlip,
            active: flipped,
            onTap: () => setState(() => flipped = !flipped),
          ),
          const SizedBox(width: 4),
          CustomPopupMenuButton<double>(
            alignment: Alignment.topCenter,
            color: cropRatio != null
                ? context.mixinTheme.accent
                : context.mixinTheme.icon,
            icon: MixinAssets.editImageClip,
            onSelected: (value) =>
                setState(() => cropRatio = value == 0 ? null : value),
            itemBuilder: (context) => [
              CustomPopupMenuItem(value: 0, title: context.l10n.originalImage),
              const CustomPopupMenuItem(value: 1, title: '1:1'),
              const CustomPopupMenuItem(value: 2 / 3, title: '2:3'),
              const CustomPopupMenuItem(value: 3 / 2, title: '3:2'),
              const CustomPopupMenuItem(value: 3 / 4, title: '3:4'),
              const CustomPopupMenuItem(value: 4 / 3, title: '4:3'),
              const CustomPopupMenuItem(value: 9 / 16, title: '9:16'),
              const CustomPopupMenuItem(value: 16 / 9, title: '16:9'),
            ],
          ),
          const SizedBox(width: 4),
          _EditorAction(
            asset: MixinAssets.editImageDraw,
            active: lines.isNotEmpty,
            onTap: () => setState(() {
              backupLines
                ..clear()
                ..addAll(lines);
              drawMode = _DrawMode.brush;
            }),
          ),
          TextButton(
            onPressed: saving ? null : _done,
            child: saving
                ? const SizedBox.square(
                    dimension: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : Text(context.l10n.done),
          ),
        ],
      ),
    ),
  );

  Widget _drawOperationBar() => Material(
    borderRadius: const BorderRadius.all(Radius.circular(8)),
    color: context.mixinTheme.stickerPlaceholderColor,
    child: SizedBox(
      height: 40,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          TextButton(
            onPressed: () => setState(() {
              lines
                ..clear()
                ..addAll(backupLines);
              backupLines.clear();
              redoLines.clear();
              drawMode = _DrawMode.none;
            }),
            child: Text(
              context.l10n.cancel,
              style: TextStyle(color: context.mixinTheme.text),
            ),
          ),
          _EditorAction(
            asset: MixinAssets.editImageUndo,
            enabled: lines.isNotEmpty,
            onTap: () => setState(() => redoLines.add(lines.removeLast())),
          ),
          const SizedBox(width: 4),
          _EditorAction(
            asset: MixinAssets.editImageRedo,
            enabled: redoLines.isNotEmpty,
            onTap: () => setState(() => lines.add(redoLines.removeLast())),
          ),
          const SizedBox(width: 4),
          _EditorAction(
            asset: MixinAssets.editImageDraw,
            active: drawMode == _DrawMode.brush,
            onTap: () => setState(() => drawMode = _DrawMode.brush),
          ),
          _EditorAction(
            asset: MixinAssets.editImageErase,
            active: drawMode == _DrawMode.eraser,
            onTap: () => setState(() => drawMode = _DrawMode.eraser),
          ),
          TextButton(
            onPressed: () => setState(() {
              backupLines.clear();
              drawMode = _DrawMode.none;
            }),
            child: Text(context.l10n.done),
          ),
        ],
      ),
    ),
  );
}

class _DrawPainter extends CustomPainter {
  const _DrawPainter(this.lines);

  final List<_DrawLine> lines;

  @override
  void paint(Canvas canvas, Size size) {
    for (final line in lines) {
      if (line.points.length < 2) continue;
      final path = Path()..moveTo(line.points.first.dx, line.points.first.dy);
      for (final point in line.points.skip(1)) {
        path.lineTo(point.dx, point.dy);
      }
      canvas.drawPath(
        path,
        Paint()
          ..color = line.color
          ..strokeWidth = 11
          ..strokeCap = StrokeCap.round
          ..strokeJoin = StrokeJoin.round
          ..style = PaintingStyle.stroke,
      );
    }
  }

  @override
  bool shouldRepaint(covariant _DrawPainter oldDelegate) => true;
}

class _ColorSelector extends StatelessWidget {
  const _ColorSelector({required this.selected, required this.onSelected});

  final Color selected;
  final ValueChanged<Color> onSelected;

  static const colors = [
    Color.fromRGBO(255, 76, 79, 1),
    Color.fromRGBO(255, 174, 35, 1),
    Color.fromRGBO(57, 197, 187, 1),
    Color.fromRGBO(61, 117, 227, 1),
    Color.fromRGBO(142, 92, 227, 1),
    Colors.black,
    Colors.white,
  ];

  @override
  Widget build(BuildContext context) => Row(
    mainAxisSize: MainAxisSize.min,
    children: colors
        .map(
          (color) => GestureDetector(
            onTap: () => onSelected(color),
            child: Container(
              width: 24,
              height: 24,
              margin: const EdgeInsets.symmetric(horizontal: 4),
              decoration: BoxDecoration(
                color: color,
                shape: BoxShape.circle,
                border: Border.all(
                  color: color == selected
                      ? context.mixinTheme.accent
                      : Colors.transparent,
                  width: 2,
                ),
              ),
            ),
          ),
        )
        .toList(growable: false),
  );
}

class _EditorAction extends StatelessWidget {
  const _EditorAction({
    required this.asset,
    required this.onTap,
    this.active = false,
    this.enabled = true,
  });

  final String asset;
  final VoidCallback onTap;
  final bool active;
  final bool enabled;

  @override
  Widget build(BuildContext context) => ActionButton(
    name: asset,
    color: !enabled
        ? context.mixinTheme.icon.withValues(alpha: 0.2)
        : active
        ? context.mixinTheme.accent
        : context.mixinTheme.icon,
    onTap: enabled ? onTap : null,
  );
}
