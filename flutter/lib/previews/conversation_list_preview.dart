import 'package:flutter/material.dart';
import 'package:flutter/widget_previews.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:mixin_desktop_ui/l10n/generated/app_localizations.dart';
import 'package:mixin_desktop_ui/controllers/paging_controller.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/conversation_list_view.dart';
import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';

@Preview(name: 'Conversation List', group: 'Home', size: Size(300, 720))
Widget conversationListPreview() => MaterialApp(
  theme: buildMixinTheme(Brightness.light),
  localizationsDelegates: const [
    AppLocalizations.delegate,
    GlobalMaterialLocalizations.delegate,
    GlobalWidgetsLocalizations.delegate,
    GlobalCupertinoLocalizations.delegate,
  ],
  supportedLocales: AppLocalizations.supportedLocales,
  home: ConversationListView(
    pagingState: PagingState(
      map: {
        0: ConversationListEntry(
          id: '1',
          ownerId: 'user-1',
          name: 'Mixin Team',
          avatarUrl: '',
          category: 'GROUP',
          draft: '',
          status: 2,
          content: 'Welcome to Mixin Messenger',
          contentType: 'PLAIN_TEXT',
          messageStatus: 'DELIVERED',
          senderId: 'user-1',
          senderName: 'Mixin',
          updatedAt: _previewDate,
          unseenCount: 3,
          mentionCount: 1,
          isMuted: false,
          isVerified: true,
          isBot: false,
          isPinned: true,
          relationship: '',
          identityNumber: '7000',
          circleIds: [],
          groupAvatars: [],
        ),
        1: ConversationListEntry(
          id: '2',
          ownerId: 'user-2',
          name: 'Desktop Development',
          avatarUrl: '',
          category: 'CONTACT',
          draft: '',
          status: 2,
          content: '',
          contentType: 'PLAIN_IMAGE',
          messageStatus: 'READ',
          senderId: 'me',
          senderName: 'Me',
          updatedAt: _previewDate,
          unseenCount: 0,
          mentionCount: 0,
          isMuted: true,
          isVerified: false,
          isBot: false,
          isPinned: false,
          relationship: 'FRIEND',
          identityNumber: '7001',
          circleIds: [],
          groupAvatars: [],
        ),
      },
      count: 2,
      initialized: true,
      hasData: true,
    ),
    itemPositionsListener: ItemPositionsListener.create(),
    itemScrollController: ItemScrollController(),
    loading: false,
    currentUserId: 'me',
    circles: const {},
    currentCircleId: null,
    filterUnseen: false,
    selectedConversationId: '1',
    onQueryChanged: _ignoreQuery,
    onToggleUnseen: _noop,
    onSelected: _ignoreConversation,
    onPinned: _ignoreConversation,
    onMuted: _ignoreMute,
    onDeleted: _ignoreConversation,
    onCircleChanged: _ignoreCircle,
    onRetry: _noop,
  ),
);

final _previewDate = DateTime(2026, 7, 16, 12, 30);

void _ignoreConversation(ConversationListEntry _) {}
void _ignoreMute(ConversationListEntry _, int _) {}
void _ignoreCircle(ConversationListEntry _, String _, bool _) {}
void _ignoreQuery(String _) {}
void _noop() {}
