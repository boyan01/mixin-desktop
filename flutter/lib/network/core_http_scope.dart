import 'package:flutter/widgets.dart';
import 'package:http/http.dart' as http;

class CoreHttpScope extends InheritedWidget {
  const CoreHttpScope({
    required this.client,
    required this.revision,
    required super.child,
    super.key,
  });

  final http.Client client;
  final int revision;

  static CoreHttpScope? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<CoreHttpScope>();

  @override
  bool updateShouldNotify(CoreHttpScope oldWidget) =>
      client != oldWidget.client || revision != oldWidget.revision;
}
