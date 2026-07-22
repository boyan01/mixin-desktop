import 'package:flutter/widgets.dart';
import 'app_logger.dart';

const chatJumpTraceEnabled = bool.fromEnvironment('MIXIN_CHAT_JUMP_TRACE');
const chatScrollTraceEnabled =
    bool.fromEnvironment('MIXIN_CHAT_SCROLL_TRACE') || chatJumpTraceEnabled;

void traceChatJump(String message) {
  if (chatJumpTraceEnabled) i('[chat-jump] $message');
}

void traceChatScroll(String message) {
  if (chatScrollTraceEnabled) i('[chat-scroll] $message');
}

String shortMessageId(String? messageId) {
  if (messageId == null || messageId.isEmpty) return '-';
  return messageId.length <= 8 ? messageId : messageId.substring(0, 8);
}

String formatDouble(num? value) {
  if (value == null || !value.isFinite) return '$value';
  return value.toStringAsFixed(1);
}

String formatScrollMetrics(ScrollMetrics metrics) =>
    'px=${formatDouble(metrics.pixels)} '
    'min=${formatDouble(metrics.minScrollExtent)} '
    'max=${formatDouble(metrics.maxScrollExtent)} '
    'vp=${formatDouble(metrics.viewportDimension)}';
