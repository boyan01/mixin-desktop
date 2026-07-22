import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import '../constants/assets.dart';
import '../l10n/l10n.dart';
import '../theme.dart';
import 'mixin_dialog.dart';

Future<int?> showMuteDialog(BuildContext context) =>
    showMixinDialog<int>(context: context, child: const MuteDialog());

class MuteDialog extends StatefulWidget {
  const MuteDialog({super.key});

  @override
  State<MuteDialog> createState() => _MuteDialogState();
}

class _MuteDialogState extends State<MuteDialog> {
  int? selected;

  @override
  Widget build(BuildContext context) => AlertDialogLayout(
    title: Text(context.l10n.contactMuteTitle),
    content: SingleChildScrollView(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children:
            [
                  (context.l10n.oneHour, 60 * 60),
                  (context.l10n.hour(8, 8), 8 * 60 * 60),
                  ('1 ${context.l10n.unitWeek(1)}', 7 * 24 * 60 * 60),
                  (context.l10n.oneYear, 365 * 24 * 60 * 60),
                ]
                .map(
                  (option) => _RadioItem(
                    title: option.$1,
                    selected: selected == option.$2,
                    onTap: () => setState(() => selected = option.$2),
                  ),
                )
                .toList(growable: false),
      ),
    ),
    actions: [
      MixinButton(
        backgroundTransparent: true,
        onTap: () => Navigator.pop(context),
        child: Text(context.l10n.cancel),
      ),
      MixinButton(
        onTap: () => Navigator.pop(context, selected),
        child: Text(context.l10n.confirm),
      ),
    ],
  );
}

class _RadioItem extends StatelessWidget {
  const _RadioItem({
    required this.title,
    required this.selected,
    required this.onTap,
  });

  final String title;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => GestureDetector(
    behavior: HitTestBehavior.opaque,
    onTap: onTap,
    child: Padding(
      padding: const EdgeInsets.symmetric(vertical: 10),
      child: Row(
        children: [
          ClipOval(
            child: Container(
              color: selected
                  ? context.theme.accent
                  : context.theme.secondaryText,
              height: 16,
              width: 16,
              alignment: const Alignment(0, -0.2),
              child: SvgPicture.asset(
                MixinAssets.selected,
                height: 10,
                width: 10,
              ),
            ),
          ),
          const SizedBox(width: 30),
          Text(
            title,
            style: TextStyle(color: context.theme.text, fontSize: 16),
          ),
        ],
      ),
    ),
  );
}
