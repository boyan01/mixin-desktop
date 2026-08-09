import 'dart:convert';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:provider/provider.dart';

import '../../constants/assets.dart';
import '../../models/message_list_entry.dart';
import '../../src/rust/desktop_api.dart' as rust;
import '../../theme.dart';
import '../../utils/web_view.dart';
import '../interactive_decorated_box.dart';
import '../message_bubble.dart';
import '../message_style.dart';
import 'unknown_special_message.dart';

class AppButtonGroupMessageItem extends StatelessWidget {
  const AppButtonGroupMessageItem({
    required this.message,
    required this.isCurrentUser,
    required this.showNip,
    required this.highlighted,
    this.highlightOpacity = 0,
    super.key,
    this.quote,
    this.mentionNames = const {},
    this.keyword = '',
    this.isPinnedPage = false,
  });

  final MessageListEntry message;
  final bool isCurrentUser;
  final bool showNip;
  final bool highlighted;
  final double highlightOpacity;
  final Widget? quote;
  final Map<String, String> mentionNames;
  final String keyword;
  final bool isPinnedPage;

  @override
  Widget build(BuildContext context) {
    final actions = ActionData.parseList(message.content);
    if (actions == null) {
      return UnknownSpecialMessage(category: message.category);
    }
    return MessageBubble(
      isCurrentUser: isCurrentUser,
      showNip: showNip,
      showBubble: false,
      padding: EdgeInsets.zero,
      quote: quote,
      isPinnedPage: isPinnedPage,
      pinnedMessageId: message.id,
      isDisappearingMessage: (message.expireIn ?? 0) > 0,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minWidth: 240, maxWidth: 340),
        child: ActionButtonLayout(
          children: actions
              .map(
                (action) => ActionButton(
                  action: action,
                  conversationId: message.conversationId,
                  highlighted: highlighted,
                  highlightOpacity: highlightOpacity,
                ),
              )
              .toList(growable: false),
        ),
      ),
    );
  }
}

class ActionButton extends StatelessWidget {
  const ActionButton({
    required this.action,
    required this.conversationId,
    required this.highlighted,
    required this.highlightOpacity,
    this.interactive = true,
    super.key,
  });

  final ActionData action;
  final String conversationId;
  final bool highlighted;
  final double highlightOpacity;
  final bool interactive;

  @override
  Widget build(BuildContext context) {
    final bubbleClipper = BubbleClipper(
      currentUser: false,
      showNip: false,
      nipPadding: false,
    );
    return InteractiveDecoratedBox.color(
      cursor: SystemMouseCursors.click,
      onTap: interactive
          ? () => openMessageAction(
              context: context,
              account: context.read<rust.AccountHandle>(),
              conversationId: conversationId,
              action: action.action,
              title: action.label,
            )
          : null,
      child: MessageBubbleHighlight(
        clipper: bubbleClipper,
        currentUser: false,
        media: true,
        enabled: highlighted || highlightOpacity > 0,
        opacity: highlighted ? 1 : highlightOpacity,
        child: CustomPaint(
          painter: BubblePainter(
            color: context.theme.primary,
            clipper: bubbleClipper,
          ),
          child: IntrinsicWidth(
            child: Stack(
              fit: StackFit.passthrough,
              children: [
                Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 14,
                    vertical: 8,
                  ),
                  child: Text(
                    action.label,
                    textAlign: TextAlign.center,
                    style: TextStyle(
                      color: action.color,
                      fontSize: context.messageStyle.secondaryFontSize,
                      height: 1,
                    ),
                  ),
                ),
                if (action.isSendUserLink)
                  Align(
                    alignment: Alignment.topRight,
                    child: Padding(
                      padding: const EdgeInsets.all(6),
                      child: SvgPicture.asset(
                        MixinAssets.linkSend,
                        width: 8,
                        height: 8,
                      ),
                    ),
                  )
                else if (action.isExternalLink)
                  Align(
                    alignment: Alignment.topRight,
                    child: Padding(
                      padding: const EdgeInsets.all(6),
                      child: SvgPicture.asset(
                        MixinAssets.externalLink,
                        width: 6,
                        height: 6,
                      ),
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

class ActionButtonLayout extends MultiChildRenderObjectWidget {
  const ActionButtonLayout({required super.children, super.key});

  @override
  RenderObject createRenderObject(BuildContext context) =>
      _RenderActionButtonLayout(horizontalSpacing: 8, verticalSpacing: 8);
}

class _ActionButtonParentData extends ContainerBoxParentData<RenderBox> {
  int row = 0;
  double height = 0;
}

class _RenderActionButtonLayout extends RenderBox
    with
        ContainerRenderObjectMixin<RenderBox, _ActionButtonParentData>,
        RenderBoxContainerDefaultsMixin<RenderBox, _ActionButtonParentData> {
  _RenderActionButtonLayout({
    required this.horizontalSpacing,
    required this.verticalSpacing,
  });

  final double horizontalSpacing;
  final double verticalSpacing;

  @override
  void setupParentData(covariant RenderObject child) {
    if (child.parentData is! _ActionButtonParentData) {
      child.parentData = _ActionButtonParentData();
    }
  }

  @override
  Size computeDryLayout(covariant BoxConstraints constraints) {
    final childWidths = [
      (constraints.maxWidth - horizontalSpacing * 2) / 3,
      (constraints.maxWidth - horizontalSpacing) / 2,
    ];
    final childConstraints = BoxConstraints(
      maxWidth: constraints.maxWidth,
      maxHeight: constraints.maxHeight,
    );
    var height = 0.0;
    var rowItems = 0;
    var maxRowHeight = 0.0;
    var row = 0;
    var child = firstChild;
    while (child != null) {
      final parentData = child.parentData! as _ActionButtonParentData;
      final childSize = child.getDryLayout(childConstraints);
      final lastMaxRowHeight = maxRowHeight;
      maxRowHeight = math.max(childSize.height, maxRowHeight);
      parentData
        ..row = row
        ..height = maxRowHeight;

      if (childSize.width <= childWidths[0]) {
        rowItems = rowItems == 0 || rowItems == 2 || rowItems == 4
            ? rowItems + 2
            : 6;
      } else if (childSize.width <= childWidths[1]) {
        if (rowItems == 0) {
          rowItems = 3;
        } else if (rowItems == 2 || rowItems == 3) {
          rowItems = 6;
        } else {
          rowItems = 2;
          height += lastMaxRowHeight + verticalSpacing;
          maxRowHeight = childSize.height;
          row++;
          parentData
            ..row = row
            ..height = maxRowHeight;
        }
      } else if (rowItems == 0) {
        rowItems = 6;
      } else {
        rowItems = 0;
        height += lastMaxRowHeight + verticalSpacing;
        height += childSize.height + verticalSpacing;
        maxRowHeight = 0;
        parentData
          ..row = row + 1
          ..height = childSize.height;
        row += 2;
      }

      if (rowItems >= 6) {
        rowItems = 0;
        height += maxRowHeight + verticalSpacing;
        maxRowHeight = 0;
        row++;
      }
      child = parentData.nextSibling;
    }
    if (maxRowHeight != 0) height += maxRowHeight + verticalSpacing;
    if (height > 0) height -= verticalSpacing;
    return constraints.constrain(Size(constraints.maxWidth, height));
  }

  @override
  void performLayout() {
    size = computeDryLayout(constraints);
    final rows = <int, List<RenderBox>>{};
    var child = firstChild;
    while (child != null) {
      final parentData = child.parentData! as _ActionButtonParentData;
      (rows[parentData.row] ??= []).add(child);
      child = parentData.nextSibling;
    }

    var offsetY = 0.0;
    for (final row in rows.values) {
      final childWidth =
          (constraints.maxWidth - (row.length - 1) * horizontalSpacing) /
          row.length;
      final rowHeight = row.fold<double>(
        0,
        (value, item) => math.max(
          value,
          (item.parentData! as _ActionButtonParentData).height,
        ),
      );
      final childConstraints = BoxConstraints.expand(
        width: childWidth,
        height: rowHeight,
      );
      var offsetX = 0.0;
      for (final item in row) {
        item.layout(childConstraints);
        (item.parentData! as _ActionButtonParentData).offset = Offset(
          offsetX,
          offsetY,
        );
        offsetX += childWidth + horizontalSpacing;
      }
      offsetY += rowHeight + verticalSpacing;
    }
  }

  @override
  void paint(PaintingContext context, Offset offset) =>
      defaultPaint(context, offset);

  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) =>
      defaultHitTestChildren(result, position: position);
}

class ActionData {
  const ActionData(this.label, this.action, this.color);

  factory ActionData.fromJson(Map<String, dynamic> json) => ActionData(
    json['label']?.toString() ?? '',
    json['action']?.toString() ?? '',
    _parseColor(json['color']?.toString()),
  );

  final String label;
  final String action;
  final Color color;

  Uri? get _uri => Uri.tryParse(action);

  bool get isExternalLink {
    final uri = _uri;
    if (uri == null || (uri.scheme != 'http' && uri.scheme != 'https')) {
      return false;
    }
    const mixinActionPaths = {
      'codes',
      'pay',
      'users',
      'transfer',
      'device',
      'send',
      'address',
      'withdrawal',
      'apps',
      'snapshots',
      'conversations',
      'multisigs',
      'swap',
      'markets',
      'membership',
    };
    final isMixinHost = uri.host == 'mixin.one' || uri.host == 'www.mixin.one';
    final isMixinAction =
        isMixinHost &&
        uri.pathSegments.isNotEmpty &&
        mixinActionPaths.contains(uri.pathSegments.first);
    return !isMixinAction;
  }

  bool get isSendUserLink {
    final uri = _uri;
    if (uri == null || uri.queryParameters['user']?.trim().isEmpty != false) {
      return false;
    }
    return (uri.scheme == 'mixin' && uri.host == 'send') ||
        ((uri.host == 'mixin.one' || uri.host == 'www.mixin.one') &&
            uri.pathSegments.isNotEmpty &&
            uri.pathSegments.first == 'send');
  }

  static List<ActionData>? parseList(String raw) {
    try {
      final json = jsonDecode(raw) as List<dynamic>;
      return json
          .map((item) => ActionData.fromJson(item as Map<String, dynamic>))
          .toList();
    } on Object {
      return null;
    }
  }
}

Color _parseColor(String? raw) {
  final value = raw?.replaceFirst('#', '') ?? '';
  final parsed = int.tryParse(value, radix: 16);
  if (parsed == null) return Colors.black;
  return Color(value.length == 6 ? 0xFF000000 | parsed : parsed);
}
