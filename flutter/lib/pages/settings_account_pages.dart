import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:mixin_desktop_ui/l10n/l10n.dart';
import 'package:mixin_desktop_ui/theme.dart';
import 'package:mixin_desktop_ui/widgets/avatar_view.dart';
import 'package:mixin_desktop_ui/widgets/settings_widgets.dart';

typedef SaveProfileCallback =
    Future<void> Function(String fullName, String biography);

class EditProfileSettingsPage extends StatefulWidget {
  const EditProfileSettingsPage({
    required this.userId,
    required this.fullName,
    required this.biography,
    required this.identityNumber,
    required this.phone,
    required this.avatarUrl,
    required this.onSave,
    super.key,
    this.createdAt,
    this.onBack,
  });

  final String userId;
  final String fullName;
  final String biography;
  final String identityNumber;
  final String phone;
  final String avatarUrl;
  final DateTime? createdAt;
  final SaveProfileCallback onSave;
  final VoidCallback? onBack;

  @override
  State<EditProfileSettingsPage> createState() =>
      _EditProfileSettingsPageState();
}

class _EditProfileSettingsPageState extends State<EditProfileSettingsPage> {
  late final TextEditingController _nameController;
  late final TextEditingController _biographyController;
  late final TextEditingController _phoneController;
  var _saving = false;

  @override
  void initState() {
    super.initState();
    _nameController = TextEditingController(text: widget.fullName);
    _biographyController = TextEditingController(text: widget.biography);
    _phoneController = TextEditingController(text: widget.phone);
  }

  @override
  void dispose() {
    _nameController.dispose();
    _biographyController.dispose();
    _phoneController.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (_saving) return;
    setState(() => _saving = true);
    try {
      await widget.onSave(
        _nameController.text.trim(),
        _biographyController.text.trim(),
      );
      if (mounted) widget.onBack?.call();
    } catch (_) {
      if (mounted) _showFailure(context);
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    backgroundColor: context.theme.background,
    appBar: MixinAppBar(
      title: Text(context.l10n.editProfile),
      leading: widget.onBack == null
          ? null
          : _BackButton(onPressed: widget.onBack!),
      actions: [
        TextButton(
          onPressed: _saving ? null : _save,
          child: _saving
              ? SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    color: context.theme.accent,
                  ),
                )
              : Text(context.l10n.save),
        ),
      ],
    ),
    body: SingleChildScrollView(
      child: Column(
        children: [
          const SizedBox(height: 40),
          AvatarView(
            userId: widget.userId,
            name: widget.fullName,
            avatarUrl: widget.avatarUrl,
            size: 100,
          ),
          const SizedBox(height: 10),
          SelectableText(
            'Mixin ID: ${widget.identityNumber}',
            style: TextStyle(
              fontSize: 14,
              color: context.dynamicColor(
                const Color.fromRGBO(188, 190, 195, 1),
                darkColor: const Color.fromRGBO(255, 255, 255, 0.4),
              ),
            ),
          ),
          const SizedBox(height: 32),
          _ProfileField(
            title: context.l10n.name,
            controller: _nameController,
            maxLength: 40,
          ),
          const SizedBox(height: 32),
          _ProfileField(
            title: context.l10n.biography,
            controller: _biographyController,
            maxLength: 140,
          ),
          const SizedBox(height: 32),
          _ProfileField(
            title: context.l10n.phoneNumber,
            controller: _phoneController,
            readOnly: true,
          ),
          const SizedBox(height: 70),
          Text(
            widget.createdAt == null
                ? ''
                : context.l10n.joinedIn(
                    DateFormat.yMMMd().format(widget.createdAt!.toLocal()),
                  ),
            style: TextStyle(fontSize: 14, color: context.theme.secondaryText),
          ),
          const SizedBox(height: 48),
        ],
      ),
    ),
  );
}

class AccountSettingsPage extends StatelessWidget {
  const AccountSettingsPage({
    required this.onBack,
    required this.onChangeNumber,
    required this.onDeleteAccount,
    super.key,
  });

  final VoidCallback onBack;
  final VoidCallback onChangeNumber;
  final VoidCallback onDeleteAccount;

  @override
  Widget build(BuildContext context) => Scaffold(
    backgroundColor: context.theme.background,
    appBar: MixinAppBar(
      title: Text(context.l10n.account),
      leading: _BackButton(onPressed: onBack),
    ),
    body: SingleChildScrollView(
      child: Padding(
        padding: const EdgeInsets.only(top: 40),
        child: Column(
          children: [
            CellGroup(
              cellBackgroundColor: context.theme.settingCellBackgroundColor,
              child: CellItem(
                title: Text(context.l10n.changeNumber),
                onTap: onChangeNumber,
              ),
            ),
            CellGroup(
              cellBackgroundColor: context.theme.settingCellBackgroundColor,
              child: CellItem(
                title: Text(context.l10n.deleteMyAccount),
                onTap: onDeleteAccount,
              ),
            ),
          ],
        ),
      ),
    ),
  );
}

class SecuritySettingsPage extends StatefulWidget {
  const SecuritySettingsPage({
    required this.onPasscodeChanged,
    required this.onBiometricChanged,
    required this.onAutoLockChanged,
    super.key,
    this.hasPasscode = false,
    this.biometricEnabled = false,
    this.autoLockDuration = Duration.zero,
    this.onBack,
  });

  final VoidCallback? onBack;
  final bool hasPasscode;
  final bool biometricEnabled;
  final Duration autoLockDuration;
  final Future<void> Function(String? passcode) onPasscodeChanged;
  final Future<bool> Function(bool enabled) onBiometricChanged;
  final ValueChanged<Duration> onAutoLockChanged;

  @override
  State<SecuritySettingsPage> createState() => _SecuritySettingsPageState();
}

class _SecuritySettingsPageState extends State<SecuritySettingsPage> {
  late bool _hasPasscode;
  late bool _biometricEnabled;
  late Duration _autoLockDuration;

  @override
  void initState() {
    super.initState();
    _hasPasscode = widget.hasPasscode;
    _biometricEnabled = widget.biometricEnabled;
    _autoLockDuration = widget.autoLockDuration;
  }

  Future<void> _togglePasscode(bool value) async {
    if (!value) {
      await widget.onPasscodeChanged(null);
      if (mounted) setState(() => _hasPasscode = false);
      return;
    }
    final passcode = await showDialog<String>(
      context: context,
      builder: (context) => const _SetPasscodeDialog(),
    );
    if (passcode == null) return;
    try {
      await widget.onPasscodeChanged(passcode);
      if (mounted) setState(() => _hasPasscode = true);
    } catch (_) {
      if (mounted) _showFailure(context);
    }
  }

  Future<void> _toggleBiometric(bool value) async {
    final accepted = await widget.onBiometricChanged(value);
    if (!mounted) return;
    if (accepted) {
      setState(() => _biometricEnabled = value);
    } else {
      _showFailure(context);
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    backgroundColor: context.theme.background,
    appBar: MixinAppBar(
      title: Text(context.l10n.security),
      leading: widget.onBack == null
          ? null
          : _BackButton(onPressed: widget.onBack!),
    ),
    body: SingleChildScrollView(
      child: Column(
        children: [
          const SizedBox(height: 40),
          CellGroup(
            cellBackgroundColor: context.theme.settingCellBackgroundColor,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                CellItem(
                  title: Text(context.l10n.screenPasscode),
                  trailing: SettingsSwitch(
                    value: _hasPasscode,
                    onChanged: _togglePasscode,
                  ),
                ),
                if (_hasPasscode)
                  CellItem(
                    title: Text(context.l10n.autoLock),
                    description: Text(
                      _durationLabel(context, _autoLockDuration),
                    ),
                    onTap: _showAutoLockMenu,
                  ),
              ],
            ),
          ),
          if (_hasPasscode)
            CellGroup(
              cellBackgroundColor: context.theme.settingCellBackgroundColor,
              child: CellItem(
                title: Text(context.l10n.biometric),
                trailing: SettingsSwitch(
                  value: _biometricEnabled,
                  onChanged: _toggleBiometric,
                ),
              ),
            ),
        ],
      ),
    ),
  );

  Future<void> _showAutoLockMenu() async {
    final value = await showMenu<Duration>(
      context: context,
      color: Color.alphaBlend(
        context.theme.listSelected,
        context.theme.background,
      ),
      position: const RelativeRect.fromLTRB(180, 140, 40, 0),
      items:
          [
                Duration.zero,
                const Duration(minutes: 1),
                const Duration(minutes: 5),
                const Duration(hours: 1),
                const Duration(hours: 5),
              ]
              .map(
                (duration) => PopupMenuItem(
                  value: duration,
                  child: Text(
                    _durationLabel(context, duration),
                    style: TextStyle(color: context.theme.text),
                  ),
                ),
              )
              .toList(),
    );
    if (value == null || !mounted) return;
    setState(() => _autoLockDuration = value);
    widget.onAutoLockChanged(value);
  }
}

class BackupSettingsPage extends StatefulWidget {
  const BackupSettingsPage({
    required this.onBack,
    required this.onBackup,
    required this.onAutoBackupChanged,
    required this.onIncludeFilesChanged,
    required this.onIncludeVideosChanged,
    super.key,
    this.autoBackup = true,
    this.includeFiles = true,
    this.includeVideos = true,
  });

  final VoidCallback onBack;
  final Future<void> Function() onBackup;
  final bool autoBackup;
  final bool includeFiles;
  final bool includeVideos;
  final ValueChanged<bool> onAutoBackupChanged;
  final ValueChanged<bool> onIncludeFilesChanged;
  final ValueChanged<bool> onIncludeVideosChanged;

  @override
  State<BackupSettingsPage> createState() => _BackupSettingsPageState();
}

class _BackupSettingsPageState extends State<BackupSettingsPage> {
  late bool _autoBackup;
  late bool _includeFiles;
  late bool _includeVideos;
  var _backingUp = false;

  @override
  void initState() {
    super.initState();
    _autoBackup = widget.autoBackup;
    _includeFiles = widget.includeFiles;
    _includeVideos = widget.includeVideos;
  }

  Future<void> _backup() async {
    if (_backingUp) return;
    setState(() => _backingUp = true);
    try {
      await widget.onBackup();
    } catch (_) {
      if (mounted) _showFailure(context);
    } finally {
      if (mounted) setState(() => _backingUp = false);
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    backgroundColor: context.theme.background,
    appBar: MixinAppBar(
      title: Text(context.l10n.chatBackup),
      leading: _BackButton(onPressed: widget.onBack),
    ),
    body: SingleChildScrollView(
      child: Padding(
        padding: const EdgeInsets.only(top: 40),
        child: Column(
          children: [
            Icon(
              Icons.cloud_upload_outlined,
              size: 72,
              color: context.theme.secondaryText.withValues(alpha: 0.4),
            ),
            const SizedBox(height: 20),
            ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 500),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 20),
                child: Text(
                  context.l10n.settingBackupTips,
                  textAlign: TextAlign.center,
                  style: TextStyle(
                    fontSize: 16,
                    color: context.theme.secondaryText,
                  ),
                ),
              ),
            ),
            const SizedBox(height: 30),
            CellGroup(
              cellBackgroundColor: context.theme.settingCellBackgroundColor,
              child: CellItem(
                title: Text(context.l10n.backup),
                onTap: _backingUp ? null : _backup,
                trailing: _backingUp
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Arrow(),
              ),
            ),
            CellGroup(
              cellBackgroundColor: context.theme.settingCellBackgroundColor,
              child: Column(
                children: [
                  CellItem(
                    title: Text(context.l10n.autoBackup),
                    trailing: SettingsSwitch(
                      value: _autoBackup,
                      onChanged: (value) {
                        setState(() => _autoBackup = value);
                        widget.onAutoBackupChanged(value);
                      },
                    ),
                  ),
                  CellItem(
                    title: Text(context.l10n.includeFiles),
                    trailing: SettingsSwitch(
                      value: _includeFiles,
                      onChanged: (value) {
                        setState(() => _includeFiles = value);
                        widget.onIncludeFilesChanged(value);
                      },
                    ),
                  ),
                  CellItem(
                    title: Text(context.l10n.includeVideos),
                    trailing: SettingsSwitch(
                      value: _includeVideos,
                      onChanged: (value) {
                        setState(() => _includeVideos = value);
                        widget.onIncludeVideosChanged(value);
                      },
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    ),
  );
}

class AccountDeleteSettingsPage extends StatefulWidget {
  const AccountDeleteSettingsPage({
    required this.onBack,
    required this.onDeleteAccount,
    required this.onChangeNumber,
    super.key,
  });

  final VoidCallback onBack;
  final Future<void> Function() onDeleteAccount;
  final VoidCallback onChangeNumber;

  @override
  State<AccountDeleteSettingsPage> createState() =>
      _AccountDeleteSettingsPageState();
}

class _AccountDeleteSettingsPageState extends State<AccountDeleteSettingsPage> {
  var _deleting = false;

  Future<void> _delete() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        backgroundColor: context.theme.popUp,
        title: Text(
          context.l10n.deleteMyAccount,
          style: TextStyle(color: context.theme.red),
        ),
        content: Text(
          context.l10n.deleteAccountDetailHint,
          style: TextStyle(color: context.theme.text),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(MaterialLocalizations.of(context).cancelButtonLabel),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context, true),
            child: Text(
              context.l10n.deleteMyAccount,
              style: TextStyle(color: context.theme.red),
            ),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;
    setState(() => _deleting = true);
    try {
      await widget.onDeleteAccount();
    } catch (_) {
      if (mounted) _showFailure(context);
    } finally {
      if (mounted) setState(() => _deleting = false);
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    backgroundColor: context.theme.background,
    appBar: MixinAppBar(
      title: Text(context.l10n.deleteMyAccount),
      leading: _BackButton(onPressed: widget.onBack),
    ),
    body: SingleChildScrollView(
      child: Padding(
        padding: const EdgeInsets.only(top: 50),
        child: Column(
          children: [
            Icon(Icons.person_off_outlined, size: 72, color: context.theme.red),
            const SizedBox(height: 20),
            ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 380),
              child: Column(
                children: [
                  _WarningItem(title: context.l10n.deleteAccountHint),
                  _WarningItem(title: context.l10n.deleteAccountDetailHint),
                  _WarningItem(title: context.l10n.transactionsCannotBeDeleted),
                ],
              ),
            ),
            const SizedBox(height: 30),
            CellGroup(
              cellBackgroundColor: context.theme.settingCellBackgroundColor,
              child: CellItem(
                title: Text(context.l10n.deleteMyAccount),
                color: context.theme.red,
                onTap: _deleting ? null : _delete,
                trailing: _deleting
                    ? SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(
                          strokeWidth: 2,
                          color: context.theme.red,
                        ),
                      )
                    : const Arrow(),
              ),
            ),
            CellGroup(
              cellBackgroundColor: context.theme.settingCellBackgroundColor,
              child: CellItem(
                title: Text(context.l10n.changeNumberInstead),
                onTap: widget.onChangeNumber,
              ),
            ),
          ],
        ),
      ),
    ),
  );
}

class _ProfileField extends StatelessWidget {
  const _ProfileField({
    required this.title,
    required this.controller,
    this.readOnly = false,
    this.maxLength,
  });

  final String title;
  final TextEditingController controller;
  final bool readOnly;
  final int? maxLength;

  @override
  Widget build(BuildContext context) => _DynamicHorizontalPadding(
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          title,
          style: TextStyle(
            fontSize: 14,
            fontWeight: FontWeight.w500,
            color: context.theme.secondaryText,
          ),
        ),
        const SizedBox(height: 16),
        TextField(
          readOnly: readOnly,
          controller: controller,
          maxLines: 10,
          minLines: 1,
          maxLength: maxLength,
          style: TextStyle(
            fontSize: 16,
            color: readOnly ? context.theme.secondaryText : context.theme.text,
          ),
          decoration: InputDecoration(
            isDense: true,
            filled: true,
            fillColor: readOnly
                ? context.dynamicColor(
                    const Color.fromRGBO(236, 238, 242, 1),
                    darkColor: const Color.fromRGBO(255, 255, 255, 0.04),
                  )
                : context.dynamicColor(
                    Colors.white,
                    darkColor: const Color.fromRGBO(255, 255, 255, 0.08),
                  ),
            border: _inputBorder,
            focusedBorder: _inputBorder,
            enabledBorder: _inputBorder,
            contentPadding: const EdgeInsets.symmetric(
              horizontal: 20,
              vertical: 24,
            ),
            counterStyle: TextStyle(color: context.theme.secondaryText),
          ),
        ),
      ],
    ),
  );
}

class _SetPasscodeDialog extends StatefulWidget {
  const _SetPasscodeDialog();

  @override
  State<_SetPasscodeDialog> createState() => _SetPasscodeDialogState();
}

class _SetPasscodeDialogState extends State<_SetPasscodeDialog> {
  final _controller = TextEditingController();
  String? _passcode;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _submit(String value) {
    if (value.length != 6) return;
    if (_passcode == null) {
      setState(() {
        _passcode = value;
        _controller.clear();
      });
      return;
    }
    if (_passcode != value) {
      setState(() {
        _passcode = null;
        _controller.clear();
      });
      _showFailure(context, context.l10n.passcodeIncorrect);
      return;
    }
    Navigator.pop(context, value);
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    backgroundColor: context.theme.popUp,
    contentPadding: const EdgeInsets.fromLTRB(72, 40, 72, 56),
    content: SizedBox(
      width: 376,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            _passcode == null
                ? context.l10n.setPasscodeDesc
                : context.l10n.confirmPasscodeDesc,
            textAlign: TextAlign.center,
            style: TextStyle(
              color: context.theme.text,
              fontSize: 18,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 40),
          SizedBox(
            width: 215,
            child: TextField(
              autofocus: true,
              controller: _controller,
              keyboardType: TextInputType.number,
              obscureText: true,
              textAlign: TextAlign.center,
              maxLength: 6,
              style: TextStyle(
                letterSpacing: 14,
                fontSize: 22,
                color: context.theme.text,
              ),
              decoration: const InputDecoration(counterText: ''),
              onChanged: _submit,
            ),
          ),
        ],
      ),
    ),
  );
}

class _BackButton extends StatelessWidget {
  const _BackButton({required this.onPressed});

  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) => IconButton(
    key: const ValueKey('settings-back'),
    onPressed: onPressed,
    icon: const Icon(Icons.arrow_back_ios_new_rounded, size: 18),
  );
}

class _DynamicHorizontalPadding extends StatelessWidget {
  const _DynamicHorizontalPadding({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final padding = math.min(
        90.0,
        math.max(20.0, (constraints.maxWidth - 500) / 2),
      );
      return Padding(
        padding: EdgeInsets.symmetric(horizontal: padding),
        child: child,
      );
    },
  );
}

class _WarningItem extends StatelessWidget {
  const _WarningItem({required this.title});

  final String title;

  @override
  Widget build(BuildContext context) => SizedBox(
    width: 380,
    child: Padding(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 2),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 4,
            height: 4,
            margin: const EdgeInsets.only(top: 7, right: 6),
            decoration: BoxDecoration(
              color: context.theme.text,
              shape: BoxShape.circle,
            ),
          ),
          Expanded(
            child: Text(
              title,
              style: TextStyle(color: context.theme.text, fontSize: 14),
            ),
          ),
        ],
      ),
    ),
  );
}

String _durationLabel(BuildContext context, Duration duration) {
  if (duration == Duration.zero) return context.l10n.disabled;
  if (duration.inMinutes < 60) {
    return context.l10n.minute(duration.inMinutes, duration.inMinutes);
  }
  return context.l10n.hour(duration.inHours, duration.inHours);
}

void _showFailure(BuildContext context, [String? message]) {
  ScaffoldMessenger.maybeOf(
    context,
  )?.showSnackBar(SnackBar(content: Text(message ?? context.l10n.failed)));
}

const _inputBorder = OutlineInputBorder(
  borderSide: BorderSide(color: Colors.transparent),
  borderRadius: BorderRadius.all(Radius.circular(8)),
  gapPadding: 0,
);
