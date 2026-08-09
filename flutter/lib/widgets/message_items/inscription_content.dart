import 'dart:convert';
import 'dart:typed_data';

import 'package:auto_size_text/auto_size_text.dart';
import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:hexagon/hexagon.dart';
import 'package:http/http.dart' as http;
import 'package:pointycastle/digests/sha3.dart';

import '../../constants/assets.dart';

enum InscriptionContentMode {
  small(2, 14),
  large(5, 24);

  const InscriptionContentMode(this.maxLines, this.fontSize);

  final int maxLines;
  final double fontSize;
}

class InscriptionContent extends StatelessWidget {
  const InscriptionContent({
    required this.mode,
    this.contentType,
    this.contentUrl,
    this.iconUrl,
    super.key,
  });

  final String? contentType;
  final String? contentUrl;
  final String? iconUrl;
  final InscriptionContentMode mode;

  @override
  Widget build(BuildContext context) {
    final url = contentUrl;
    final placeholder = SvgPicture.asset(MixinAssets.inscriptionPlaceholder);
    return AspectRatio(
      aspectRatio: 1,
      child: switch (contentType) {
        final type when type?.startsWith('image') == true && url != null =>
          Image.network(
            url,
            fit: BoxFit.cover,
            errorBuilder: (_, _, _) => placeholder,
          ),
        final type
            when type?.startsWith('text') == true &&
                url != null &&
                iconUrl != null =>
          _TextInscriptionContent(
            contentUrl: url,
            iconUrl: iconUrl!,
            mode: mode,
          ),
        _ => placeholder,
      },
    );
  }
}

class _TextInscriptionContent extends StatefulWidget {
  const _TextInscriptionContent({
    required this.contentUrl,
    required this.iconUrl,
    required this.mode,
  });

  final String contentUrl;
  final String iconUrl;
  final InscriptionContentMode mode;

  @override
  State<_TextInscriptionContent> createState() =>
      _TextInscriptionContentState();
}

class _TextInscriptionContentState extends State<_TextInscriptionContent> {
  late Future<String> _content = _loadContent();

  Future<String> _loadContent() async {
    final response = await http.get(Uri.parse(widget.contentUrl));
    return _inscriptionDisplayContent(
      utf8.decode(response.bodyBytes, allowMalformed: true),
    );
  }

  @override
  void didUpdateWidget(covariant _TextInscriptionContent oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.contentUrl != widget.contentUrl) {
      _content = _loadContent();
    }
  }

  @override
  Widget build(BuildContext context) => Stack(
    fit: StackFit.expand,
    children: [
      Image.asset(MixinAssets.inscriptionTextBackground),
      LayoutBuilder(
        builder: (context, constraints) {
          final textStyle = TextStyle(
            color: const Color.fromRGBO(255, 167, 36, 1),
            fontSize: widget.mode.fontSize,
            fontWeight: FontWeight.bold,
          );
          return Column(
            mainAxisSize: MainAxisSize.min,
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              HexagonWidget(
                type: HexagonType.FLAT,
                cornerRadius: constraints.maxWidth / 3 / 5,
                height: constraints.maxWidth / 3,
                width: constraints.maxWidth / 3,
                child: SizedBox.square(
                  dimension: constraints.maxWidth / 3,
                  child: Image.network(
                    widget.iconUrl,
                    fit: BoxFit.cover,
                    errorBuilder: (_, _, _) =>
                        SvgPicture.asset(MixinAssets.collectionPlaceholder),
                  ),
                ),
              ),
              Padding(
                padding: EdgeInsets.only(
                  top: constraints.maxWidth / 40,
                  right: constraints.maxWidth / 10,
                  left: constraints.maxWidth / 10,
                ),
                child: FutureBuilder<String>(
                  future: _content,
                  builder: (context, snapshot) => _MinLinesWrapper(
                    text: snapshot.data,
                    style: textStyle,
                    minLines: widget.mode.maxLines,
                    child: AutoSizeText(
                      snapshot.data ?? '',
                      maxLines: widget.mode.maxLines,
                      maxFontSize: 24,
                      minFontSize: widget.mode.fontSize,
                      overflow: TextOverflow.ellipsis,
                      style: textStyle,
                      textAlign: TextAlign.center,
                    ),
                  ),
                ),
              ),
            ],
          );
        },
      ),
    ],
  );
}

class _MinLinesWrapper extends StatelessWidget {
  const _MinLinesWrapper({
    required this.text,
    required this.style,
    required this.minLines,
    required this.child,
  });

  final String? text;
  final TextStyle style;
  final int minLines;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final textPainter = TextPainter(
      text: TextSpan(text: text, style: style),
      textDirection: TextDirection.ltr,
      maxLines: minLines,
    )..layout(maxWidth: MediaQuery.sizeOf(context).width);
    return ConstrainedBox(
      constraints: BoxConstraints(
        minHeight: textPainter.preferredLineHeight * minLines,
      ),
      child: child,
    );
  }
}

String _inscriptionDisplayContent(String content) => content.replaceAll(
  RegExp(r'[\s\p{Other}\p{Cf}\p{Cc}\p{Cn}]', unicode: true),
  '■',
);

class ColoredHashWidget extends StatelessWidget {
  const ColoredHashWidget({
    required this.inscriptionHex,
    this.blockSize = const Size(5, 16),
    this.space = 3,
    super.key,
  });

  final String? inscriptionHex;
  final Size blockSize;
  final double space;

  @override
  Widget build(BuildContext context) {
    final colors = _inscriptionColors(inscriptionHex);
    return Row(
      children: [
        for (var index = 0; index < colors.length; index++) ...[
          if (index > 0) SizedBox(width: space),
          Container(
            width: blockSize.width,
            height: blockSize.height,
            decoration: BoxDecoration(
              color: colors[index],
              borderRadius: BorderRadius.circular(1),
            ),
          ),
        ],
      ],
    );
  }
}

List<Color> _inscriptionColors(String? value) {
  final bytes = _hexBytes(value);
  if (bytes == null) return List<Color>.filled(12, Colors.black12);
  final digest = SHA3Digest(256).process(Uint8List.fromList(bytes));
  final data = [...bytes, ...digest.take(4)];
  return [
    for (var index = 0; index + 2 < data.length; index += 3)
      Color.fromARGB(0xFF, data[index], data[index + 1], data[index + 2]),
  ];
}

List<int>? _hexBytes(String? value) {
  if (value == null ||
      value.length.isOdd ||
      !RegExp(r'^[0-9a-fA-F]+$').hasMatch(value)) {
    return null;
  }
  return [
    for (var index = 0; index < value.length; index += 2)
      int.parse(value.substring(index, index + 2), radix: 16),
  ];
}
