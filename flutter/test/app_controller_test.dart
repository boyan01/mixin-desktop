import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/controllers/app_controller.dart';
import 'package:mixin_desktop_ui/src/rust/desktop_api.dart';

void main() {
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

class _FakeAccountHandle
    implements
        AccountHandle,
        AttachmentAccess,
        ConversationAccess,
        MessageAccess,
        StickerAccess,
        UserAccess {
  @override
  AttachmentAccess attachment() => this;

  @override
  ConversationAccess conversation() => this;

  @override
  MessageAccess message() => this;

  @override
  StickerAccess sticker() => this;

  @override
  UserAccess user() => this;

  _FakeAccountHandle({this.error});

  final Object? error;
  var signOutCalls = 0;
  var disposed = false;

  @override
  Future<void> signOut() async {
    signOutCalls++;
    final exception = error;
    if (exception != null) throw exception;
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
