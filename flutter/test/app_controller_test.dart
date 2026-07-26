import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/app_controller.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';

void main() {
  test(
    'initialize enters signed out when Rust reports no saved account',
    () async {
      final controller = AppController(
        desktop: _FakeDesktopHandle(
          restoreError: const CoreError.notFound(),
        ),
      );

      await controller.initialize();

      expect(controller.stage, AppStage.signedOut);
      expect(controller.error, isNull);
      controller.dispose();
    },
  );

  test('initialize enters failed for other restore errors', () async {
    final controller = AppController(
      desktop: _FakeDesktopHandle(
        restoreError: const CoreError.other(message: 'database unavailable'),
      ),
    );

    await controller.initialize();

    expect(controller.stage, AppStage.failed);
    expect(controller.error, contains('database unavailable'));
    controller.dispose();
  });

  test('sign out clears the active account after Rust succeeds', () async {
    final account = _FakeAccountHandle();
    final controller = AppController()..setAccount(account);

    await controller.signOut();

    expect(controller.stage, AppStage.signedOut);
    expect(account.signOutCalls, 1);
    expect(account.disposed, isTrue);
    controller.dispose();
  });

  test('sign out never keeps a potentially stopped account active', () async {
    final account = _FakeAccountHandle(error: StateError('clear auth failed'));
    final controller = AppController()..setAccount(account);

    await expectLater(controller.signOut(), throwsStateError);

    expect(controller.stage, AppStage.signedOut);
    expect(controller.error, contains('clear auth failed'));
    expect(account.disposed, isTrue);
    controller.dispose();
  });
}

class _FakeDesktopHandle implements DesktopHandle {
  _FakeDesktopHandle({required this.restoreError});

  final Exception restoreError;
  var _isDisposed = false;

  @override
  Future<AccountHandle> restoreAccount() async => throw restoreError;

  @override
  void dispose() => _isDisposed = true;

  @override
  bool get isDisposed => _isDisposed;

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _FakeAccountHandle
    implements
        AccountHandle,
        ConversationAccess,
        MessageAccess,
        StickerAccess,
        UserAccess {
  _FakeAccountHandle({this.error});
  @override
  AttachmentAccess attachment() => _attachment;

  final AttachmentAccess _attachment = _FakeAttachmentAccess();

  @override
  ConversationAccess conversation() => this;

  @override
  MessageAccess message() => this;

  @override
  StickerAccess sticker() => this;

  @override
  UserAccess user() => this;

  final Object? error;
  int signOutCalls = 0;
  bool disposed = false;

  @override
  Future<void> signOut() async {
    signOutCalls++;
    final exception = error;
    if (exception is Error) throw exception;
    if (exception is Exception) throw exception;
    if (exception != null) throw StateError(exception.toString());
  }

  @override
  Future<void> shutdown() async {}

  @override
  void dispose() => disposed = true;

  @override
  bool get isDisposed => disposed;

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _FakeAttachmentAccess implements AttachmentAccess {
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}
