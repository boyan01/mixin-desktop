import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:markdown_widget/markdown_widget.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:open_file/open_file.dart';
import 'package:path/path.dart' as path;
import 'package:mixin_desktop_ui/utils/system_clipboard.dart';
import 'package:mixin_desktop_ui/widgets/post_markdown.dart';
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
    this.canForward = false,
  });

  final String id;
  final String source;
  final String? name;
  final bool canForward;
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
    try {
      final forwarded = await callback(_image);
      if (!forwarded || !mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('${context.l10n.forward} ✓')));
    } on Object catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(error.toString())));
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_images.isEmpty) {
      return const Scaffold(body: Center(child: Icon(Icons.broken_image)));
    }
    final canGoPrevious = _index > 0;
    final canGoNext = _index + 1 < _images.length;
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.escape): () =>
            Navigator.maybePop(context),
        const SingleActivator(LogicalKeyboardKey.arrowLeft): () =>
            _select(_index - 1),
        const SingleActivator(LogicalKeyboardKey.arrowRight): () =>
            _select(_index + 1),
        const SingleActivator(LogicalKeyboardKey.add): () => _zoom(1.25),
        const SingleActivator(LogicalKeyboardKey.equal, shift: true): () =>
            _zoom(1.25),
        const SingleActivator(LogicalKeyboardKey.numpadAdd): () => _zoom(1.25),
        const SingleActivator(LogicalKeyboardKey.minus): () => _zoom(0.8),
        const SingleActivator(LogicalKeyboardKey.numpadSubtract): () =>
            _zoom(0.8),
        const SingleActivator(LogicalKeyboardKey.keyR): () =>
            setState(() => _quarterTurns = (_quarterTurns + 1) % 4),
      },
      child: Focus(
        autofocus: true,
        child: Scaffold(
          backgroundColor: Colors.black,
          body: SafeArea(
            child: Stack(
              children: [
                Positioned.fill(
                  child: InteractiveViewer(
                    transformationController: _transformationController,
                    minScale: 0.5,
                    maxScale: 5,
                    child: Center(
                      child: RotatedBox(
                        quarterTurns: _quarterTurns,
                        child: _PreviewImage(source: _image.source),
                      ),
                    ),
                  ),
                ),
                Positioned(
                  top: 12,
                  left: 12,
                  right: 12,
                  child: _PreviewToolbar(
                    image: _image,
                    onClose: () => Navigator.maybePop(context),
                    onZoomOut: () => _zoom(0.8),
                    onZoomIn: () => _zoom(1.25),
                    onRotate: () =>
                        setState(() => _quarterTurns = (_quarterTurns + 1) % 4),
                    onCopy: widget.onCopy,
                    onSave: widget.onSave,
                    onForward: _image.canForward ? _forward : null,
                  ),
                ),
                if (canGoPrevious)
                  Align(
                    alignment: Alignment.centerLeft,
                    child: _NavigationButton(
                      icon: Icons.chevron_left,
                      onPressed: () => _select(_index - 1),
                    ),
                  ),
                if (canGoNext)
                  Align(
                    alignment: Alignment.centerRight,
                    child: _NavigationButton(
                      icon: Icons.chevron_right,
                      onPressed: () => _select(_index + 1),
                    ),
                  ),
                if (_images.length > 1)
                  Positioned(
                    bottom: 18,
                    left: 0,
                    right: 0,
                    child: Text(
                      '${_index + 1} / ${_images.length}',
                      textAlign: TextAlign.center,
                      style: const TextStyle(color: Colors.white, fontSize: 12),
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

class _PreviewToolbar extends StatelessWidget {
  const _PreviewToolbar({
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
  Widget build(BuildContext context) => DecoratedBox(
    decoration: BoxDecoration(
      color: Colors.black.withValues(alpha: 0.55),
      borderRadius: const BorderRadius.all(Radius.circular(8)),
    ),
    child: Row(
      children: [
        IconButton(
          tooltip: MaterialLocalizations.of(context).closeButtonTooltip,
          onPressed: onClose,
          icon: const Icon(Icons.close, color: Colors.white),
        ),
        Expanded(
          child: Text(
            image.name ?? '',
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: const TextStyle(color: Colors.white),
          ),
        ),
        IconButton(
          onPressed: onZoomOut,
          icon: const Icon(Icons.zoom_out, color: Colors.white),
        ),
        IconButton(
          onPressed: onZoomIn,
          icon: const Icon(Icons.zoom_in, color: Colors.white),
        ),
        IconButton(
          onPressed: onRotate,
          icon: const Icon(Icons.rotate_right, color: Colors.white),
        ),
        if (onCopy != null)
          IconButton(
            onPressed: () => onCopy!(image),
            icon: const Icon(Icons.copy, color: Colors.white),
          ),
        if (onSave != null)
          IconButton(
            onPressed: () => onSave!(image),
            icon: const Icon(Icons.download, color: Colors.white),
          ),
        if (onForward != null)
          IconButton(
            tooltip: context.l10n.forward,
            onPressed: onForward,
            icon: const Icon(Icons.forward, color: Colors.white),
          ),
      ],
    ),
  );
}

class _NavigationButton extends StatelessWidget {
  const _NavigationButton({required this.icon, required this.onPressed});

  final IconData icon;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.all(12),
    child: IconButton.filledTonal(
      onPressed: onPressed,
      icon: Icon(icon, color: Colors.white, size: 32),
    ),
  );
}

class _PreviewImage extends StatelessWidget {
  const _PreviewImage({required this.source});

  final String source;

  @override
  Widget build(BuildContext context) {
    final provider = imageProviderForSource(source);
    if (provider == null) {
      return const Icon(Icons.broken_image_outlined, color: Colors.white);
    }
    return Image(
      image: provider,
      fit: BoxFit.contain,
      filterQuality: FilterQuality.high,
      errorBuilder: (_, _, _) =>
          const Icon(Icons.broken_image_outlined, color: Colors.white),
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
  });

  final String source;
  final String? title;
  final VideoPreviewForward? onForward;

  @override
  State<VideoPreviewPage> createState() => _VideoPreviewPageState();
}

class _VideoPreviewPageState extends State<VideoPreviewPage> {
  late final VideoPlayerController _controller;
  late final Future<void> _initialize;
  double _lastAudibleVolume = 1;

  @override
  void initState() {
    super.initState();
    _controller = _videoController(widget.source);
    _initialize = _initializePlayer();
    _controller.addListener(_onPlayerChanged);
  }

  Future<void> _initializePlayer() async {
    await _controller.initialize();
    if (mounted) await _controller.play();
  }

  @override
  void dispose() {
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

  Future<void> _forward() async {
    final callback = widget.onForward;
    if (callback == null) return;
    try {
      final forwarded = await callback();
      if (!forwarded || !mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('${context.l10n.forward} ✓')));
    } on Object catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(error.toString())));
    }
  }

  @override
  Widget build(BuildContext context) {
    final localFile = existingLocalFile(widget.source);
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.escape): () =>
            Navigator.maybePop(context),
        const SingleActivator(LogicalKeyboardKey.space): _togglePlayback,
        const SingleActivator(LogicalKeyboardKey.arrowLeft): () =>
            _seekBy(const Duration(seconds: -15)),
        const SingleActivator(LogicalKeyboardKey.arrowRight): () =>
            _seekBy(const Duration(seconds: 15)),
        const SingleActivator(LogicalKeyboardKey.keyM): _toggleMute,
      },
      child: Focus(
        autofocus: true,
        child: Scaffold(
          backgroundColor: Colors.black,
          appBar: AppBar(
            backgroundColor: Colors.black,
            foregroundColor: Colors.white,
            title: Text(widget.title ?? ''),
            actions: [
              if (widget.onForward != null)
                IconButton(
                  tooltip: context.l10n.forward,
                  onPressed: _forward,
                  icon: const Icon(Icons.forward),
                ),
              if (localFile != null)
                IconButton(
                  tooltip: context.l10n.copy,
                  onPressed: () =>
                      unawaited(copyLocalFileToClipboard(localFile)),
                  icon: const Icon(Icons.copy),
                ),
              if (localFile != null)
                IconButton(
                  tooltip: context.l10n.saveAs,
                  onPressed: () => unawaited(
                    saveMessageFileAs(
                      widget.source,
                      suggestedName: widget.title,
                    ),
                  ),
                  icon: const Icon(Icons.download_outlined),
                ),
            ],
          ),
          body: FutureBuilder<void>(
            future: _initialize,
            builder: (context, snapshot) {
              if (snapshot.connectionState != ConnectionState.done) {
                return const Center(child: CircularProgressIndicator());
              }
              if (snapshot.hasError) {
                return const Center(
                  child: Icon(
                    Icons.error_outline,
                    color: Colors.white,
                    size: 36,
                  ),
                );
              }
              final value = _controller.value;
              return Column(
                children: [
                  Expanded(
                    child: Center(
                      child: AspectRatio(
                        aspectRatio: value.aspectRatio <= 0
                            ? 1
                            : value.aspectRatio,
                        child: VideoPlayer(_controller),
                      ),
                    ),
                  ),
                  SafeArea(
                    top: false,
                    child: Row(
                      children: [
                        IconButton(
                          tooltip: '-15s',
                          onPressed: () =>
                              _seekBy(const Duration(seconds: -15)),
                          icon: const Icon(
                            Icons.replay_10,
                            color: Colors.white,
                          ),
                        ),
                        IconButton(
                          onPressed: _togglePlayback,
                          icon: Icon(
                            value.isPlaying ? Icons.pause : Icons.play_arrow,
                            color: Colors.white,
                          ),
                        ),
                        IconButton(
                          tooltip: '+15s',
                          onPressed: () => _seekBy(const Duration(seconds: 15)),
                          icon: const Icon(
                            Icons.forward_10,
                            color: Colors.white,
                          ),
                        ),
                        Expanded(
                          child: VideoProgressIndicator(
                            _controller,
                            allowScrubbing: true,
                            padding: const EdgeInsets.symmetric(vertical: 18),
                          ),
                        ),
                        Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 8),
                          child: Text(
                            '${_formatPlayerDuration(value.position)} / '
                            '${_formatPlayerDuration(value.duration)}',
                            style: const TextStyle(
                              color: Colors.white,
                              fontSize: 12,
                            ),
                          ),
                        ),
                        IconButton(
                          onPressed: _toggleMute,
                          icon: Icon(
                            value.volume == 0
                                ? Icons.volume_off
                                : Icons.volume_up,
                            color: Colors.white,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              );
            },
          ),
        ),
      ),
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

  @override
  Widget build(BuildContext context) {
    final fontSize = 16 + context.watch<SettingsController>().chatFontSizeDelta;
    return Scaffold(
      appBar: AppBar(title: Text(title ?? '')),
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 760),
          child: DefaultTextStyle.merge(
            style: TextStyle(fontSize: fontSize, color: context.theme.text),
            child: MarkdownWidget(
              data: content,
              selectable: true,
              padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 24),
              config: postMarkdownConfig(context, fontSize: fontSize),
            ),
          ),
        ),
      ),
    );
  }
}

Future<OpenResult> openMessageFile(String source) {
  final file = _localFile(source);
  return OpenFile.open(file.path);
}

Future<String?> saveMessageFileAs(
  String source, {
  String? suggestedName,
}) async {
  final file = _localFile(source);
  final location = await getSaveLocation(
    suggestedName: suggestedName ?? path.basename(file.path),
  );
  if (location == null) return null;
  await file.copy(location.path);
  return location.path;
}

File _localFile(String source) {
  final uri = Uri.tryParse(source);
  return uri?.scheme == 'file' ? File.fromUri(uri!) : File(source);
}
