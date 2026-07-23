import 'dart:async';
import 'dart:io';

import 'package:flutter/widgets.dart';
import 'package:flutter_app_icon_badge/flutter_app_icon_badge.dart';
import 'package:flutter_hooks/flutter_hooks.dart';

import '../src/rust/desktop_api.dart';
import '../utils/app_logger.dart';

class AppIconBadge extends HookWidget {
  const AppIconBadge({required this.account, required this.child, super.key});

  final AccountHandle account;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    useEffect(() {
      var pending = Future<void>.value();
      final subscription = account.unseenMessageCountChanges().listen(
        (count) {
          pending = pending.then((_) async {
            try {
              if (!Platform.isMacOS) return;
              if (count == 0) {
                await FlutterAppIconBadge.removeBadge();
              } else {
                await FlutterAppIconBadge.updateBadge(count);
              }
            } catch (exception, stackTrace) {
              e('Update app icon badge failed', exception, stackTrace);
            }
          });
        },
        onError: (Object exception, StackTrace stackTrace) {
          e('App icon badge stream failed', exception, stackTrace);
        },
      );
      return () => unawaited(subscription.cancel());
    }, [account]);
    return child;
  }
}
