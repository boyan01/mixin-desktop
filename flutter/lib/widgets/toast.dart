import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/utils/app_logger.dart';
import 'package:overlay_support/overlay_support.dart';

class Toast {
  Toast._();

  static const shortDuration = Duration(seconds: 1);

  static OverlaySupportEntry? _entry;

  static void createView({
    required WidgetBuilder builder,
    Duration? duration = shortDuration,
    BuildContext? context,
  }) {
    dismiss();
    _entry = showOverlay(
      context: context,
      (context, progress) =>
          Opacity(opacity: progress, child: builder(context)),
      duration: duration ?? Duration.zero,
    );
  }

  static void dismiss() {
    _entry?.dismiss();
    _entry = null;
  }
}

class ToastWidget extends StatelessWidget {
  const ToastWidget({
    required this.text,
    this.barrierColor = const Color(0x80000000),
    this.icon,
    this.ignoring = true,
    super.key,
  });

  final Color barrierColor;
  final Widget? icon;
  final String text;
  final bool ignoring;

  @override
  Widget build(BuildContext context) => IgnorePointer(
    ignoring: ignoring,
    child: Material(
      color: Colors.transparent,
      child: Container(
        color: barrierColor,
        alignment: Alignment.center,
        child: Container(
          constraints: const BoxConstraints(minWidth: 130),
          decoration: const BoxDecoration(
            borderRadius: BorderRadius.all(Radius.circular(8)),
            color: Color.fromRGBO(62, 65, 72, 0.7),
          ),
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const SizedBox(height: 20),
              if (icon != null) SizedBox(height: 30, width: 30, child: icon),
              if (icon != null) const SizedBox(height: 12),
              Text(
                text,
                style: const TextStyle(color: Colors.white, fontSize: 14),
              ),
              const SizedBox(height: 20),
            ],
          ),
        ),
      ),
    ),
  );
}

void showToastSuccessful({BuildContext? context}) => Toast.createView(
  context: context,
  builder: (context) => ToastWidget(
    barrierColor: Colors.transparent,
    icon: const _Successful(),
    text: context.l10n.successful,
  ),
);

void showToast(String message, {BuildContext? context}) => Toast.createView(
  context: context,
  builder: (context) =>
      ToastWidget(barrierColor: Colors.transparent, text: message),
);

class ToastError extends Error {
  factory ToastError(String message) => ToastError._internal(message: message);

  factory ToastError.builder(String Function(BuildContext context)? builder) =>
      ToastError._internal(messageBuilder: builder);

  ToastError._internal({this.message, this.messageBuilder});

  final String? message;
  final String Function(BuildContext context)? messageBuilder;

  static String errorToString(BuildContext context, Object? error) {
    if (error is ToastError) {
      return error.message ??
          error.messageBuilder?.call(context) ??
          context.l10n.failed;
    }
    final message = switch (error) {
      AnyhowException(:final message) => message,
      String() => error,
      _ => error?.toString(),
    };
    if (message == null) return context.l10n.failed;
    if (message.contains('attachment_upload_failed:')) {
      return context.l10n.errorUploadAttachmentFailed;
    }
    final apiError = _MixinApiError.tryParse(message);
    return apiError?.toDisplayString(context) ?? message;
  }
}

class _MixinApiError {
  const _MixinApiError({required this.code, required this.description});

  final int code;
  final String description;

  static final _pattern = RegExp(
    r'(?:server error: )?Error: status: -?\d+, code: (-?\d+), description: ([\s\S]*)',
  );

  static _MixinApiError? tryParse(String value) {
    final match = _pattern.firstMatch(value);
    if (match == null) return null;
    final code = int.tryParse(match.group(1)!);
    if (code == null) return null;
    return _MixinApiError(code: code, description: match.group(2)!);
  }

  String toDisplayString(BuildContext context) {
    final l10n = context.l10n;
    return switch (code) {
      10001 => '$code TRANSACTION',
      10002 => l10n.errorBadData,
      10003 => l10n.errorPhoneSmsDelivery,
      10004 => l10n.errorRecaptchaIsInvalid,
      10006 => l10n.errorOldVersion(description),
      20110 => l10n.errorPhoneInvalidFormat,
      20111 => '$code INSUFFICIENT_IDENTITY_NUMBER',
      20112 => '$code INVALID_INVITATION_CODE',
      20113 => l10n.errorPhoneVerificationCodeInvalid,
      20114 => l10n.errorPhoneVerificationCodeExpired,
      20115 => '$code INVALID_QR_CODE',
      404 => l10n.errorNotFound,
      20116 => l10n.errorFullGroup,
      20117 => l10n.errorInsufficientBalance,
      20118 => l10n.errorInvalidPinFormat,
      20119 => l10n.errorPinIncorrect,
      20120 => l10n.errorTooSmallTransferAmount,
      429 => l10n.errorTooManyRequest,
      20122 => l10n.errorUsedPhone,
      20126 => l10n.errorTooManyStickers,
      30100 => l10n.errorBlockchain,
      30102 => l10n.errorInvalidAddressPlain,
      20127 => l10n.errorTooSmallWithdrawAmount,
      20129 => l10n.errorInvalidCodeTooFrequent,
      20130 => l10n.errorInvalidEmergencyContact,
      20131 => l10n.errorWithdrawalMemoFormatIncorrect,
      20132 || 20133 => l10n.errorNumberReachedLimit,
      403 => l10n.errorAccessLimited,
      500 || 30103 => l10n.errorServer5xxCode(description),
      911 => '$code TIME_INACCURATE',
      _ => l10n.errorUnknownWithCode(description),
    };
  }
}

void showToastFailed(Object? error, {BuildContext? context}) =>
    Toast.createView(
      context: context,
      builder: (context) => ToastWidget(
        barrierColor: Colors.transparent,
        icon: const _Failed(),
        text: ToastError.errorToString(context, error),
      ),
    );

void showToastLoading({BuildContext? context}) => Toast.createView(
  context: context,
  builder: (context) => ToastWidget(
    icon: const _Loading(),
    text: context.l10n.loading,
    ignoring: false,
  ),
  duration: null,
);

Future<bool> runFutureWithToast(Future<dynamic> future) async {
  showToastLoading();
  try {
    await future;
  } on Object catch (error, stackTrace) {
    writeAppLog('runFutureWithToast error: $error\n$stackTrace');
    showToastFailed(error);
    return false;
  }
  showToastSuccessful();
  return true;
}

Future<bool> runWithToast(FutureOr<void> Function() function) async {
  showToastLoading();
  try {
    await function();
  } on Object catch (error, stackTrace) {
    writeAppLog('runWithToast error: $error\n$stackTrace');
    showToastFailed(error);
    return false;
  }
  showToastSuccessful();
  return true;
}

Future<void> runWithLoading(Future<void> Function() function) async {
  showToastLoading();
  try {
    await function();
    Toast.dismiss();
  } on Object catch (error, stackTrace) {
    writeAppLog('runWithLoading error: $error\n$stackTrace');
    showToastFailed(error);
  }
}

class _Loading extends StatelessWidget {
  const _Loading();

  @override
  Widget build(BuildContext context) => const CircularProgressIndicator(
    valueColor: AlwaysStoppedAnimation(Colors.white),
    strokeWidth: 3,
  );
}

class _Failed extends StatelessWidget {
  const _Failed();

  @override
  Widget build(BuildContext context) => SvgPicture.asset(MixinAssets.failed);
}

class _Successful extends StatelessWidget {
  const _Successful();

  @override
  Widget build(BuildContext context) =>
      SvgPicture.asset(MixinAssets.successful);
}
