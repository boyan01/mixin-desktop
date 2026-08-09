import 'dart:async';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../controllers/message_action_controller.dart';
import '../../l10n/l10n.dart';
import '../../models/message_list_entry.dart';
import '../../src/rust/desktop_api.dart' as rust;
import '../../theme.dart';
import '../../utils/app_logger.dart';
import '../../utils/web_view.dart';
import '../interactive_decorated_box.dart';
import '../message_style.dart';
import '../toast.dart';

class StrangerMessageItem extends StatefulWidget {
  const StrangerMessageItem({required this.message, super.key});

  final MessageListEntry message;

  @override
  State<StrangerMessageItem> createState() => _StrangerMessageItemState();
}

class _StrangerMessageItemState extends State<StrangerMessageItem> {
  bool _resolved = false;

  @override
  Widget build(BuildContext context) {
    if (_resolved) return const SizedBox.shrink();
    final bot = widget.message.senderIsBot;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          bot ? context.l10n.chatBotReceptionTitle : context.l10n.strangerHint,
          style: TextStyle(
            fontSize: context.messageStyle.primaryFontSize,
            color: context.theme.text,
          ),
        ),
        const SizedBox(height: 10),
        Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            _StrangerButton(
              text: bot ? context.l10n.openHomePage : context.l10n.block,
              onTap: () => _handle(bot ? 'open_home' : 'block'),
            ),
            const SizedBox(width: 16),
            _StrangerButton(
              text: bot ? context.l10n.sayHi : context.l10n.addContact,
              onTap: () => _handle(bot ? 'say_hi' : 'add_contact'),
            ),
          ],
        ),
      ],
    );
  }

  void _handle(String action) {
    final account = context.read<rust.AccountHandle?>();
    if (account == null) return;
    final messageController = context
        .read<MessageActionController?>()
        ?.messageController;
    unawaited(() async {
      try {
        String? uri;
        switch (action) {
          case 'block':
            await account.user().blockUser(userId: widget.message.senderId);
            messageController?.loadLatestWindow();
          case 'add_contact':
            await account.user().addContact(
              userId: widget.message.senderId,
              fullName: widget.message.senderName,
            );
            messageController?.loadLatestWindow();
          case 'say_hi':
            final messageId = await account.message().sendText(
              conversationId: widget.message.conversationId,
              content: 'Hi',
              silent: false,
            );
            await messageController?.refreshMessages([messageId]);
          case 'open_home':
            final appId = widget.message.senderAppId?.trim();
            if (appId != null && appId.isNotEmpty) {
              uri = await account.user().botHomeUri(appId: appId);
            }
        }
        if (!mounted) return;
        if (action == 'block' || action == 'add_contact') {
          setState(() => _resolved = true);
        }
        if (uri == null) return;
        if (action == 'open_home') {
          await openBotWebViewWindow(
            context: context,
            url: uri,
            title: widget.message.senderName,
            conversationId: widget.message.conversationId,
            currency: account.profile().fiatCurrency,
          );
        } else {
          openMessageAction(
            context: context,
            account: account,
            conversationId: widget.message.conversationId,
            action: uri,
          );
        }
      } on Object catch (error, stackTrace) {
        e(
          'Handle stranger message action failed: '
          'message_id=${widget.message.id}, action=$action',
          error,
          stackTrace,
        );
        if (mounted) showToastFailed(error);
      }
    }());
  }
}

class _StrangerButton extends StatelessWidget {
  const _StrangerButton({
    required this.text,
    required this.onTap,
  });
  final String text;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => InteractiveDecoratedBox.color(
    onTap: onTap,
    decoration: BoxDecoration(
      color: context.theme.primary,
      borderRadius: const BorderRadius.all(Radius.circular(8)),
    ),
    child: ConstrainedBox(
      constraints: const BoxConstraints(
        minWidth: 162,
        minHeight: 36,
        maxHeight: 36,
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Center(
          child: Text(
            text,
            style: TextStyle(
              fontSize: context.messageStyle.primaryFontSize,
              color: context.theme.accent,
            ),
          ),
        ),
      ),
    ),
  );
}
