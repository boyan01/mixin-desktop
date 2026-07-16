import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/app.dart';
import 'package:mixin_desktop_ui/controllers/app_controller.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/src/rust/frb_generated.dart';
import 'package:provider/provider.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  final controller = AppController()..initialize();
  final settingsController = SettingsController();
  await settingsController.initialize();
  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider.value(value: controller),
        ChangeNotifierProvider.value(value: settingsController),
      ],
      child: const MixinDesktopApp(),
    ),
  );
}
