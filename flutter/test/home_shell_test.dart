import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/app_controller.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/pages/home_page.dart';
import 'package:mixin_desktop_ui/pages/chat_side/chat_info_page.dart';
import 'package:mixin_desktop_ui/pages/settings_page.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/chat_view.dart';
import 'package:mixin_desktop_ui/widgets/conversation_list_view.dart';
import 'package:mixin_desktop_ui/widgets/home_sidebar.dart';
import 'package:provider/provider.dart';

void main() {
  test('resolves the original desktop sidebar breakpoints', () {
    expect(
      DesktopShellLayout.resolve(
        maxWidth: 384,
        userCollapse: false,
        isPhone: false,
      ).mode,
      DesktopShellLayoutMode.drawer,
    );

    final compact = DesktopShellLayout.resolve(
      maxWidth: 385,
      userCollapse: false,
      isPhone: false,
    );
    expect(compact.mode, DesktopShellLayoutMode.compactRail);
    expect(compact.slideWidth, kSlidePageMinWidth);
    expect(compact.showCollapseControl, isFalse);

    final full = DesktopShellLayout.resolve(
      maxWidth: 496,
      userCollapse: false,
      isPhone: false,
    );
    expect(full.mode, DesktopShellLayoutMode.fullRail);
    expect(full.slideWidth, kSlidePageMaxWidth);
    expect(full.showCollapseControl, isTrue);

    final userCollapsed = DesktopShellLayout.resolve(
      maxWidth: 800,
      userCollapse: true,
      isPhone: false,
    );
    expect(userCollapsed.mode, DesktopShellLayoutMode.compactRail);
    expect(userCollapsed.showCollapseControl, isTrue);
  });

  testWidgets('splits the conversation and chat panes at 620 content pixels', (
    tester,
  ) async {
    await _pumpHome(tester, size: const Size(795, 700));

    expect(tester.getSize(find.byType(HomeSidebar)).width, 176);
    expect(tester.getSize(find.byType(ConversationListView)).width, 619);
    expect(
      find.text('Select a conversation and start sending a message'),
      findsNothing,
    );

    tester.view.physicalSize = const Size(796, 700);
    await tester.pumpAndSettle();

    expect(tester.getSize(find.byType(ConversationListView)).width, 300);
    expect(
      find.text('Select a conversation and start sending a message'),
      findsOneWidget,
    );

    await tester.tap(find.text('Collapse'));
    await tester.pumpAndSettle();

    expect(tester.getSize(find.byType(HomeSidebar)).width, 64);
    expect(tester.getSize(find.byType(ConversationListView)).width, 300);

    tester.view.physicalSize = const Size(384, 700);
    await tester.pumpAndSettle();

    final scaffold = tester.state<ScaffoldState>(find.byType(Scaffold));
    expect(find.byKey(const ValueKey('open-drawer')), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('open-drawer')));
    await tester.pumpAndSettle();
    expect(scaffold.isDrawerOpen, isTrue);

    await tester.tap(find.text('Contacts'));
    await tester.pumpAndSettle();
    expect(scaffold.isDrawerOpen, isFalse);
  });

  testWidgets('opens ChatView alongside the wide conversation list', (
    tester,
  ) async {
    await _pumpHome(tester, size: const Size(1000, 700));

    expect(find.byType(ConversationListView), findsOneWidget);
    expect(find.byType(ChatView), findsNothing);

    await tester.tap(find.text('Mixin Team'));
    await tester.pumpAndSettle();

    expect(find.byType(ConversationListView), findsOneWidget);
    expect(find.byType(ChatView), findsOneWidget);
    expect(tester.widget<ChatView>(find.byType(ChatView)).onBack, isNull);
    expect(find.byKey(const Key('chat-header')), findsOneWidget);
  });

  testWidgets('opens and closes the original chat side info route', (
    tester,
  ) async {
    await _pumpHome(tester, size: const Size(1300, 700));

    await tester.tap(find.text('Mixin Team'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('chat-info')));
    await tester.pumpAndSettle();

    expect(find.byType(ChatInfoPage), findsOneWidget);
    expect(find.byKey(const Key('chat-side-close')), findsOneWidget);
    expect(tester.getSize(find.byType(ChatInfoPage)).width, 300);

    await tester.tap(find.byKey(const Key('chat-side-close')));
    await tester.pumpAndSettle();
    expect(find.byType(ChatInfoPage), findsNothing);
    expect(find.byType(ChatView), findsOneWidget);
  });

  testWidgets('opens ChatView in narrow mode and returns to conversations', (
    tester,
  ) async {
    await _pumpHome(tester, size: const Size(600, 700));

    await tester.tap(find.text('Mixin Team'));
    await tester.pumpAndSettle();

    expect(find.byType(ConversationListView), findsNothing);
    expect(find.byType(ChatView), findsOneWidget);
    expect(tester.widget<ChatView>(find.byType(ChatView)).onBack, isNotNull);

    final back = find.byKey(const Key('chat-back'));
    expect(back, findsOneWidget);
    await tester.tap(back);
    await tester.pumpAndSettle();

    expect(find.byType(ChatView), findsNothing);
    expect(find.byType(ConversationListView), findsOneWidget);
    expect(find.text('Mixin Team'), findsOneWidget);
  });

  testWidgets('keeps per-conversation drafts across responsive remounts', (
    tester,
  ) async {
    final account = _FakeAccountHandle(
      conversations: const [
        _FakeAccountHandle._conversation,
        _FakeAccountHandle._secondConversation,
      ],
    );
    await _pumpHome(tester, size: const Size(1000, 700), account: account);

    await tester.tap(
      find.descendant(
        of: find.byType(ConversationListView),
        matching: find.text('Mixin Team'),
      ),
    );
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('chat-input')),
      'First local draft',
    );

    tester.view.physicalSize = const Size(600, 700);
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<TextField>(find.byKey(const Key('chat-input')))
          .controller!
          .text,
      'First local draft',
    );

    await tester.tap(find.byKey(const Key('chat-back')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Second Chat'));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<TextField>(find.byKey(const Key('chat-input')))
          .controller!
          .text,
      'Second saved draft',
    );
    await tester.enterText(
      find.byKey(const Key('chat-input')),
      'Second local draft',
    );

    await tester.tap(find.byKey(const Key('chat-back')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Mixin Team'));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<TextField>(find.byKey(const Key('chat-input')))
          .controller!
          .text,
      'First local draft',
    );

    await tester.tap(find.byKey(const Key('chat-back')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Second Chat'));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<TextField>(find.byKey(const Key('chat-input')))
          .controller!
          .text,
      'Second local draft',
    );

    await tester.tap(find.byKey(const Key('chat-send')));
    await tester.pumpAndSettle();
    expect(account.sentTexts, ['Second local draft']);
    expect(
      tester
          .widget<TextField>(find.byKey(const Key('chat-input')))
          .controller!
          .text,
      isEmpty,
    );

    await tester.tap(find.byKey(const Key('chat-back')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Second Chat'));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<TextField>(find.byKey(const Key('chat-input')))
          .controller!
          .text,
      isEmpty,
    );
  });

  testWidgets('opens SettingsPage from the profile without using FFI', (
    tester,
  ) async {
    final appController = _FakeAppController();
    await _pumpHome(
      tester,
      size: const Size(1000, 700),
      appController: appController,
    );

    await tester.tap(find.text('Mixin User'));
    await tester.pumpAndSettle();

    expect(find.byType(SettingsPage), findsOneWidget);
    expect(find.byType(ConversationListView), findsNothing);
    expect(find.byType(ChatView), findsNothing);
    expect(appController.signOutCalls, 0);

    await tester.tap(find.byKey(const ValueKey('settings-close')));
    await tester.pumpAndSettle();

    expect(find.byType(SettingsPage), findsNothing);
    expect(find.byType(ConversationListView), findsOneWidget);
  });

  testWidgets('restores the search query when the conversation list remounts', (
    tester,
  ) async {
    final account = _FakeAccountHandle();
    await _pumpHome(tester, size: const Size(1000, 700), account: account);

    await tester.enterText(find.byType(TextField), 'Mixin');
    await tester.pumpAndSettle();
    expect(account.conversationKeywords, contains('Mixin'));

    await tester.tap(
      find.descendant(
        of: find.byType(HomeSidebar),
        matching: find.text('Mixin User'),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(ConversationListView), findsNothing);

    await tester.tap(find.byKey(const ValueKey('settings-close')));
    await tester.pumpAndSettle();

    final textField = tester.widget<TextField>(find.byType(TextField));
    expect(textField.controller?.text, 'Mixin');
    expect(find.text('Mixin Team'), findsOneWidget);
  });

  testWidgets('shows mutation failures and clears selection after delete', (
    tester,
  ) async {
    final account = _FakeAccountHandle()..mutationError = StateError('failed');
    await _pumpHome(tester, size: const Size(1000, 700), account: account);

    await tester.tap(find.text('Mixin Team'));
    await tester.pumpAndSettle();
    expect(find.byType(ChatView), findsOneWidget);

    final conversation = tester
        .widget<ConversationItem>(find.byType(ConversationItem))
        .conversation;
    final conversationList = tester.widget<ConversationListView>(
      find.byType(ConversationListView),
    );

    Future<void> expectFailure(VoidCallback action) async {
      action();
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 250));
      expect(find.text('Failed'), findsOneWidget);
      ScaffoldMessenger.of(
        tester.element(find.byType(HomePage)),
      ).hideCurrentSnackBar();
      await tester.pumpAndSettle();
    }

    await expectFailure(() => conversationList.onPinned(conversation));
    await expectFailure(() => conversationList.onMuted(conversation, 0));
    await expectFailure(
      () => conversationList.onCircleChanged(conversation, 'circle', true),
    );
    await expectFailure(() => conversationList.onDeleted(conversation));
    expect(find.byType(ChatView), findsOneWidget);

    account
      ..mutationError = null
      ..deleteCompleter = Completer<void>();
    conversationList.onDeleted(conversation);
    await tester.pump();
    expect(find.byType(ChatView), findsOneWidget);

    account.deleteCompleter!.complete();
    await tester.pumpAndSettle();
    expect(find.byType(ChatView), findsNothing);
    expect(
      find.text('Select a conversation and start sending a message'),
      findsOneWidget,
    );
  });
}

Future<void> _pumpHome(
  WidgetTester tester, {
  required Size size,
  _FakeAccountHandle? account,
  _FakeAppController? appController,
}) async {
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = size;

  await tester.pumpWidget(
    _LocalizedApp(
      child: MultiProvider(
        providers: [
          Provider<AccountHandle>.value(value: account ?? _FakeAccountHandle()),
          ChangeNotifierProvider<AppController>.value(
            value: appController ?? _FakeAppController(),
          ),
        ],
        child: const HomePage(),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

class _LocalizedApp extends StatelessWidget {
  const _LocalizedApp({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => MaterialApp(
    theme: buildMixinTheme(Brightness.light),
    localizationsDelegates: const [
      AppLocalizations.delegate,
      GlobalMaterialLocalizations.delegate,
      GlobalWidgetsLocalizations.delegate,
      GlobalCupertinoLocalizations.delegate,
    ],
    supportedLocales: AppLocalizations.supportedLocales,
    home: child,
  );
}

class _FakeAccountHandle implements AccountHandle {
  _FakeAccountHandle({List<ConversationListItem>? conversations})
    : _conversationItems = conversations ?? const [_conversation];

  static const _conversation = ConversationListItem(
    conversationId: 'conversation',
    ownerId: 'owner',
    name: 'Mixin Team',
    avatarUrl: '',
    category: 'CONTACT',
    draft: '',
    status: 2,
    lastMessage: 'Welcome to Mixin',
    lastMessageCategory: 'PLAIN_TEXT',
    lastMessageStatus: 'DELIVERED',
    lastMessageSenderId: 'owner',
    lastMessageSenderName: 'Mixin Team',
    updatedAtMillis: 0,
    unseenCount: 0,
    mentionCount: 0,
    isMuted: false,
    isVerified: true,
    isBot: false,
    isPinned: false,
    relationship: 'FRIEND',
    identityNumber: '7000',
    circleIds: [],
    participantCount: 0,
    groupAvatars: [],
  );

  static const _secondConversation = ConversationListItem(
    conversationId: 'second-conversation',
    ownerId: 'second-owner',
    name: 'Second Chat',
    avatarUrl: '',
    category: 'CONTACT',
    draft: 'Second saved draft',
    status: 2,
    lastMessage: 'Second welcome',
    lastMessageCategory: 'PLAIN_TEXT',
    lastMessageStatus: 'DELIVERED',
    lastMessageSenderId: 'second-owner',
    lastMessageSenderName: 'Second Chat',
    updatedAtMillis: 0,
    unseenCount: 0,
    mentionCount: 0,
    isMuted: false,
    isVerified: false,
    isBot: false,
    isPinned: false,
    relationship: 'FRIEND',
    identityNumber: '7001',
    circleIds: [],
    participantCount: 0,
    groupAvatars: [],
  );

  final List<ConversationListItem> _conversationItems;
  final conversationKeywords = <String>[];
  final sentTexts = <String>[];
  Object? mutationError;
  Completer<void>? deleteCompleter;

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);

  @override
  String accountId() => 'account';

  @override
  Future<List<CircleItem>> circles() async => const [];

  @override
  Stream<BigInt> conversationChanges() => const Stream.empty();

  @override
  Future<PlatformInt64> conversationCount({
    required String category,
    String? circleId,
    required String keyword,
    required bool unseenOnly,
  }) async {
    conversationKeywords.add(keyword);
    return category == 'chats' && !unseenOnly ? _conversationItems.length : 0;
  }

  @override
  Future<List<ConversationListItem>> conversations({
    required String category,
    String? circleId,
    required String keyword,
    required bool unseenOnly,
    required PlatformInt64 limit,
    required PlatformInt64 offset,
  }) async {
    conversationKeywords.add(keyword);
    return category == 'chats' && !unseenOnly ? _conversationItems : const [];
  }

  @override
  Future<void> deleteConversation({required String conversationId}) async {
    if (mutationError case final error?) throw error;
    await deleteCompleter?.future;
  }

  @override
  Future<void> editCircleConversation({
    required String circleId,
    required String conversationId,
    required String ownerId,
    required bool isGroup,
    required bool add,
  }) async {
    if (mutationError case final error?) throw error;
  }

  @override
  Future<void> markConversationRead({required String conversationId}) async {}

  @override
  Stream<BigInt> messageChanges() => const Stream.empty();

  @override
  Future<List<MessageListItem>> messages({
    required String conversationId,
    int? beforeCreatedAtMicros,
    String? beforeMessageId,
    required PlatformInt64 limit,
  }) async => const [];

  @override
  Future<ConversationDetailItem> conversationDetail({
    required String conversationId,
  }) async => ConversationDetailItem(
    conversationId: conversationId,
    name: conversationId == _secondConversation.conversationId
        ? _secondConversation.name
        : _conversation.name,
    announcement: '',
    codeUrl: '',
    createdAtMillis: 0,
    muteUntilMillis: 0,
    expireIn: 0,
  );

  @override
  Future<List<MessageListItem>> pinnedMessages({
    required String conversationId,
  }) async => const [];

  @override
  Future<List<SharedAppItem>> sharedApps({required String userId}) async =>
      const [];

  @override
  Future<String?> botCreatorId({required String userId}) async => null;

  @override
  Future<UserProfileItem?> userProfile({
    String? userId,
    String? identityNumber,
  }) async => UserProfileItem(
    userId: userId ?? 'owner',
    identityNumber: identityNumber ?? '7000',
    fullName: 'Mixin Team',
    avatarUrl: '',
    biography: '',
    isVerified: true,
    isBot: false,
    relationship: 'FRIEND',
    codeUrl: '',
  );

  @override
  AccountProfile profile() => const AccountProfile(
    userId: 'user',
    fullName: 'Mixin User',
    avatarUrl: '',
    identityNumber: '7000',
    biography: '',
    phone: '',
    createdAt: '',
  );

  @override
  Future<String> sendText({
    required String conversationId,
    required String content,
    String? quoteMessageId,
  }) async {
    sentTexts.add(content);
    return 'message';
  }

  @override
  Future<void> setConversationMuted({
    required String conversationId,
    required String ownerId,
    required String category,
    required PlatformInt64 durationSeconds,
  }) async {
    if (mutationError case final error?) throw error;
  }

  @override
  Future<void> setConversationPinned({
    required String conversationId,
    required bool pinned,
  }) async {
    if (mutationError case final error?) throw error;
  }

  @override
  Future<void> shutdown() async {}

  @override
  Future<void> signOut() async {}

  @override
  void dispose() {}

  @override
  bool get isDisposed => false;
}

class _FakeAppController extends AppController {
  var signOutCalls = 0;

  @override
  Future<void> signOut() async {
    signOutCalls++;
  }
}
