import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:url_launcher/url_launcher.dart';

import '../l10n/l10n.dart';
import '../src/rust/desktop_api.dart';
import '../theme.dart';
import 'mixin_dialog.dart';

class AccountHealthOverlays extends HookWidget {
  const AccountHealthOverlays({
    required this.account,
    required this.child,
    super.key,
  });

  final AccountHandle account;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final health = useStream(
      useMemoized(() => account.accountHealth().distinct(), [account]),
      initialData: 'ready',
    ).data;
    return Stack(
      fit: StackFit.expand,
      children: [
        child,
        if (health == 'time_inaccurate') _LocalTimeError(account: account),
        if (health == 'update_required') const _RequiredUpdate(),
      ],
    );
  }
}

class _LocalTimeError extends StatelessWidget {
  const _LocalTimeError({required this.account});

  final AccountHandle account;

  @override
  Widget build(BuildContext context) => HookBuilder(
    builder: (context) {
      final loading = useState(false);
      return Material(
        color: context.theme.background,
        child: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                context.l10n.loadingTime,
                style: TextStyle(color: context.theme.text, fontSize: 16),
              ),
              const SizedBox(height: 24),
              if (loading.value)
                CircularProgressIndicator(color: context.theme.accent),
              if (!loading.value)
                MixinButton(
                  onTap: () async {
                    loading.value = true;
                    try {
                      await account.refreshAccountHealth();
                    } on Object {
                      // The overlay remains visible until account health recovers.
                    }
                    loading.value = false;
                  },
                  child: Text(context.l10n.continueText),
                ),
            ],
          ),
        ),
      );
    },
  );
}

class _RequiredUpdate extends HookWidget {
  const _RequiredUpdate();

  @override
  Widget build(BuildContext context) {
    final info = useFuture(useMemoized(PackageInfo.fromPlatform)).data;
    return Material(
      color: context.theme.background,
      child: Stack(
        children: [
          Center(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  context.l10n.updateMixin,
                  style: TextStyle(color: context.theme.text, fontSize: 16),
                ),
                const SizedBox(height: 10),
                Text(
                  context.l10n.updateMixinDescription(info?.version ?? ''),
                  textAlign: TextAlign.center,
                  style: TextStyle(color: context.theme.text, fontSize: 14),
                ),
                const SizedBox(height: 32),
                MixinButton(
                  onTap: () =>
                      launchUrl(Uri.parse('https://mixin.one/messenger')),
                  child: Text(context.l10n.upgrade),
                ),
              ],
            ),
          ),
          Positioned(
            bottom: 16,
            right: 16,
            child: Text(
              info == null ? '' : '${info.version}+${info.buildNumber}',
              style: TextStyle(
                color: context.theme.secondaryText,
                fontSize: 12,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
