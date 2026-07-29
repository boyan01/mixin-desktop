import 'package:scrollable_positioned_list/scrollable_positioned_list.dart';

class ConversationListViewport {
  final itemPositionsListener = ItemPositionsListener.create();
  final itemScrollController = ItemScrollController();
}
