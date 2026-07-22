import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_portal/flutter_portal.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/conversation_list_view.dart';
import 'package:mixin_desktop_ui/widgets/qr_login_card.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';

void main() {
  testWidgets('shows the QR login instructions', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: QrLoginCard(
            authUrl: 'mixin://device/auth?id=test&pub_key=test',
            loading: false,
            provisioning: false,
            onRetry: () {},
          ),
        ),
      ),
    );

    expect(
      find.text('Log in to Mixin Messenger with a QR code'),
      findsOneWidget,
    );
  });

  testWidgets('reports conversation search and selection actions', (
    tester,
  ) async {
    ConversationListEntry? selected;
    var query = '';
    final conversation = ConversationListEntry(
      id: 'conversation',
      ownerId: 'owner',
      name: 'Mixin Team',
      avatarUrl: '',
      category: 'CONTACT',
      draft: '',
      status: 2,
      content: 'Welcome to Mixin',
      contentType: 'PLAIN_TEXT',
      messageStatus: 'DELIVERED',
      senderId: 'owner',
      senderName: 'Mixin Team',
      updatedAt: DateTime(2026, 7, 16, 12),
      unseenCount: 2,
      mentionCount: 0,
      isMuted: false,
      isVerified: true,
      isBot: false,
      isPinned: false,
      relationship: 'FRIEND',
      identityNumber: '7000',
      circleIds: const [],
      groupAvatars: const [],
    );

    await tester.pumpWidget(
      _LocalizedApp(
        child: ConversationListView(
          conversations: [conversation],
          initialized: true,
          itemPositionsListener: ItemPositionsListener.create(),
          itemScrollController: ItemScrollController(),
          loading: false,
          currentUserId: 'me',
          circles: const {},
          currentCircleId: null,
          filterUnseen: false,
          selectedConversationId: null,
          onQueryChanged: (value) => query = value,
          onToggleUnseen: () {},
          onCreateActionSelected: (_) {},
          onSelected: (value) => selected = value,
          onPinned: (_) {},
          onMuted: (_, _) {},
          onDeleted: (_) {},
          onCircleChanged: (_, _, _) {},
        ),
      ),
    );

    expect(find.text('Mixin Team', findRichText: true), findsOneWidget);
    expect(find.text('2'), findsOneWidget);
    await tester.enterText(find.byType(TextField), 'Mixin');
    expect(query, 'Mixin');
    await tester.tap(find.text('Mixin Team', findRichText: true));
    expect(selected, conversation);
  });
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
    home: Portal(child: child),
  );
}
