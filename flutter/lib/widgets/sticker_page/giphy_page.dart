import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import '../../l10n/l10n.dart';
import '../../network/core_http_scope.dart';
import '../../theme.dart';
import '../interactive_decorated_box.dart';
import '../mixin_image.dart';
import '../search_text_field.dart';

const giphyApiKey = String.fromEnvironment('MIXIN_GIPHY_KEY');
const _giphyUrl = 'https://api.giphy.com/v1/';
const _limit = 51;

typedef GiphySelected =
    Future<void> Function({
      required String url,
      required String previewUrl,
      required int? width,
      required int? height,
    });

class GiphyPage extends StatefulWidget {
  const GiphyPage({required this.onSelected, super.key});

  final GiphySelected onSelected;

  @override
  State<GiphyPage> createState() => _GiphyPageState();
}

class _GiphyPageState extends State<GiphyPage> {
  final controller = TextEditingController();
  Timer? timer;
  String query = '';

  @override
  void initState() {
    super.initState();
    controller.addListener(_onTextChanged);
  }

  void _onTextChanged() {
    timer?.cancel();
    timer = Timer(const Duration(seconds: 1), () {
      final value = controller.text.trim();
      if (mounted && value != query) setState(() => query = value);
    });
  }

  @override
  void dispose() {
    timer?.cancel();
    controller
      ..removeListener(_onTextChanged)
      ..dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Column(
    children: [
      _SearchBar(controller: controller),
      Divider(color: context.theme.divider, height: 1),
      const SizedBox(height: 12),
      Expanded(
        child: _GifGridView(
          key: ValueKey(query),
          query: query,
          onSelected: widget.onSelected,
        ),
      ),
    ],
  );
}

class _SearchBar extends StatelessWidget {
  const _SearchBar({required this.controller});

  final TextEditingController controller;

  @override
  Widget build(BuildContext context) => SizedBox(
    height: 48,
    child: Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
      child: SearchTextField(
        controller: controller,
        hintText: context.l10n.search,
      ),
    ),
  );
}

class _GifGridView extends StatefulWidget {
  const _GifGridView({
    required this.query,
    required this.onSelected,
    super.key,
  });

  final String query;
  final GiphySelected onSelected;

  @override
  State<_GifGridView> createState() => _GifGridViewState();
}

class _GifGridViewState extends State<_GifGridView>
    with AutomaticKeepAliveClientMixin {
  final controller = ScrollController();
  List<_GiphyGif> gifs = const [];
  bool hasMore = true;
  bool loading = false;

  @override
  bool get wantKeepAlive => true;

  @override
  void initState() {
    super.initState();
    controller.addListener(_onScroll);
    WidgetsBinding.instance.addPostFrameCallback((_) => unawaited(_load()));
  }

  void _onScroll() {
    if (controller.position.pixels == controller.position.maxScrollExtent) {
      unawaited(_load());
    }
  }

  Future<void> _load() async {
    if (loading || !hasMore || !mounted) return;
    setState(() => loading = true);
    try {
      final client = CoreHttpScope.maybeOf(context)?.client;
      if (client == null) throw StateError('network client unavailable');
      final endpoint = widget.query.isEmpty ? 'gifs/trending' : 'gifs/search';
      final uri = Uri.parse('$_giphyUrl$endpoint').replace(
        queryParameters: {
          if (widget.query.isNotEmpty) 'q': widget.query,
          'limit': '$_limit',
          'offset': '${gifs.length}',
          'api_key': giphyApiKey,
        },
      );
      final response = await client.get(uri);
      if (response.statusCode < 200 || response.statusCode >= 300) {
        throw StateError('GIPHY HTTP ${response.statusCode}');
      }
      final data =
          (jsonDecode(response.body) as Map<String, dynamic>)['data']
              as List<dynamic>;
      final page = data
          .map((value) => _GiphyGif.fromJson(value as Map<String, dynamic>))
          .toList(growable: false);
      if (!mounted) return;
      setState(() {
        hasMore = page.length >= _limit;
        gifs = [...gifs, ...page];
      });
    } on Object {
      // The source keeps the loading placeholder when GIPHY is unavailable.
    } finally {
      if (mounted) setState(() => loading = false);
    }
  }

  @override
  void dispose() {
    controller
      ..removeListener(_onScroll)
      ..dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    if (gifs.isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }
    return GridView.builder(
      padding: const EdgeInsets.symmetric(horizontal: 20),
      gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: 4,
        crossAxisSpacing: 8,
        mainAxisSpacing: 8,
      ),
      controller: controller,
      itemCount: gifs.length,
      itemBuilder: (context, index) =>
          _GifItem(gif: gifs[index], onSelected: widget.onSelected),
    );
  }
}

class _GifItem extends StatelessWidget {
  const _GifItem({required this.gif, required this.onSelected});

  final _GiphyGif gif;
  final GiphySelected onSelected;

  @override
  Widget build(BuildContext context) => InteractiveDecoratedBox(
    onTap: () => onSelected(
      url: gif.send.url,
      previewUrl: gif.preview.url,
      width: gif.send.width,
      height: gif.send.height,
    ),
    child: MixinImage.network(
      gif.preview.url,
      placeholder: () => ColoredBox(color: context.theme.secondaryText),
    ),
  );
}

class _GiphyGif {
  const _GiphyGif({required this.preview, required this.send});

  factory _GiphyGif.fromJson(Map<String, dynamic> json) {
    final images = json['images'] as Map<String, dynamic>;
    return _GiphyGif(
      preview: _GiphyImage.fromJson(
        images['fixed_width_downsampled'] as Map<String, dynamic>,
      ),
      send: _GiphyImage.fromJson(images['fixed_width'] as Map<String, dynamic>),
    );
  }

  final _GiphyImage preview;
  final _GiphyImage send;
}

class _GiphyImage {
  const _GiphyImage({required this.url, this.width, this.height});

  factory _GiphyImage.fromJson(Map<String, dynamic> json) => _GiphyImage(
    url: json['url']?.toString() ?? '',
    width: int.tryParse(json['width']?.toString() ?? ''),
    height: int.tryParse(json['height']?.toString() ?? ''),
  );

  final String url;
  final int? width;
  final int? height;
}
