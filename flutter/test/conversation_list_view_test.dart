import 'package:flutter/material.dart';
import 'package:flutter_portal/flutter_portal.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/conversation_list_view.dart';
import 'package:mixin_desktop_ui/widgets/mixin_image.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';

void main() {
  testWidgets(
    'shows default avatars while contact and group avatar images load',
    (
      tester,
    ) async {
      await tester.pumpWidget(
        const Directionality(
          textDirection: TextDirection.ltr,
          child: Column(
            children: [
              AvatarView(
                userId: 'alice',
                name: 'Alice',
                avatarUrl: 'https://images.mixin.one/alice.png',
                size: 48,
              ),
              AvatarPuzzlesView(
                avatars: [
                  ConversationAvatarEntry(
                    userId: 'bob',
                    name: 'Bob',
                    avatarUrl: 'https://images.mixin.one/bob.png',
                  ),
                ],
                size: 48,
              ),
            ],
          ),
        ),
      );

      final avatars = tester.widgetList<MixinImage>(find.byType(MixinImage));
      expect(avatars, hasLength(2));
      expect(avatars.every((avatar) => avatar.placeholder != null), isTrue);
      expect(find.text('A'), findsOneWidget);
      expect(find.text('B'), findsOneWidget);
    },
  );

  testWidgets('keeps the source item, avatar, badge and selected dimensions', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(320, 200));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    var tapped = false;
    final conversation = _conversation(
      name: 'Mixin Team',
      draft: 'Ship the desktop client',
      unseenCount: 12,
      mentionCount: 1,
      isVerified: true,
      isBot: true,
    );

    await tester.pumpWidget(
      _TestApp(
        child: ConversationItem(
          conversation: conversation,
          currentUserId: 'current-user',
          selected: true,
          onTap: () => tapped = true,
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(tester.getSize(find.byType(ConversationItem)), const Size(320, 78));
    expect(tester.getSize(find.byType(AvatarView)), const Size.square(50));
    expect(
      find.byWidgetPredicate(
        (widget) =>
            widget is DecoratedBox &&
            widget.decoration is BoxDecoration &&
            (widget.decoration as BoxDecoration).color ==
                lightMixinColors.listSelected,
      ),
      findsOneWidget,
    );
    expect(find.text('Draft:'), findsOneWidget);
    final mentionBadgeSize = _badgeSize(tester, '@');
    final unseenBadgeSize = _badgeSize(tester, '12');
    expect(mentionBadgeSize, const Size(26, 20));
    expect(unseenBadgeSize.height, 20);
    expect(unseenBadgeSize.width, greaterThanOrEqualTo(26));
    expect(_svgAsset('assets/images/verified.svg'), findsOneWidget);
    expect(_svgAsset('assets/images/bot_fill.svg'), findsNothing);

    await tester.tap(find.text('Mixin Team', findRichText: true));
    expect(tapped, isTrue);
  });

  testWidgets('uses the original three-person group avatar puzzle', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(320, 200));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final conversation = _conversation(
      category: 'GROUP',
      groupAvatars: const [
        ConversationAvatarEntry(
          userId: 'ea91421a-98bb-41d2-abcf-af013d8b874b',
          name: 'Alice',
          avatarUrl: '',
        ),
        ConversationAvatarEntry(
          userId: '0364f490-49cc-4988-88c2-481707687e5b',
          name: 'Bob',
          avatarUrl: '',
        ),
        ConversationAvatarEntry(
          userId: '8df4972f-702f-4ae9-bc76-68e489351357',
          name: 'Carol',
          avatarUrl: '',
        ),
      ],
    );

    await tester.pumpWidget(
      _TestApp(
        child: ConversationItem(
          conversation: conversation,
          currentUserId: 'current-user',
          selected: false,
          onTap: () {},
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byType(AvatarView), findsNWidgets(3));
    expect(tester.getSize(find.byType(AvatarView).at(0)), const Size(25, 50));
    expect(
      tester.getSize(find.byType(AvatarView).at(1)),
      const Size.square(25),
    );
    expect(
      tester.getSize(find.byType(AvatarView).at(2)),
      const Size.square(25),
    );
  });

  testWidgets('opens the responsive drawer from the search bar', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(360, 720));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    await tester.pumpWidget(
      MaterialApp(
        theme: buildMixinTheme(Brightness.light),
        locale: const Locale('en'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Portal(
          child: Scaffold(
            drawer: const Drawer(child: Text('Navigation')),
            body: ConversationListView(
              conversations: const [],
              initialized: true,
              itemPositionsListener: ItemPositionsListener.create(),
              itemScrollController: ItemScrollController(),
              loading: false,
              currentUserId: 'current-user',
              circles: const {},
              currentCircleId: null,
              filterUnseen: false,
              selectedConversationId: null,
              onQueryChanged: (_) {},
              onToggleUnseen: () {},
              onCreateActionSelected: (_) {},
              onSelected: (_) {},
              onPinned: (_) {},
              onMuted: (_, _) {},
              onDeleted: (_) {},
              onCircleChanged: (_, _, _) {},
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('open-drawer')));
    await tester.pumpAndSettle();

    expect(find.text('Navigation'), findsOneWidget);
    expect(tester.getTopLeft(find.text('Navigation')).dx, 0);
  });
}

Size _badgeSize(WidgetTester tester, String text) => tester.getSize(
  find.ancestor(of: find.text(text), matching: find.byType(Container)).first,
);

Finder _svgAsset(String assetName) => find.byWidgetPredicate(
  (widget) =>
      widget is SvgPicture &&
      widget.bytesLoader is SvgAssetLoader &&
      (widget.bytesLoader as SvgAssetLoader).assetName == assetName,
);

ConversationListEntry _conversation({
  String name = 'Conversation',
  String category = 'CONTACT',
  String draft = '',
  int unseenCount = 0,
  int mentionCount = 0,
  bool isVerified = false,
  bool isBot = false,
  List<ConversationAvatarEntry> groupAvatars = const [],
}) => ConversationListEntry(
  id: 'conversation-id',
  ownerId: 'ea91421a-98bb-41d2-abcf-af013d8b874b',
  name: name,
  avatarUrl: '',
  category: category,
  draft: draft,
  status: 2,
  content: 'Latest message',
  contentType: 'PLAIN_TEXT',
  messageStatus: 'DELIVERED',
  senderId: 'sender-id',
  senderName: 'Sender',
  updatedAt: DateTime(2026, 7, 16, 12),
  unseenCount: unseenCount,
  mentionCount: mentionCount,
  isMuted: false,
  isVerified: isVerified,
  isBot: isBot,
  isPinned: false,
  relationship: 'FRIEND',
  identityNumber: '7000',
  circleIds: const [],
  groupAvatars: groupAvatars,
);

class _TestApp extends StatelessWidget {
  const _TestApp({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => MaterialApp(
    theme: buildMixinTheme(Brightness.light),
    locale: const Locale('en'),
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Portal(
      child: Scaffold(
        body: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(width: 320, child: child),
        ),
      ),
    ),
  );
}
