import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:pretty_qr_code/pretty_qr_code.dart';

import '../constants/assets.dart';
import '../l10n/l10n.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../theme.dart';
import 'avatar_view.dart';
import 'buttons.dart';
import 'mixin_dialog.dart';
import 'show_snapshot_detail_dialog.dart';

Future<void> showMultisigsPaymentDialog(
  BuildContext context, {
  required rust.CodeResult item,
  required Uri uri,
}) => showMixinDialog<void>(
  context: context,
  child: _PaymentDialog(item: item, uri: uri),
);

class _PaymentDialog extends StatelessWidget {
  const _PaymentDialog({required this.item, required this.uri});

  final rust.CodeResult item;
  final Uri uri;

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    children: [
      SizedBox(
        width: 340,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Row(
              children: [
                Spacer(),
                Padding(
                  padding: EdgeInsets.only(right: 12, top: 12),
                  child: MixinCloseButton(),
                ),
              ],
            ),
            _PaymentBody(item: item, uri: uri),
          ],
        ),
      ),
    ],
  );
}

class _PaymentBody extends StatelessWidget {
  const _PaymentBody({required this.item, required this.uri});

  final rust.CodeResult item;
  final Uri uri;

  @override
  Widget build(BuildContext context) {
    final users = {
      for (final user in item.participantAvatars) user.userId: user,
    };
    final done = const {
      'signed',
      'unlocked',
      'paid',
    }.contains(item.state?.toLowerCase());
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          item.kind == 'multisig_request' && item.action == 'unlock'
              ? context.l10n.revokeMultisigTransaction
              : context.l10n.multisigTransaction,
          style: TextStyle(fontSize: 18, color: context.theme.text),
        ),
        const SizedBox(height: 24),
        _UsersLayout(
          senders: item.senders,
          receivers: item.receivers,
          users: users,
        ),
        const SizedBox(height: 24),
        SymbolIconWithBorder(
          size: 48,
          symbolUrl: item.assetIconUrl ?? '',
          chainUrl: item.chainIconUrl ?? '',
          chainSize: 14,
        ),
        const SizedBox(height: 10),
        Text(
          '${_formatAmount(item.amount ?? '')} ${item.assetSymbol ?? ''}',
          style: TextStyle(fontSize: 16, color: context.theme.text),
        ),
        const SizedBox(height: 8),
        if (done) const _DoneLayout() else _QrCodeLayout(uri: uri),
        const SizedBox(height: 20),
      ],
    );
  }
}

class _UsersLayout extends StatelessWidget {
  const _UsersLayout({
    required this.senders,
    required this.receivers,
    required this.users,
  });

  final List<String> senders;
  final List<String> receivers;
  final Map<String, rust.GroupAvatar> users;

  @override
  Widget build(BuildContext context) => Row(
    mainAxisAlignment: MainAxisAlignment.center,
    children: [
      _OverlappedUserAvatars(children: _avatars(senders)),
      SizedBox.square(
        dimension: 24,
        child: SvgPicture.asset(
          MixinAssets.arrowRight,
          colorFilter: ColorFilter.mode(context.theme.green, BlendMode.srcIn),
        ),
      ),
      _OverlappedUserAvatars(children: _avatars(receivers)),
    ],
  );

  List<Widget> _avatars(List<String> ids) => [
    for (final id in ids.take(ids.length <= 3 ? 3 : 2))
      _UserIcon(userId: id, user: users[id]),
    if (ids.length > 3) _UserCountIcon(count: ids.length - 2),
  ];
}

class _UserCountIcon extends StatelessWidget {
  const _UserCountIcon({required this.count});

  final int count;

  @override
  Widget build(BuildContext context) => Container(
    width: 24,
    height: 24,
    decoration: BoxDecoration(
      color: context.theme.listSelected,
      shape: BoxShape.circle,
    ),
    child: Center(
      child: Text(
        '+$count',
        style: TextStyle(fontSize: 12, color: context.theme.secondaryText),
      ),
    ),
  );
}

class _OverlappedUserAvatars extends StatelessWidget {
  const _OverlappedUserAvatars({required this.children});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) => Stack(
    children: [
      for (var index = 0; index < children.length; index++)
        Padding(
          padding: EdgeInsets.fromLTRB(index.toDouble() * 20, 0, 0, 0),
          child: ClipOval(
            child: Container(
              color: context.theme.popUp,
              padding: const EdgeInsets.all(2),
              child: children[index],
            ),
          ),
        ),
    ].reversed.toList(),
  );
}

class _UserIcon extends StatelessWidget {
  const _UserIcon({required this.userId, required this.user});

  final String userId;
  final rust.GroupAvatar? user;

  @override
  Widget build(BuildContext context) {
    final user = this.user;
    if (user == null) {
      return Container(
        width: 24,
        height: 24,
        decoration: BoxDecoration(
          color: Color.alphaBlend(
            context.theme.listSelected,
            context.theme.popUp,
          ),
          shape: BoxShape.circle,
        ),
      );
    }
    return AvatarView(
      userId: userId,
      name: user.name,
      avatarUrl: user.avatarUrl,
      size: 24,
    );
  }
}

class _QrCodeLayout extends StatelessWidget {
  const _QrCodeLayout({required this.uri});

  final Uri uri;

  @override
  Widget build(BuildContext context) => Column(
    children: [
      const SizedBox(height: 32),
      ClipRRect(
        borderRadius: const BorderRadius.all(Radius.circular(8)),
        child: ColoredBox(
          color: Colors.white,
          child: SizedBox.square(
            dimension: 180,
            child: PrettyQrView.data(data: uri.toString()),
          ),
        ),
      ),
      const SizedBox(height: 32),
    ],
  );
}

class _DoneLayout extends StatelessWidget {
  const _DoneLayout();

  @override
  Widget build(BuildContext context) => Column(
    children: [
      const SizedBox(height: 40),
      Container(
        width: 80,
        height: 80,
        decoration: BoxDecoration(
          color: context.theme.green.withValues(alpha: 0.2),
          shape: BoxShape.circle,
        ),
        child: Center(
          child: SizedBox.square(
            dimension: 60,
            child: SvgPicture.asset(
              MixinAssets.checked,
              colorFilter: ColorFilter.mode(
                context.theme.green,
                BlendMode.srcIn,
              ),
            ),
          ),
        ),
      ),
      const SizedBox(height: 10),
      Text(
        context.l10n.done,
        style: TextStyle(fontSize: 14, color: context.theme.secondaryText),
      ),
      const SizedBox(height: 40),
    ],
  );
}

String _formatAmount(String amount) {
  final parts = amount.split('.');
  final sign = parts.first.startsWith('-') ? '-' : '';
  final digits = parts.first.replaceFirst('-', '');
  final buffer = StringBuffer(sign);
  for (var index = 0; index < digits.length; index++) {
    if (index > 0 && (digits.length - index) % 3 == 0) buffer.write(',');
    buffer.write(digits[index]);
  }
  if (parts.length > 1) buffer.write('.${parts.sublist(1).join('.')}');
  return buffer.toString();
}
