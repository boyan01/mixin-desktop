import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/widgets/mixin_image.dart';

class BadgesWidget extends StatelessWidget {
  const BadgesWidget({
    required this.verified,
    required this.isBot,
    required this.membership,
    super.key,
  });

  final bool verified;
  final bool isBot;
  final String? membership;

  @override
  Widget build(BuildContext context) {
    final plan = _validMembershipPlan(membership);
    Widget child;
    if (plan != null) {
      child = MixinImage.asset(
        switch (plan) {
          'advance' => 'assets/images/plan_basic.png',
          'elite' => 'assets/images/plan_standard.png',
          'prosperity' => 'assets/images/plan_premium.gif',
          _ => throw StateError('Unsupported membership plan: $plan'),
        },
        width: 14,
        height: 14,
        isAntiAlias: true,
      );
    } else if (verified) {
      child = SvgPicture.asset(MixinAssets.verified, width: 12, height: 12);
    } else if (isBot) {
      child = SvgPicture.asset(MixinAssets.botBadge, width: 12, height: 12);
    } else {
      return const SizedBox();
    }

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4),
      child: child,
    );
  }
}

String? _validMembershipPlan(String? raw) {
  if (raw == null || raw.isEmpty) return null;
  try {
    final membership = jsonDecode(raw) as Map<String, dynamic>;
    final plan = membership['plan'] as String?;
    final expiredAt = DateTime.tryParse(
      membership['expired_at'] as String? ?? '',
    );
    if (expiredAt == null || !DateTime.now().isBefore(expiredAt)) return null;
    return const {'advance', 'elite', 'prosperity'}.contains(plan)
        ? plan
        : null;
  } on FormatException {
    return null;
  } on TypeError {
    return null;
  }
}
