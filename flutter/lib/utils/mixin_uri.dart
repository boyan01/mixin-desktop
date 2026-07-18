const _mixinScheme = 'mixin';
const _mixinHost = 'mixin.one';

enum MixinSchemeHost {
  codes,
  pay,
  users,
  transfer,
  device,
  send,
  address,
  withdrawal,
  apps,
  snapshots,
  conversations,
  multisigs,
  swap,
  markets,
  membership,
}

extension MixinUriExtension on Uri {
  bool get isMixinScheme => isScheme(_mixinScheme);

  bool get _isMixinHost => host == _mixinHost || host == 'www.$_mixinHost';

  bool get isMixin => isMixinScheme || _isMixinHost;

  bool _isTypeScheme(MixinSchemeHost type) =>
      isMixinScheme && host == type.name;

  bool _isTypeHost(MixinSchemeHost type) =>
      _isMixinHost &&
      pathSegments.isNotEmpty &&
      pathSegments.first == type.name;

  String? _getValue(MixinSchemeHost type) {
    if (_isTypeScheme(type)) {
      return pathSegments.length == 1 ? pathSegments.single : null;
    }
    if (_isTypeHost(type)) {
      return pathSegments.length > 1 ? pathSegments[1] : null;
    }
    return null;
  }

  String? get appId => _getValue(MixinSchemeHost.apps);

  bool get actionIsOpen => queryParameters['action'] == 'open';

  String? get userId => _getValue(MixinSchemeHost.users);

  String? get code => _getValue(MixinSchemeHost.codes);

  String? get conversationId => _getValue(MixinSchemeHost.conversations);

  String? get snapshotTraceId => _isTypeScheme(MixinSchemeHost.snapshots)
      ? queryParameters['trace']
      : null;

  bool get isSend =>
      _isTypeScheme(MixinSchemeHost.send) || _isTypeHost(MixinSchemeHost.send);

  bool get isPay => _isTypeHost(MixinSchemeHost.pay);

  bool get isMultisigs => _isTypeHost(MixinSchemeHost.multisigs);

  bool get isSwap =>
      _isTypeHost(MixinSchemeHost.swap) || _isTypeScheme(MixinSchemeHost.swap);

  bool get isMarkets =>
      _isTypeHost(MixinSchemeHost.markets) ||
      _isTypeScheme(MixinSchemeHost.markets);

  bool get isMembership => _isTypeHost(MixinSchemeHost.membership);

  String? get startTextOfConversation =>
      isMixin ? queryParameters['start'] : null;

  String? get userOfSend => isSend ? queryParameters['user'] : null;

  String? get categoryOfSend => isSend ? queryParameters['category'] : null;

  String? get conversationIdOfSend =>
      isSend ? queryParameters['conversation'] : null;

  String? get dataOfSend => isSend ? queryParameters['data'] : null;
}
