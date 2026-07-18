import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:file_selector/file_selector.dart';
import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:markdown_widget/markdown_widget.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:open_file/open_file.dart';
import 'package:path/path.dart' as path;
import 'package:mixin_desktop_ui/utils/system_clipboard.dart';
import 'package:mixin_desktop_ui/widgets/post_markdown.dart';
import 'package:mixin_desktop_ui/widgets/action_button.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/buttons.dart';
import 'package:mixin_desktop_ui/widgets/custom_popup_menu.dart';
import 'package:mixin_desktop_ui/widgets/interactive_decorated_box.dart';
import 'package:mixin_desktop_ui/widgets/image_by_blur_hash.dart';
import 'package:mixin_desktop_ui/widgets/settings_widgets.dart';
import 'package:mixin_desktop_ui/widgets/unbounded_slider.dart';
import 'package:mixin_desktop_ui/widgets/video_progress_bar.dart';
import 'package:mixin_desktop_ui/widgets/toast.dart';
import 'package:provider/provider.dart';
import 'package:video_player/video_player.dart';

typedef ImagePreviewAction = FutureOr<void> Function(ImagePreviewEntry image);
typedef ImagePreviewForward = Future<bool> Function(ImagePreviewEntry image);
typedef ImagePreviewLoader =
    Future<List<ImagePreviewEntry>> Function(String boundaryMessageId);
typedef VideoPreviewForward = Future<bool> Function();

class ImagePreviewEntry {
  const ImagePreviewEntry({
    required this.id,
    required this.source,
    this.name,
    this.thumbImage,
    this.canForward = false,
    this.userId = '',
    this.userFullName = '',
    this.userIdentityNumber = '',
    this.avatarUrl = '',
  });

  final String id;
  final String source;
  final String? name;
  final String? thumbImage;
  final bool canForward;
  final String userId;
  final String userFullName;
  final String userIdentityNumber;
  final String avatarUrl;
}

class ImagePreviewPage extends StatefulWidget {
  const ImagePreviewPage({
    required this.images,
    super.key,
    this.initialIndex = 0,
    this.onCopy,
    this.onSave,
    this.onForward,
    this.loadOlder,
    this.loadNewer,
  });

  final List<ImagePreviewEntry> images;
  final int initialIndex;
  final ImagePreviewAction? onCopy;
  final ImagePreviewAction? onSave;
  final ImagePreviewForward? onForward;
  final ImagePreviewLoader? loadOlder;
  final ImagePreviewLoader? loadNewer;

  static Future<void> show(BuildContext context, ImagePreviewPage page) =>
      showGeneralDialog<void>(
        context: context,
        barrierColor: Colors.transparent,
        barrierDismissible: true,
        barrierLabel: MaterialLocalizations.of(
          context,
        ).modalBarrierDismissLabel,
        pageBuilder: (_, _, _) => page,
      );

  @override
  State<ImagePreviewPage> createState() => _ImagePreviewPageState();
}

class _ImagePreviewPageState extends State<ImagePreviewPage> {
  final TransformationController _transformationController =
      TransformationController();
  late final List<ImagePreviewEntry> _images = [...widget.images];
  late int _index;
  bool _loadingOlder = false;
  bool _loadingNewer = false;
  late bool _hasOlder = widget.loadOlder != null;
  late bool _hasNewer = widget.loadNewer != null;
  int _quarterTurns = 0;

  ImagePreviewEntry get _image => _images[_index];

  @override
  void initState() {
    super.initState();
    _index = _images.isEmpty
        ? 0
        : widget.initialIndex.clamp(0, _images.length - 1);
    WidgetsBinding.instance.addPostFrameCallback((_) => _prefetchAtBoundary());
  }

  @override
  void dispose() {
    _transformationController.dispose();
    super.dispose();
  }

  void _select(int index) {
    if (index < 0 || index >= _images.length) return;
    setState(() {
      _index = index;
      _quarterTurns = 0;
      _transformationController.value = Matrix4.identity();
    });
    _prefetchAtBoundary();
  }

  void _prefetchAtBoundary() {
    if (!mounted || _images.isEmpty) return;
    if (_index <= 1) unawaited(_loadOlder());
    if (_index + 2 >= _images.length) unawaited(_loadNewer());
  }

  Future<void> _loadOlder() async {
    final loader = widget.loadOlder;
    if (loader == null || !_hasOlder || _loadingOlder || _images.isEmpty) {
      return;
    }
    _loadingOlder = true;
    try {
      final loaded = await loader(_images.first.id);
      if (!mounted) return;
      final existing = _images.map((image) => image.id).toSet();
      final added = loaded
          .where((image) => !existing.contains(image.id))
          .toList(growable: false);
      setState(() {
        _hasOlder = added.isNotEmpty;
        _images.insertAll(0, added);
        _index += added.length;
      });
    } on Object {
      // Keep the boundary retryable after a transient database error.
    } finally {
      _loadingOlder = false;
    }
  }

  Future<void> _loadNewer() async {
    final loader = widget.loadNewer;
    if (loader == null || !_hasNewer || _loadingNewer || _images.isEmpty) {
      return;
    }
    _loadingNewer = true;
    try {
      final loaded = await loader(_images.last.id);
      if (!mounted) return;
      final existing = _images.map((image) => image.id).toSet();
      final added = loaded
          .where((image) => !existing.contains(image.id))
          .toList(growable: false);
      setState(() {
        _hasNewer = added.isNotEmpty;
        _images.addAll(added);
      });
    } on Object {
      // Keep the boundary retryable after a transient database error.
    } finally {
      _loadingNewer = false;
    }
  }

  void _zoom(double factor) {
    final scale = _transformationController.value.getMaxScaleOnAxis();
    final nextScale = (scale * factor).clamp(0.5, 5.0);
    _transformationController.value = Matrix4.diagonal3Values(
      nextScale,
      nextScale,
      1,
    );
  }

  Future<void> _forward() async {
    final callback = widget.onForward;
    if (callback == null || !_image.canForward) return;
    await callback(_image);
  }

  @override
  Widget build(BuildContext context) {
    if (_images.isEmpty) {
      return const SizedBox();
    }
    final canGoPrevious = _index > 0;
    final canGoNext = _index + 1 < _images.length;
    final darwin = Platform.isMacOS || Platform.isIOS;
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.escape): () =>
            Navigator.maybePop(context),
        SingleActivator(
          LogicalKeyboardKey.keyC,
          meta: darwin,
          control: !darwin,
        ): () =>
            widget.onCopy?.call(_image),
        const SingleActivator(LogicalKeyboardKey.arrowLeft): () =>
            _select(_index - 1),
        const SingleActivator(LogicalKeyboardKey.arrowRight): () =>
            _select(_index + 1),
        const SingleActivator(LogicalKeyboardKey.zoomIn): () => _zoom(1.25),
        SingleActivator(
          LogicalKeyboardKey.equal,
          meta: darwin,
          control: !darwin,
        ): () =>
            _zoom(1.25),
        const SingleActivator(LogicalKeyboardKey.zoomOut): () => _zoom(0.8),
        SingleActivator(
          LogicalKeyboardKey.minus,
          meta: darwin,
          control: !darwin,
        ): () =>
            _zoom(0.8),
        SingleActivator(
          LogicalKeyboardKey.keyR,
          meta: darwin,
          control: !darwin,
        ): () =>
            setState(() => _quarterTurns = (_quarterTurns + 1) % 4),
      },
      child: Focus(
        autofocus: true,
        child: Scaffold(
          backgroundColor: Colors.transparent,
          body: SafeArea(
            child: Column(
              children: [
                Container(
                  height: 70,
                  decoration: BoxDecoration(color: context.theme.primary),
                  child: Row(
                    children: [
                      const SizedBox(width: 100),
                      Expanded(
                        child: _PreviewBar(
                          image: _image,
                          onClose: () => Navigator.maybePop(context),
                          onZoomOut: () => _zoom(0.8),
                          onZoomIn: () => _zoom(1.25),
                          onRotate: () => setState(
                            () => _quarterTurns = (_quarterTurns + 1) % 4,
                          ),
                          onCopy: widget.onCopy,
                          onSave: widget.onSave,
                          onForward: _image.canForward ? _forward : null,
                        ),
                      ),
                    ],
                  ),
                ),
                Expanded(
                  child: Stack(
                    fit: StackFit.expand,
                    children: [
                      InteractiveViewer(
                        transformationController: _transformationController,
                        minScale: 0.5,
                        maxScale: 5,
                        child: Center(
                          child: RotatedBox(
                            quarterTurns: _quarterTurns,
                            child: _PreviewImage(
                              source: _image.source,
                              thumbImage: _image.thumbImage,
                            ),
                          ),
                        ),
                      ),
                      Center(
                        child: Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 30),
                          child: Row(
                            children: [
                              if (canGoPrevious)
                                InteractiveDecoratedBox(
                                  onTap: () => _select(_index - 1),
                                  child: SvgPicture.asset(
                                    MixinAssets.previewPrevious,
                                  ),
                                ),
                              const Spacer(),
                              if (canGoNext)
                                InteractiveDecoratedBox(
                                  onTap: () => _select(_index + 1),
                                  child: SvgPicture.asset(
                                    MixinAssets.previewNext,
                                  ),
                                ),
                            ],
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _PreviewBar extends StatelessWidget {
  const _PreviewBar({
    required this.image,
    required this.onClose,
    required this.onZoomOut,
    required this.onZoomIn,
    required this.onRotate,
    required this.onCopy,
    required this.onSave,
    required this.onForward,
  });

  final ImagePreviewEntry image;
  final VoidCallback onClose;
  final VoidCallback onZoomOut;
  final VoidCallback onZoomIn;
  final VoidCallback onRotate;
  final ImagePreviewAction? onCopy;
  final ImagePreviewAction? onSave;
  final VoidCallback? onForward;

  @override
  Widget build(BuildContext context) => Row(
    children: [
      AvatarView(
        userId: image.userId,
        name: image.userFullName,
        avatarUrl: image.avatarUrl,
        size: 36,
      ),
      const SizedBox(width: 10),
      Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            image.userFullName,
            style: TextStyle(fontSize: 16, color: context.theme.text),
          ),
          Text(
            image.userIdentityNumber,
            style: TextStyle(fontSize: 14, color: context.theme.secondaryText),
          ),
        ],
      ),
      const SizedBox(width: 14),
      _PreviewActions(
        image: image,
        onClose: onClose,
        onZoomOut: onZoomOut,
        onZoomIn: onZoomIn,
        onRotate: onRotate,
        onCopy: onCopy,
        onSave: onSave,
        onForward: onForward,
      ),
      const SizedBox(width: 24),
    ],
  );
}

class _PreviewActions extends StatelessWidget {
  const _PreviewActions({
    required this.image,
    required this.onClose,
    required this.onZoomOut,
    required this.onZoomIn,
    required this.onRotate,
    required this.onCopy,
    required this.onSave,
    required this.onForward,
  });

  final ImagePreviewEntry image;
  final VoidCallback onClose;
  final VoidCallback onZoomOut;
  final VoidCallback onZoomIn;
  final VoidCallback onRotate;
  final ImagePreviewAction? onCopy;
  final ImagePreviewAction? onSave;
  final VoidCallback? onForward;

  @override
  Widget build(BuildContext context) {
    const divider = SizedBox(width: 14);
    final collapsible = [
      if (onForward != null)
        ActionButton(
          name: MixinAssets.previewShare,
          size: 20,
          color: context.theme.icon,
          onTap: onForward,
        ),
      if (onCopy != null)
        ActionButton(
          name: MixinAssets.previewCopy,
          size: 20,
          color: context.theme.icon,
          onTap: () => onCopy!(image),
        ),
      if (onSave != null)
        ActionButton(
          name: MixinAssets.previewDownload,
          size: 20,
          color: context.theme.icon,
          onTap: () => onSave!(image),
        ),
    ];
    final common = [
      ActionButton(
        name: MixinAssets.previewZoomIn,
        size: 20,
        color: context.theme.icon,
        onTap: onZoomIn,
      ),
      ActionButton(
        name: MixinAssets.previewZoomOut,
        size: 20,
        color: context.theme.icon,
        onTap: onZoomOut,
      ),
      ActionButton(
        name: MixinAssets.previewRotate,
        size: 20,
        color: context.theme.icon,
        onTap: onRotate,
      ),
    ];
    final close = ActionButton(
      name: MixinAssets.previewClose,
      size: 20,
      color: context.theme.icon,
      onTap: onClose,
    );
    return Expanded(
      child: LayoutBuilder(
        builder: (context, constraints) {
          final count = common.length + collapsible.length + 1;
          final collapsed =
              count * 36 + (count - 1) * 14 >= constraints.maxWidth;
          final children = <Widget>[
            ...common,
            if (!collapsed) ...collapsible,
            close,
            if (collapsed)
              CustomPopupMenuButton<_PreviewActionType>(
                icon: MixinAssets.previewEllipsis,
                onSelected: (value) {
                  switch (value) {
                    case _PreviewActionType.forward:
                      onForward?.call();
                    case _PreviewActionType.copy:
                      onCopy?.call(image);
                    case _PreviewActionType.download:
                      onSave?.call(image);
                  }
                },
                itemBuilder: (context) => [
                  if (onForward != null)
                    CustomPopupMenuItem(
                      value: _PreviewActionType.forward,
                      icon: MixinAssets.previewShare,
                      title: context.l10n.forward,
                    ),
                  if (onCopy != null)
                    CustomPopupMenuItem(
                      value: _PreviewActionType.copy,
                      icon: MixinAssets.previewCopy,
                      title: context.l10n.copy,
                    ),
                  if (onSave != null)
                    CustomPopupMenuItem(
                      value: _PreviewActionType.download,
                      icon: MixinAssets.previewDownload,
                      title: context.l10n.download,
                    ),
                ],
              ),
          ];
          return Row(
            mainAxisSize: MainAxisSize.min,
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              for (var index = 0; index < children.length; index++) ...[
                if (index > 0) divider,
                children[index],
              ],
            ],
          );
        },
      ),
    );
  }
}

enum _PreviewActionType { forward, copy, download }

class _PreviewImage extends StatelessWidget {
  const _PreviewImage({required this.source, this.thumbImage});

  final String source;
  final String? thumbImage;

  @override
  Widget build(BuildContext context) {
    final provider = imageProviderForSource(source);
    final fallback = ImageByBlurHashOrBase64(
      imageData: thumbImage ?? '',
      fit: BoxFit.contain,
    );
    if (provider == null) return fallback;
    return Image(
      image: provider,
      fit: BoxFit.contain,
      filterQuality: FilterQuality.high,
      errorBuilder: (_, _, _) => fallback,
    );
  }
}

ImageProvider<Object>? imageProviderForSource(String source) {
  final value = source.trim();
  if (value.isEmpty) return null;
  final uri = Uri.tryParse(value);
  if (uri != null && (uri.scheme == 'http' || uri.scheme == 'https')) {
    return NetworkImage(value);
  }
  if (uri != null && uri.scheme == 'file') return FileImage(File.fromUri(uri));
  if (value.startsWith('data:')) {
    final bytes = _decodeImage(value);
    return bytes == null ? null : MemoryImage(bytes);
  }
  if (File(value).existsSync()) return FileImage(File(value));
  final bytes = _decodeImage(value);
  return bytes == null ? null : MemoryImage(bytes);
}

Uint8List? _decodeImage(String source) {
  try {
    final separator = source.indexOf(',');
    final payload = source.startsWith('data:')
        ? source.substring(separator + 1)
        : source;
    return base64Decode(payload);
  } on FormatException {
    return null;
  }
}

class VideoPreviewPage extends StatefulWidget {
  const VideoPreviewPage({
    required this.source,
    super.key,
    this.title,
    this.onForward,
    this.userId = '',
    this.userFullName = '',
    this.userIdentityNumber = '',
    this.avatarUrl = '',
    this.isTranscriptPage = false,
  });

  final String source;
  final String? title;
  final VideoPreviewForward? onForward;
  final String userId;
  final String userFullName;
  final String userIdentityNumber;
  final String avatarUrl;
  final bool isTranscriptPage;

  static Future<void> show(BuildContext context, VideoPreviewPage page) =>
      showDialog<void>(
        context: context,
        barrierDismissible: false,
        builder: (_) => page,
      );

  @override
  State<VideoPreviewPage> createState() => _VideoPreviewPageState();
}

class _VideoPreviewPageState extends State<VideoPreviewPage> {
  late final VideoPlayerController _controller;
  late final Future<void> _initialize;
  double _lastAudibleVolume = 1;
  Timer? _hideControlsTimer;
  bool _showControls = true;

  @override
  void initState() {
    super.initState();
    _controller = _videoController(widget.source);
    _initialize = _initializePlayer();
    _controller.addListener(_onPlayerChanged);
    _restartHideControlsTimer();
  }

  Future<void> _initializePlayer() async {
    await _controller.initialize();
    if (mounted) await _controller.play();
  }

  @override
  void dispose() {
    _hideControlsTimer?.cancel();
    _controller
      ..removeListener(_onPlayerChanged)
      ..dispose();
    super.dispose();
  }

  void _onPlayerChanged() {
    if (mounted) setState(() {});
  }

  void _togglePlayback() {
    if (!_controller.value.isInitialized) return;
    if (_controller.value.isPlaying) {
      unawaited(_controller.pause());
    } else {
      unawaited(_controller.play());
    }
    _showControlsTemporarily();
  }

  void _seekBy(Duration offset) {
    final value = _controller.value;
    if (!value.isInitialized) return;
    final target = (value.position.inMilliseconds + offset.inMilliseconds)
        .clamp(0, value.duration.inMilliseconds);
    unawaited(_controller.seekTo(Duration(milliseconds: target)));
  }

  void _toggleMute() {
    final value = _controller.value;
    if (!value.isInitialized) return;
    if (value.volume > 0) {
      _lastAudibleVolume = value.volume;
      unawaited(_controller.setVolume(0));
    } else {
      unawaited(_controller.setVolume(_lastAudibleVolume));
    }
  }

  void _showControlsTemporarily() {
    if (mounted) setState(() => _showControls = true);
    _restartHideControlsTimer();
  }

  void _restartHideControlsTimer() {
    _hideControlsTimer?.cancel();
    _hideControlsTimer = Timer(const Duration(seconds: 3), () {
      if (mounted) setState(() => _showControls = false);
    });
  }

  Future<void> _forward() async {
    final callback = widget.onForward;
    if (callback == null) return;
    await callback();
  }

  @override
  Widget build(BuildContext context) {
    final localFile = existingLocalFile(widget.source);
    final darwin = Platform.isMacOS || Platform.isIOS;
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.escape): () =>
            Navigator.maybePop(context),
        const SingleActivator(LogicalKeyboardKey.space): _togglePlayback,
        const SingleActivator(LogicalKeyboardKey.arrowLeft): () =>
            _seekBy(const Duration(seconds: -15)),
        const SingleActivator(LogicalKeyboardKey.arrowRight): () =>
            _seekBy(const Duration(seconds: 15)),
        SingleActivator(
          LogicalKeyboardKey.keyM,
          meta: darwin,
          control: !darwin,
        ): _toggleMute,
        SingleActivator(
          LogicalKeyboardKey.keyC,
          meta: darwin,
          control: !darwin,
        ): () {
          if (localFile != null) unawaited(copyLocalFileToClipboard(localFile));
        },
        const SingleActivator(LogicalKeyboardKey.arrowUp): () => unawaited(
          _controller.setVolume((_controller.value.volume + 0.1).clamp(0, 1)),
        ),
        const SingleActivator(LogicalKeyboardKey.arrowDown): () => unawaited(
          _controller.setVolume((_controller.value.volume - 0.1).clamp(0, 1)),
        ),
      },
      child: Focus(
        autofocus: true,
        child: Column(
          children: [
            Container(
              color: context.theme.primary,
              height: 70,
              child: Row(
                children: [
                  const SizedBox(width: 100),
                  AvatarView(
                    userId: widget.userId,
                    name: widget.userFullName,
                    avatarUrl: widget.avatarUrl,
                    size: 36,
                  ),
                  const SizedBox(width: 10),
                  Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      ConstrainedBox(
                        constraints: const BoxConstraints(maxWidth: 200),
                        child: Text(
                          widget.userFullName,
                          style: TextStyle(
                            fontSize: 16,
                            color: context.theme.text,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ),
                      Text(
                        widget.userIdentityNumber,
                        style: TextStyle(
                          fontSize: 14,
                          color: context.theme.secondaryText,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(width: 14),
                  const Spacer(),
                  if (!widget.isTranscriptPage && widget.onForward != null)
                    ActionButton(
                      name: MixinAssets.previewShare,
                      size: 20,
                      color: context.theme.icon,
                      onTap: _forward,
                    ),
                  const SizedBox(width: 14),
                  ActionButton(
                    name: MixinAssets.previewCopy,
                    size: 20,
                    color: context.theme.icon,
                    onTap: localFile == null
                        ? null
                        : () => unawaited(copyLocalFileToClipboard(localFile)),
                  ),
                  const SizedBox(width: 14),
                  ActionButton(
                    name: MixinAssets.previewDownload,
                    size: 20,
                    color: context.theme.icon,
                    onTap: () => unawaited(
                      saveMessageFileAs(
                        widget.source,
                        suggestedName: widget.title,
                      ),
                    ),
                  ),
                  const SizedBox(width: 14),
                  ActionButton(
                    name: MixinAssets.previewClose,
                    size: 20,
                    color: context.theme.icon,
                    onTap: () => Navigator.pop(context),
                  ),
                  const SizedBox(width: 24),
                ],
              ),
            ),
            Expanded(
              child: Stack(
                children: [
                  ColoredBox(
                    color: context.theme.background,
                    child: const SizedBox.expand(),
                  ),
                  FutureBuilder<void>(
                    future: _initialize,
                    builder: (context, snapshot) {
                      if (snapshot.connectionState != ConnectionState.done ||
                          snapshot.hasError) {
                        return const SizedBox.expand();
                      }
                      final aspect = _controller.value.aspectRatio;
                      return Center(
                        child: AspectRatio(
                          aspectRatio: aspect <= 0 ? 1 : aspect,
                          child: VideoPlayer(_controller),
                        ),
                      );
                    },
                  ),
                  SizedBox.expand(
                    child: MouseRegion(
                      onHover: (_) => _showControlsTemporarily(),
                      child: Stack(
                        children: [
                          if (_showControls)
                            Align(
                              alignment: Alignment.bottomCenter,
                              child: Padding(
                                padding: const EdgeInsets.only(bottom: 20),
                                child: _VideoOperationBar(
                                  controller: _controller,
                                  onToggleMute: _toggleMute,
                                  onBackward: () =>
                                      _seekBy(const Duration(seconds: -15)),
                                  onTogglePlayback: _togglePlayback,
                                  onForward: () =>
                                      _seekBy(const Duration(seconds: 15)),
                                ),
                              ),
                            ),
                        ],
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _VideoOperationBar extends StatelessWidget {
  const _VideoOperationBar({
    required this.controller,
    required this.onToggleMute,
    required this.onBackward,
    required this.onTogglePlayback,
    required this.onForward,
  });

  final VideoPlayerController controller;
  final VoidCallback onToggleMute;
  final VoidCallback onBackward;
  final VoidCallback onTogglePlayback;
  final VoidCallback onForward;

  @override
  Widget build(BuildContext context) {
    const foreground = Color.fromARGB(255, 200, 200, 200);
    return ClipRRect(
      borderRadius: BorderRadius.circular(10),
      child: Material(
        color: const Color.fromRGBO(41, 41, 41, 0.7),
        child: BackdropFilter(
          filter: ui.ImageFilter.blur(sigmaX: 10, sigmaY: 10),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 500, minWidth: 300),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: _VideoVolumeBar(
                          controller: controller,
                          onToggleMute: onToggleMute,
                        ),
                      ),
                      ActionButton(
                        onTap: onBackward,
                        child: const Icon(
                          CupertinoIcons.gobackward_15,
                          color: foreground,
                        ),
                      ),
                      const SizedBox(width: 8),
                      ActionButton(
                        name: controller.value.isPlaying
                            ? MixinAssets.playerPause
                            : MixinAssets.playerPlay,
                        size: 32,
                        color: foreground,
                        onTap: onTogglePlayback,
                      ),
                      const SizedBox(width: 8),
                      ActionButton(
                        onTap: onForward,
                        child: const Icon(
                          CupertinoIcons.goforward_15,
                          color: foreground,
                        ),
                      ),
                      const Spacer(),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        _formatPlayerDuration(controller.value.position),
                        style: const TextStyle(fontSize: 12, color: foreground),
                      ),
                      const SizedBox(width: 12),
                      Expanded(child: CupertinoVideoProgressBar(controller)),
                      const SizedBox(width: 12),
                      Text(
                        _formatPlayerDuration(controller.value.duration),
                        style: const TextStyle(fontSize: 12, color: foreground),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _VideoVolumeBar extends StatelessWidget {
  const _VideoVolumeBar({required this.controller, required this.onToggleMute});

  final VideoPlayerController controller;
  final VoidCallback onToggleMute;

  @override
  Widget build(BuildContext context) {
    const foreground = Color.fromARGB(255, 200, 200, 200);
    final volume = controller.value.volume.clamp(0.0, 1.0);
    return Row(
      children: [
        ActionButton(
          padding: const EdgeInsets.all(4),
          onTap: onToggleMute,
          child: Icon(switch (volume) {
            0 => CupertinoIcons.speaker_slash,
            < 0.25 => CupertinoIcons.speaker,
            < 0.5 => CupertinoIcons.speaker_1,
            < 0.75 => CupertinoIcons.speaker_2,
            _ => CupertinoIcons.speaker_3,
          }, color: foreground),
        ),
        const SizedBox(width: 8),
        SizedBox(
          width: 80,
          child: SliderTheme(
            data: const SliderThemeData(
              trackHeight: 2,
              thumbShape: RoundSliderThumbShape(
                enabledThumbRadius: 6,
                elevation: 0,
              ),
              trackShape: UnboundedRoundedRectSliderTrackShape(
                removeAdditionalActiveTrackHeight: true,
              ),
              overlayShape: RoundSliderOverlayShape(overlayRadius: 10),
              showValueIndicator: ShowValueIndicator.onDrag,
            ),
            child: Slider(
              value: volume,
              allowedInteraction: SliderInteraction.tapAndSlide,
              activeColor: context.theme.accent,
              inactiveColor: foreground.withValues(alpha: 0.6),
              thumbColor: foreground,
              onChanged: (value) => unawaited(controller.setVolume(value)),
            ),
          ),
        ),
      ],
    );
  }
}

VideoPlayerController _videoController(String source) {
  final uri = Uri.tryParse(source);
  if (uri != null && (uri.scheme == 'http' || uri.scheme == 'https')) {
    return VideoPlayerController.networkUrl(uri);
  }
  return VideoPlayerController.file(
    uri?.scheme == 'file' ? File.fromUri(uri!) : File(source),
  );
}

String _formatPlayerDuration(Duration duration) {
  final hours = duration.inHours;
  final minutes = duration.inMinutes.remainder(60);
  final seconds = duration.inSeconds.remainder(60);
  if (hours > 0) {
    return '$hours:${minutes.toString().padLeft(2, '0')}:'
        '${seconds.toString().padLeft(2, '0')}';
  }
  return '$minutes:${seconds.toString().padLeft(2, '0')}';
}

class PostPreviewPage extends StatelessWidget {
  const PostPreviewPage({required this.content, super.key, this.title});

  final String content;
  final String? title;

  static Future<void> show(BuildContext context, PostPreviewPage page) =>
      showGeneralDialog<void>(
        context: context,
        barrierColor: Colors.transparent,
        barrierDismissible: true,
        barrierLabel: MaterialLocalizations.of(
          context,
        ).modalBarrierDismissLabel,
        pageBuilder: (_, _, _) => page,
      );

  @override
  Widget build(BuildContext context) {
    final fontSize = 16 + context.watch<SettingsController>().chatFontSizeDelta;
    return Material(
      color: context.theme.background,
      child: Column(
        children: [
          MixinAppBar(
            leading: const SizedBox(),
            actions: [MixinCloseButton(onTap: () => Navigator.pop(context))],
          ),
          Expanded(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 600),
              child: DefaultTextStyle.merge(
                style: TextStyle(fontSize: fontSize, color: context.theme.text),
                child: MarkdownWidget(
                  data: content,
                  selectable: true,
                  padding: const EdgeInsets.symmetric(
                    horizontal: 32,
                    vertical: 8,
                  ),
                  config: postMarkdownConfig(context, fontSize: fontSize),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

Future<OpenResult> openMessageFile(String source) {
  final file = _localFile(source);
  return OpenFile.open(file.path);
}

Future<void> openOrSaveMessageFile(
  BuildContext context,
  String source, {
  String? mediaName,
}) async {
  final name = mediaName?.trim().isNotEmpty == true
      ? mediaName!.trim()
      : path.basename(_localFile(source).path);
  if (!_shouldOpenDirectly(name)) {
    await saveMessageFileAs(source, suggestedName: name);
    return;
  }
  try {
    final result = await openMessageFile(source);
    if (result.type == ResultType.done) return;
  } on Object {
    // The source UI reports the localized open failure below.
  }
  if (context.mounted) {
    showToastFailed(ToastError(context.l10n.unableToOpenFile(name)));
  }
}

Future<String?> saveMessageFileAs(
  String source, {
  String? suggestedName,
}) async {
  try {
    final file = _localFile(source);
    final location = await getSaveLocation(
      suggestedName: suggestedName ?? path.basename(file.path),
    );
    if (location == null) return null;
    await file.copy(location.path);
    showToastSuccessful();
    return location.path;
  } on Object catch (error) {
    showToastFailed(error);
    return null;
  }
}

bool _shouldOpenDirectly(String mediaName) {
  const allowList = {
    '.jpg',
    '.jpeg',
    '.png',
    '.gif',
    '.bmp',
    '.webp',
    '.avif',
    '.mp4',
    '.mp3',
    '.wav',
    '.m4a',
    '.m4v',
    '.mov',
    '.avi',
    '.mkv',
    '.flv',
    '.wmv',
    '.3gp',
    '.mpg',
    '.mpeg',
    '.ogv',
    '.ogm',
    '.ogg',
    '.webm',
    '.m3u8',
    '.ts',
    '.pdf',
    '.doc',
    '.docx',
    '.xls',
    '.xlsx',
    '.ppt',
    '.pptx',
    '.txt',
    '.rtf',
    '.csv',
    '.log',
  };
  return allowList.contains(path.extension(mediaName).toLowerCase());
}

File _localFile(String source) {
  final uri = Uri.tryParse(source);
  return uri?.scheme == 'file' ? File.fromUri(uri!) : File(source);
}
