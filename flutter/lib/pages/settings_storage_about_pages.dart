import 'dart:async';

import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/constants/assets.dart';
import 'package:mixin_desktop_ui/controllers/settings_controller.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/models/conversation_list_entry.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/settings_widgets.dart';
import 'package:provider/provider.dart';

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

class StorageUsageListPage extends StatelessWidget {
  const StorageUsageListPage({
    required this.entries,
    required this.onSelected,
    super.key,
  });

  final List<ConversationStorageUsageEntry> entries;
  final ValueChanged<ConversationStorageUsageEntry> onSelected;

  @override
  Widget build(BuildContext context) => Scaffold(
    backgroundColor: context.mixinTheme.background,
    appBar: MixinAppBar(title: Text(context.l10n.storageUsage)),
    body: entries.isEmpty
        ? Center(
            child: Text(
              context.l10n.noData,
              style: TextStyle(color: context.mixinTheme.secondaryText),
            ),
          )
        : ListView.separated(
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
                          _formatBytes(entry.sizeBytes),
                          style: TextStyle(
                            color: context.mixinTheme.secondaryText,
                            fontSize: 14,
                          ),
                        ),
                      ],
                    ),
                    onTap: () => onSelected(entry),
                  ),
                ),
              );
            },
          ),
  );
}

class StorageCategoryUsage {
  const StorageCategoryUsage({required this.label, required this.sizeBytes});

  final String label;
  final int sizeBytes;
}

class StorageUsageDetailPage extends StatefulWidget {
  const StorageUsageDetailPage({
    required this.name,
    required this.categories,
    required this.onClear,
    super.key,
  });

  final String name;
  final List<StorageCategoryUsage> categories;
  final Future<void> Function(Set<int> selectedIndexes) onClear;

  @override
  State<StorageUsageDetailPage> createState() => _StorageUsageDetailPageState();
}

class _StorageUsageDetailPageState extends State<StorageUsageDetailPage> {
  final selectedIndexes = <int>{};
  var clearing = false;

  Future<void> _clear() async {
    if (selectedIndexes.isEmpty || clearing) return;
    setState(() => clearing = true);
    try {
      await widget.onClear({...selectedIndexes});
      if (mounted) setState(selectedIndexes.clear);
    } finally {
      if (mounted) setState(() => clearing = false);
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    backgroundColor: context.mixinTheme.background,
    appBar: MixinAppBar(
      title: Text(widget.name),
      actions: [
        TextButton(
          key: const ValueKey('storage-clear'),
          onPressed: selectedIndexes.isEmpty || clearing ? null : _clear,
          child: clearing
              ? const SizedBox.square(
                  dimension: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Text(context.l10n.clear),
        ),
      ],
    ),
    body: Container(
      alignment: Alignment.topCenter,
      padding: const EdgeInsets.only(top: 40),
      child: CellGroup(
        cellBackgroundColor: context.mixinTheme.settingCellBackgroundColor,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            for (var index = 0; index < widget.categories.length; index++)
              CellItem(
                title: RadioItem<bool>(
                  groupValue: true,
                  value: selectedIndexes.contains(index),
                  title: Text(widget.categories[index].label),
                  onChanged: (_) => setState(() {
                    if (!selectedIndexes.add(index)) {
                      selectedIndexes.remove(index);
                    }
                  }),
                ),
                description: Text(
                  _formatBytes(widget.categories[index].sizeBytes),
                ),
                trailing: null,
              ),
          ],
        ),
      ),
    ),
  );
}

class AboutPage extends StatefulWidget {
  const AboutPage({
    required this.version,
    required this.onOpenUri,
    super.key,
    this.logs = const [],
    this.onOpenLogDirectory,
  });

  final String version;
  final Future<void> Function(Uri uri) onOpenUri;
  final List<String> logs;
  final VoidCallback? onOpenLogDirectory;

  @override
  State<AboutPage> createState() => _AboutPageState();
}

class _AboutPageState extends State<AboutPage> {
  DateTime? lastTitleTap;
  int titleTapCount = 0;
  int logoTapCount = 0;
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
    logoTapCount++;
    if (logoTapCount < 5) return;
    logoTapCount = 0;
    Navigator.push(
      context,
      MaterialPageRoute<void>(
        builder: (_) => SettingsLogPage(
          logs: widget.logs,
          onOpenDirectory: widget.onOpenLogDirectory,
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
            GestureDetector(
              key: const ValueKey('about-logo'),
              onTap: _tapLogo,
              child: Image.asset(MixinAssets.aboutLogo, width: 60, height: 60),
            ),
            const SizedBox(height: 24),
            GestureDetector(
              onTap: _tapTitle,
              child: Text(
                context.l10n.mixinMessengerDesktop,
                style: TextStyle(color: context.mixinTheme.text, fontSize: 18),
              ),
            ),
            const SizedBox(height: 8),
            SelectableText(
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
                ],
              ),
            ),
            if (debugMode && widget.onOpenLogDirectory != null)
              CellGroup(
                child: CellItem(
                  title: Text(context.l10n.openLogDirectory),
                  onTap: widget.onOpenLogDirectory,
                ),
              ),
          ],
        ),
      ),
    ),
  );
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

class SettingsLogPage extends StatelessWidget {
  const SettingsLogPage({required this.logs, super.key, this.onOpenDirectory});

  final List<String> logs;
  final VoidCallback? onOpenDirectory;

  @override
  Widget build(BuildContext context) => Scaffold(
    backgroundColor: context.mixinTheme.background,
    appBar: MixinAppBar(
      actions: [
        if (onOpenDirectory != null)
          IconButton(
            onPressed: onOpenDirectory,
            icon: const Icon(Icons.launch),
          ),
      ],
    ),
    body: SelectionArea(
      child: ListView.builder(
        reverse: true,
        itemCount: logs.length,
        itemBuilder: (context, index) => Padding(
          padding: const EdgeInsets.symmetric(vertical: 2, horizontal: 4),
          child: Text(logs[logs.length - 1 - index]),
        ),
      ),
    ),
  );
}

String _formatBytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  final kilobytes = bytes / 1024;
  if (kilobytes < 1024) return '${kilobytes.toStringAsFixed(1)} KB';
  final megabytes = kilobytes / 1024;
  if (megabytes < 1024) return '${megabytes.toStringAsFixed(1)} MB';
  return '${(megabytes / 1024).toStringAsFixed(1)} GB';
}
