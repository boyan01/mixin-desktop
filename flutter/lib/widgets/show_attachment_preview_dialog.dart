import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:archive/archive_io.dart';
import 'package:desktop_drop/desktop_drop.dart';
import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:image/image.dart' as image;
import 'package:path/path.dart' as path;
import 'package:path_provider/path_provider.dart';
import 'package:super_context_menu/super_context_menu.dart';
import 'package:video_player/video_player.dart';

import '../constants/assets.dart';
import '../constants/icon_fonts.dart';
import '../l10n/l10n.dart';
import '../theme.dart';
import 'adaptive_selection_toolbar.dart';
import 'buttons.dart';
import 'custom_context_menu.dart';
import 'image_editor.dart';
import 'interactive_decorated_box.dart';
import 'mixin_dialog.dart';

typedef SendAttachmentCallback =
    Future<bool> Function({
      required String path,
      required String kind,
      required String mimeType,
      required bool silent,
      String? name,
      int? width,
      int? height,
      int? durationMillis,
      String? thumbnail,
      String? caption,
    });

Future<bool> showAttachmentPreviewDialog({
  required BuildContext context,
  required List<XFile> files,
  required SendAttachmentCallback onSend,
}) async {
  if (files.isEmpty) return false;
  return await showMixinDialog<bool>(
        context: context,
        barrierDismissible: false,
        child: _AttachmentPreviewDialog(
          initialFiles: files.map(_PreviewFile.new).toList(growable: false),
          onSend: onSend,
        ),
      ) ??
      false;
}

enum _PreviewMode { media, files, zip }

class _PreviewFile {
  _PreviewFile(this.file);

  XFile file;
  String get mimeType => _mimeType(file);
  bool get isImage => mimeType.startsWith('image/');
  bool get isVideo => mimeType.startsWith('video/');
  bool get isMedia => isImage || isVideo;
}

class _AttachmentPreviewDialog extends StatefulWidget {
  const _AttachmentPreviewDialog({
    required this.initialFiles,
    required this.onSend,
  });

  final List<_PreviewFile> initialFiles;
  final SendAttachmentCallback onSend;

  @override
  State<_AttachmentPreviewDialog> createState() =>
      _AttachmentPreviewDialogState();
}

class _AttachmentPreviewDialogState extends State<_AttachmentPreviewDialog> {
  late final List<_PreviewFile> files = [...widget.initialFiles];
  final captionController = TextEditingController();
  final zipPasswordController = TextEditingController();
  late _PreviewMode mode = hasMedia ? _PreviewMode.media : _PreviewMode.files;

  bool get hasMedia => files.any((file) => file.isMedia);
  bool get showZip => files.length > 1;
  bool get singleImage => files.length == 1 && files.single.isImage;

  @override
  void dispose() {
    captionController.dispose();
    zipPasswordController.dispose();
    super.dispose();
  }

  void _addFiles(Iterable<XFile> added) {
    final paths = files.map((file) => file.file.path).toSet();
    setState(() {
      files.addAll(
        added.where((file) => paths.add(file.path)).map(_PreviewFile.new),
      );
      if (!hasMedia && mode == _PreviewMode.media) mode = _PreviewMode.files;
      if (!showZip && mode == _PreviewMode.zip) mode = _PreviewMode.files;
    });
  }

  void _remove(_PreviewFile file) {
    setState(() {
      files.remove(file);
      if (!hasMedia && mode == _PreviewMode.media) mode = _PreviewMode.files;
      if (!showZip && mode == _PreviewMode.zip) mode = _PreviewMode.files;
    });
    if (files.isEmpty) Navigator.pop(context, false);
  }

  Future<void> _send({bool silent = false}) async {
    if (files.isEmpty) return;
    if (mode == _PreviewMode.zip) {
      final archive = _PreviewFile(XFile(await _archiveFiles()));
      if (!mounted) return;
      unawaited(_sendFiles([archive], silent: silent));
      Navigator.pop(context, true);
      return;
    }
    final outgoing = [...files];
    final caption = singleImage ? captionController.text.trim() : null;
    unawaited(_sendFiles(outgoing, silent: silent, caption: caption));
    Navigator.pop(context, true);
  }

  Future<void> _sendFiles(
    List<_PreviewFile> outgoing, {
    required bool silent,
    String? caption,
  }) async {
    for (final file in outgoing) {
      if (!await _sendFile(file, silent: silent, caption: caption)) return;
    }
  }

  Future<bool> _sendFile(
    _PreviewFile preview, {
    required bool silent,
    String? caption,
  }) async {
    var file = preview.file;
    var mimeType = preview.mimeType;
    var kind = 'DATA';
    int? width;
    int? height;
    int? durationMillis;
    String? thumbnail;

    if (preview.isImage) {
      kind = 'IMAGE';
      if (mode == _PreviewMode.media) {
        final compressed = await _compressImage(file);
        if (compressed != null) {
          file = compressed.$1;
          mimeType = compressed.$2;
          width = compressed.$3;
          height = compressed.$4;
        }
      }
      final dimensions = width == null ? await _imageDimensions(file) : null;
      width ??= dimensions?.$1;
      height ??= dimensions?.$2;
    } else if (preview.isVideo && mode == _PreviewMode.media) {
      final metadata = await _videoMetadata(file);
      if (metadata != null) {
        kind = 'VIDEO';
        width = metadata.$1;
        height = metadata.$2;
        durationMillis = metadata.$3;
        thumbnail = 'L1GIo.]day]K-;jsfQjsRjfQj[fQ';
      }
    }
    return widget.onSend(
      path: file.path,
      kind: kind,
      mimeType: mimeType,
      name: file.name,
      width: width,
      height: height,
      durationMillis: durationMillis,
      thumbnail: thumbnail,
      caption: caption,
      silent: silent,
    );
  }

  Future<String> _archiveFiles() async {
    final temporary = await getTemporaryDirectory();
    final target = path.join(
      temporary.path,
      'mixin_archive_${DateTime.now().millisecondsSinceEpoch}.zip',
    );
    final password = zipPasswordController.text.trim();
    final encoder = ZipFileEncoder(password: password.isEmpty ? null : password)
      ..create(target);
    for (final file in files) {
      await encoder.addFile(File(file.file.path), file.file.name);
    }
    await encoder.close();
    return target;
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.mixinTheme;
    return Shortcuts(
      shortcuts: const {
        SingleActivator(LogicalKeyboardKey.enter): ActivateIntent(),
      },
      child: Actions(
        actions: {
          ActivateIntent: CallbackAction<ActivateIntent>(
            onInvoke: (_) {
              unawaited(_send());
              return null;
            },
          ),
        },
        child: Material(
          color: colors.popUp,
          child: DropTarget(
            onDragDone: (detail) => _addFiles(detail.files),
            child: SizedBox(
              width: 480,
              height: 600,
              child: Stack(
                children: [
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const SizedBox(height: 20),
                      Padding(
                        padding: const EdgeInsets.only(left: 15),
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            _PreviewTab(
                              asset: MixinAssets.filePreviewImages,
                              tooltip: context.l10n.sendQuickly,
                              selected: mode == _PreviewMode.media,
                              show: hasMedia,
                              onTap: () =>
                                  setState(() => mode = _PreviewMode.media),
                            ),
                            _PreviewTab(
                              asset: MixinAssets.filePreviewFiles,
                              tooltip: context.l10n.sendWithoutCompression,
                              selected: mode == _PreviewMode.files,
                              onTap: () =>
                                  setState(() => mode = _PreviewMode.files),
                            ),
                            _PreviewTab(
                              asset: MixinAssets.filePreviewZip,
                              tooltip: context.l10n.sendArchived,
                              selected: mode == _PreviewMode.zip,
                              show: showZip,
                              onTap: () =>
                                  setState(() => mode = _PreviewMode.zip),
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(height: 4),
                      Expanded(
                        child: LayoutBuilder(
                          builder: (context, constraints) =>
                              mode == _PreviewMode.zip
                              ? _ZipPage(controller: zipPasswordController)
                              : ListView.builder(
                                  padding: EdgeInsets.zero,
                                  itemCount: files.length,
                                  itemBuilder: (context, index) {
                                    final file = files[index];
                                    return Padding(
                                      padding: const EdgeInsets.symmetric(
                                        vertical: 15,
                                      ),
                                      child:
                                          mode == _PreviewMode.media &&
                                              file.isImage
                                          ? _BigImageTile(
                                              file: file.file,
                                              maxHeight:
                                                  constraints.maxHeight - 30,
                                              onDelete: () => _remove(file),
                                              onEdited: (editedPath) =>
                                                  setState(
                                                    () => file.file = XFile(
                                                      editedPath,
                                                      mimeType: 'image/png',
                                                    ),
                                                  ),
                                            )
                                          : mode == _PreviewMode.media &&
                                                file.isVideo
                                          ? _BigVideoTile(
                                              file: file.file,
                                              onDelete: () => _remove(file),
                                            )
                                          : _NormalFileTile(
                                              file: file,
                                              onDelete: () => _remove(file),
                                            ),
                                    );
                                  },
                                ),
                        ),
                      ),
                      const SizedBox(height: 16),
                      if (singleImage)
                        Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 30),
                          child: Container(
                            constraints: const BoxConstraints(minHeight: 40),
                            decoration: BoxDecoration(
                              borderRadius: const BorderRadius.all(
                                Radius.circular(4),
                              ),
                              color: context.dynamicColor(
                                const Color.fromRGBO(245, 247, 250, 1),
                                darkColor: const Color.fromRGBO(
                                  255,
                                  255,
                                  255,
                                  0.08,
                                ),
                              ),
                            ),
                            alignment: Alignment.center,
                            child: TextField(
                              controller: captionController,
                              maxLines: 3,
                              minLines: 1,
                              inputFormatters: [
                                LengthLimitingTextInputFormatter(64 * 1024),
                              ],
                              contextMenuBuilder: (context, state) =>
                                  MixinAdaptiveSelectionToolbar(
                                    editableTextState: state,
                                  ),
                              style: TextStyle(
                                color: colors.text,
                                fontSize: 14,
                              ),
                              textAlignVertical: TextAlignVertical.center,
                              selectionHeightStyle:
                                  ui.BoxHeightStyle.includeLineSpacingMiddle,
                              decoration: InputDecoration(
                                isDense: true,
                                hintText: context.l10n.addACaption,
                                enabledBorder: InputBorder.none,
                                focusedBorder: InputBorder.none,
                                hintStyle: TextStyle(
                                  color: colors.secondaryText,
                                  fontSize: 14,
                                ),
                                contentPadding: const EdgeInsets.only(
                                  left: 8,
                                  top: 8,
                                  bottom: 8,
                                ),
                              ),
                            ),
                          ),
                        ),
                      const SizedBox(height: 16),
                      Center(
                        child: ContextMenuWidget(
                          desktopMenuWidgetBuilder:
                              CustomDesktopMenuWidgetBuilder(),
                          menuProvider: (_) => Menu(
                            children: [
                              MenuAction(
                                image: MenuImage.icon(IconFonts.mute),
                                title: context.l10n.sendWithoutSound,
                                callback: () => _send(silent: true),
                              ),
                            ],
                          ),
                          child: ElevatedButton(
                            key: const ValueKey('attachment-send'),
                            onPressed: _send,
                            style: ElevatedButton.styleFrom(
                              padding: const EdgeInsets.fromLTRB(
                                32,
                                18,
                                32,
                                18,
                              ),
                              backgroundColor: colors.accent,
                            ),
                            child: Text(
                              context.l10n.send.toUpperCase(),
                              style: const TextStyle(color: Colors.white),
                            ),
                          ),
                        ),
                      ),
                      const SizedBox(height: 24),
                      Center(
                        child: Text(
                          context.l10n.enterToSend,
                          style: const TextStyle(
                            fontSize: 12,
                            color: Colors.grey,
                          ),
                        ),
                      ),
                      const SizedBox(height: 24),
                    ],
                  ),
                  const Align(
                    alignment: AlignmentDirectional.topEnd,
                    child: Padding(
                      padding: EdgeInsets.all(22),
                      child: MixinCloseButton(),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _PreviewTab extends StatelessWidget {
  const _PreviewTab({
    required this.asset,
    required this.tooltip,
    required this.selected,
    required this.onTap,
    this.show = true,
  });

  final String asset;
  final String tooltip;
  final bool selected;
  final VoidCallback onTap;
  final bool show;

  @override
  Widget build(BuildContext context) => AnimatedSize(
    duration: const Duration(milliseconds: 300),
    curve: Curves.easeInOut,
    child: show
        ? GestureDetector(
            onTap: onTap,
            child: Padding(
              padding: const EdgeInsets.all(15),
              child: Tooltip(
                message: tooltip,
                textStyle: const TextStyle(color: Colors.white),
                child: SvgPicture.asset(
                  asset,
                  colorFilter: ColorFilter.mode(
                    selected
                        ? context.mixinTheme.accent
                        : context.mixinTheme.icon,
                    BlendMode.srcIn,
                  ),
                  width: 24,
                  height: 24,
                ),
              ),
            ),
          )
        : const SizedBox(),
  );
}

class _BigImageTile extends StatelessWidget {
  const _BigImageTile({
    required this.file,
    required this.maxHeight,
    required this.onDelete,
    required this.onEdited,
  });

  final XFile file;
  final double maxHeight;
  final VoidCallback onDelete;
  final ValueChanged<String> onEdited;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(horizontal: 30),
    child: ClipRRect(
      borderRadius: const BorderRadius.all(Radius.circular(6)),
      child: Stack(
        alignment: Alignment.bottomCenter,
        children: [
          ConstrainedBox(
            constraints: BoxConstraints(
              maxHeight: maxHeight,
              minWidth: 420,
              maxWidth: 420,
            ),
            child: Image.file(File(file.path), fit: BoxFit.fitWidth),
          ),
          _MediaActionBar(
            onDelete: onDelete,
            onEdit: () async {
              final edited = await showImageEditor(
                context,
                imagePath: file.path,
              );
              if (edited != null) onEdited(edited);
            },
          ),
        ],
      ),
    ),
  );
}

class _BigVideoTile extends StatefulWidget {
  const _BigVideoTile({required this.file, required this.onDelete});

  final XFile file;
  final VoidCallback onDelete;

  @override
  State<_BigVideoTile> createState() => _BigVideoTileState();
}

class _BigVideoTileState extends State<_BigVideoTile> {
  late final VideoPlayerController controller = VideoPlayerController.file(
    File(widget.file.path),
  );

  @override
  void initState() {
    super.initState();
    unawaited(
      controller.initialize().then((_) async {
        await controller.setVolume(0);
        await controller.setLooping(true);
        await controller.play();
        if (mounted) setState(() {});
      }),
    );
  }

  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => SizedBox(
    height: 200,
    child: Padding(
      padding: const EdgeInsets.symmetric(horizontal: 30),
      child: ClipRRect(
        borderRadius: const BorderRadius.all(Radius.circular(6)),
        child: Stack(
          fit: StackFit.expand,
          children: [
            const ColoredBox(color: Colors.black),
            if (controller.value.isInitialized)
              Center(
                child: AspectRatio(
                  aspectRatio: controller.value.aspectRatio,
                  child: VideoPlayer(controller),
                ),
              ),
            if (controller.value.isInitialized)
              Positioned(
                left: 6,
                top: 6,
                child: _VideoPositionText(controller: controller),
              ),
            Align(
              alignment: Alignment.bottomCenter,
              child: _MediaActionBar(onDelete: widget.onDelete),
            ),
          ],
        ),
      ),
    ),
  );
}

class _MediaActionBar extends StatelessWidget {
  const _MediaActionBar({required this.onDelete, this.onEdit});

  final VoidCallback onDelete;
  final VoidCallback? onEdit;

  @override
  Widget build(BuildContext context) => Container(
    height: 50,
    decoration: BoxDecoration(
      gradient: LinearGradient(
        colors: [Colors.transparent, Colors.black.withValues(alpha: 0.28)],
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
      ),
    ),
    child: Row(
      children: [
        const Spacer(),
        if (onEdit != null)
          IconButton(
            onPressed: onEdit,
            icon: SvgPicture.asset(
              MixinAssets.editImage,
              width: 24,
              height: 24,
              colorFilter: const ColorFilter.mode(
                Colors.white,
                BlendMode.srcIn,
              ),
            ),
          ),
        IconButton(
          onPressed: onDelete,
          icon: SvgPicture.asset(MixinAssets.delete, width: 24, height: 24),
        ),
      ],
    ),
  );
}

class _VideoPositionText extends StatelessWidget {
  const _VideoPositionText({required this.controller});

  final VideoPlayerController controller;

  @override
  Widget build(BuildContext context) => ValueListenableBuilder(
    valueListenable: controller,
    builder: (context, value, child) {
      final remaining = value.duration - value.position;
      final minutes = remaining.inMinutes;
      final seconds = remaining.inSeconds.remainder(60).abs();
      return DecoratedBox(
        decoration: const BoxDecoration(
          color: Color.fromRGBO(0, 0, 0, 0.3),
          borderRadius: BorderRadius.all(Radius.circular(5)),
        ),
        child: Padding(
          padding: const EdgeInsets.all(4),
          child: Text(
            '$minutes:${seconds.toString().padLeft(2, '0')}',
            style: const TextStyle(fontSize: 12, color: Colors.white),
          ),
        ),
      );
    },
  );
}

class _NormalFileTile extends StatelessWidget {
  const _NormalFileTile({required this.file, required this.onDelete});

  final _PreviewFile file;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) => Row(
    children: [
      const SizedBox(width: 30),
      _FileIcon(extension: _extension(file.file.name)),
      const SizedBox(width: 16),
      Expanded(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              file.file.name,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                color: context.mixinTheme.text,
                fontSize: 16,
                height: 1.5,
              ),
            ),
            FutureBuilder<int>(
              future: file.file.length(),
              builder: (context, snapshot) => Text(
                _formatBytes(snapshot.data ?? 0),
                style: TextStyle(
                  color: context.mixinTheme.secondaryText,
                  fontSize: 14,
                ),
              ),
            ),
          ],
        ),
      ),
      GestureDetector(
        onTap: onDelete,
        child: Padding(
          padding: const EdgeInsets.all(10),
          child: SvgPicture.asset(
            MixinAssets.delete,
            width: 24,
            height: 24,
            colorFilter: ColorFilter.mode(
              context.mixinTheme.secondaryText,
              BlendMode.srcIn,
            ),
          ),
        ),
      ),
      const SizedBox(width: 10),
    ],
  );
}

class _FileIcon extends StatelessWidget {
  const _FileIcon({required this.extension});

  final String extension;

  @override
  Widget build(BuildContext context) => Container(
    width: 50,
    height: 50,
    alignment: Alignment.center,
    decoration: BoxDecoration(
      color: context.mixinTheme.statusBackground,
      shape: BoxShape.circle,
    ),
    child: Text(
      extension,
      style: const TextStyle(color: mixinSecondaryText, fontSize: 16),
    ),
  );
}

class _ZipPage extends StatefulWidget {
  const _ZipPage({required this.controller});

  final TextEditingController controller;

  @override
  State<_ZipPage> createState() => _ZipPageState();
}

class _ZipPageState extends State<_ZipPage> {
  bool obscure = true;
  final focusNode = FocusNode();

  @override
  void dispose() {
    focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Column(
    children: [
      Row(
        children: [
          const SizedBox(width: 30),
          const _FileIcon(extension: 'ZIP'),
          const SizedBox(width: 16),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Archive.zip',
                  style: TextStyle(
                    color: context.mixinTheme.text,
                    fontSize: 16,
                    height: 1.5,
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  context.l10n.archivedFolder,
                  style: TextStyle(
                    color: context.mixinTheme.secondaryText,
                    fontSize: 14,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: 30),
        ],
      ),
      const Spacer(),
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 30),
        child: SizedBox(
          width: 300,
          child: ValueListenableBuilder<TextEditingValue>(
            valueListenable: widget.controller,
            builder: (context, value, _) => InteractiveDecoratedBox(
              decoration: ShapeDecoration(
                color: context.dynamicColor(
                  const Color.fromRGBO(245, 247, 250, 1),
                  darkColor: const Color.fromRGBO(255, 255, 255, 0.08),
                ),
                shape: const StadiumBorder(),
              ),
              cursor: SystemMouseCursors.text,
              onTap: focusNode.requestFocus,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: SizedBox(
                  height: 36,
                  child: Row(
                    children: [
                      SvgPicture.asset(
                        MixinAssets.lock,
                        width: 18,
                        height: 18,
                        colorFilter: ColorFilter.mode(
                          value.text.isNotEmpty
                              ? context.theme.text
                              : context.theme.secondaryText,
                          BlendMode.srcIn,
                        ),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: TextField(
                          focusNode: focusNode,
                          autofocus: true,
                          controller: widget.controller,
                          obscureText: obscure,
                          scrollPadding: EdgeInsets.zero,
                          contextMenuBuilder: (context, state) =>
                              MixinAdaptiveSelectionToolbar(
                                editableTextState: state,
                              ),
                          inputFormatters: [
                            LengthLimitingTextInputFormatter(128),
                          ],
                          style: TextStyle(
                            color: context.theme.text,
                            fontSize: 14,
                          ),
                          decoration: InputDecoration(
                            isDense: true,
                            filled: true,
                            border: InputBorder.none,
                            fillColor: Colors.transparent,
                            hoverColor: Colors.transparent,
                            focusColor: Colors.transparent,
                            contentPadding: EdgeInsets.zero,
                            hintText: context.l10n.encryptZipFileWithPassword,
                            hintStyle: TextStyle(
                              color: context.theme.secondaryText,
                              fontSize: 14,
                            ),
                          ),
                        ),
                      ),
                      MouseRegion(
                        cursor: SystemMouseCursors.click,
                        child: GestureDetector(
                          onTap: () => setState(() => obscure = !obscure),
                          child: Icon(
                            obscure
                                ? Icons.visibility_off_outlined
                                : Icons.visibility_outlined,
                            size: 20,
                            color: value.text.isNotEmpty
                                ? context.theme.text
                                : context.theme.secondaryText,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    ],
  );
}

Future<(XFile, String, int, int)?> _compressImage(XFile file) async {
  try {
    final bytes = await file.readAsBytes();
    final decoder = image.findDecoderForData(bytes);
    var decoded = decoder?.decode(bytes);
    if (decoded == null || decoder is image.GifDecoder) return null;
    const maxDimension = 1920;
    if (decoded.width > maxDimension || decoded.height > maxDimension) {
      final ratio = decoded.width / decoded.height;
      decoded = ratio > 1
          ? image.copyResize(
              decoded,
              width: maxDimension,
              height: (maxDimension / ratio).round(),
            )
          : image.copyResize(
              decoded,
              width: (maxDimension * ratio).round(),
              height: maxDimension,
            );
    }
    final transparent = decoded.any(
      (pixel) => pixel.a < decoded!.maxChannelValue,
    );
    final data = transparent
        ? image.encodePng(decoded)
        : image.JpegEncoder(quality: 85).encode(decoded);
    final mimeType = transparent ? 'image/png' : 'image/jpeg';
    final extension = transparent ? 'png' : 'jpg';
    final directory = await getTemporaryDirectory();
    final target = File(
      path.join(
        directory.path,
        'mixin_image_${DateTime.now().microsecondsSinceEpoch}.$extension',
      ),
    );
    await target.writeAsBytes(data, flush: true);
    return (
      XFile(target.path, mimeType: mimeType),
      mimeType,
      decoded.width,
      decoded.height,
    );
  } on Object {
    return null;
  }
}

Future<(int, int)?> _imageDimensions(XFile file) async {
  try {
    final codec = await ui.instantiateImageCodec(await file.readAsBytes());
    final frame = await codec.getNextFrame();
    final dimensions = (frame.image.width, frame.image.height);
    frame.image.dispose();
    codec.dispose();
    return dimensions;
  } on Object {
    return null;
  }
}

Future<(int, int, int)?> _videoMetadata(XFile file) async {
  final controller = VideoPlayerController.file(File(file.path));
  try {
    await controller.initialize();
    return (
      controller.value.size.width.round(),
      controller.value.size.height.round(),
      controller.value.duration.inMilliseconds,
    );
  } on Object {
    return null;
  } finally {
    await controller.dispose();
  }
}

String _mimeType(XFile file) {
  final provided = file.mimeType?.trim();
  if (provided?.isNotEmpty == true) return provided!;
  return switch (_extension(file.name).toLowerCase()) {
    'png' => 'image/png',
    'jpg' || 'jpeg' => 'image/jpeg',
    'gif' => 'image/gif',
    'webp' => 'image/webp',
    'bmp' => 'image/bmp',
    'mp4' => 'video/mp4',
    'mov' => 'video/quicktime',
    'm4v' => 'video/x-m4v',
    'pdf' => 'application/pdf',
    'txt' => 'text/plain',
    'json' => 'application/json',
    'zip' => 'application/zip',
    _ => 'application/octet-stream',
  };
}

String _extension(String name) {
  final extension = path.extension(name).replaceFirst('.', '').toUpperCase();
  return extension.isEmpty ? 'FILE' : extension;
}

String _formatBytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  if (bytes < 1024 * 1024 * 1024) {
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
  return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(1)} GB';
}
