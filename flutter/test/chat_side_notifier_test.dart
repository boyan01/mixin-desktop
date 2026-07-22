import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/chat_side_notifier.dart';
import 'package:mixin_desktop_ui/pages/conversation_info_destination.dart';

void main() {
  test('keeps the original chat side destination stack semantics', () {
    final notifier = ChatSideNotifier();

    // Keep the intermediate assertion explicit instead of combining calls.
    // ignore: cascade_invocations
    notifier.toggleInfoPage();
    expect(notifier.state.destinations, [ConversationInfoDestination.infoPage]);

    notifier.openDestination(ConversationInfoDestination.sharedMedia);
    // ignore: cascade_invocations
    notifier.openDestination(ConversationInfoDestination.searchMessageHistory);
    expect(notifier.state.destinations, [
      ConversationInfoDestination.infoPage,
      ConversationInfoDestination.sharedMedia,
      ConversationInfoDestination.searchMessageHistory,
    ]);

    notifier.openDestination(ConversationInfoDestination.sharedMedia);
    expect(notifier.state.destinations, [
      ConversationInfoDestination.infoPage,
      ConversationInfoDestination.sharedMedia,
    ]);

    notifier.closeAfterContentJump(routeMode: false);
    expect(
      notifier.currentDestination,
      ConversationInfoDestination.sharedMedia,
    );

    notifier.closeAfterContentJump(routeMode: true);
    expect(notifier.state.destinations, isEmpty);

    notifier.dispose();
  });
}
