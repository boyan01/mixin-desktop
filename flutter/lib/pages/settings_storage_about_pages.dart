import 'dart:async';
import 'dart:io';

import 'package:filesize/filesize.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:provider/provider.dart';

import '../constants/assets.dart';
import '../controllers/settings_controller.dart';
import '../l10n/l10n.dart';
import '../models/conversation_list_entry.dart';
import '../src/rust/desktop_api.dart';
import '../theme.dart';
import '../widgets/action_button.dart';
import '../widgets/avatar_view.dart';
import '../widgets/buttons.dart';
import '../widgets/high_light_text.dart';
import '../widgets/mixin_dialog.dart';
import '../widgets/settings_widgets.dart';
import '../widgets/toast.dart';

class StoragePage extends StatelessWidget {
  const StoragePage({required this.onOpenStorageUsage, super.key});

  final VoidCallback onOpenStorageUsage;

  @override
  Widget build(BuildContext context) {
    final settings = context.watch<SettingsController>();
    return Scaffold(
      backgroundColor: context.mixinTheme.background,
      appBar: MixinAppBar(title: Text(context.l10n.dataAndStorageUsage)),
      body: SingleChildScrollView(
        child: Container(
          alignment: Alignment.topCenter,
          padding: const EdgeInsets.only(top: 40),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              CellGroup(
                cellBackgroundColor:
                    context.mixinTheme.settingCellBackgroundColor,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    CellItem(
                      title: Text(context.l10n.photos),
                      trailing: SettingsSwitch(
                        value: settings.photoAutoDownload,
                        onChanged: settings.setPhotoAutoDownload,
                      ),
                    ),
                    CellItem(
                      title: Text(context.l10n.videos),
                      trailing: SettingsSwitch(
                        value: settings.videoAutoDownload,
                        onChanged: settings.setVideoAutoDownload,
                      ),
                    ),
                    CellItem(
                      title: Text(context.l10n.files),
                      trailing: SettingsSwitch(
                        value: settings.fileAutoDownload,
                        onChanged: settings.setFileAutoDownload,
                      ),
                    ),
                  ],
                ),
              ),
              Padding(
                padding: const EdgeInsets.only(left: 20, bottom: 14, top: 10),
                child: Text(
                  context.l10n.storageAutoDownloadDescription,
                  style: TextStyle(
                    color: context.mixinTheme.secondaryText,
                    fontSize: 14,
                  ),
                ),
              ),
              CellGroup(
                cellBackgroundColor:
                    context.mixinTheme.settingCellBackgroundColor,
                child: CellItem(
                  key: const ValueKey('settings-storage-usage'),
                  title: Text(context.l10n.storageUsage),
                  onTap: onOpenStorageUsage,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class ConversationStorageUsageEntry {
  const ConversationStorageUsageEntry({
    required this.conversation,
    required this.sizeBytes,
  });

  final ConversationListEntry conversation;
  final int sizeBytes;
}

class StorageUsageListPage extends StatefulWidget {
  const StorageUsageListPage({
    required this.account,
    required this.onSelected,
    super.key,
  });

  final AccountHandle account;
  final ValueChanged<ConversationStorageUsageEntry> onSelected;

  @override
  State<StorageUsageListPage> createState() => _StorageUsageListPageState();
}

class _StorageUsageListPageState extends State<StorageUsageListPage> {
  late Future<List<ConversationStorageUsageEntry>> entries = _load();
  StreamSubscription<FileSystemEvent>? watcher;
  Timer? reloadTimer;

  @override
  void initState() {
    super.initState();
    watcher = Directory(widget.account.mediaDirectory())
        .watch(recursive: true)
        .listen((_) {
          reloadTimer?.cancel();
          reloadTimer = Timer(const Duration(milliseconds: 400), () {
            if (mounted) setState(() => entries = _load());
          });
        });
  }

  @override
  void dispose() {
    reloadTimer?.cancel();
    unawaited(watcher?.cancel());
    super.dispose();
  }

  Future<List<ConversationStorageUsageEntry>> _load() async {
    try {
      return (await widget.account.storageUsage())
          .map(
            (item) => ConversationStorageUsageEntry(
              conversation: ConversationListEntry.fromRust(item.conversation),
              sizeBytes: item.sizeBytes,
            ),
          )
          .toList(growable: false);
    } catch (_) {
      return const [];
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    backgroundColor: context.mixinTheme.background,
    appBar: MixinAppBar(title: Text(context.l10n.storageUsage)),
    body: FutureBuilder<List<ConversationStorageUsageEntry>>(
      future: entries,
      builder: (context, snapshot) {
        if (!snapshot.hasData) {
          return Center(
            child: CircularProgressIndicator(
              valueColor: AlwaysStoppedAnimation(context.mixinTheme.accent),
            ),
          );
        }
        final entries = snapshot.data!;
        return ListView.separated(
          padding: const EdgeInsets.symmetric(vertical: 40),
          itemCount: entries.length,
          separatorBuilder: (_, _) => const SizedBox(height: 10),
          itemBuilder: (context, index) {
            final entry = entries[index];
            return Align(
              child: CellGroup(
                padding: EdgeInsets.zero,
                cellBackgroundColor:
                    context.mixinTheme.settingCellBackgroundColor,
                child: CellItem(
                  leading: ConversationAvatarView(
                    conversation: entry.conversation,
                    size: 50,
                  ),
                  title: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(entry.conversation.name),
                      Text(
                        filesize(entry.sizeBytes),
                        style: TextStyle(
                          color: context.mixinTheme.secondaryText,
                          fontSize: 14,
                        ),
                      ),
                    ],
                  ),
                  onTap: () => widget.onSelected(entry),
                ),
              ),
            );
          },
        );
      },
    ),
  );
}

class StorageUsageDetailPage extends StatefulWidget {
  const StorageUsageDetailPage({
    required this.account,
    required this.name,
    required this.conversationId,
    super.key,
  });

  final AccountHandle account;
  final String name;
  final String conversationId;

  @override
  State<StorageUsageDetailPage> createState() => _StorageUsageDetailPageState();
}

class _StorageUsageDetailPageState extends State<StorageUsageDetailPage> {
  final selectedIndexes = <int>{};
  late Future<List<StorageCategoryUsageEntry>> categories = _load();
  StreamSubscription<FileSystemEvent>? watcher;
  Timer? reloadTimer;

  @override
  void initState() {
    super.initState();
    watcher = Directory(widget.account.mediaDirectory())
        .watch(recursive: true)
        .listen((_) {
          reloadTimer?.cancel();
          reloadTimer = Timer(const Duration(milliseconds: 400), () {
            if (mounted) setState(() => categories = _load());
          });
        });
  }

  @override
  void dispose() {
    reloadTimer?.cancel();
    unawaited(watcher?.cancel());
    super.dispose();
  }

  Future<List<StorageCategoryUsageEntry>> _load() async =>
      (await widget.account.conversationStorageUsage(
            conversationId: widget.conversationId,
          ))
          .map((item) {
            final label = switch (item.category) {
              'photos' => context.l10n.photos,
              'videos' => context.l10n.videos,
              'audio' => context.l10n.audio,
              _ => context.l10n.files,
            };
            return StorageCategoryUsageEntry(
              category: item.category,
              label: label,
              sizeBytes: item.sizeBytes,
            );
          })
          .toList(growable: false);

  Future<void> _clear(List<StorageCategoryUsageEntry> items) async {
    if (selectedIndexes.isEmpty) return;
    final successful = await runWithToast(() async {
      await widget.account.clearConversationStorage(
        conversationId: widget.conversationId,
        categories: selectedIndexes
            .map((index) => items[index].category)
            .toList(growable: false),
      );
    });
    if (!successful || !mounted) return;
    setState(() {
      categories = _load();
    });
  }

  @override
  Widget build(BuildContext context) =>
      FutureBuilder<List<StorageCategoryUsageEntry>>(
        future: categories,
        builder: (context, snapshot) {
          final items =
              snapshot.data ??
              [
                StorageCategoryUsageEntry(
                  category: 'photos',
                  label: context.l10n.photos,
                  sizeBytes: 0,
                ),
                StorageCategoryUsageEntry(
                  category: 'videos',
                  label: context.l10n.videos,
                  sizeBytes: 0,
                ),
                StorageCategoryUsageEntry(
                  category: 'audio',
                  label: context.l10n.audio,
                  sizeBytes: 0,
                ),
                StorageCategoryUsageEntry(
                  category: 'files',
                  label: context.l10n.files,
                  sizeBytes: 0,
                ),
              ];
          return Scaffold(
            backgroundColor: context.mixinTheme.background,
            appBar: MixinAppBar(
              title: Text(widget.name),
              actions: [
                MixinButton(
                  key: const ValueKey('storage-clear'),
                  disable: selectedIndexes.isEmpty,
                  backgroundTransparent: true,
                  onTap: () => _clear(items),
                  child: Center(child: Text(context.l10n.clear)),
                ),
              ],
            ),
            body: Container(
              alignment: Alignment.topCenter,
              padding: const EdgeInsets.only(top: 40),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  CellGroup(
                    cellBackgroundColor:
                        context.mixinTheme.settingCellBackgroundColor,
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        for (var index = 0; index < items.length; index++)
                          CellItem(
                            title: RadioItem<bool>(
                              groupValue: true,
                              value: selectedIndexes.contains(index),
                              title: Text(items[index].label),
                              onChanged: (_) => setState(() {
                                if (!selectedIndexes.add(index)) {
                                  selectedIndexes.remove(index);
                                }
                              }),
                            ),
                            description: Text(
                              filesize(items[index].sizeBytes),
                              style: TextStyle(
                                color: context.mixinTheme.secondaryText,
                                fontSize: 14,
                              ),
                            ),
                          ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          );
        },
      );
}

class StorageCategoryUsageEntry {
  const StorageCategoryUsageEntry({
    required this.category,
    required this.label,
    required this.sizeBytes,
  });

  final String category;
  final String label;
  final int sizeBytes;
}

class AboutPage extends StatefulWidget {
  const AboutPage({
    required this.version,
    required this.onOpenUri,
    super.key,
    this.onOpenLogDirectory,
    this.onLoadLogs,
  });

  final String version;
  final Future<void> Function(Uri uri) onOpenUri;
  final Future<void> Function()? onOpenLogDirectory;
  final Future<List<String>> Function()? onLoadLogs;

  @override
  State<AboutPage> createState() => _AboutPageState();
}

class _AboutPageState extends State<AboutPage> {
  DateTime? lastTitleTap;
  int titleTapCount = 0;
  bool debugMode = false;

  void _tapTitle() {
    final now = DateTime.now();
    titleTapCount =
        lastTitleTap != null &&
            now.difference(lastTitleTap!) < const Duration(seconds: 1)
        ? titleTapCount + 1
        : 1;
    lastTitleTap = now;
    if (titleTapCount > 6) setState(() => debugMode = true);
  }

  void _tapLogo() {
    showGeneralDialog<void>(
      context: context,
      barrierColor: Colors.transparent,
      barrierDismissible: true,
      barrierLabel: MaterialLocalizations.of(context).modalBarrierDismissLabel,
      pageBuilder: (buildContext, animation, secondaryAnimation) =>
          InheritedTheme.capture(
            from: context,
            to: Navigator.of(context, rootNavigator: true).context,
          ).wrap(
            SettingsLogPage(
              onOpenDirectory: widget.onOpenLogDirectory,
              onLoadLogs: widget.onLoadLogs,
            ),
          ),
    );
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    backgroundColor: context.mixinTheme.background,
    appBar: MixinAppBar(title: Text(context.l10n.about)),
    body: SingleChildScrollView(
      child: Container(
        alignment: Alignment.topCenter,
        padding: const EdgeInsets.only(top: 40),
        child: Column(
          children: [
            NTapGestureDetector(
              key: const ValueKey('about-logo'),
              n: 5,
              onTap: _tapLogo,
              child: Image.asset(MixinAssets.aboutLogo, width: 60, height: 60),
            ),
            const SizedBox(height: 24),
            GestureDetector(
              onTap: _tapTitle,
              child: Animate(
                effects: const [
                  FadeEffect(duration: Duration(milliseconds: 1000)),
                  ScaleEffect(duration: Duration(milliseconds: 1000)),
                ],
                child: Text(
                  context.l10n.mixinMessengerDesktop,
                  style: TextStyle(
                    color: context.mixinTheme.text,
                    fontSize: 18,
                  ),
                ),
              ),
            ),
            const SizedBox(height: 8),
            CustomSelectableText(
              widget.version,
              style: TextStyle(
                color: context.mixinTheme.secondaryText,
                fontSize: 16,
              ),
            ),
            const SizedBox(height: 50),
            CellGroup(
              cellBackgroundColor:
                  context.mixinTheme.settingCellBackgroundColor,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  _AboutLink(
                    title: context.l10n.followUsOnX,
                    uri: Uri.parse('https://x.com/MixinMessenger'),
                    onOpen: widget.onOpenUri,
                  ),
                  _AboutLink(
                    title: context.l10n.followUsOnFacebook,
                    uri: Uri.parse('https://fb.com/MixinMessenger'),
                    onOpen: widget.onOpenUri,
                  ),
                  _AboutLink(
                    title: context.l10n.helpCenter,
                    uri: Uri.parse('https://support.mixin.one'),
                    onOpen: widget.onOpenUri,
                  ),
                  _AboutLink(
                    title: context.l10n.termsOfService,
                    uri: Uri.parse('https://mixin.one/pages/terms'),
                    onOpen: widget.onOpenUri,
                  ),
                  _AboutLink(
                    title: context.l10n.privacyPolicy,
                    uri: Uri.parse('https://mixin.one/pages/privacy'),
                    onOpen: widget.onOpenUri,
                  ),
                  if (defaultTargetPlatform != TargetPlatform.macOS)
                    CellItem(
                      title: Text(context.l10n.checkNewVersion),
                      onTap: _openCheckUpdate,
                    ),
                ],
              ),
            ),
            if (debugMode && widget.onOpenLogDirectory != null)
              CellGroup(
                child: CellItem(
                  title: Text(context.l10n.openLogDirectory),
                  onTap: () => unawaited(widget.onOpenLogDirectory!()),
                ),
              ),
          ],
        ),
      ),
    ),
  );

  void _openCheckUpdate() {
    final uri = switch (defaultTargetPlatform) {
      TargetPlatform.linux => Uri.parse('https://mixin.one/messenger'),
      TargetPlatform.iOS || TargetPlatform.macOS => Uri.parse(
        'https://apps.apple.com/app/mixin-messenger/id1571128582',
      ),
      TargetPlatform.windows => Uri.parse(
        'https://apps.microsoft.com/store/detail/mixin-desktop/9NQ6HF99B8NJ',
      ),
      _ => Uri.parse('https://mixin.one/messenger'),
    };
    unawaited(widget.onOpenUri(uri));
  }
}

class _AboutLink extends StatelessWidget {
  const _AboutLink({
    required this.title,
    required this.uri,
    required this.onOpen,
  });

  final String title;
  final Uri uri;
  final Future<void> Function(Uri uri) onOpen;

  @override
  Widget build(BuildContext context) =>
      CellItem(title: Text(title), onTap: () => unawaited(onOpen(uri)));
}

class SettingsLogPage extends StatefulWidget {
  const SettingsLogPage({super.key, this.onOpenDirectory, this.onLoadLogs});

  final Future<void> Function()? onOpenDirectory;
  final Future<List<String>> Function()? onLoadLogs;

  @override
  State<SettingsLogPage> createState() => _SettingsLogPageState();
}

class _SettingsLogPageState extends State<SettingsLogPage> {
  late Future<List<String>> _logs;

  @override
  void initState() {
    super.initState();
    _reload();
  }

  void _reload() {
    _logs = widget.onLoadLogs?.call() ?? Future.value(const []);
  }

  @override
  Widget build(BuildContext context) => Material(
    color: context.mixinTheme.background,
    child: Column(
      children: [
        MixinAppBar(
          leading: const SizedBox(),
          actions: [
            if (widget.onLoadLogs != null)
              ActionButton(
                color: context.mixinTheme.icon,
                onTap: () => setState(_reload),
                child: const Icon(Icons.refresh),
              ),
            if (widget.onOpenDirectory != null)
              ActionButton(
                color: context.mixinTheme.icon,
                onTap: () => unawaited(widget.onOpenDirectory!()),
                child: const Icon(Icons.launch),
              ),
            const SizedBox(width: 8),
            MixinCloseButton(onTap: () => Navigator.pop(context)),
          ],
        ),
        Expanded(
          child: FutureBuilder<List<String>>(
            future: _logs,
            builder: (context, snapshot) {
              final logs = snapshot.data;
              if (logs == null) return const SizedBox();
              return CustomSelectableArea(
                child: ListView.builder(
                  reverse: true,
                  itemCount: logs.length,
                  itemBuilder: (context, index) => Padding(
                    padding: const EdgeInsets.symmetric(
                      vertical: 2,
                      horizontal: 4,
                    ),
                    child: CustomText(logs[logs.length - 1 - index]),
                  ),
                ),
              );
            },
          ),
        ),
      ],
    ),
  );
}
