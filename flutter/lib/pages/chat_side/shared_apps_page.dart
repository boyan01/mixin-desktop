import 'dart:async';

import 'package:flutter/material.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/src/rust/api/desktop.dart' as rust;
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/mixin_image.dart';
import 'package:url_launcher/url_launcher.dart';

import 'chat_side_scope.dart';

class SharedAppsPage extends StatefulWidget {
  const SharedAppsPage({super.key});

  @override
  State<SharedAppsPage> createState() => _SharedAppsPageState();
}

class _SharedAppsPageState extends State<SharedAppsPage> {
  List<rust.SharedAppItem> apps = const [];
  bool loading = true;
  Object? error;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (loading && apps.isEmpty && error == null) unawaited(_load());
  }

  Future<void> _load() async {
    final scope = ChatSideScope.of(context);
    if (scope.conversation.isGroup) {
      setState(() => loading = false);
      return;
    }
    try {
      final values = await scope.account.sharedApps(
        userId: scope.conversation.ownerId,
      );
      if (!mounted) return;
      setState(() {
        apps = values;
        loading = false;
        error = null;
      });
    } on Object catch (exception) {
      if (!mounted) return;
      setState(() {
        loading = false;
        error = exception;
      });
    }
  }

  @override
  Widget build(BuildContext context) => ChatSidePageScaffold(
    title: context.l10n.shareApps,
    body: loading
        ? const Center(child: CircularProgressIndicator())
        : error != null
        ? ChatSideError(error: error!, onRetry: _load)
        : apps.isEmpty
        ? Center(child: Text(context.l10n.noResults))
        : ListView.builder(
            itemCount: apps.length,
            itemBuilder: (context, index) {
              final app = apps[index];
              return InkWell(
                onTap: () async {
                  final uri = Uri.tryParse(app.homeUri);
                  if (uri != null) await launchUrl(uri);
                },
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    vertical: 10,
                    horizontal: 16,
                  ),
                  child: Row(
                    children: [
                      ClipOval(
                        child: MixinImage.network(
                          app.iconUrl,
                          width: 50,
                          height: 50,
                          placeholder: () => ColoredBox(
                            color: context.theme.listSelected,
                            child: const SizedBox.square(dimension: 50),
                          ),
                        ),
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(app.name),
                            const SizedBox(height: 3),
                            Text(
                              app.description,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: TextStyle(
                                color: context.theme.secondaryText,
                                fontSize: 14,
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
          ),
  );
}
