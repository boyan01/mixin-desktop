import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_svg/svg.dart';
import 'package:intl/intl.dart';
import 'package:mixin_desktop_ui/models/message_list_entry.dart';
import 'package:mixin_desktop_ui/pages/chat_side/chat_side_scope.dart';
import 'package:mixin_desktop_ui/theme.dart';

typedef SharedMediaItemBuilder =
    Widget Function(
      BuildContext context,
      MessageListEntry message,
      List<MessageListEntry> messages,
    );

class SharedMediaList extends StatefulWidget {
  const SharedMediaList({
    required this.kind,
    required this.pageSize,
    required this.emptyAsset,
    required this.emptyText,
    required this.itemBuilder,
    this.gridDelegate,
    super.key,
  });

  final String kind;
  final int pageSize;
  final String emptyAsset;
  final String emptyText;
  final SharedMediaItemBuilder itemBuilder;
  final SliverGridDelegate? gridDelegate;

  @override
  State<SharedMediaList> createState() => _SharedMediaListState();
}

class _SharedMediaListState extends State<SharedMediaList> {
  var _loadingMore = false;
  var _hasMore = true;
  Object? _error;
  List<MessageListEntry>? _messages;
  StreamSubscription<BigInt>? _changes;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _changes ??= ChatSideScope.of(context).account.messageChanges().listen(
      (_) => unawaited(_refresh()),
      onError: (Object exception) {
        if (mounted) setState(() => _error = exception);
      },
    );
    if (_messages == null && _error == null) unawaited(_load());
  }

  @override
  void dispose() {
    unawaited(_changes?.cancel());
    super.dispose();
  }

  @override
  void didUpdateWidget(SharedMediaList oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.kind == widget.kind &&
        oldWidget.pageSize == widget.pageSize) {
      return;
    }
    _messages = null;
    _hasMore = true;
    _error = null;
    unawaited(_load());
  }

  Future<void> _refresh() async {
    final scope = ChatSideScope.of(context);
    try {
      final target = _messages?.length ?? widget.pageSize;
      final result = <MessageListEntry>[];
      while (result.length < target) {
        final page = await scope.account.sharedMessages(
          conversationId: scope.conversation.id,
          kind: widget.kind,
          offset: BigInt.from(result.length),
          limit: BigInt.from((target - result.length).clamp(1, 200)),
        );
        result.addAll(page.map(MessageListEntry.fromRust));
        if (page.length < 200) break;
      }
      if (!mounted) return;
      setState(() {
        _messages = result;
        _error = null;
      });
    } on Object catch (exception) {
      if (mounted) setState(() => _error = exception);
    }
  }

  Future<void> _load() async {
    try {
      final scope = ChatSideScope.of(context);
      final result = await scope.account.sharedMessages(
        conversationId: scope.conversation.id,
        kind: widget.kind,
        offset: BigInt.from(_messages?.length ?? 0),
        limit: BigInt.from(widget.pageSize),
      );
      if (!mounted) return;
      final loaded = result.map(MessageListEntry.fromRust).toList();
      setState(() {
        _messages = [...?_messages, ...loaded];
        _hasMore = loaded.length == widget.pageSize;
        _loadingMore = false;
        _error = null;
      });
    } on Object catch (error) {
      if (!mounted) return;
      setState(() {
        _loadingMore = false;
        _error = error;
      });
    }
  }

  Future<void> _loadMore() async {
    if (_loadingMore || !_hasMore) return;
    setState(() {
      _loadingMore = true;
    });
    await _load();
  }

  @override
  Widget build(BuildContext context) {
    final messages = _messages;
    if (messages == null) {
      if (_error != null) {
        return Center(
          child: TextButton(onPressed: _load, child: Text(_error.toString())),
        );
      }
      return const Center(child: CircularProgressIndicator());
    }
    if (messages.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            SvgPicture.asset(
              widget.emptyAsset,
              colorFilter: ColorFilter.mode(
                context.theme.secondaryText.withValues(alpha: 0.4),
                BlendMode.srcIn,
              ),
            ),
            const SizedBox(height: 24),
            Text(
              widget.emptyText,
              style: TextStyle(
                fontSize: 12,
                color: context.theme.secondaryText,
              ),
            ),
          ],
        ),
      );
    }
    final groups = <DateTime, List<MessageListEntry>>{};
    for (final message in messages) {
      final local = message.createdAt.toLocal();
      final day = DateTime(local.year, local.month, local.day);
      groups.putIfAbsent(day, () => []).add(message);
    }
    return NotificationListener<ScrollNotification>(
      onNotification: (notification) {
        if (notification.metrics.extentAfter < 240) unawaited(_loadMore());
        return false;
      },
      child: CustomScrollView(
        slivers: [
          for (final entry in groups.entries) ...[
            SliverPersistentHeader(
              pinned: true,
              delegate: _DateHeaderDelegate(entry.key),
            ),
            if (widget.gridDelegate == null)
              SliverList.builder(
                itemCount: entry.value.length,
                itemBuilder: (context, index) =>
                    widget.itemBuilder(context, entry.value[index], messages),
              )
            else
              SliverPadding(
                padding: const EdgeInsets.symmetric(horizontal: 10),
                sliver: SliverGrid.builder(
                  gridDelegate: widget.gridDelegate!,
                  itemCount: entry.value.length,
                  itemBuilder: (context, index) =>
                      widget.itemBuilder(context, entry.value[index], messages),
                ),
              ),
          ],
          if (_loadingMore)
            const SliverToBoxAdapter(
              child: Padding(
                padding: EdgeInsets.all(12),
                child: Center(child: CircularProgressIndicator()),
              ),
            ),
        ],
      ),
    );
  }
}

class _DateHeaderDelegate extends SliverPersistentHeaderDelegate {
  const _DateHeaderDelegate(this.date);

  final DateTime date;

  @override
  double get minExtent => 38;

  @override
  double get maxExtent => 38;

  @override
  Widget build(
    BuildContext context,
    double shrinkOffset,
    bool overlapsContent,
  ) => Container(
    color: context.theme.primary,
    alignment: Alignment.centerLeft,
    padding: const EdgeInsets.symmetric(horizontal: 10),
    child: Text(
      DateFormat.yMMMd().format(date),
      style: TextStyle(color: context.theme.secondaryText, fontSize: 14),
    ),
  );

  @override
  bool shouldRebuild(_DateHeaderDelegate oldDelegate) =>
      date != oldDelegate.date;
}
