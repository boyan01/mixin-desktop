import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:http/http.dart' as http;

import '../utils/app_logger.dart';
import '../utils/image.dart';

typedef PlaceholderWidgetBuilder = Widget Function();

final _checkedImageFiles = <String>{};

bool shouldUseMediaImagePipeline(
  String url, {
  http.Client? client,
  bool normalizeGif = false,
}) => normalizeGif || client != null || isLikelyGifUrl(url);

@visibleForTesting
bool isLikelyGifUrl(String url) {
  final path = Uri.tryParse(url)?.path ?? url.split('?').first;
  return path.toLowerCase().endsWith('.gif');
}

ImageProvider resolveMediaImageProvider({
  required ImageProvider image,
  http.Client? client,
  int networkRevision = 0,
  bool normalizeGif = false,
}) {
  if (image is NetworkImage &&
      shouldUseMediaImagePipeline(
        image.url,
        client: client,
        normalizeGif: normalizeGif,
      )) {
    return ProxyNetworkImage(
      image.url,
      scale: image.scale,
      client: client,
      networkRevision: networkRevision,
    );
  }
  return image;
}

class MediaImagePipeline extends StatelessWidget {
  const MediaImagePipeline({
    required this.image,
    super.key,
    this.client,
    this.networkRevision = 0,
    this.placeholder,
    this.errorBuilder,
    this.width,
    this.height,
    this.fit = BoxFit.cover,
    this.isAntiAlias = false,
    this.normalizeGif = false,
  });

  final ImageProvider image;
  final http.Client? client;
  final int networkRevision;
  final PlaceholderWidgetBuilder? placeholder;
  final ImageErrorWidgetBuilder? errorBuilder;
  final double? width;
  final double? height;
  final BoxFit? fit;
  final bool isAntiAlias;
  final bool normalizeGif;

  @override
  Widget build(BuildContext context) {
    final resolvedImage = resolveMediaImageProvider(
      image: image,
      client: client,
      networkRevision: networkRevision,
      normalizeGif: normalizeGif,
    );

    Widget fallback() =>
        placeholder?.call() ?? SizedBox(width: width, height: height);

    Widget imageView() => Image(
      image: resolvedImage,
      width: width,
      height: height,
      fit: fit,
      isAntiAlias: isAntiAlias,
      frameBuilder: (context, child, frame, wasSynchronouslyLoaded) {
        if (wasSynchronouslyLoaded || frame != null) return child;
        return fallback();
      },
      errorBuilder: (context, error, stackTrace) =>
          errorBuilder?.call(context, error, stackTrace) ?? fallback(),
    );

    return NormalizedGifImageGate(
      image: resolvedImage,
      placeholder: fallback,
      childBuilder: imageView,
    );
  }
}

@immutable
class ProxyNetworkImage extends ImageProvider<ProxyNetworkImage> {
  const ProxyNetworkImage(
    this.url, {
    this.scale = 1,
    this.client,
    this.networkRevision = 0,
  });

  final String url;
  final double scale;
  final http.Client? client;
  final int networkRevision;

  @override
  Future<ProxyNetworkImage> obtainKey(ImageConfiguration configuration) =>
      SynchronousFuture<ProxyNetworkImage>(this);

  @override
  ImageStreamCompleter loadImage(
    ProxyNetworkImage key,
    ImageDecoderCallback decode,
  ) => MultiFrameImageStreamCompleter(
    codec: _loadAsync(key, (buffer) => decode(buffer)),
    scale: key.scale,
    debugLabel: key.url,
    informationCollector: () => <DiagnosticsNode>[
      DiagnosticsProperty<ImageProvider>('Image provider', this),
      DiagnosticsProperty<ProxyNetworkImage>('Image key', key),
    ],
  );

  Future<ui.Codec> _loadAsync(
    ProxyNetworkImage key,
    Future<ui.Codec> Function(ui.ImmutableBuffer buffer) decode,
  ) async {
    try {
      final bytes = normalizeGifBytesIfNeeded(
        await downloadImageBytes(key.url, client: key.client),
      );
      return decode(await ui.ImmutableBuffer.fromUint8List(bytes));
    } catch (error, stackTrace) {
      e('Decode media image failed: ${key.url}', error, stackTrace);
      scheduleMicrotask(() => PaintingBinding.instance.imageCache.evict(key));
      rethrow;
    }
  }

  @override
  bool operator ==(Object other) =>
      other is ProxyNetworkImage &&
      other.url == url &&
      other.scale == scale &&
      identical(other.client, client) &&
      other.networkRevision == networkRevision;

  @override
  int get hashCode => Object.hash(url, scale, client, networkRevision);
}

typedef NormalizedNetworkImage = ProxyNetworkImage;

class NormalizedGifImageGate extends StatefulWidget {
  const NormalizedGifImageGate({
    required this.image,
    required this.placeholder,
    required this.childBuilder,
    super.key,
  });

  final ImageProvider image;
  final Widget Function() placeholder;
  final Widget Function() childBuilder;

  @override
  State<NormalizedGifImageGate> createState() => _NormalizedGifImageGateState();
}

class _NormalizedGifImageGateState extends State<NormalizedGifImageGate> {
  Future<void>? _future;

  @override
  void initState() {
    super.initState();
    _start();
  }

  @override
  void didUpdateWidget(covariant NormalizedGifImageGate oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.image != widget.image) _start();
  }

  void _start() {
    final image = widget.image;
    if (image is! FileImage) {
      _future = null;
      return;
    }
    final filePathSegments = image.file.uri.pathSegments;
    final fileName = filePathSegments.isEmpty
        ? ''
        : filePathSegments.last.toLowerCase();
    if (fileName.isEmpty ||
        (!fileName.endsWith('.gif') && fileName.contains('.'))) {
      _future = null;
      return;
    }

    final path = image.file.absolute.path;
    if (_checkedImageFiles.contains(path)) {
      _future = null;
      return;
    }
    _future = _normalize(image.file);
  }

  Future<void> _normalize(File file) async {
    try {
      await normalizeGifFileIfNeeded(file, null);
      await FileImage(file).evict();
    } finally {
      _checkedImageFiles.add(file.absolute.path);
    }
  }

  @override
  Widget build(BuildContext context) {
    final future = _future;
    if (future == null) return widget.childBuilder();
    return FutureBuilder<void>(
      future: future,
      builder: (context, snapshot) =>
          snapshot.connectionState == ConnectionState.done
          ? widget.childBuilder()
          : widget.placeholder(),
    );
  }
}
