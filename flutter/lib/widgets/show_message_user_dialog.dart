import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';

Future<void> showMessageUserDialog(
  BuildContext context, {
  required rust.AccountHandle account,
  String? userId,
  String? identityNumber,
}) async {
  assert(userId != null || identityNumber != null);
  if (userId == null && identityNumber == null) return;
  await showDialog<void>(
    context: context,
    barrierColor: Colors.black.withValues(alpha: 0.35),
    builder: (context) => _MessageUserDialog(
      profile: account.userProfile(
        userId: userId,
        identityNumber: identityNumber,
      ),
    ),
  );
}

class _MessageUserDialog extends StatelessWidget {
  const _MessageUserDialog({required this.profile});

  final Future<rust.UserProfileItem?> profile;

  @override
  Widget build(BuildContext context) => Dialog(
    backgroundColor: context.theme.primary,
    surfaceTintColor: Colors.transparent,
    shape: const RoundedRectangleBorder(
      borderRadius: BorderRadius.all(Radius.circular(12)),
    ),
    child: SizedBox(
      width: 340,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Align(
            alignment: Alignment.centerRight,
            child: Padding(
              padding: const EdgeInsets.only(right: 8, top: 8),
              child: IconButton(
                key: const Key('message-user-dialog-close'),
                onPressed: () => Navigator.pop(context),
                icon: Icon(Icons.close, color: context.theme.icon, size: 20),
              ),
            ),
          ),
          FutureBuilder<rust.UserProfileItem?>(
            future: profile,
            builder: (context, snapshot) {
              if (snapshot.connectionState != ConnectionState.done) {
                return const Padding(
                  padding: EdgeInsets.fromLTRB(24, 40, 24, 80),
                  child: CircularProgressIndicator(strokeWidth: 2),
                );
              }
              if (snapshot.hasError) {
                return _ProfileError(message: snapshot.error.toString());
              }
              final profile = snapshot.data;
              if (profile == null) {
                final l10n = Localizations.of<AppLocalizations>(
                  context,
                  AppLocalizations,
                );
                return _ProfileError(
                  message: l10n?.userNotFound ?? 'User not found',
                );
              }
              return _ProfileBody(profile: profile);
            },
          ),
        ],
      ),
    ),
  );
}

class _ProfileBody extends StatelessWidget {
  const _ProfileBody({required this.profile});

  final rust.UserProfileItem profile;

  @override
  Widget build(BuildContext context) {
    final anonymous = profile.identityNumber == '0';
    final l10n = Localizations.of<AppLocalizations>(context, AppLocalizations);
    return Padding(
      padding: const EdgeInsets.fromLTRB(32, 4, 32, 56),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          AvatarView(
            userId: profile.userId,
            name: profile.fullName,
            avatarUrl: profile.avatarUrl,
            size: 90,
          ),
          const SizedBox(height: 8),
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Flexible(
                child: SelectableText(
                  profile.fullName,
                  textAlign: TextAlign.center,
                  style: TextStyle(
                    color: context.theme.text,
                    fontSize: 16,
                    height: 1,
                  ),
                ),
              ),
              if (profile.isVerified)
                Padding(
                  padding: const EdgeInsets.only(left: 4),
                  child: SvgPicture.asset(
                    MixinAssets.verified,
                    width: 14,
                    height: 14,
                  ),
                ),
              if (profile.isBot && !anonymous)
                Padding(
                  padding: const EdgeInsets.only(left: 4),
                  child: Icon(
                    Icons.smart_toy_outlined,
                    color: context.theme.accent,
                    size: 15,
                  ),
                ),
            ],
          ),
          if (!anonymous)
            Padding(
              padding: const EdgeInsets.only(top: 4),
              child: SelectableText(
                l10n?.contactMixinId(profile.identityNumber) ??
                    'Mixin ID: ${profile.identityNumber}',
                textAlign: TextAlign.center,
                style: TextStyle(
                  color: context.theme.secondaryText,
                  fontSize: 12,
                ),
              ),
            ),
          if (profile.biography.trim().isNotEmpty)
            Padding(
              padding: const EdgeInsets.only(top: 20),
              child: SelectableText(
                profile.biography,
                textAlign: TextAlign.center,
                style: TextStyle(color: context.theme.text, fontSize: 14),
              ),
            ),
        ],
      ),
    );
  }
}

class _ProfileError extends StatelessWidget {
  const _ProfileError({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(24, 36, 24, 72),
    child: Text(
      message,
      textAlign: TextAlign.center,
      style: TextStyle(color: context.theme.secondaryText, fontSize: 14),
    ),
  );
}
