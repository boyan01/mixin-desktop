import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:desktop_webview_window/desktop_webview_window.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:path/path.dart' as path;
import 'package:path_provider/path_provider.dart';
import 'package:url_launcher/url_launcher.dart';

import '../l10n/l10n.dart';
import '../src/rust/desktop_api.dart' as rust;
import '../theme.dart';
import '../widgets/high_light_text.dart';
import '../widgets/mixin_dialog.dart';
import 'mixin_uri.dart';

void openMessageAction({
  required BuildContext context,
  required rust.AccountHandle account,
  required String conversationId,
  required String action,
  required ValueChanged<Uri>? onOpenUri,
  String? title,
}) {
  final value = action.trim();
  if (value.startsWith('input:')) {
    final content = value.substring(6).trim();
    if (content.isNotEmpty) {
      unawaited(
        account.message().sendText(
          conversationId: conversationId,
          content: content,
          silent: false,
        ),
      );
    }
    return;
  }
  final uri = Uri.tryParse(value);
  if (uri == null || uri.scheme.isEmpty) return;
  if (uri.isMixin) {
    onOpenUri?.call(uri);
    return;
  }
  unawaited(
    openBotWebViewWindow(
      context: context,
      url: uri.toString(),
      title: title ?? '',
      conversationId: conversationId,
      currency: account.profile().fiatCurrency,
    ),
  );
}

Future<void> openBotWebViewWindow({
  required BuildContext context,
  required String url,
  required String title,
  required String conversationId,
  required String currency,
}) async {
  if (Platform.isWindows && !await WebviewWindow.isWebviewAvailable()) {
    if (context.mounted) await _showWebViewUnavailableDialog(context);
    return;
  }
  final support = await getApplicationSupportDirectory();
  final packageInfo = await PackageInfo.fromPlatform();
  if (!context.mounted) return;
  final brightness = Theme.of(context).brightness;
  final locale = Localizations.localeOf(context).toLanguageTag();
  final contextJson = jsonEncode({
    'app_version': packageInfo.version,
    'immersive': false,
    'appearance': brightness == Brightness.light ? 'light' : 'dark',
    'platform': 'Desktop',
    'locale': locale,
    'conversation_id': conversationId,
    'currency': currency,
  });
  final webView = await WebviewWindow.create(
    configuration: CreateConfiguration(
      windowWidth: 380,
      windowHeight: 750,
      title: title,
      titleBarTopPadding: 22,
      userDataFolderWindows: path.join(support.path, 'web_view_user_data'),
    ),
  );
  webView
    ..setBrightness(brightness)
    ..addScriptToExecuteOnDocumentCreated('''
window.MixinContext = {
  getContext: function() {
    return ${jsonEncode(contextJson)};
  }
};
''');
  await webView.setApplicationNameForUserAgent(' Mixin/${packageInfo.version}');
  webView.launch(url);
}

Future<void> _showWebViewUnavailableDialog(BuildContext context) =>
    showMixinDialog<void>(
      context: context,
      child: Builder(
        builder: (context) {
          const runtimeDownloadLink =
              'https://go.microsoft.com/fwlink/p/?LinkId=2124703';
          return SizedBox(
            width: 400,
            child: AlertDialogLayout(
              title: Text(context.l10n.webviewRuntimeUnavailable),
              content: DefaultTextStyle.merge(
                style: TextStyle(
                  fontSize: 14,
                  fontWeight: FontWeight.normal,
                  color: context.mixinTheme.text,
                ),
                child: Column(
                  children: [
                    Text(context.l10n.webview2RuntimeInstallDescription),
                    const SizedBox(height: 10),
                    CustomSelectableText.rich(
                      TextSpan(
                        children: [
                          TextSpan(text: context.l10n.downloadLink),
                          TextSpan(
                            text: runtimeDownloadLink,
                            style: TextStyle(color: context.mixinTheme.accent),
                            recognizer: TapGestureRecognizer()
                              ..onTap = () =>
                                  launchUrl(Uri.parse(runtimeDownloadLink)),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(height: 10),
                  ],
                ),
              ),
              actions: [
                MixinButton(
                  onTap: () => Navigator.pop(context),
                  child: Text(context.l10n.confirm),
                ),
              ],
            ),
          );
        },
      ),
    );
