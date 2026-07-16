import 'package:flutter/widgets.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';

extension AppLocalizationsContext on BuildContext {
  AppLocalizations get l10n => AppLocalizations.of(this);
}
