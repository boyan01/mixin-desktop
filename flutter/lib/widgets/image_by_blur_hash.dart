import 'dart:convert';
import 'dart:ui' as ui;

import 'package:blurhash_dart/blurhash_dart.dart';
import 'package:flutter/material.dart';

class ImageByBlurHashOrBase64 extends StatelessWidget {
  const ImageByBlurHashOrBase64({
    required this.imageData,
    super.key,
    this.fit = BoxFit.cover,
  });

  final String imageData;
  final BoxFit fit;

  @override
  Widget build(BuildContext context) {
    try {
      return _ImageByBlurHash(blurHash: BlurHash.decode(imageData));
    } on Object {
      try {
        return Image.memory(base64Decode(imageData), fit: fit);
      } on Object {
        return const SizedBox();
      }
    }
  }
}

class _ImageByBlurHash extends StatefulWidget {
  const _ImageByBlurHash({required this.blurHash});

  final BlurHash blurHash;

  @override
  State<_ImageByBlurHash> createState() => _ImageByBlurHashState();
}

class _ImageByBlurHashState extends State<_ImageByBlurHash> {
  ui.Image? image;

  @override
  void initState() {
    super.initState();
    _decode();
  }

  @override
  void didUpdateWidget(_ImageByBlurHash oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.blurHash != widget.blurHash) _decode();
  }

  void _decode() {
    const width = 20;
    const height = 20;
    ui.decodeImageFromPixels(
      widget.blurHash.toImage(width, height).getBytes(),
      width,
      height,
      ui.PixelFormat.rgba8888,
      (result) {
        if (mounted) setState(() => image = result);
      },
    );
  }

  @override
  Widget build(BuildContext context) =>
      RawImage(fit: BoxFit.cover, image: image);
}
