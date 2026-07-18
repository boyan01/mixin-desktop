import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/adaptive_selection_toolbar.dart';
import 'package:mixin_desktop_ui/widgets/buttons.dart';
import 'package:mixin_desktop_ui/widgets/custom_popup_menu.dart';
import 'package:mixin_desktop_ui/widgets/mixin_dialog.dart';
import 'package:mixin_desktop_ui/widgets/settings_widgets.dart';
import 'package:mixin_desktop_ui/widgets/toast.dart';
import 'package:url_launcher/url_launcher.dart';

import 'chat_side_scope.dart';

class DisappearMessagePage extends StatefulWidget {
  const DisappearMessagePage({super.key});

  @override
  State<DisappearMessagePage> createState() => _DisappearMessagePageState();
}

class _DisappearMessagePageState extends State<DisappearMessagePage> {
  static const presets = <Duration>[
    Duration.zero,
    Duration(seconds: 30),
    Duration(minutes: 10),
    Duration(hours: 2),
    Duration(days: 1),
    Duration(days: 7),
  ];

  int? selected;
  bool loading = true;
  Object? error;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (selected == null && loading) unawaited(_load());
  }

  Future<void> _load() async {
    final scope = ChatSideScope.of(context);
    try {
      final detail = await scope.account.conversation().localConversationDetail(
        conversationId: scope.conversation.id,
      );
      if (!mounted) return;
      setState(() {
        selected = detail.expireIn;
        loading = false;
        error = null;
      });
      unawaited(_refresh());
    } on Object catch (exception) {
      if (!mounted) return;
      setState(() {
        loading = false;
        error = exception;
      });
    }
  }

  Future<void> _refresh() async {
    final scope = ChatSideScope.of(context);
    try {
      final detail = await scope.account.conversation().conversationDetail(
        conversationId: scope.conversation.id,
      );
      if (mounted) setState(() => selected = detail.expireIn.toInt());
    } on Object catch (exception) {
      if (mounted) setState(() => error = exception);
    }
  }

  Future<void> _set(Duration duration) async {
    final seconds = duration.inSeconds;
    final scope = ChatSideScope.of(context);
    setState(() => selected = seconds);
    try {
      await scope.account.conversation().setDisappearingMessages(
        conversationId: scope.conversation.id,
        duration: seconds,
      );
    } on Object catch (exception) {
      if (!mounted) return;
      setState(() => error = exception);
      showToastFailed(exception);
      await _load();
    }
  }

  Future<void> _custom() async {
    final duration = await showMixinDialog<Duration>(
      context: context,
      child: _CustomExpireTimeDialog(onSubmit: _set),
    );
    if (duration != null) return;
  }

  String _label(BuildContext context, Duration duration) {
    if (duration == Duration.zero) return context.l10n.close;
    if (duration.inSeconds < 60) {
      return '${duration.inSeconds} ${context.l10n.unitSecond(duration.inSeconds)}';
    }
    if (duration.inMinutes < 60) {
      return '${duration.inMinutes} ${context.l10n.unitMinute(duration.inMinutes)}';
    }
    if (duration.inHours < 24) {
      return '${duration.inHours} ${context.l10n.unitHour(duration.inHours)}';
    }
    if (duration.inDays % 7 == 0) {
      final weeks = duration.inDays ~/ 7;
      return '$weeks ${context.l10n.unitWeek(weeks)}';
    }
    return '${duration.inDays} ${context.l10n.unitDay(duration.inDays)}';
  }

  @override
  Widget build(BuildContext context) => ChatSidePageScaffold(
    title: context.l10n.disappearingMessage,
    backgroundColor: context.theme.primary,
    body: SingleChildScrollView(
      child: Column(
        children: [
          const SizedBox(height: 30),
          SvgPicture.asset(
            MixinAssets.disappearingMessage,
            width: 70,
            height: 70,
          ),
          const SizedBox(height: 16),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 20),
            child: _DisappearingHint(
              text: context.l10n.disappearingMessageHint,
            ),
          ),
          const SizedBox(height: 40),
          CellGroup(
            child: Column(
              children: [
                for (final duration in presets)
                  CellItem(
                    title: Text(_label(context, duration)),
                    trailing: selected == duration.inSeconds
                        ? SvgPicture.asset(
                            MixinAssets.checked,
                            width: 24,
                            height: 24,
                          )
                        : null,
                    onTap: selected == duration.inSeconds
                        ? null
                        : () => unawaited(_set(duration)),
                  ),
                CellItem(title: Text(context.l10n.customTime), onTap: _custom),
              ],
            ),
          ),
        ],
      ),
    ),
  );
}

class _DisappearingHint extends StatefulWidget {
  const _DisappearingHint({required this.text});

  final String text;

  @override
  State<_DisappearingHint> createState() => _DisappearingHintState();
}

class _DisappearingHintState extends State<_DisappearingHint> {
  late final TapGestureRecognizer recognizer = TapGestureRecognizer()
    ..onTap = () => launchUrl(
      Uri.parse(
        'https://support.mixin.one/en/article/how-to-enable-disappearing-messages-2nzaz8/',
      ),
    );

  @override
  void dispose() {
    recognizer.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final start = widget.text.indexOf('**');
    final end = start < 0 ? -1 : widget.text.indexOf('**', start + 2);
    final style = TextStyle(
      color: context.theme.secondaryText,
      height: 1.5,
      fontSize: 14,
    );
    if (start < 0 || end < 0) return Text(widget.text, style: style);
    return Text.rich(
      TextSpan(
        style: style,
        children: [
          TextSpan(text: widget.text.substring(0, start)),
          TextSpan(
            text: widget.text.substring(start + 2, end),
            style: TextStyle(color: context.theme.accent),
            recognizer: recognizer,
          ),
          TextSpan(text: widget.text.substring(end + 2)),
        ],
      ),
    );
  }
}

enum _CustomExpireTimeUnit { second, minute, hour, day, week }

extension on _CustomExpireTimeUnit {
  int get maxValue => switch (this) {
    _CustomExpireTimeUnit.second => 59,
    _CustomExpireTimeUnit.minute => 59,
    _CustomExpireTimeUnit.hour => 23,
    _CustomExpireTimeUnit.day => 6,
    _CustomExpireTimeUnit.week => 4,
  };

  Duration duration(int value) => switch (this) {
    _CustomExpireTimeUnit.second => Duration(seconds: value),
    _CustomExpireTimeUnit.minute => Duration(minutes: value),
    _CustomExpireTimeUnit.hour => Duration(hours: value),
    _CustomExpireTimeUnit.day => Duration(days: value),
    _CustomExpireTimeUnit.week => Duration(days: value * 7),
  };
}

class _CustomExpireTimeDialog extends StatefulWidget {
  const _CustomExpireTimeDialog({required this.onSubmit});

  final Future<void> Function(Duration duration) onSubmit;

  @override
  State<_CustomExpireTimeDialog> createState() =>
      _CustomExpireTimeDialogState();
}

class _CustomExpireTimeDialogState extends State<_CustomExpireTimeDialog> {
  final controller = TextEditingController();
  var unit = _CustomExpireTimeUnit.second;

  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final value = int.tryParse(controller.text);
    if (value == null || value <= 0) return;
    if (value > unit.maxValue) {
      showToastFailed(
        ToastError(
          context.l10n.disappearingCustomTimeMaxWarning(
            _formatDuration(context, unit.duration(unit.maxValue)),
          ),
        ),
      );
      return;
    }
    showToastLoading();
    await widget.onSubmit(unit.duration(value));
    Toast.dismiss();
    if (mounted) await Navigator.of(context).maybePop();
  }

  @override
  Widget build(BuildContext context) => SizedBox(
    width: 320,
    child: Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        MixinAppBar(
          leading: const SizedBox(),
          title: Text(context.l10n.customTime),
          actions: const [MixinCloseButton()],
        ),
        const SizedBox(height: 16),
        Row(
          children: [
            const Spacer(),
            SizedBox(
              width: 64,
              child: TextField(
                controller: controller,
                keyboardType: TextInputType.number,
                inputFormatters: [FilteringTextInputFormatter.digitsOnly],
                textAlign: TextAlign.center,
                maxLength: 2,
                contextMenuBuilder: (context, state) =>
                    MixinAdaptiveSelectionToolbar(editableTextState: state),
                style: TextStyle(color: context.theme.text, fontSize: 16),
                buildCounter:
                    (
                      context, {
                      required currentLength,
                      required maxLength,
                      required isFocused,
                    }) => null,
                decoration: InputDecoration(
                  fillColor: context.theme.sidebarSelected,
                  filled: true,
                  hintStyle: TextStyle(
                    fontSize: 16,
                    color: context.theme.secondaryText,
                  ),
                  border: const OutlineInputBorder(
                    borderRadius: BorderRadius.all(Radius.circular(8)),
                    borderSide: BorderSide.none,
                  ),
                  isDense: true,
                  contentPadding: const EdgeInsets.symmetric(vertical: 17),
                ),
              ),
            ),
            const SizedBox(width: 16),
            SizedBox(
              width: 160,
              child: _CustomExpireUnitSelection(
                unit: unit,
                onChanged: (value) => setState(() => unit = value),
              ),
            ),
            const Spacer(),
          ],
        ),
        const SizedBox(height: 28),
        MixinButton(onTap: _submit, child: Text(context.l10n.set)),
        const SizedBox(height: 24),
      ],
    ),
  );
}

class _CustomExpireUnitSelection extends StatelessWidget {
  const _CustomExpireUnitSelection({
    required this.unit,
    required this.onChanged,
  });

  final _CustomExpireTimeUnit unit;
  final ValueChanged<_CustomExpireTimeUnit> onChanged;

  @override
  Widget build(BuildContext context) =>
      CustomPopupMenuButton<_CustomExpireTimeUnit>(
        onSelected: onChanged,
        itemBuilder: (context) => _CustomExpireTimeUnit.values
            .map(
              (value) => CustomPopupMenuItem(
                value: value,
                title: _name(context, value),
              ),
            )
            .toList(growable: false),
        child: Material(
          color: context.theme.sidebarSelected,
          borderRadius: const BorderRadius.all(Radius.circular(8)),
          child: SizedBox(
            height: 46,
            child: Row(
              children: [
                Expanded(
                  child: Text(
                    _name(context, unit),
                    style: TextStyle(fontSize: 16, color: context.theme.text),
                    textAlign: TextAlign.center,
                  ),
                ),
                RotatedBox(
                  quarterTurns: 9,
                  child: SvgPicture.asset(
                    MixinAssets.arrowRight,
                    width: 30,
                    height: 30,
                    colorFilter: ColorFilter.mode(
                      context.theme.secondaryText,
                      BlendMode.srcIn,
                    ),
                  ),
                ),
                const SizedBox(width: 8),
              ],
            ),
          ),
        ),
      );

  String _name(BuildContext context, _CustomExpireTimeUnit value) =>
      switch (value) {
        _CustomExpireTimeUnit.second => context.l10n.unitSecond(1),
        _CustomExpireTimeUnit.minute => context.l10n.unitMinute(1),
        _CustomExpireTimeUnit.hour => context.l10n.unitHour(1),
        _CustomExpireTimeUnit.day => context.l10n.unitDay(1),
        _CustomExpireTimeUnit.week => context.l10n.unitWeek(1),
      };
}

String _formatDuration(BuildContext context, Duration duration) {
  if (duration.inSeconds < 60) {
    return '${duration.inSeconds} ${context.l10n.unitSecond(duration.inSeconds)}';
  }
  if (duration.inMinutes < 60) {
    return '${duration.inMinutes} ${context.l10n.unitMinute(duration.inMinutes)}';
  }
  if (duration.inHours < 24) {
    return '${duration.inHours} ${context.l10n.unitHour(duration.inHours)}';
  }
  if (duration.inDays % 7 == 0) {
    final weeks = duration.inDays ~/ 7;
    return '$weeks ${context.l10n.unitWeek(weeks)}';
  }
  return '${duration.inDays} ${context.l10n.unitDay(duration.inDays)}';
}
