import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';
import 'package:mixin_desktop_ui/theme.dart';

class NetworkStatus extends StatefulWidget {
  const NetworkStatus({required this.account, super.key});

  final AccountHandle account;

  @override
  State<NetworkStatus> createState() => _NetworkStatusState();
}

class _NetworkStatusState extends State<NetworkStatus> {
  late Stream<bool> _connectionStatus;
  bool connected = false;
  bool connectedBefore = false;

  @override
  void initState() {
    super.initState();
    _connectionStatus = widget.account.connectionStatus().distinct();
  }

  @override
  void didUpdateWidget(NetworkStatus oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.account != widget.account) {
      _connectionStatus = widget.account.connectionStatus().distinct();
      connected = false;
      connectedBefore = false;
    }
  }

  @override
  Widget build(BuildContext context) => StreamBuilder<bool>(
    stream: _connectionStatus,
    initialData: connected,
    builder: (context, snapshot) {
      connected = snapshot.data ?? false;
      if (connected) connectedBefore = true;
      return Column(
        children: [
          AnimatedSize(
            alignment: Alignment.topCenter,
            curve: Curves.easeOut,
            duration: const Duration(milliseconds: 200),
            child: !connected && connectedBefore
                ? Container(
                    padding: const EdgeInsets.symmetric(
                      vertical: 14,
                      horizontal: 22,
                    ),
                    color: context.mixinTheme.warning.withValues(alpha: 0.2),
                    child: Row(
                      children: [
                        ClipOval(
                          child: Container(
                            color: context.mixinTheme.warning,
                            width: 20,
                            height: 20,
                            alignment: Alignment.center,
                            child: SvgPicture.asset(
                              'assets/images/exclamation_mark.svg',
                              width: 2,
                              height: 10,
                              colorFilter: const ColorFilter.mode(
                                Colors.white,
                                BlendMode.srcIn,
                              ),
                            ),
                          ),
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          child: DefaultTextStyle.merge(
                            style: TextStyle(
                              color: context.mixinTheme.text,
                              fontSize: 14,
                            ),
                            child: Row(
                              children: [
                                Text(context.l10n.networkConnectionFailed),
                                const Spacer(),
                                MouseRegion(
                                  cursor: SystemMouseCursors.click,
                                  child: GestureDetector(
                                    onTap: widget.account.retryConnection,
                                    child: Text(
                                      context.l10n.retry,
                                      style: TextStyle(
                                        color: context.mixinTheme.accent,
                                      ),
                                    ),
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ),
                      ],
                    ),
                  )
                : const SizedBox.shrink(),
          ),
          AnimatedSize(
            duration: const Duration(milliseconds: 150),
            child: !connected
                ? LinearProgressIndicator(
                    backgroundColor: Colors.transparent,
                    color: context.mixinTheme.accent,
                    minHeight: 2,
                  )
                : const SizedBox.shrink(),
          ),
        ],
      );
    },
  );
}
