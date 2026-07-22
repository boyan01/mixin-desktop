import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart';
import 'package:provider/provider.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:window_manager/window_manager.dart';

import 'constants/assets.dart';
import 'controllers/app_controller.dart';
import 'controllers/network_controller.dart';
import 'controllers/security_controller.dart';
import 'controllers/settings_controller.dart';
import 'l10n/generated/app_localizations.dart';
import 'l10n/l10n.dart';
import 'network/core_http_scope.dart';
import 'pages/home_page.dart';
import 'pages/login_page.dart';
import 'src/rust/desktop_api.dart';
import 'theme.dart';
import 'utils/text_input_action_handler.dart';
import 'widgets/auth_guard.dart';
import 'widgets/mixin_dialog.dart';
import 'widgets/move_window.dart';
import 'widgets/qr_login_card.dart';
import 'widgets/system_tray.dart';
import 'widgets/toast.dart';

final rootRouteObserver = RouteObserver<ModalRoute<dynamic>>();

class MixinDesktopApp extends StatelessWidget {
  const MixinDesktopApp({super.key, this.initialProtocolUrl});

  final String? initialProtocolUrl;

  @override
  Widget build(BuildContext context) {
    precacheImage(const AssetImage(MixinAssets.chatBackground), context);
    final settings = context.watch<SettingsController>();
    return _AppLifecycleScope(
      child: _WindowShortcuts(
        child: GlobalMoveWindow(
          child: MaterialApp(
            title: 'Mixin',
            navigatorObservers: [rootRouteObserver],
            debugShowCheckedModeBanner: false,
            theme: buildMixinTheme(Brightness.light),
            darkTheme: buildMixinTheme(Brightness.dark),
            themeMode: settings.themeMode,
            localizationsDelegates: const [
              AppLocalizations.delegate,
              GlobalMaterialLocalizations.delegate,
              GlobalWidgetsLocalizations.delegate,
              GlobalCupertinoLocalizations.delegate,
            ],
            supportedLocales: AppLocalizations.supportedLocales,
            builder: (context, child) {
              Intl.defaultLocale = Intl.canonicalizedLocale(
                Localizations.localeOf(context).toString(),
              );
              final mediaQuery = MediaQuery.of(context);
              return MediaQuery(
                data: mediaQuery.copyWith(
                  textScaler: defaultTargetPlatform == TargetPlatform.linux
                      ? TextScaler.noScaling
                      : mediaQuery.textScaler,
                ),
                child: SystemTrayWidget(
                  child: TextInputActionHandler(
                    child: _WindowsTitleBarDivider(child: child!),
                  ),
                ),
              );
            },
            home: _AppBody(initialProtocolUrl: initialProtocolUrl),
          ),
        ),
      ),
    );
  }
}

class _AppLifecycleScope extends StatefulWidget {
  const _AppLifecycleScope({required this.child});

  final Widget child;

  @override
  State<_AppLifecycleScope> createState() => _AppLifecycleScopeState();
}

class _AppLifecycleScopeState extends State<_AppLifecycleScope>
    with WidgetsBindingObserver {
  bool active = true;
  FocusNode? previousFocus;
  FocusNode? previousEditableFocus;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    FocusManager.instance.addListener(_handlePrimaryFocusChanged);
  }

  void _handlePrimaryFocusChanged() {
    if (!active) return;
    final primaryFocus = FocusManager.instance.primaryFocus;
    if (previousEditableFocus == primaryFocus) return;
    _cleanSelection(previousEditableFocus?.context);
    previousEditableFocus = primaryFocus;
  }

  void _cleanSelection(BuildContext? context) {
    if (context?.mounted != true) return;
    final editableText = context?.findAncestorWidgetOfExactType<EditableText>();
    if (editableText == null) return;
    final selection = editableText.controller.selection;
    editableText.controller.selection = selection.copyWith(
      baseOffset: selection.baseOffset,
      extentOffset: selection.baseOffset,
      isDirectional: true,
    );
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    final nextActive = state == AppLifecycleState.resumed;
    if (nextActive == active) return;
    if (nextActive) {
      previousFocus?.requestFocus();
      previousFocus = null;
    } else {
      previousFocus = FocusManager.instance.primaryFocus;
      previousFocus?.unfocus();
    }
    setState(() => active = nextActive);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    FocusManager.instance.removeListener(_handlePrimaryFocusChanged);
    previousFocus = null;
    previousEditableFocus = null;
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.child;
}

class _WindowShortcuts extends StatelessWidget {
  const _WindowShortcuts({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => FocusableActionDetector(
    shortcuts: const {
      SingleActivator(LogicalKeyboardKey.keyW, meta: true):
          _CloseWindowIntent(),
    },
    actions: {
      _CloseWindowIntent: CallbackAction<_CloseWindowIntent>(
        onInvoke: (_) {
          if (_isDesktopPlatform) windowManager.hide();
          return null;
        },
      ),
    },
    child: child,
  );
}

class _CloseWindowIntent extends Intent {
  const _CloseWindowIntent();
}

class _WindowsTitleBarDivider extends StatelessWidget {
  const _WindowsTitleBarDivider({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => Column(
    children: [
      if (defaultTargetPlatform == TargetPlatform.windows &&
          Theme.of(context).brightness == Brightness.light)
        Divider(height: 1, thickness: 1, color: context.mixinTheme.divider),
      Expanded(child: child),
    ],
  );
}

bool get _isDesktopPlatform =>
    !kIsWeb &&
    const {
      TargetPlatform.linux,
      TargetPlatform.macOS,
      TargetPlatform.windows,
    }.contains(defaultTargetPlatform);

class _AppBody extends StatelessWidget {
  const _AppBody({this.initialProtocolUrl});

  final String? initialProtocolUrl;

  @override
  Widget build(BuildContext context) {
    final controller = context.watch<AppController>();
    final body = switch (controller.stage) {
      AppStage.starting => const _StartupPage(),
      AppStage.signedOut => LoginPage(
        desktop: controller.desktop,
        onAuthenticated: controller.setAccount,
        onFailure: controller.setLoginFailure,
      ),
      AppStage.signedIn => _SignedInBody(
        desktop: controller.desktop,
        account: controller.account,
        initialProtocolUrl: initialProtocolUrl,
      ),
      AppStage.failed =>
        controller.databaseOpenFailure != null
            ? _DatabaseOpenFailedPage(
                failure: controller.databaseOpenFailure!,
                onRecreate: controller.recreateAccountDatabase,
              )
            : controller.showLoginFailure
            ? _LoginFailedPage(
                error: controller.error ?? '',
                stackTrace: controller.errorStackTrace ?? '',
                onRetry: controller.abortFailedLogin,
              )
            : _LandingFailedPage(
                title: context.l10n.unknowError,
                message: controller.error ?? 'Unable to start Mixin Messenger.',
                actions: [
                  ElevatedButton(
                    style: ElevatedButton.styleFrom(
                      backgroundColor: context.mixinTheme.accent,
                      foregroundColor: Colors.white,
                    ),
                    onPressed: controller.initialize,
                    child: Text(context.l10n.retry),
                  ),
                ],
              ),
    };
    if (defaultTargetPlatform == TargetPlatform.macOS &&
        controller.stage != AppStage.signedIn) {
      return _SignedOutMacMenuBar(child: body);
    }
    return body;
  }
}

class _SignedOutMacMenuBar extends StatelessWidget {
  const _SignedOutMacMenuBar({required this.child});

  static const _platformMenusChannel = MethodChannel(
    'mixin_desktop/platform_menus',
  );

  final Widget child;

  @override
  Widget build(BuildContext context) => PlatformMenuBar(
    menus: [
      PlatformMenu(
        label: 'Mixin',
        menus: [
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: '${context.l10n.about} Mixin',
                onSelected: () =>
                    _platformMenusChannel.invokeMethod<void>('showAbout'),
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: context.l10n.preferences,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.comma,
                  meta: true,
                ),
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: context.l10n.lock,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.keyL,
                  meta: true,
                  shift: true,
                ),
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: context.l10n.quickSearch,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.keyK,
                  meta: true,
                ),
              ),
              PlatformMenuItem(
                label: context.l10n.hideMixin,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.keyH,
                  meta: true,
                ),
                onSelected: windowManager.hide,
              ),
              PlatformMenuItem(
                label: context.l10n.showMixin,
                onSelected: windowManager.show,
              ),
            ],
          ),
          PlatformMenuItem(
            label: context.l10n.quitMixin,
            shortcut: const SingleActivator(
              LogicalKeyboardKey.keyQ,
              meta: true,
            ),
            onSelected: () => exit(0),
          ),
        ],
      ),
      PlatformMenu(
        label: context.l10n.file,
        menus: [
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: context.l10n.createConversation,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.keyN,
                  meta: true,
                ),
              ),
              PlatformMenuItem(
                label: context.l10n.createGroup,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.keyN,
                  shift: true,
                  meta: true,
                ),
              ),
              if (kDebugMode)
                const PlatformMenuItemGroup(
                  members: [
                    PlatformMenuItem(label: 'chat backup and restore'),
                  ],
                ),
              PlatformMenuItem(label: context.l10n.createCircle),
              PlatformMenuItemGroup(
                members: [
                  PlatformMenuItem(
                    label: context.l10n.closeWindow,
                    onSelected: windowManager.close,
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
      PlatformMenu(
        label: context.l10n.conversation,
        menus: [
          PlatformMenuItem(label: context.l10n.mute),
          PlatformMenuItem(label: context.l10n.search),
          PlatformMenuItem(label: context.l10n.deleteChat),
          PlatformMenuItem(label: context.l10n.pinTitle),
          PlatformMenuItem(label: context.l10n.toggleChatInfo),
        ],
      ),
      PlatformMenu(
        label: context.l10n.window,
        menus: [
          PlatformMenuItem(
            label: context.l10n.minimize,
            shortcut: const SingleActivator(
              LogicalKeyboardKey.keyM,
              meta: true,
            ),
            onSelected: windowManager.minimize,
          ),
          PlatformMenuItem(
            label: context.l10n.zoom,
            onSelected: () async => await windowManager.isMaximized()
                ? windowManager.restore()
                : windowManager.maximize(),
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: context.l10n.previousConversation,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.arrowUp,
                  meta: true,
                ),
              ),
              PlatformMenuItem(
                label: context.l10n.nextConversation,
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.arrowDown,
                  meta: true,
                ),
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'Mixin',
                shortcut: const SingleActivator(
                  LogicalKeyboardKey.keyO,
                  meta: true,
                ),
                onSelected: windowManager.show,
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: context.l10n.bringAllToFront,
                onSelected: windowManager.show,
              ),
            ],
          ),
          PlatformMenuItem(label: 'Mixin', onSelected: windowManager.show),
        ],
      ),
      PlatformMenu(
        label: context.l10n.help,
        menus: [
          PlatformMenuItem(
            label: context.l10n.helpCenter,
            onSelected: () =>
                unawaited(launchUrl(Uri.parse('https://support.mixin.one/'))),
          ),
          PlatformMenuItem(
            label: context.l10n.termsOfService,
            onSelected: () => unawaited(
              launchUrl(Uri.parse('https://mixin.one/pages/terms')),
            ),
          ),
          PlatformMenuItem(
            label: context.l10n.privacyPolicy,
            onSelected: () => unawaited(
              launchUrl(Uri.parse('https://mixin.one/pages/privacy')),
            ),
          ),
        ],
      ),
    ],
    child: child,
  );
}

class _SignedInBody extends StatefulWidget {
  const _SignedInBody({
    required this.desktop,
    required this.account,
    this.initialProtocolUrl,
  });

  final DesktopHandle desktop;
  final AccountHandle account;
  final String? initialProtocolUrl;

  @override
  State<_SignedInBody> createState() => _SignedInBodyState();
}

class _SignedInBodyState extends State<_SignedInBody> {
  late final NetworkController _networkController = NetworkController(
    widget.desktop,
  )..initialize();
  late final SecurityController _securityController = SecurityController(
    widget.account.profile().userId,
  );
  late final Future<void> _securityInitialized = _securityController
      .initialize();

  @override
  void dispose() {
    _networkController.dispose();
    _securityController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => FutureBuilder<void>(
    future: _securityInitialized,
    builder: (context, snapshot) {
      if (snapshot.connectionState != ConnectionState.done) {
        return const _StartupPage();
      }
      return Provider<AccountHandle>.value(
        value: widget.account,
        child: ChangeNotifierProvider<SecurityController>.value(
          value: _securityController,
          child: ChangeNotifierProvider<NetworkController>.value(
            value: _networkController,
            child: Consumer<NetworkController>(
              builder: (context, network, child) => CoreHttpScope(
                client: network.httpClient,
                revision: network.revision,
                child: child!,
              ),
              child: AuthGuard(
                child: HomePage(initialProtocolUrl: widget.initialProtocolUrl),
              ),
            ),
          ),
        ),
      );
    },
  );
}

class _StartupPage extends StatelessWidget {
  const _StartupPage();

  @override
  Widget build(BuildContext context) => LandingScaffold(
    child: QrLoginCard(
      authUrl: null,
      loading: true,
      provisioning: false,
      onRetry: () {},
    ),
  );
}

class _LoginFailedPage extends StatelessWidget {
  const _LoginFailedPage({
    required this.error,
    required this.stackTrace,
    required this.onRetry,
  });

  final String error;
  final String stackTrace;
  final AsyncCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final errorText = 'Error: $error';
    final stackTraceText = 'StackTrace: $stackTrace';
    return LandingScaffold(
      child: Padding(
        padding: const EdgeInsets.only(
          top: 56,
          bottom: 30,
          right: 48,
          left: 48,
        ),
        child: Column(
          children: [
            Text(
              context.l10n.unknowError,
              textAlign: TextAlign.center,
              style: TextStyle(
                color: context.mixinTheme.red,
                fontSize: 18,
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 16),
            Expanded(
              child: DefaultTextStyle(
                style: TextStyle(
                  color: context.mixinTheme.text,
                  fontSize: 16,
                  fontWeight: FontWeight.w400,
                ),
                child: SelectionArea(
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      color: context.mixinTheme.sidebarSelected,
                      borderRadius: const BorderRadius.all(Radius.circular(8)),
                    ),
                    child: SingleChildScrollView(
                      child: Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 20),
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            const SizedBox(height: 20),
                            Text(errorText),
                            Text(stackTraceText),
                            const SizedBox(height: 20),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
            const SizedBox(height: 42),
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                MixinButton(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 56,
                    vertical: 14,
                  ),
                  backgroundTransparent: true,
                  onTap: () async {
                    await Clipboard.setData(
                      ClipboardData(text: '$errorText\n$stackTraceText'),
                    );
                    showToastSuccessful();
                  },
                  child: Text(context.l10n.copy),
                ),
                MixinButton(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 56,
                    vertical: 14,
                  ),
                  onTap: onRetry,
                  child: Text(context.l10n.retry),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _LandingFailedPage extends StatelessWidget {
  const _LandingFailedPage({
    required this.title,
    required this.message,
    required this.actions,
  });

  final String title;
  final String message;
  final List<Widget> actions;

  @override
  Widget build(BuildContext context) => LandingScaffold(
    child: Column(
      children: [
        const SizedBox(height: 32),
        const Spacer(),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: Text(
            title,
            style: TextStyle(
              color: context.mixinTheme.text,
              fontSize: 20,
              fontWeight: FontWeight.bold,
            ),
          ),
        ),
        const SizedBox(height: 32),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 30),
          child: Text(
            message,
            style: TextStyle(color: context.mixinTheme.text, fontSize: 14),
          ),
        ),
        const Spacer(),
        ...actions,
        const SizedBox(height: 32),
      ],
    ),
  );
}

class _DatabaseOpenFailedPage extends StatelessWidget {
  const _DatabaseOpenFailedPage({
    required this.failure,
    required this.onRecreate,
  });

  static const _sqliteCorrupt = 11;
  static const _sqliteLocked = 6;
  static const _sqliteNotADb = 26;
  static const _sqliteIOErr = 10;

  final DatabaseOpenFailure failure;
  final AsyncCallback onRecreate;

  @override
  Widget build(BuildContext context) {
    final message = switch (failure.resultCode) {
      _sqliteCorrupt => context.l10n.databaseCorruptedTips,
      _sqliteLocked => context.l10n.databaseLockedTips,
      _sqliteNotADb => context.l10n.databaseNotADbTips,
      _ => failure.explanation,
    };
    final canDeleteDatabase = const {
      _sqliteCorrupt,
      _sqliteNotADb,
      _sqliteIOErr,
    }.contains(failure.resultCode);
    return _LandingFailedPage(
      title: context.l10n.failedToOpenDatabase,
      message: message,
      actions: [
        if (canDeleteDatabase)
          Padding(
            padding: const EdgeInsets.only(bottom: 16),
            child: TextButton(
              onPressed: () async {
                final result = await showConfirmMixinDialog(
                  context,
                  context.l10n.databaseRecreateTips,
                  positiveText: context.l10n.create,
                );
                if (result == DialogEvent.positive) {
                  await onRecreate();
                }
              },
              child: Text(
                context.l10n.continueText,
                style: TextStyle(color: context.mixinTheme.red),
              ),
            ),
          ),
        ElevatedButton(
          style: ElevatedButton.styleFrom(
            backgroundColor: context.mixinTheme.accent,
            foregroundColor: Colors.white,
          ),
          onPressed: () => exit(1),
          child: Text(context.l10n.exit),
        ),
      ],
    );
  }
}
