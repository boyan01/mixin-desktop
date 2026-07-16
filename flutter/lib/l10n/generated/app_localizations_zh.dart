// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get aMessage => '一条消息';

  @override
  String get about => '关于';

  @override
  String get account => '账号';

  @override
  String get activity => '活动';

  @override
  String get add => '添加';

  @override
  String get addACaption => '添加说明';

  @override
  String get addBotWithPlus => '+ 添加机器人';

  @override
  String get addContact => '添加联系人';

  @override
  String get addContactWithPlus => '+ 添加联系人';

  @override
  String get addFile => '添加文件';

  @override
  String get addGroupDescription => '添加群公告';

  @override
  String get addParticipants => '添加成员';

  @override
  String get addPeopleSearchHint => 'Mixin ID 或手机号';

  @override
  String get addProxy => '添加代理';

  @override
  String get addSticker => '添加贴纸';

  @override
  String get addStickerFailed => '添加贴纸失败';

  @override
  String get addStickers => '添加贴纸';

  @override
  String get addToCircle => '添加到圈子';

  @override
  String get added => '已添加';

  @override
  String get address => '地址';

  @override
  String get admin => '管理员';

  @override
  String get alertKeyContactContactMessage => '分享了一个联系人';

  @override
  String get allChats => '全部聊天';

  @override
  String get animalsAndNature => '动物与自然';

  @override
  String get anonymous => '匿名';

  @override
  String get anonymousNumber => '匿名号码';

  @override
  String get appCardShareDisallow => '此链接无法分享';

  @override
  String get appearance => '外观';

  @override
  String get archivedFolder => '存档文件夹';

  @override
  String get assetType => '资产类型';

  @override
  String get audio => '语音';

  @override
  String get audios => '音频';

  @override
  String get autoBackup => '自动备份';

  @override
  String get autoLock => '自动锁定';

  @override
  String get avatar => '头像';

  @override
  String get backup => '备份';

  @override
  String get backupChat => '备份聊天记录';

  @override
  String get backupToOtherDevice => '备份到其他设备';

  @override
  String get backupToOtherDeviceTips => '将聊天记录备份到其他设备。请确保两台设备连接到同一个 Wi-Fi 或热点。';

  @override
  String get backupWaitingOtherDevice => '请在另一台设备上打开 Mixin，并在那边开始恢复。';

  @override
  String get biography => '简介';

  @override
  String get biometric => '生物识别';

  @override
  String get block => '屏蔽用户';

  @override
  String get botNotFound => '找不到这个机器人';

  @override
  String get bots => '机器人';

  @override
  String get botsTitle => '机器人';

  @override
  String get bringAllToFront => '前置所有窗口';

  @override
  String get canNotRecognizeQrCode => '无法识别二维码';

  @override
  String get cancel => '取消';

  @override
  String get card => '卡片';

  @override
  String get change => '更改';

  @override
  String get changeNumber => '修改手机号';

  @override
  String get changeNumberInstead => '仅修改手机号码';

  @override
  String changedDisappearingMessageSettings(Object arg0) {
    return '$arg0修改了限时消息设置';
  }

  @override
  String get chatBackup => '聊天记录备份';

  @override
  String get chatBackupAndRestore => '聊天记录备份与恢复';

  @override
  String get chatBotReceptionTitle => '点击按钮使用机器人';

  @override
  String chatDecryptionFailedHint(Object arg0) {
    return '等待$arg0上线后建立加密会话。';
  }

  @override
  String chatDeleteMessage(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '删除 $arg0 条消息吗？',
      one: '删除 $arg0 条消息吗？',
    );
    return '$_temp0';
  }

  @override
  String chatGroupAdd(Object arg0, Object arg1) {
    return '$arg0添加了$arg1';
  }

  @override
  String chatGroupExit(Object arg0) {
    return '$arg0离开了群组';
  }

  @override
  String chatGroupJoin(Object arg0) {
    return '$arg0通过邀请链接加入群组';
  }

  @override
  String chatGroupRemove(Object arg0, Object arg1) {
    return '$arg0移除了$arg1';
  }

  @override
  String get chatHintE2e => '端对端加密';

  @override
  String get chatNotSupportUriOnPhone => '不支持此链接，请在手机上查看。';

  @override
  String get chatNotSupportUrl =>
      'https://support.mixin.one/zh/article/5ye6546w4occ6lz5liq57g75z6l55qe5rai5ogv5lin5psv5oyb4ocd5oco5lmi5yqe77yf-h92cxa/';

  @override
  String get chatNotSupportViewOnPhone => '不支持此类型消息，请在手机上查看。';

  @override
  String chatPinMessage(Object arg0, Object arg1) {
    return '$arg0置顶了$arg1';
  }

  @override
  String get chatTextSize => '聊天字体大小';

  @override
  String get chats => '聊天';

  @override
  String get checkNewVersion => '检查新版本';

  @override
  String circleSubtitle(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 会话',
      one: '$arg0 会话',
    );
    return '$_temp0';
  }

  @override
  String circleTitle(Object arg0) {
    return '$arg0的圈子';
  }

  @override
  String get circles => '圈子';

  @override
  String get clear => '清除';

  @override
  String get clearChat => '清除聊天记录';

  @override
  String get clearFilter => '清除筛选条件';

  @override
  String get clickToReloadQrcode => '点击重新加载二维码';

  @override
  String get close => '关闭';

  @override
  String get closeWindow => '关闭窗口';

  @override
  String get closingBalance => '期末余额';

  @override
  String get collapse => '折叠';

  @override
  String get collectible => '藏品';

  @override
  String get collectibles => '藏品';

  @override
  String get collection => '合集';

  @override
  String get combineAndForward => '合并转发';

  @override
  String get confirm => '确认';

  @override
  String get confirmPasscodeDesc => '再次确认密码';

  @override
  String get confirmSyncChatsFromPhone => '确认从手机端同步聊天记录吗？';

  @override
  String get confirmSyncChatsToPhone => '确认同步聊天记录到手机端吗？';

  @override
  String get confirmations => '区块确认数';

  @override
  String get contact => '联系人';

  @override
  String contactMixinId(Object arg0) {
    return 'Mixin ID：$arg0';
  }

  @override
  String get contactMuteTitle => '静音通知';

  @override
  String get contactTitle => '联系人';

  @override
  String get contentTooLong => '内容过长';

  @override
  String get contentVoice => '[语音电话]';

  @override
  String get continueText => '继续';

  @override
  String get conversation => '会话';

  @override
  String conversationDeleteTitle(Object arg0) {
    return '删除会话：$arg0';
  }

  @override
  String get copy => '复制';

  @override
  String get copyImage => '复制图片';

  @override
  String get copyInvite => '复制邀请链接';

  @override
  String get copyLink => '复制链接';

  @override
  String get copySelectedText => '复制已选择的文本';

  @override
  String get copyText => '复制文字';

  @override
  String get create => '创建';

  @override
  String get createCircle => '新建圈子';

  @override
  String get createConversation => '新建会话';

  @override
  String get createGroup => '新建群组';

  @override
  String createdAt(Object arg0) {
    return '创建于 $arg0';
  }

  @override
  String createdThisGroup(Object arg0) {
    return '$arg0创建了这个群组';
  }

  @override
  String get customTime => '自定义时间';

  @override
  String get dark => '深色';

  @override
  String get dataAndStorageUsage => '数据与存储空间';

  @override
  String get dataError => '数据错误';

  @override
  String get dataLoading => '数据加载中，请稍后';

  @override
  String get databaseCorruptedTips => '数据库已损坏，暂无法恢复。点击继续将重新创建一个新的数据库文件。';

  @override
  String get databaseLockedTips => '数据库文件已被锁定，无法访问。请尝试重启应用或者重启系统后再试。';

  @override
  String get databaseNotADbTips => '无法打开数据库，文件不是一个有效的数据库文件。';

  @override
  String get databaseRecreateTips => '重新创建一个新的数据库文件，旧文件将被删除。';

  @override
  String get databaseUpgradeTips => '正在进行数据库升级，可能需要几分钟，请不要强制关闭应用。';

  @override
  String get delete => '删除';

  @override
  String get deleteAccountDetailHint => '本地消息和 iCloud 备份不会被自动删除';

  @override
  String get deleteAccountHint => '删除你的账户和个人照片';

  @override
  String get deleteChat => '删除聊天';

  @override
  String get deleteChatDescription => '删除会话只会删除此设备的聊天记录，不会影响其他设备。';

  @override
  String get deleteCircle => '删除圈子';

  @override
  String get deleteForEveryone => '撤回';

  @override
  String get deleteForMe => '删除';

  @override
  String get deleteGroup => '删除群组';

  @override
  String get deleteMyAccount => '删除账号';

  @override
  String deleteTheCircle(Object arg0) {
    return '确定删除$arg0圈子吗？';
  }

  @override
  String get deposit => '充值';

  @override
  String get depositHash => '充值哈希';

  @override
  String get developer => '开发者';

  @override
  String get deviceTransferFailed => '同步失败';

  @override
  String disableDisappearingMessage(Object arg0) {
    return '$arg0禁用了限时消息';
  }

  @override
  String get disabled => '禁用';

  @override
  String disappearingCustomTimeMaxWarning(Object arg0) {
    return '最高设置 $arg0';
  }

  @override
  String get disappearingMessage => '限时消息';

  @override
  String get disappearingMessageHint =>
      '启用后，在此聊天中发送和接收的新信息在被看到后会消失，阅读文档以**了解更多**。';

  @override
  String get discard => '废弃';

  @override
  String get discardRecordingWarning => '是否要停止并废弃已录制的语音消息？';

  @override
  String get dismissAsAdmin => '撤销管理员身份';

  @override
  String get done => '完成';

  @override
  String get download => '下载';

  @override
  String get downloadLink => '下载链接：';

  @override
  String get draft => '草稿';

  @override
  String get dragAndDropFileHere => '拖放文件到此处';

  @override
  String get durationIsTooShort => '时间太短';

  @override
  String get edit => '编辑';

  @override
  String get editCircleName => '编辑名称';

  @override
  String get editConversations => '管理圈子';

  @override
  String get editGroupDescription => '编辑群公告';

  @override
  String get editGroupName => '编辑名称';

  @override
  String get editImageClearWarning => '退出将会清除此次所有的改动。';

  @override
  String get editName => '修改昵称';

  @override
  String get editProfile => '编辑资料';

  @override
  String get enablePushNotification => '启用推送通知';

  @override
  String get encryptZipFileWithPassword => '使用密码来加密 zip 文件';

  @override
  String get enterPinToDeleteAccount => '输入你的 PIN 以注销你的账户';

  @override
  String get enterToSend => '按下回车 ⏎ 发送';

  @override
  String get enterYourPhoneNumber => '输入你的手机号码';

  @override
  String get enterYourPinToContinue => '输入你的 PIN 以继续';

  @override
  String get errorAccessLimited => '错误 403：访问受限';

  @override
  String get errorAddressExists => '地址不存在，请确保地址是否添加成功';

  @override
  String get errorAddressNotSync => '地址刷新失败，请重试';

  @override
  String get errorAlreadyBondedReferralCode => '错误 10731：当前账号已绑定邀请码，无法修改绑定。';

  @override
  String get errorAssetExists => '没有相关资产';

  @override
  String get errorAuthentication => '错误 401：请重新登录';

  @override
  String get errorBadData => '错误 10002：请求数据不合法';

  @override
  String get errorBlockchain => '错误 30100：区块链同步异常，请稍后重试';

  @override
  String get errorConnectionTimeout => '网络连接超时';

  @override
  String get errorFullGroup => '错误 20116：群组已满';

  @override
  String get errorInsufficientBalance => '错误 20117：余额不足';

  @override
  String errorInsufficientTransactionFeeWithAmount(Object arg0) {
    return '错误 20124：手续费不足。请确保钱包至少有 $arg0 当作手续费。';
  }

  @override
  String errorInvalidAddress(Object arg0, Object arg1) {
    return '错误 30102：地址格式错误。请输入正确的 $arg0 $arg1 的地址！';
  }

  @override
  String get errorInvalidAddressPlain => '错误 30102：地址格式错误。';

  @override
  String get errorInvalidCodeTooFrequent => '错误 20129：发送验证码太频繁，请稍后再试';

  @override
  String get errorInvalidEmergencyContact => '错误 20130：恢复联系人不正确';

  @override
  String get errorInvalidPinFormat => '错误 20118：PIN 格式不正确';

  @override
  String get errorInviterPlanExpired => '错误 10737：邀请人的会员已过期';

  @override
  String get errorLegacyPin =>
      '错误 20118：为了加强 Mixin 网络的安全，Mixin API 现已暂停 D3M-PIN 升级到 TIP，详情请查看文档并登记等待处理。';

  @override
  String get errorNetworkTaskFailed => '网络连接失败。检查或切换网络并重试';

  @override
  String get errorNoPinToken => '缺少凭据，请重新登录之后再尝试使用此功能。';

  @override
  String get errorNotFound => '错误 404：没有找到相应的信息';

  @override
  String get errorNotSupportedAudioFormat => '不支持的音频格式，请用其他app打开。';

  @override
  String get errorNumberReachedLimit => '错误 20132： 已达到上限';

  @override
  String errorOldVersion(Object arg0) {
    return '错误 10006：请更新 Mixin（$arg0） 至最新版。';
  }

  @override
  String get errorOpenLocation => '无法找到地图应用';

  @override
  String get errorPermission => '请开启相关权限';

  @override
  String get errorPhoneInvalidFormat => '错误 20110：手机号码不合法';

  @override
  String get errorPhoneSmsDelivery => '错误 10003：发送短信失败';

  @override
  String get errorPhoneVerificationCodeExpired => '错误 20114：验证码已过期';

  @override
  String get errorPhoneVerificationCodeInvalid => '错误 20113：验证码错误';

  @override
  String get errorPinCheckTooManyRequest => '你已经尝试了超过 5 次，请等待 24 小时后再次尝试。';

  @override
  String get errorPinIncorrect => '错误 20119：PIN 不正确';

  @override
  String errorPinIncorrectWithTimes(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '错误 20119：PIN 不正确。你还有 $arg0 次机会，使用完需等待 24 小时后再次尝试。',
      one: '错误 20119：PIN 不正确。你还有 $arg0 次机会，使用完需等待 24 小时后再次尝试。',
    );
    return '$_temp0';
  }

  @override
  String get errorRecaptchaIsInvalid => '错误 10004：验证失败';

  @override
  String errorServer5xxCode(Object arg0) {
    return '服务器出错，请稍后重试：$arg0';
  }

  @override
  String get errorTooManyRequest => '错误 429：请求过于频繁';

  @override
  String get errorTooManyStickers => '错误 20126：贴纸数已达上限';

  @override
  String get errorTooSmallTransferAmount => '错误 20120：转账金额太小';

  @override
  String get errorTooSmallWithdrawAmount => '错误 20127：提现金额太小';

  @override
  String get errorTranscriptForward => '请在所有附件下载完成之后再转发';

  @override
  String get errorTransferToDeactivatedUser => '错误 20160：无法给已经删除的账号转账';

  @override
  String get errorUnableToOpenMedia => '无法找到能打开该媒体的应用';

  @override
  String errorUnknownWithCode(Object arg0) {
    return '错误：$arg0';
  }

  @override
  String errorUnknownWithMessage(Object arg0) {
    return '错误：$arg0';
  }

  @override
  String get errorUploadAttachmentFailed => '消息附件上传失败';

  @override
  String get errorUsedPhone => '错误 20122：电话号码已经被占用。';

  @override
  String get errorUserInvalidFormat => '用户数据不合法';

  @override
  String get errorWithdrawalMemoFormatIncorrect => '错误 20131：提现备注格式不正确';

  @override
  String get errorWithdrawalSuspend => '错误 20137：提现暂停';

  @override
  String get exit => '退出';

  @override
  String get exitGroup => '退出群组';

  @override
  String get failed => '失败';

  @override
  String get failedToOpenDatabase => '打开数据库时出现了错误。';

  @override
  String get fee => '手续费';

  @override
  String get file => '文件';

  @override
  String get fileChooserError => '文件选择错误';

  @override
  String get fileDoesNotExist => '文件不存在';

  @override
  String get fileError => '文件错误';

  @override
  String get files => '文档';

  @override
  String get flags => '旗帜';

  @override
  String get followSystem => '跟随系统';

  @override
  String get followUsOnFacebook => '关注我们的 Facebook';

  @override
  String get followUsOnX => '关注我们的 X';

  @override
  String get foodAndDrink => '食物与饮料';

  @override
  String get formatNotSupported => '不支持该格式';

  @override
  String get forward => '转发';

  @override
  String get from => '来自';

  @override
  String get fromWithColon => '来自：';

  @override
  String get generateQrcode => '生成二维码';

  @override
  String get groupAlreadyIn => '你已经在该群组里';

  @override
  String get groupCantSend => '您不能发送消息，因为您已经不再是此群组成员。';

  @override
  String get groupName => '群组名称';

  @override
  String get groupParticipants => '群成员';

  @override
  String groupPopMenuMessage(Object arg0) {
    return '发送消息至 $arg0';
  }

  @override
  String groupPopMenuRemove(Object arg0) {
    return '移除 $arg0';
  }

  @override
  String get groups => '群组';

  @override
  String get groupsInCommon => '共同群组';

  @override
  String get hash => '哈希';

  @override
  String get help => '帮助';

  @override
  String get helpCenter => '帮助中心';

  @override
  String get hideMixin => '隐藏 Mixin';

  @override
  String get host => '主机名';

  @override
  String hour(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 小时',
      one: '$arg0 小时',
    );
    return '$_temp0';
  }

  @override
  String get howAreYou => '嗨，你好吗？';

  @override
  String get iAmGood => '我很好。';

  @override
  String get id => 'ID';

  @override
  String get ignoreThisVersion => '忽略这次版本更新';

  @override
  String get image => '图像';

  @override
  String get includeFiles => '包含文件';

  @override
  String get includeVideos => '包括视频';

  @override
  String get initializing => '初始化…';

  @override
  String get invalidStickerFormat => '贴纸格式不支持';

  @override
  String get inviteInfo => 'Mixin 使用者可以使用此链接加入这个群组，请只跟您信任的人共享链接。';

  @override
  String get inviteToGroupViaLink => '群邀请链接';

  @override
  String get joinGroupWithPlus => '+ 加入群组';

  @override
  String joinedIn(Object arg0) {
    return '加入于 $arg0';
  }

  @override
  String landingDeleteContent(Object arg0, Object arg1) {
    return '您在 $arg0 申请了删除账号，账号将于 $arg1 被删除，如果您继续登录，删除您账户的请求将被取消。';
  }

  @override
  String landingInvitationDialogContent(Object arg0) {
    return '我们将发送4位验证码到手机 $arg0, 请在下一个页面输入';
  }

  @override
  String landingValidationTitle(Object arg0) {
    return '请输入发送至以下号码的 4 位验证码：$arg0';
  }

  @override
  String get learnMore => '了解更多';

  @override
  String get less => '更少';

  @override
  String get light => '浅色';

  @override
  String get linkedDevice => '连接的设备';

  @override
  String get live => '直播';

  @override
  String get loading => '正在加载...';

  @override
  String get loadingTime => '检测到系统时间异常，请校正后再继续使用';

  @override
  String get locateToChat => '前往聊天';

  @override
  String get location => '位置';

  @override
  String get lock => '锁定';

  @override
  String get logIn => '登录';

  @override
  String get loginAndAbortAccountDeletion => '继续登录并放弃删除账户';

  @override
  String get loginByQrcode => '通过二维码登录 Mixin Messenger';

  @override
  String get loginByQrcodeTips1 => '打开手机上的 Mixin Messenger。';

  @override
  String get loginByQrcodeTips2 => '扫描屏幕上的二维码，确认登录。';

  @override
  String get makeGroupAdmin => '设定为群组管理员';

  @override
  String get media => '媒体';

  @override
  String get memo => '备注';

  @override
  String get messageE2ee => '此对话中的消息使用端对端加密。点击了解更多。';

  @override
  String get messageNotFound => '找不到该消息';

  @override
  String get messageNotSupport => '不支持此类消息，请升级 Mixin 到最新版本。';

  @override
  String get messagePreview => '消息预览';

  @override
  String get messagePreviewDescription => '预览新消息通知中的消息文本。';

  @override
  String get messages => '消息';

  @override
  String get minimize => '最小化';

  @override
  String minute(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 分钟',
      one: '$arg0 分钟',
    );
    return '$_temp0';
  }

  @override
  String get mixinMessengerDesktop => 'Mixin Messenger 桌面';

  @override
  String get more => '更多';

  @override
  String get multisigTransaction => '多重签名交易';

  @override
  String get mute => '静音';

  @override
  String myMixinId(Object arg0) {
    return '我的 Mixin ID：$arg0';
  }

  @override
  String get myStickers => '我的表情';

  @override
  String get na => '暂无价格';

  @override
  String get name => '名称';

  @override
  String get networkConnectionFailed => '网络连接失败';

  @override
  String get networkError => '网络错误';

  @override
  String get newVersionAvailable => '发现新版本';

  @override
  String newVersionDescription(Object arg0, Object arg1) {
    return '发现新版本 Mixin Messenger $arg0，当前版本为 $arg1。是否要下载最新的版本？';
  }

  @override
  String get next => '下一步';

  @override
  String get nextConversation => '下一个会话';

  @override
  String get noAudio => '没有音频';

  @override
  String get noCamera => '没有相机';

  @override
  String get noData => '没有数据';

  @override
  String get noFiles => '没有文件';

  @override
  String get noLinks => '没有链接';

  @override
  String get noMedia => '没有媒体';

  @override
  String get noNetworkConnection => '无网络连接';

  @override
  String get noPosts => '没有文章';

  @override
  String get noResults => '未找到相关结果';

  @override
  String get notFound => '没有找到相应的消息';

  @override
  String get notSupportBiometric => '此设备不支持生物验证';

  @override
  String get notificationContent => '启用推送通知以实时更新价格警报和消息。';

  @override
  String get notificationPermissionManually => '未允许通知，请到通知设置开启。';

  @override
  String get notifications => '通知';

  @override
  String nowAnAddmin(Object arg0) {
    return '$arg0现在是管理员';
  }

  @override
  String get objects => '物件';

  @override
  String get oneByOneForward => '逐条转发';

  @override
  String get oneHour => '1 小时';

  @override
  String get oneYear => '1 年';

  @override
  String get open => '打开';

  @override
  String get openHomePage => '打开主页';

  @override
  String openLink(Object arg0) {
    return '打开链接：$arg0';
  }

  @override
  String get openLogDirectory => '打开日志文件夹';

  @override
  String get openingBalance => '期初余额';

  @override
  String get originalImage => '原图';

  @override
  String get owner => '群主';

  @override
  String participantsCount(Object arg0) {
    return '$arg0 位群组成员';
  }

  @override
  String get passcodeIncorrect => '密码不正确';

  @override
  String get password => '密码';

  @override
  String pendingConfirmation(Object arg0, Object arg1, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0/$arg1 确认',
      one: '$arg0/$arg1 确认',
    );
    return '$_temp0';
  }

  @override
  String get phoneNumber => '手机号码';

  @override
  String get photos => '照片';

  @override
  String get pickAConversation => '选择一个对话，开始发送信息';

  @override
  String get picturesAndVideos => '图像 & 视频';

  @override
  String get pinTitle => '置顶';

  @override
  String pinnedMessageTitle(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 条置顶消息',
      one: '$arg0 条置顶消息',
    );
    return '$_temp0';
  }

  @override
  String get port => '端口号';

  @override
  String get post => '文章';

  @override
  String get preferences => '偏好设置';

  @override
  String get previousConversation => '上一个会话';

  @override
  String get privacyPolicy => '隐私政策';

  @override
  String get proxy => '代理';

  @override
  String get proxyAuth => '验证（可选）';

  @override
  String get proxyConnection => '连接';

  @override
  String get proxyType => '代理类型';

  @override
  String get qrCodeExpiredDesc => '二维码过期，请重试';

  @override
  String get quickSearch => '快速搜索';

  @override
  String get quitMixin => '退出 Mixin';

  @override
  String get raw => '其他';

  @override
  String get rebate => '退款';

  @override
  String get recaptchaTimeout => '验证超时';

  @override
  String get receiver => '至';

  @override
  String get recentChats => '最近聊天';

  @override
  String get reedit => '重新编辑';

  @override
  String get refresh => '刷新';

  @override
  String get removeBot => '删除机器人';

  @override
  String get removeChatFromCircle => '从圈子里移除对话';

  @override
  String get removeContact => '删除联系人';

  @override
  String get removeStickers => '移除所有表情';

  @override
  String get reply => '回复';

  @override
  String get report => '举报';

  @override
  String get reportAndBlock => '举报并屏蔽？';

  @override
  String get reportTitle => '给开发人员发送聊天日志？';

  @override
  String get resendCode => '重发验证码';

  @override
  String resendCodeIn(Object arg0) {
    return '$arg0 秒后重新发送验证码';
  }

  @override
  String get reset => '重置';

  @override
  String get resetLink => '重置邀请链接';

  @override
  String get restoreChat => '恢复聊天记录';

  @override
  String get restoreChatTip => '从其他设备恢复聊天记录。请确保两台设备连接到同一个 Wi-Fi 或热点。';

  @override
  String get restoreFromOtherDevice => '从其他设备恢复';

  @override
  String get retry => '重试';

  @override
  String get retryUploadFailed => '重新上传失败。';

  @override
  String get revokeMultisigTransaction => '撤销多重签名交易';

  @override
  String get save => '保存';

  @override
  String get saveAs => '另存为';

  @override
  String get saveToCameraRoll => '保存到相册';

  @override
  String get sayHi => '打招呼';

  @override
  String get scamWarning => '警告：此账号被大量用户举报，请谨防网络诈骗，注意个人财产安全';

  @override
  String get screenPasscode => '锁屏密码';

  @override
  String get search => '搜索';

  @override
  String get searchContact => '搜索用户';

  @override
  String get searchConversation => '搜索聊天记录';

  @override
  String get searchEmpty => '找不到联系人或消息。';

  @override
  String get searchPlaceholderNumber => '搜索 Mixin ID 或手机号码：';

  @override
  String searchRelatedMessage(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 条相关消息',
      one: '$arg0 条相关消息',
    );
    return '$_temp0';
  }

  @override
  String get searchUnread => '搜索未读会话';

  @override
  String get secretUrl => 'https://mixin.one/pages/1000007';

  @override
  String get security => '安全';

  @override
  String get select => '选择';

  @override
  String get send => '发送';

  @override
  String get sendArchived => '打包成 zip 发送';

  @override
  String get sendQuickly => '快速发送';

  @override
  String get sendToDeveloper => '把日志发给开发者';

  @override
  String get sendWithoutCompression => '发送原始文件';

  @override
  String get sendWithoutSound => '静音发送';

  @override
  String get set => '设置';

  @override
  String setDisappearingMessageTimeTo(Object arg0, Object arg1) {
    return '$arg0将限时消息设置为 $arg1';
  }

  @override
  String get setPasscodeDesc => '设置解锁密码';

  @override
  String get settingAuthSearchHint => 'Mixin ID, 昵称';

  @override
  String get settingBackupTips =>
      '备份你的聊天记录到 iCloud。如果你丢失或者更换手机，你可以在重新安装 Mixin Messenger 时恢复你的聊天记录。注意备份到 iCloud 中的聊天记录不受端对端加密保护！';

  @override
  String settingDeleteAccountPinContent(Object arg0) {
    return '如果您继续，您的个人资料和账户信息将在$arg0被删除。阅读我们的文档以**了解更多**。';
  }

  @override
  String get settingDeleteAccountUrl =>
      'https://support.mixin.one/zh/article/5aac5l2v5yig6zmk6lsm5y377yf-1uteq30';

  @override
  String get share => '分享';

  @override
  String get shareApps => '分享的应用';

  @override
  String get shareContact => '分享联系人';

  @override
  String get shareError => '分享出错';

  @override
  String get shareLink => '分享邀请链接';

  @override
  String shareMessageDescription(Object arg0, Object arg1) {
    return '你确定要发送来自$arg0的$arg1？';
  }

  @override
  String shareMessageDescriptionEmpty(Object arg0) {
    return '你确定要发送该$arg0？';
  }

  @override
  String get sharedMedia => '媒体内容';

  @override
  String get show => '显示';

  @override
  String get showAvatar => '显示头像';

  @override
  String get showIdentityNumber => '显示 ID';

  @override
  String get showMixin => '显示 Mixin';

  @override
  String get signIn => '登录';

  @override
  String get signOut => '退出登录';

  @override
  String get signWithMobileNumber => '通过手机号登录';

  @override
  String get signWithQrcode => '通过二维码登录';

  @override
  String get smileysAndPeople => '表情符号与人物';

  @override
  String get snapshotHash => '快照哈希';

  @override
  String get status => '交易状态';

  @override
  String get sticker => '贴纸';

  @override
  String get stickerAddInvalidSize =>
      '贴纸要求大于 1KB 且小于 1MB，宽高大于 128 像素且小于 1024 像素。';

  @override
  String get stickerAlbumDetail => '表情详情';

  @override
  String get stickerStore => '表情商店';

  @override
  String get storageAutoDownloadDescription => '更改媒体的自动下载设置。';

  @override
  String get storageUsage => '储存空间';

  @override
  String get strangerHint => '对方不是你的联系人';

  @override
  String get strangers => '陌生人';

  @override
  String get successful => '成功';

  @override
  String get symbols => '符号';

  @override
  String get syncFromOtherDevice => '从其他设备同步';

  @override
  String get syncToOtherDevice => '同步到其他设备';

  @override
  String get termsOfService => '服务条款';

  @override
  String get text => '文字';

  @override
  String get theme => '主题';

  @override
  String get thisMessageWasDeleted => '此消息已撤回';

  @override
  String get time => '时间';

  @override
  String get to => '至';

  @override
  String get today => '今天';

  @override
  String get toggleChatInfo => '展开/关闭会话信息';

  @override
  String get trace => 'Trace';

  @override
  String get transactionHash => '交易哈希';

  @override
  String get transactionId => '交易编号';

  @override
  String get transactionType => '交易类型';

  @override
  String get transactions => '交易记录';

  @override
  String get transactionsCannotBeDeleted => '交易记录不会被删除';

  @override
  String get transcript => '聊天记录';

  @override
  String get transfer => '转账';

  @override
  String get transferCompleted => '同步完成';

  @override
  String get transferProtocolVersionNotMatched => '版本不匹配，无法同步数据，请先升级应用。';

  @override
  String get transferringChats => '同步聊天记录中';

  @override
  String get transferringChatsTips => '同步时请不要关闭屏幕并保持 Mixin 在前台运行。';

  @override
  String get travelAndPlaces => '旅行与地点';

  @override
  String get typeMessage => '输入消息';

  @override
  String unableToOpenFile(Object arg0) {
    return '无法打开文件：$arg0';
  }

  @override
  String get unblock => '解除屏蔽';

  @override
  String unitDay(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '天',
      one: '天',
    );
    return '$_temp0';
  }

  @override
  String unitHour(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '小时',
      one: '小时',
    );
    return '$_temp0';
  }

  @override
  String unitMinute(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '分',
      one: '分',
    );
    return '$_temp0';
  }

  @override
  String unitSecond(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '秒',
      one: '秒',
    );
    return '$_temp0';
  }

  @override
  String unitWeek(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '周',
      one: '周',
    );
    return '$_temp0';
  }

  @override
  String get unknowError => '未知错误';

  @override
  String get unlockMixinMessenger => '解锁 Mixin Messenger';

  @override
  String get unlockWithWasscode => '输入密码解锁 Mixin Messenger';

  @override
  String get unmute => '取消静音';

  @override
  String get unpin => '取消置顶';

  @override
  String get unpinAllMessages => '取消所有置顶消息';

  @override
  String get unpinAllMessagesConfirmation => '确定取消置顶所有消息么？';

  @override
  String get unreadMessages => '未读消息';

  @override
  String get updateMixin => '升级 Mixin';

  @override
  String updateMixinDescription(Object arg0) {
    return '当前版本（$arg0）不再可用!\n请点击下面的“更新”按钮以更新到最新版本。';
  }

  @override
  String get upgrade => '升级';

  @override
  String get upgrading => '升级中...';

  @override
  String get useBiometric => '使用生物识别';

  @override
  String get userDeleteHint => '该用户已经删除了账号。';

  @override
  String get userNotFound => '找不到这个用户';

  @override
  String get username => '用户名';

  @override
  String valueNow(Object arg0) {
    return '价值 $arg0';
  }

  @override
  String valueThen(Object arg0) {
    return '当时价值 $arg0';
  }

  @override
  String get verifyPin => '验证 PIN';

  @override
  String get video => '视频';

  @override
  String get videos => '视频';

  @override
  String get waitingForThisMessage => '正在等待这条消息。';

  @override
  String get waitingOtherDeviceConnection => '等待其他设备连接。';

  @override
  String get webview2RuntimeInstallDescription =>
      '该设备暂未安装 WebView2 组件，请先下载并安装 WebView2 Runtime。';

  @override
  String get webviewRuntimeUnavailable => 'WebView2 组件不可用';

  @override
  String get window => '窗口';

  @override
  String get withdrawal => '提现';

  @override
  String get withdrawalHash => '提现哈希';

  @override
  String get you => '你';

  @override
  String get youDeletedThisMessage => '你撤回了一条消息';

  @override
  String get zoom => '缩放';
}

/// The translations for Chinese, as used in Hong Kong (`zh_HK`).
class AppLocalizationsZhHk extends AppLocalizationsZh {
  AppLocalizationsZhHk() : super('zh_HK');

  @override
  String get aMessage => '一條消息';

  @override
  String get about => '關於';

  @override
  String get account => '賬號';

  @override
  String get activity => '活動';

  @override
  String get add => '添加';

  @override
  String get addACaption => '添加説明';

  @override
  String get addBotWithPlus => '+ 添加機器人';

  @override
  String get addContact => '添加聯繫人';

  @override
  String get addContactWithPlus => '+ 添加聯繫人';

  @override
  String get addFile => '添加檔案';

  @override
  String get addGroupDescription => '添加羣公告';

  @override
  String get addParticipants => '添加成員';

  @override
  String get addPeopleSearchHint => 'Mixin ID 或手機號';

  @override
  String get addProxy => '添加代理';

  @override
  String get addSticker => '添加貼紙';

  @override
  String get addStickerFailed => '添加貼紙失敗';

  @override
  String get addStickers => '添加貼紙';

  @override
  String get addToCircle => '添加到圈子';

  @override
  String get added => '已添加';

  @override
  String get address => '地址';

  @override
  String get admin => '管理員';

  @override
  String get alertKeyContactContactMessage => '分享了一個聯繫人';

  @override
  String get allChats => '全部聊天';

  @override
  String get animalsAndNature => '動物與自然';

  @override
  String get anonymous => '匿名';

  @override
  String get anonymousNumber => '匿名號碼';

  @override
  String get appCardShareDisallow => '此鏈接無法分享';

  @override
  String get appearance => '外觀';

  @override
  String get archivedFolder => '存檔檔案夾';

  @override
  String get assetType => '資產類型';

  @override
  String get audio => '語音';

  @override
  String get audios => '音頻';

  @override
  String get autoBackup => '自動備份';

  @override
  String get autoLock => '自動鎖定';

  @override
  String get avatar => '頭像';

  @override
  String get backup => '備份';

  @override
  String get backupChat => '備份聊天記錄';

  @override
  String get backupToOtherDevice => '備份到其他設備';

  @override
  String get backupToOtherDeviceTips => '將聊天記錄備份到其他設備。請確保兩台設備連接到同一個 Wi-Fi 或熱點。';

  @override
  String get backupWaitingOtherDevice => '請在另一台設備上打開 Mixin，並在那邊開始恢復。';

  @override
  String get biography => '簡介';

  @override
  String get biometric => '生物識別';

  @override
  String get block => '屏蔽用户';

  @override
  String get botNotFound => '找不到這個機器人';

  @override
  String get bots => '機器人';

  @override
  String get botsTitle => '機器人';

  @override
  String get bringAllToFront => '前置所有窗口';

  @override
  String get canNotRecognizeQrCode => '無法識別二維碼';

  @override
  String get cancel => '取消';

  @override
  String get card => '卡片';

  @override
  String get change => '更改';

  @override
  String get changeNumber => '修改手機號';

  @override
  String get changeNumberInstead => '僅修改手機號碼';

  @override
  String changedDisappearingMessageSettings(Object arg0) {
    return '$arg0修改了限時消息設置';
  }

  @override
  String get chatBackup => '聊天記錄備份';

  @override
  String get chatBackupAndRestore => '聊天記錄備份與恢復';

  @override
  String get chatBotReceptionTitle => '點擊按鈕使用機器人';

  @override
  String chatDecryptionFailedHint(Object arg0) {
    return '等待$arg0上線後建立加密會話。';
  }

  @override
  String chatDeleteMessage(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '刪除 $arg0 條消息嗎？',
      one: '刪除 $arg0 條消息嗎？',
    );
    return '$_temp0';
  }

  @override
  String chatGroupAdd(Object arg0, Object arg1) {
    return '$arg0添加了$arg1';
  }

  @override
  String chatGroupExit(Object arg0) {
    return '$arg0離開了羣組';
  }

  @override
  String chatGroupJoin(Object arg0) {
    return '$arg0通過邀請鏈接加入羣組';
  }

  @override
  String chatGroupRemove(Object arg0, Object arg1) {
    return '$arg0移除了$arg1';
  }

  @override
  String get chatHintE2e => '端對端加密';

  @override
  String get chatNotSupportUriOnPhone => '不支持此鏈接，請在手機上查看。';

  @override
  String get chatNotSupportUrl =>
      'https://support.mixin.one/zh/article/5ye6546w4occ6lz5liq57g75z6l55qe5rai5ogv5lin5psv5oyb4ocd5oco5lmi5yqe77yf-h92cxa/';

  @override
  String get chatNotSupportViewOnPhone => '不支持此類型消息，請在手機上查看。';

  @override
  String chatPinMessage(Object arg0, Object arg1) {
    return '$arg0置頂了$arg1';
  }

  @override
  String get chatTextSize => '聊天字體大小';

  @override
  String get chats => '聊天';

  @override
  String get checkNewVersion => '檢查新版本';

  @override
  String circleSubtitle(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 會話',
      one: '$arg0 會話',
    );
    return '$_temp0';
  }

  @override
  String circleTitle(Object arg0) {
    return '$arg0的圈子';
  }

  @override
  String get circles => '圈子';

  @override
  String get clear => '清除';

  @override
  String get clearChat => '清除聊天記錄';

  @override
  String get clearFilter => '清除篩選條件';

  @override
  String get clickToReloadQrcode => '點擊重新加載二維碼';

  @override
  String get close => '關閉';

  @override
  String get closeWindow => '關閉窗口';

  @override
  String get closingBalance => '期末餘額';

  @override
  String get collapse => '摺疊';

  @override
  String get collectible => '藏品';

  @override
  String get collectibles => '藏品';

  @override
  String get collection => '合集';

  @override
  String get combineAndForward => '合併轉發';

  @override
  String get confirm => '確認';

  @override
  String get confirmPasscodeDesc => '再次確認密碼';

  @override
  String get confirmSyncChatsFromPhone => '確認從手機端同步聊天記錄嗎？';

  @override
  String get confirmSyncChatsToPhone => '確認同步聊天記錄到手機端嗎？';

  @override
  String get confirmations => '區塊確認數';

  @override
  String get contact => '聯繫人';

  @override
  String contactMixinId(Object arg0) {
    return 'Mixin ID：$arg0';
  }

  @override
  String get contactMuteTitle => '靜音通知';

  @override
  String get contactTitle => '聯繫人';

  @override
  String get contentTooLong => '內容過長';

  @override
  String get contentVoice => '[語音電話]';

  @override
  String get continueText => '繼續';

  @override
  String get conversation => '會話';

  @override
  String conversationDeleteTitle(Object arg0) {
    return '刪除會話：$arg0';
  }

  @override
  String get copy => '複製';

  @override
  String get copyImage => '複製圖片';

  @override
  String get copyInvite => '複製邀請鏈接';

  @override
  String get copyLink => '複製鏈接';

  @override
  String get copySelectedText => '複製已選擇的文本';

  @override
  String get copyText => '複製文字';

  @override
  String get create => '創建';

  @override
  String get createCircle => '新建圈子';

  @override
  String get createConversation => '新建會話';

  @override
  String get createGroup => '新建羣組';

  @override
  String createdAt(Object arg0) {
    return '創建於 $arg0';
  }

  @override
  String createdThisGroup(Object arg0) {
    return '$arg0創建了這個羣組';
  }

  @override
  String get customTime => '自定義時間';

  @override
  String get dark => '深色';

  @override
  String get dataAndStorageUsage => '數據與存儲空間';

  @override
  String get dataError => '數據錯誤';

  @override
  String get dataLoading => '數據加載中，請稍後';

  @override
  String get databaseCorruptedTips => '數據庫已損壞，暫無法恢復。點擊繼續將重新創建一個新的數據庫檔案。';

  @override
  String get databaseLockedTips => '數據庫檔案已被鎖定，無法訪問。請嘗試重啓應用或者重啓系統後再試。';

  @override
  String get databaseNotADbTips => '無法打開數據庫，檔案不是一個有效的數據庫檔案。';

  @override
  String get databaseRecreateTips => '重新創建一個新的數據庫檔案，舊檔案將被刪除。';

  @override
  String get databaseUpgradeTips => '正在進行數據庫升級，可能需要幾分鐘，請不要強制關閉應用。';

  @override
  String get delete => '刪除';

  @override
  String get deleteAccountDetailHint => '本地消息和 iCloud 備份不會被自動刪除';

  @override
  String get deleteAccountHint => '刪除你的賬户和個人照片';

  @override
  String get deleteChat => '刪除聊天';

  @override
  String get deleteChatDescription => '刪除會話只會刪除此設備的聊天記錄，不會影響其他設備。';

  @override
  String get deleteCircle => '刪除圈子';

  @override
  String get deleteForEveryone => '撤回';

  @override
  String get deleteForMe => '刪除';

  @override
  String get deleteGroup => '刪除羣組';

  @override
  String get deleteMyAccount => '刪除賬號';

  @override
  String deleteTheCircle(Object arg0) {
    return '確定刪除$arg0圈子嗎？';
  }

  @override
  String get deposit => '充值';

  @override
  String get depositHash => '充值哈希';

  @override
  String get developer => '開發者';

  @override
  String get deviceTransferFailed => '同步失敗';

  @override
  String disableDisappearingMessage(Object arg0) {
    return '$arg0禁用了限時消息';
  }

  @override
  String get disabled => '禁用';

  @override
  String disappearingCustomTimeMaxWarning(Object arg0) {
    return '最高設置 $arg0';
  }

  @override
  String get disappearingMessage => '限時消息';

  @override
  String get disappearingMessageHint =>
      '啓用後，在此聊天中發送和接收的新信息在被看到後會消失，閲讀文檔以**瞭解更多**。';

  @override
  String get discard => '廢棄';

  @override
  String get discardRecordingWarning => '是否要停止並廢棄已錄製的語音消息？';

  @override
  String get dismissAsAdmin => '撤銷管理員身份';

  @override
  String get done => '完成';

  @override
  String get download => '下載';

  @override
  String get downloadLink => '下載鏈接：';

  @override
  String get draft => '草稿';

  @override
  String get dragAndDropFileHere => '拖放檔案到此處';

  @override
  String get durationIsTooShort => '時間太短';

  @override
  String get edit => '編輯';

  @override
  String get editCircleName => '編輯名稱';

  @override
  String get editConversations => '管理圈子';

  @override
  String get editGroupDescription => '編輯羣公告';

  @override
  String get editGroupName => '編輯名稱';

  @override
  String get editImageClearWarning => '退出將會清除此次所有的改動。';

  @override
  String get editName => '修改暱稱';

  @override
  String get editProfile => '編輯資料';

  @override
  String get enablePushNotification => '啓用推送通知';

  @override
  String get encryptZipFileWithPassword => '使用密碼來加密 zip 檔案';

  @override
  String get enterPinToDeleteAccount => '輸入你的 PIN 以註銷你的賬户';

  @override
  String get enterToSend => '按下回車 ⏎ 發送';

  @override
  String get enterYourPhoneNumber => '輸入你的手機號碼';

  @override
  String get enterYourPinToContinue => '輸入你的 PIN 以繼續';

  @override
  String get errorAccessLimited => '錯誤 403：訪問受限';

  @override
  String get errorAddressExists => '地址不存在，請確保地址是否添加成功';

  @override
  String get errorAddressNotSync => '地址刷新失敗，請重試';

  @override
  String get errorAlreadyBondedReferralCode => '錯誤 10731：當前賬號已綁定邀請碼，無法修改綁定。';

  @override
  String get errorAssetExists => '沒有相關資產';

  @override
  String get errorAuthentication => '錯誤 401：請重新登錄';

  @override
  String get errorBadData => '錯誤 10002：請求數據不合法';

  @override
  String get errorBlockchain => '錯誤 30100：區塊鏈同步異常，請稍後重試';

  @override
  String get errorConnectionTimeout => '網絡連接超時';

  @override
  String get errorFullGroup => '錯誤 20116：羣組已滿';

  @override
  String get errorInsufficientBalance => '錯誤 20117：餘額不足';

  @override
  String errorInsufficientTransactionFeeWithAmount(Object arg0) {
    return '錯誤 20124：手續費不足。請確保錢包至少有 $arg0 當作手續費。';
  }

  @override
  String errorInvalidAddress(Object arg0, Object arg1) {
    return '錯誤 30102：地址格式錯誤。請輸入正確的 $arg0 $arg1 的地址！';
  }

  @override
  String get errorInvalidAddressPlain => '錯誤 30102：地址格式錯誤。';

  @override
  String get errorInvalidCodeTooFrequent => '錯誤 20129：發送驗證碼太頻繁，請稍後再試';

  @override
  String get errorInvalidEmergencyContact => '錯誤 20130：恢復聯繫人不正確';

  @override
  String get errorInvalidPinFormat => '錯誤 20118：PIN 格式不正確';

  @override
  String get errorInviterPlanExpired => '錯誤 10737：邀請人的會員已過期';

  @override
  String get errorLegacyPin =>
      '錯誤 20118：為了加強 Mixin 網絡的安全，Mixin API 現已暫停 D3M-PIN 升級到 TIP，詳情請查看文檔並登記等待處理。';

  @override
  String get errorNetworkTaskFailed => '網絡連接失敗。檢查或切換網絡並重試';

  @override
  String get errorNoPinToken => '缺少憑據，請重新登錄之後再嘗試使用此功能。';

  @override
  String get errorNotFound => '錯誤 404：沒有找到相應的信息';

  @override
  String get errorNotSupportedAudioFormat => '不支持的音頻格式，請用其他app打開。';

  @override
  String get errorNumberReachedLimit => '錯誤 20132： 已達到上限';

  @override
  String errorOldVersion(Object arg0) {
    return '錯誤 10006：請更新 Mixin（$arg0） 至最新版。';
  }

  @override
  String get errorOpenLocation => '無法找到地圖應用';

  @override
  String get errorPermission => '請開啓相關權限';

  @override
  String get errorPhoneInvalidFormat => '錯誤 20110：手機號碼不合法';

  @override
  String get errorPhoneSmsDelivery => '錯誤 10003：發送短信失敗';

  @override
  String get errorPhoneVerificationCodeExpired => '錯誤 20114：驗證碼已過期';

  @override
  String get errorPhoneVerificationCodeInvalid => '錯誤 20113：驗證碼錯誤';

  @override
  String get errorPinCheckTooManyRequest => '你已經嘗試了超過 5 次，請等待 24 小時後再次嘗試。';

  @override
  String get errorPinIncorrect => '錯誤 20119：PIN 不正確';

  @override
  String errorPinIncorrectWithTimes(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '錯誤 20119：PIN 不正確。你還有 $arg0 次機會，使用完需等待 24 小時後再次嘗試。',
      one: '錯誤 20119：PIN 不正確。你還有 $arg0 次機會，使用完需等待 24 小時後再次嘗試。',
    );
    return '$_temp0';
  }

  @override
  String get errorRecaptchaIsInvalid => '錯誤 10004：驗證失敗';

  @override
  String errorServer5xxCode(Object arg0) {
    return '服務器出錯，請稍後重試：$arg0';
  }

  @override
  String get errorTooManyRequest => '錯誤 429：請求過於頻繁';

  @override
  String get errorTooManyStickers => '錯誤 20126：貼紙數已達上限';

  @override
  String get errorTooSmallTransferAmount => '錯誤 20120：轉賬金額太小';

  @override
  String get errorTooSmallWithdrawAmount => '錯誤 20127：提現金額太小';

  @override
  String get errorTranscriptForward => '請在所有附件下載完成之後再轉發';

  @override
  String get errorTransferToDeactivatedUser => '錯誤 20160：無法給已經刪除的賬號轉賬';

  @override
  String get errorUnableToOpenMedia => '無法找到能打開該媒體的應用';

  @override
  String errorUnknownWithCode(Object arg0) {
    return '錯誤：$arg0';
  }

  @override
  String errorUnknownWithMessage(Object arg0) {
    return '錯誤：$arg0';
  }

  @override
  String get errorUploadAttachmentFailed => '消息附件上傳失敗';

  @override
  String get errorUsedPhone => '錯誤 20122：電話號碼已經被佔用。';

  @override
  String get errorUserInvalidFormat => '用户數據不合法';

  @override
  String get errorWithdrawalMemoFormatIncorrect => '錯誤 20131：提現備註格式不正確';

  @override
  String get errorWithdrawalSuspend => '錯誤 20137：提現暫停';

  @override
  String get exit => '退出';

  @override
  String get exitGroup => '退出羣組';

  @override
  String get failed => '失敗';

  @override
  String get failedToOpenDatabase => '打開數據庫時出現了錯誤。';

  @override
  String get fee => '手續費';

  @override
  String get file => '檔案';

  @override
  String get fileChooserError => '檔案選擇錯誤';

  @override
  String get fileDoesNotExist => '檔案不存在';

  @override
  String get fileError => '檔案錯誤';

  @override
  String get files => '文檔';

  @override
  String get flags => '旗幟';

  @override
  String get followSystem => '跟隨系統';

  @override
  String get followUsOnFacebook => '關注我們的 Facebook';

  @override
  String get followUsOnX => '關注我們的 X';

  @override
  String get foodAndDrink => '食物與飲料';

  @override
  String get formatNotSupported => '不支持該格式';

  @override
  String get forward => '轉發';

  @override
  String get from => '來自';

  @override
  String get fromWithColon => '來自：';

  @override
  String get generateQrcode => '生成二維碼';

  @override
  String get groupAlreadyIn => '你已經在該羣組裏';

  @override
  String get groupCantSend => '您不能發送消息，因為您已經不再是此羣組成員。';

  @override
  String get groupName => '羣組名稱';

  @override
  String get groupParticipants => '羣成員';

  @override
  String groupPopMenuMessage(Object arg0) {
    return '發送消息至 $arg0';
  }

  @override
  String groupPopMenuRemove(Object arg0) {
    return '移除 $arg0';
  }

  @override
  String get groups => '羣組';

  @override
  String get groupsInCommon => '共同羣組';

  @override
  String get hash => '哈希';

  @override
  String get help => '幫助';

  @override
  String get helpCenter => '幫助中心';

  @override
  String get hideMixin => '隱藏 Mixin';

  @override
  String get host => '主機名';

  @override
  String hour(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 小時',
      one: '$arg0 小時',
    );
    return '$_temp0';
  }

  @override
  String get howAreYou => '嗨，你好嗎？';

  @override
  String get iAmGood => '我很好。';

  @override
  String get id => 'ID';

  @override
  String get ignoreThisVersion => '忽略這次版本更新';

  @override
  String get image => '圖像';

  @override
  String get includeFiles => '包含檔案';

  @override
  String get includeVideos => '包括視頻';

  @override
  String get initializing => '初始化…';

  @override
  String get invalidStickerFormat => '貼紙格式不支持';

  @override
  String get inviteInfo => 'Mixin 使用者可以使用此鏈接加入這個羣組，請只跟您信任的人共享鏈接。';

  @override
  String get inviteToGroupViaLink => '羣邀請鏈接';

  @override
  String get joinGroupWithPlus => '+ 加入羣組';

  @override
  String joinedIn(Object arg0) {
    return '加入於 $arg0';
  }

  @override
  String landingDeleteContent(Object arg0, Object arg1) {
    return '您在 $arg0 申請了刪除賬號，賬號將於 $arg1 被刪除，如果您繼續登錄，刪除您賬户的請求將被取消。';
  }

  @override
  String landingInvitationDialogContent(Object arg0) {
    return '我們將發送4位驗證碼到手機 $arg0, 請在下一個頁面輸入';
  }

  @override
  String landingValidationTitle(Object arg0) {
    return '請輸入發送至以下號碼的 4 位驗證碼：$arg0';
  }

  @override
  String get learnMore => '瞭解更多';

  @override
  String get less => '更少';

  @override
  String get light => '淺色';

  @override
  String get linkedDevice => '連接的設備';

  @override
  String get live => '直播';

  @override
  String get loading => '正在加載...';

  @override
  String get loadingTime => '檢測到系統時間異常，請校正後再繼續使用';

  @override
  String get locateToChat => '前往聊天';

  @override
  String get location => '位置';

  @override
  String get lock => '鎖定';

  @override
  String get logIn => '登錄';

  @override
  String get loginAndAbortAccountDeletion => '繼續登錄並放棄刪除賬户';

  @override
  String get loginByQrcode => '通過二維碼登錄 Mixin Messenger';

  @override
  String get loginByQrcodeTips1 => '打開手機上的 Mixin Messenger。';

  @override
  String get loginByQrcodeTips2 => '掃描屏幕上的二維碼，確認登錄。';

  @override
  String get makeGroupAdmin => '設定為羣組管理員';

  @override
  String get media => '媒體';

  @override
  String get memo => '備註';

  @override
  String get messageE2ee => '此對話中的消息使用端對端加密。點擊瞭解更多。';

  @override
  String get messageNotFound => '找不到該消息';

  @override
  String get messageNotSupport => '不支持此類消息，請升級 Mixin 到最新版本。';

  @override
  String get messagePreview => '消息預覽';

  @override
  String get messagePreviewDescription => '預覽新消息通知中的消息文本。';

  @override
  String get messages => '消息';

  @override
  String get minimize => '最小化';

  @override
  String minute(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 分鐘',
      one: '$arg0 分鐘',
    );
    return '$_temp0';
  }

  @override
  String get mixinMessengerDesktop => 'Mixin Messenger 桌面';

  @override
  String get more => '更多';

  @override
  String get multisigTransaction => '多重簽名交易';

  @override
  String get mute => '靜音';

  @override
  String myMixinId(Object arg0) {
    return '我的 Mixin ID：$arg0';
  }

  @override
  String get myStickers => '我的表情';

  @override
  String get na => '暫無價格';

  @override
  String get name => '名稱';

  @override
  String get networkConnectionFailed => '網絡連接失敗';

  @override
  String get networkError => '網絡錯誤';

  @override
  String get newVersionAvailable => '發現新版本';

  @override
  String newVersionDescription(Object arg0, Object arg1) {
    return '發現新版本 Mixin Messenger $arg0，當前版本為 $arg1。是否要下載最新的版本？';
  }

  @override
  String get next => '下一步';

  @override
  String get nextConversation => '下一個會話';

  @override
  String get noAudio => '沒有音頻';

  @override
  String get noCamera => '沒有相機';

  @override
  String get noData => '沒有數據';

  @override
  String get noFiles => '沒有檔案';

  @override
  String get noLinks => '沒有鏈接';

  @override
  String get noMedia => '沒有媒體';

  @override
  String get noNetworkConnection => '無網絡連接';

  @override
  String get noPosts => '沒有文章';

  @override
  String get noResults => '未找到相關結果';

  @override
  String get notFound => '沒有找到相應的消息';

  @override
  String get notSupportBiometric => '此設備不支持生物驗證';

  @override
  String get notificationContent => '啓用推送通知以實時更新價格警報和消息。';

  @override
  String get notificationPermissionManually => '未允許通知，請到通知設置開啓。';

  @override
  String get notifications => '通知';

  @override
  String nowAnAddmin(Object arg0) {
    return '$arg0現在是管理員';
  }

  @override
  String get objects => '物件';

  @override
  String get oneByOneForward => '逐條轉發';

  @override
  String get oneHour => '1 小時';

  @override
  String get oneYear => '1 年';

  @override
  String get open => '打開';

  @override
  String get openHomePage => '打開主頁';

  @override
  String openLink(Object arg0) {
    return '打開鏈接：$arg0';
  }

  @override
  String get openLogDirectory => '打開日誌檔案夾';

  @override
  String get openingBalance => '期初餘額';

  @override
  String get originalImage => '原圖';

  @override
  String get owner => '羣主';

  @override
  String participantsCount(Object arg0) {
    return '$arg0 位羣組成員';
  }

  @override
  String get passcodeIncorrect => '密碼不正確';

  @override
  String get password => '密碼';

  @override
  String pendingConfirmation(Object arg0, Object arg1, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0/$arg1 確認',
      one: '$arg0/$arg1 確認',
    );
    return '$_temp0';
  }

  @override
  String get phoneNumber => '手機號碼';

  @override
  String get photos => '照片';

  @override
  String get pickAConversation => '選擇一個對話，開始發送信息';

  @override
  String get picturesAndVideos => '圖像 & 視頻';

  @override
  String get pinTitle => '置頂';

  @override
  String pinnedMessageTitle(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 條置頂消息',
      one: '$arg0 條置頂消息',
    );
    return '$_temp0';
  }

  @override
  String get port => '端口號';

  @override
  String get post => '文章';

  @override
  String get preferences => '偏好設置';

  @override
  String get previousConversation => '上一個會話';

  @override
  String get privacyPolicy => '隱私政策';

  @override
  String get proxy => '代理';

  @override
  String get proxyAuth => '驗證（可選）';

  @override
  String get proxyConnection => '連接';

  @override
  String get proxyType => '代理類型';

  @override
  String get qrCodeExpiredDesc => '二維碼過期，請重試';

  @override
  String get quickSearch => '快速搜索';

  @override
  String get quitMixin => '退出 Mixin';

  @override
  String get raw => '其他';

  @override
  String get rebate => '退款';

  @override
  String get recaptchaTimeout => '驗證超時';

  @override
  String get receiver => '至';

  @override
  String get recentChats => '最近聊天';

  @override
  String get reedit => '重新編輯';

  @override
  String get refresh => '刷新';

  @override
  String get removeBot => '刪除機器人';

  @override
  String get removeChatFromCircle => '從圈子裏移除對話';

  @override
  String get removeContact => '刪除聯繫人';

  @override
  String get removeStickers => '移除所有表情';

  @override
  String get reply => '回覆';

  @override
  String get report => '舉報';

  @override
  String get reportAndBlock => '舉報並屏蔽？';

  @override
  String get reportTitle => '給開發人員發送聊天日誌？';

  @override
  String get resendCode => '重發驗證碼';

  @override
  String resendCodeIn(Object arg0) {
    return '$arg0 秒後重新發送驗證碼';
  }

  @override
  String get reset => '重置';

  @override
  String get resetLink => '重置邀請鏈接';

  @override
  String get restoreChat => '恢復聊天記錄';

  @override
  String get restoreChatTip => '從其他設備恢復聊天記錄。請確保兩台設備連接到同一個 Wi-Fi 或熱點。';

  @override
  String get restoreFromOtherDevice => '從其他設備恢復';

  @override
  String get retry => '重試';

  @override
  String get retryUploadFailed => '重新上傳失敗。';

  @override
  String get revokeMultisigTransaction => '撤銷多重簽名交易';

  @override
  String get save => '保存';

  @override
  String get saveAs => '另存為';

  @override
  String get saveToCameraRoll => '保存到相冊';

  @override
  String get sayHi => '打招呼';

  @override
  String get scamWarning => '警告：此賬號被大量用户舉報，請謹防網絡詐騙，注意個人財產安全';

  @override
  String get screenPasscode => '鎖屏密碼';

  @override
  String get search => '搜索';

  @override
  String get searchContact => '搜索用户';

  @override
  String get searchConversation => '搜索聊天記錄';

  @override
  String get searchEmpty => '找不到聯繫人或消息。';

  @override
  String get searchPlaceholderNumber => '搜索 Mixin ID 或手機號碼：';

  @override
  String searchRelatedMessage(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 條相關消息',
      one: '$arg0 條相關消息',
    );
    return '$_temp0';
  }

  @override
  String get searchUnread => '搜索未讀會話';

  @override
  String get security => '安全';

  @override
  String get select => '選擇';

  @override
  String get send => '發送';

  @override
  String get sendArchived => '打包成 zip 發送';

  @override
  String get sendQuickly => '快速發送';

  @override
  String get sendToDeveloper => '把日誌發給開發者';

  @override
  String get sendWithoutCompression => '發送原始檔案';

  @override
  String get sendWithoutSound => '靜音發送';

  @override
  String get set => '設置';

  @override
  String setDisappearingMessageTimeTo(Object arg0, Object arg1) {
    return '$arg0將限時消息設置為 $arg1';
  }

  @override
  String get setPasscodeDesc => '設置解鎖密碼';

  @override
  String get settingAuthSearchHint => 'Mixin ID, 暱稱';

  @override
  String get settingBackupTips =>
      '備份你的聊天記錄到 iCloud。如果你丟失或者更換手機，你可以在重新安裝 Mixin Messenger 時恢復你的聊天記錄。注意備份到 iCloud 中的聊天記錄不受端對端加密保護！';

  @override
  String settingDeleteAccountPinContent(Object arg0) {
    return '如果您繼續，您的個人資料和賬户信息將在$arg0被刪除。閲讀我們的文檔以**瞭解更多**。';
  }

  @override
  String get settingDeleteAccountUrl =>
      'https://support.mixin.one/zh/article/5aac5l2v5yig6zmk6lsm5y377yf-1uteq30';

  @override
  String get share => '分享';

  @override
  String get shareApps => '分享的應用';

  @override
  String get shareContact => '分享聯繫人';

  @override
  String get shareError => '分享出錯';

  @override
  String get shareLink => '分享邀請鏈接';

  @override
  String shareMessageDescription(Object arg0, Object arg1) {
    return '你確定要發送來自$arg0的$arg1？';
  }

  @override
  String shareMessageDescriptionEmpty(Object arg0) {
    return '你確定要發送該$arg0？';
  }

  @override
  String get sharedMedia => '媒體內容';

  @override
  String get show => '顯示';

  @override
  String get showAvatar => '顯示頭像';

  @override
  String get showIdentityNumber => '顯示 ID';

  @override
  String get showMixin => '顯示 Mixin';

  @override
  String get signIn => '登錄';

  @override
  String get signOut => '退出登錄';

  @override
  String get signWithMobileNumber => '通過手機號登錄';

  @override
  String get signWithQrcode => '通過二維碼登錄';

  @override
  String get smileysAndPeople => '表情符號與人物';

  @override
  String get snapshotHash => '快照哈希';

  @override
  String get status => '交易狀態';

  @override
  String get sticker => '貼紙';

  @override
  String get stickerAddInvalidSize =>
      '貼紙要求大於 1KB 且小於 1MB，寬高大於 128 像素且小於 1024 像素。';

  @override
  String get stickerAlbumDetail => '表情詳情';

  @override
  String get stickerStore => '表情商店';

  @override
  String get storageAutoDownloadDescription => '更改媒體的自動下載設置。';

  @override
  String get storageUsage => '儲存空間';

  @override
  String get strangerHint => '對方不是你的聯繫人';

  @override
  String get strangers => '陌生人';

  @override
  String get successful => '成功';

  @override
  String get symbols => '符號';

  @override
  String get syncFromOtherDevice => '從其他設備同步';

  @override
  String get syncToOtherDevice => '同步到其他設備';

  @override
  String get termsOfService => '服務條款';

  @override
  String get text => '文字';

  @override
  String get theme => '主題';

  @override
  String get thisMessageWasDeleted => '此消息已撤回';

  @override
  String get time => '時間';

  @override
  String get to => '至';

  @override
  String get today => '今天';

  @override
  String get toggleChatInfo => '展開/關閉會話信息';

  @override
  String get transactionHash => '交易哈希';

  @override
  String get transactionId => '交易編號';

  @override
  String get transactionType => '交易類型';

  @override
  String get transactions => '交易記錄';

  @override
  String get transactionsCannotBeDeleted => '交易記錄不會被刪除';

  @override
  String get transcript => '聊天記錄';

  @override
  String get transfer => '轉賬';

  @override
  String get transferCompleted => '同步完成';

  @override
  String get transferProtocolVersionNotMatched => '版本不匹配，無法同步數據，請先升級應用。';

  @override
  String get transferringChats => '同步聊天記錄中';

  @override
  String get transferringChatsTips => '同步時請不要關閉屏幕並保持 Mixin 在前台運行。';

  @override
  String get travelAndPlaces => '旅行與地點';

  @override
  String get typeMessage => '輸入消息';

  @override
  String unableToOpenFile(Object arg0) {
    return '無法打開檔案：$arg0';
  }

  @override
  String get unblock => '解除屏蔽';

  @override
  String unitDay(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '天',
      one: '天',
    );
    return '$_temp0';
  }

  @override
  String unitHour(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '小時',
      one: '小時',
    );
    return '$_temp0';
  }

  @override
  String unitMinute(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '分',
      one: '分',
    );
    return '$_temp0';
  }

  @override
  String unitSecond(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '秒',
      one: '秒',
    );
    return '$_temp0';
  }

  @override
  String unitWeek(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '週',
      one: '週',
    );
    return '$_temp0';
  }

  @override
  String get unknowError => '未知錯誤';

  @override
  String get unlockMixinMessenger => '解鎖 Mixin Messenger';

  @override
  String get unlockWithWasscode => '輸入密碼解鎖 Mixin Messenger';

  @override
  String get unmute => '取消靜音';

  @override
  String get unpin => '取消置頂';

  @override
  String get unpinAllMessages => '取消所有置頂消息';

  @override
  String get unpinAllMessagesConfirmation => '確定取消置頂所有消息麼？';

  @override
  String get unreadMessages => '未讀消息';

  @override
  String get updateMixin => '升級 Mixin';

  @override
  String updateMixinDescription(Object arg0) {
    return '當前版本（$arg0）不再可用!\n請點擊下面的“更新”按鈕以更新到最新版本。';
  }

  @override
  String get upgrade => '升級';

  @override
  String get upgrading => '升級中...';

  @override
  String get useBiometric => '使用生物識別';

  @override
  String get userDeleteHint => '該用户已經刪除了賬號。';

  @override
  String get userNotFound => '找不到這個用户';

  @override
  String get username => '用户名';

  @override
  String valueNow(Object arg0) {
    return '價值 $arg0';
  }

  @override
  String valueThen(Object arg0) {
    return '當時價值 $arg0';
  }

  @override
  String get verifyPin => '驗證 PIN';

  @override
  String get video => '視頻';

  @override
  String get videos => '視頻';

  @override
  String get waitingForThisMessage => '正在等待這條消息。';

  @override
  String get waitingOtherDeviceConnection => '等待其他設備連接。';

  @override
  String get webview2RuntimeInstallDescription =>
      '該設備暫未安裝 WebView2 組件，請先下載並安裝 WebView2 Runtime。';

  @override
  String get webviewRuntimeUnavailable => 'WebView2 組件不可用';

  @override
  String get window => '窗口';

  @override
  String get withdrawal => '提現';

  @override
  String get withdrawalHash => '提現哈希';

  @override
  String get you => '你';

  @override
  String get youDeletedThisMessage => '你撤回了一條消息';

  @override
  String get zoom => '縮放';
}

/// The translations for Chinese, as used in Taiwan (`zh_TW`).
class AppLocalizationsZhTw extends AppLocalizationsZh {
  AppLocalizationsZhTw() : super('zh_TW');

  @override
  String get aMessage => '一條訊息';

  @override
  String get about => '關於';

  @override
  String get account => '賬號';

  @override
  String get activity => '活動';

  @override
  String get add => '新增';

  @override
  String get addACaption => '新增說明';

  @override
  String get addBotWithPlus => '+ 新增機器人';

  @override
  String get addContact => '新增聯絡人';

  @override
  String get addContactWithPlus => '+ 新增聯絡人';

  @override
  String get addFile => '新增檔案';

  @override
  String get addGroupDescription => '新增群公告';

  @override
  String get addParticipants => '新增成員';

  @override
  String get addPeopleSearchHint => 'Mixin ID 或手機號';

  @override
  String get addProxy => '新增代理';

  @override
  String get addSticker => '新增貼紙';

  @override
  String get addStickerFailed => '新增貼紙失敗';

  @override
  String get addStickers => '新增貼紙';

  @override
  String get addToCircle => '新增到圈子';

  @override
  String get added => '已新增';

  @override
  String get address => '地址';

  @override
  String get admin => '管理員';

  @override
  String get alertKeyContactContactMessage => '分享了一個聯絡人';

  @override
  String get allChats => '全部聊天';

  @override
  String get animalsAndNature => '動物與自然';

  @override
  String get anonymous => '匿名';

  @override
  String get anonymousNumber => '匿名號碼';

  @override
  String get appCardShareDisallow => '此連結無法分享';

  @override
  String get appearance => '外觀';

  @override
  String get archivedFolder => '存檔資料夾';

  @override
  String get assetType => '資產型別';

  @override
  String get audio => '語音';

  @override
  String get audios => '音訊';

  @override
  String get autoBackup => '自動備份';

  @override
  String get autoLock => '自動鎖定';

  @override
  String get avatar => '頭像';

  @override
  String get backup => '備份';

  @override
  String get backupChat => '備份聊天記錄';

  @override
  String get backupToOtherDevice => '備份到其他裝置';

  @override
  String get backupToOtherDeviceTips => '將聊天記錄備份到其他裝置。請確保兩臺裝置連線到同一個 Wi-Fi 或熱點。';

  @override
  String get backupWaitingOtherDevice => '請在另一臺裝置上開啟 Mixin，並在那邊開始恢復。';

  @override
  String get biography => '簡介';

  @override
  String get biometric => '生物識別';

  @override
  String get block => '封鎖使用者';

  @override
  String get botNotFound => '找不到這個機器人';

  @override
  String get bots => '機器人';

  @override
  String get botsTitle => '機器人';

  @override
  String get bringAllToFront => '前置所有視窗';

  @override
  String get canNotRecognizeQrCode => '無法識別二維碼';

  @override
  String get cancel => '取消';

  @override
  String get card => '卡片';

  @override
  String get change => '更改';

  @override
  String get changeNumber => '修改手機號';

  @override
  String get changeNumberInstead => '僅修改手機號碼';

  @override
  String changedDisappearingMessageSettings(Object arg0) {
    return '$arg0修改了限時訊息設定';
  }

  @override
  String get chatBackup => '聊天記錄備份';

  @override
  String get chatBackupAndRestore => '聊天記錄備份與恢復';

  @override
  String get chatBotReceptionTitle => '點選按鈕使用機器人';

  @override
  String chatDecryptionFailedHint(Object arg0) {
    return '等待$arg0上線後建立加密會話。';
  }

  @override
  String chatDeleteMessage(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '刪除 $arg0 條訊息嗎？',
      one: '刪除 $arg0 條訊息嗎？',
    );
    return '$_temp0';
  }

  @override
  String chatGroupAdd(Object arg0, Object arg1) {
    return '$arg0添加了$arg1';
  }

  @override
  String chatGroupExit(Object arg0) {
    return '$arg0離開了群組';
  }

  @override
  String chatGroupJoin(Object arg0) {
    return '$arg0透過邀請連結加入群組';
  }

  @override
  String chatGroupRemove(Object arg0, Object arg1) {
    return '$arg0移除了$arg1';
  }

  @override
  String get chatHintE2e => '端對端加密';

  @override
  String get chatNotSupportUriOnPhone => '不支援此連結，請在手機上檢視。';

  @override
  String get chatNotSupportUrl =>
      'https://support.mixin.one/zh/article/5ye6546w4occ6lz5liq57g75z6l55qe5rai5ogv5lin5psv5oyb4ocd5oco5lmi5yqe77yf-h92cxa/';

  @override
  String get chatNotSupportViewOnPhone => '不支援此型別訊息，請在手機上檢視。';

  @override
  String chatPinMessage(Object arg0, Object arg1) {
    return '$arg0置頂了$arg1';
  }

  @override
  String get chatTextSize => '聊天字型大小';

  @override
  String get chats => '聊天';

  @override
  String get checkNewVersion => '檢查新版本';

  @override
  String circleSubtitle(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 會話',
      one: '$arg0 會話',
    );
    return '$_temp0';
  }

  @override
  String circleTitle(Object arg0) {
    return '$arg0的圈子';
  }

  @override
  String get circles => '圈子';

  @override
  String get clear => '清除';

  @override
  String get clearChat => '清除聊天記錄';

  @override
  String get clearFilter => '清除篩選條件';

  @override
  String get clickToReloadQrcode => '點選重新載入二維碼';

  @override
  String get close => '關閉';

  @override
  String get closeWindow => '關閉視窗';

  @override
  String get closingBalance => '期末餘額';

  @override
  String get collapse => '摺疊';

  @override
  String get collectible => '藏品';

  @override
  String get collectibles => '藏品';

  @override
  String get collection => '合集';

  @override
  String get combineAndForward => '合併轉發';

  @override
  String get confirm => '確認';

  @override
  String get confirmPasscodeDesc => '再次確認密碼';

  @override
  String get confirmSyncChatsFromPhone => '確認從手機端同步聊天記錄嗎？';

  @override
  String get confirmSyncChatsToPhone => '確認同步聊天記錄到手機端嗎？';

  @override
  String get confirmations => '區塊確認數';

  @override
  String get contact => '聯絡人';

  @override
  String contactMixinId(Object arg0) {
    return 'Mixin ID：$arg0';
  }

  @override
  String get contactMuteTitle => '靜音通知';

  @override
  String get contactTitle => '聯絡人';

  @override
  String get contentTooLong => '內容過長';

  @override
  String get contentVoice => '[語音電話]';

  @override
  String get continueText => '繼續';

  @override
  String get conversation => '會話';

  @override
  String conversationDeleteTitle(Object arg0) {
    return '刪除會話：$arg0';
  }

  @override
  String get copy => '複製';

  @override
  String get copyImage => '複製圖片';

  @override
  String get copyInvite => '複製邀請連結';

  @override
  String get copyLink => '複製連結';

  @override
  String get copySelectedText => '複製已選擇的文字';

  @override
  String get copyText => '複製文字';

  @override
  String get create => '建立';

  @override
  String get createCircle => '新建圈子';

  @override
  String get createConversation => '新建會話';

  @override
  String get createGroup => '新建群組';

  @override
  String createdAt(Object arg0) {
    return '創建於 $arg0';
  }

  @override
  String createdThisGroup(Object arg0) {
    return '$arg0建立了這個群組';
  }

  @override
  String get customTime => '自定義時間';

  @override
  String get dark => '深色';

  @override
  String get dataAndStorageUsage => '資料與儲存空間';

  @override
  String get dataError => '資料錯誤';

  @override
  String get dataLoading => '資料載入中，請稍後';

  @override
  String get databaseCorruptedTips => '資料庫已損壞，暫無法恢復。點選繼續將重新建立一個新的資料庫檔案。';

  @override
  String get databaseLockedTips => '資料庫檔案已被鎖定，無法訪問。請嘗試重啟應用或者重啟系統後再試。';

  @override
  String get databaseNotADbTips => '無法開啟資料庫，檔案不是一個有效的資料庫檔案。';

  @override
  String get databaseRecreateTips => '重新建立一個新的資料庫檔案，舊檔案將被刪除。';

  @override
  String get databaseUpgradeTips => '正在進行資料庫升級，可能需要幾分鐘，請不要強制關閉應用。';

  @override
  String get delete => '刪除';

  @override
  String get deleteAccountDetailHint => '本地訊息和 iCloud 備份不會被自動刪除';

  @override
  String get deleteAccountHint => '刪除你的賬戶和個人照片';

  @override
  String get deleteChat => '刪除聊天';

  @override
  String get deleteChatDescription => '刪除會話只會刪除此裝置的聊天記錄，不會影響其他裝置。';

  @override
  String get deleteCircle => '刪除圈子';

  @override
  String get deleteForEveryone => '撤回';

  @override
  String get deleteForMe => '刪除';

  @override
  String get deleteGroup => '刪除群組';

  @override
  String get deleteMyAccount => '刪除賬號';

  @override
  String deleteTheCircle(Object arg0) {
    return '確定刪除$arg0圈子嗎？';
  }

  @override
  String get deposit => '充值';

  @override
  String get depositHash => '充值雜湊';

  @override
  String get developer => '開發者';

  @override
  String get deviceTransferFailed => '同步失敗';

  @override
  String disableDisappearingMessage(Object arg0) {
    return '$arg0停用了限時訊息';
  }

  @override
  String get disabled => '停用';

  @override
  String disappearingCustomTimeMaxWarning(Object arg0) {
    return '最高設定 $arg0';
  }

  @override
  String get disappearingMessage => '限時訊息';

  @override
  String get disappearingMessageHint =>
      '啟用後，在此聊天中傳送和接收的新資訊在被看到後會消失，閱讀檔案以**瞭解更多**。';

  @override
  String get discard => '廢棄';

  @override
  String get discardRecordingWarning => '是否要停止並廢棄已錄製的語音訊息？';

  @override
  String get dismissAsAdmin => '撤銷管理員身份';

  @override
  String get done => '完成';

  @override
  String get download => '下載';

  @override
  String get downloadLink => '下載連結：';

  @override
  String get draft => '草稿';

  @override
  String get dragAndDropFileHere => '拖放檔案到此處';

  @override
  String get durationIsTooShort => '時間太短';

  @override
  String get edit => '編輯';

  @override
  String get editCircleName => '編輯名稱';

  @override
  String get editConversations => '管理圈子';

  @override
  String get editGroupDescription => '編輯群公告';

  @override
  String get editGroupName => '編輯名稱';

  @override
  String get editImageClearWarning => '退出將會清除此次所有的改動。';

  @override
  String get editName => '修改暱稱';

  @override
  String get editProfile => '編輯資料';

  @override
  String get enablePushNotification => '啟用推送通知';

  @override
  String get encryptZipFileWithPassword => '使用密碼來加密 zip 檔案';

  @override
  String get enterPinToDeleteAccount => '輸入你的 PIN 以登出你的賬戶';

  @override
  String get enterToSend => '按下回車 ⏎ 傳送';

  @override
  String get enterYourPhoneNumber => '輸入你的手機號碼';

  @override
  String get enterYourPinToContinue => '輸入你的 PIN 以繼續';

  @override
  String get errorAccessLimited => '錯誤 403：訪問受限';

  @override
  String get errorAddressExists => '地址不存在，請確保地址是否新增成功';

  @override
  String get errorAddressNotSync => '地址重新整理失敗，請重試';

  @override
  String get errorAlreadyBondedReferralCode => '錯誤 10731：當前賬號已繫結邀請碼，無法修改繫結。';

  @override
  String get errorAssetExists => '沒有相關資產';

  @override
  String get errorAuthentication => '錯誤 401：請重新登入';

  @override
  String get errorBadData => '錯誤 10002：請求資料不合法';

  @override
  String get errorBlockchain => '錯誤 30100：區塊鏈同步異常，請稍後重試';

  @override
  String get errorConnectionTimeout => '網路連線超時';

  @override
  String get errorFullGroup => '錯誤 20116：群組已滿';

  @override
  String get errorInsufficientBalance => '錯誤 20117：餘額不足';

  @override
  String errorInsufficientTransactionFeeWithAmount(Object arg0) {
    return '錯誤 20124：手續費不足。請確保錢包至少有 $arg0 當作手續費。';
  }

  @override
  String errorInvalidAddress(Object arg0, Object arg1) {
    return '錯誤 30102：地址格式錯誤。請輸入正確的 $arg0 $arg1 的地址！';
  }

  @override
  String get errorInvalidAddressPlain => '錯誤 30102：地址格式錯誤。';

  @override
  String get errorInvalidCodeTooFrequent => '錯誤 20129：傳送驗證碼太頻繁，請稍後再試';

  @override
  String get errorInvalidEmergencyContact => '錯誤 20130：恢復聯絡人不正確';

  @override
  String get errorInvalidPinFormat => '錯誤 20118：PIN 格式不正確';

  @override
  String get errorInviterPlanExpired => '錯誤 10737：邀請人的會員已過期';

  @override
  String get errorLegacyPin =>
      '錯誤 20118：為了加強 Mixin 網路的安全，Mixin API 現已暫停 D3M-PIN 升級到 TIP，詳情請檢視檔案並登記等待處理。';

  @override
  String get errorNetworkTaskFailed => '網路連線失敗。檢查或切換網路並重試';

  @override
  String get errorNoPinToken => '缺少憑據，請重新登入之後再嘗試使用此功能。';

  @override
  String get errorNotFound => '錯誤 404：沒有找到相應的資訊';

  @override
  String get errorNotSupportedAudioFormat => '不支援的音訊格式，請用其他app開啟。';

  @override
  String get errorNumberReachedLimit => '錯誤 20132： 已達到上限';

  @override
  String errorOldVersion(Object arg0) {
    return '錯誤 10006：請更新 Mixin（$arg0） 至最新版。';
  }

  @override
  String get errorOpenLocation => '無法找到地圖應用';

  @override
  String get errorPermission => '請開啟相關許可權';

  @override
  String get errorPhoneInvalidFormat => '錯誤 20110：手機號碼不合法';

  @override
  String get errorPhoneSmsDelivery => '錯誤 10003：傳送簡訊失敗';

  @override
  String get errorPhoneVerificationCodeExpired => '錯誤 20114：驗證碼已過期';

  @override
  String get errorPhoneVerificationCodeInvalid => '錯誤 20113：驗證碼錯誤';

  @override
  String get errorPinCheckTooManyRequest => '你已經嘗試了超過 5 次，請等待 24 小時後再次嘗試。';

  @override
  String get errorPinIncorrect => '錯誤 20119：PIN 不正確';

  @override
  String errorPinIncorrectWithTimes(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '錯誤 20119：PIN 不正確。你還有 $arg0 次機會，使用完需等待 24 小時後再次嘗試。',
      one: '錯誤 20119：PIN 不正確。你還有 $arg0 次機會，使用完需等待 24 小時後再次嘗試。',
    );
    return '$_temp0';
  }

  @override
  String get errorRecaptchaIsInvalid => '錯誤 10004：驗證失敗';

  @override
  String errorServer5xxCode(Object arg0) {
    return '伺服器出錯，請稍後重試：$arg0';
  }

  @override
  String get errorTooManyRequest => '錯誤 429：請求過於頻繁';

  @override
  String get errorTooManyStickers => '錯誤 20126：貼紙數已達上限';

  @override
  String get errorTooSmallTransferAmount => '錯誤 20120：轉賬金額太小';

  @override
  String get errorTooSmallWithdrawAmount => '錯誤 20127：提現金額太小';

  @override
  String get errorTranscriptForward => '請在所有附件下載完成之後再轉發';

  @override
  String get errorTransferToDeactivatedUser => '錯誤 20160：無法給已經刪除的賬號轉賬';

  @override
  String get errorUnableToOpenMedia => '無法找到能開啟該媒體的應用';

  @override
  String errorUnknownWithCode(Object arg0) {
    return '錯誤：$arg0';
  }

  @override
  String errorUnknownWithMessage(Object arg0) {
    return '錯誤：$arg0';
  }

  @override
  String get errorUploadAttachmentFailed => '訊息附件上傳失敗';

  @override
  String get errorUsedPhone => '錯誤 20122：電話號碼已經被佔用。';

  @override
  String get errorUserInvalidFormat => '使用者資料不合法';

  @override
  String get errorWithdrawalMemoFormatIncorrect => '錯誤 20131：提現備註格式不正確';

  @override
  String get errorWithdrawalSuspend => '錯誤 20137：提現暫停';

  @override
  String get exit => '退出';

  @override
  String get exitGroup => '退出群組';

  @override
  String get failed => '失敗';

  @override
  String get failedToOpenDatabase => '開啟資料庫時出現了錯誤。';

  @override
  String get fee => '手續費';

  @override
  String get file => '檔案';

  @override
  String get fileChooserError => '檔案選擇錯誤';

  @override
  String get fileDoesNotExist => '檔案不存在';

  @override
  String get fileError => '檔案錯誤';

  @override
  String get files => '檔案';

  @override
  String get flags => '旗幟';

  @override
  String get followSystem => '跟隨系統';

  @override
  String get followUsOnFacebook => '關注我們的 Facebook';

  @override
  String get followUsOnX => '關注我們的 X';

  @override
  String get foodAndDrink => '食物與飲料';

  @override
  String get formatNotSupported => '不支援該格式';

  @override
  String get forward => '轉發';

  @override
  String get from => '來自';

  @override
  String get fromWithColon => '來自：';

  @override
  String get generateQrcode => '生成二維碼';

  @override
  String get groupAlreadyIn => '你已經在該群組裡';

  @override
  String get groupCantSend => '您不能傳送訊息，因為您已經不再是此群組成員。';

  @override
  String get groupName => '群組名稱';

  @override
  String get groupParticipants => '群成員';

  @override
  String groupPopMenuMessage(Object arg0) {
    return '傳送訊息至 $arg0';
  }

  @override
  String groupPopMenuRemove(Object arg0) {
    return '移除 $arg0';
  }

  @override
  String get groups => '群組';

  @override
  String get groupsInCommon => '共同群組';

  @override
  String get hash => '雜湊';

  @override
  String get help => '幫助';

  @override
  String get helpCenter => '幫助中心';

  @override
  String get hideMixin => '隱藏 Mixin';

  @override
  String get host => '主機名';

  @override
  String hour(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 小時',
      one: '$arg0 小時',
    );
    return '$_temp0';
  }

  @override
  String get howAreYou => '嗨，你好嗎？';

  @override
  String get iAmGood => '我很好。';

  @override
  String get id => 'ID';

  @override
  String get ignoreThisVersion => '忽略這次版本更新';

  @override
  String get image => '影像';

  @override
  String get includeFiles => '包含檔案';

  @override
  String get includeVideos => '包括影片';

  @override
  String get initializing => '初始化…';

  @override
  String get invalidStickerFormat => '貼紙格式不支援';

  @override
  String get inviteInfo => 'Mixin 使用者可以使用此連結加入這個群組，請只跟您信任的人共享連結。';

  @override
  String get inviteToGroupViaLink => '群邀請連結';

  @override
  String get joinGroupWithPlus => '+ 加入群組';

  @override
  String joinedIn(Object arg0) {
    return '加入於 $arg0';
  }

  @override
  String landingDeleteContent(Object arg0, Object arg1) {
    return '您在 $arg0 申請了刪除賬號，賬號將於 $arg1 被刪除，如果您繼續登入，刪除您賬戶的請求將被取消。';
  }

  @override
  String landingInvitationDialogContent(Object arg0) {
    return '我們將傳送4位驗證碼到手機 $arg0, 請在下一個頁面輸入';
  }

  @override
  String landingValidationTitle(Object arg0) {
    return '請輸入傳送至以下號碼的 4 位驗證碼：$arg0';
  }

  @override
  String get learnMore => '瞭解更多';

  @override
  String get less => '更少';

  @override
  String get light => '淺色';

  @override
  String get linkedDevice => '連線的裝置';

  @override
  String get live => '直播';

  @override
  String get loading => '正在載入...';

  @override
  String get loadingTime => '檢測到系統時間異常，請校正後再繼續使用';

  @override
  String get locateToChat => '前往聊天';

  @override
  String get location => '位置';

  @override
  String get lock => '鎖定';

  @override
  String get logIn => '登入';

  @override
  String get loginAndAbortAccountDeletion => '繼續登入並放棄刪除賬戶';

  @override
  String get loginByQrcode => '透過二維碼登入 Mixin Messenger';

  @override
  String get loginByQrcodeTips1 => '開啟手機上的 Mixin Messenger。';

  @override
  String get loginByQrcodeTips2 => '掃描螢幕上的二維碼，確認登入。';

  @override
  String get makeGroupAdmin => '設定為群組管理員';

  @override
  String get media => '媒體';

  @override
  String get memo => '備註';

  @override
  String get messageE2ee => '此對話中的訊息使用端對端加密。點選瞭解更多。';

  @override
  String get messageNotFound => '找不到該訊息';

  @override
  String get messageNotSupport => '不支援此類訊息，請升級 Mixin 到最新版本。';

  @override
  String get messagePreview => '訊息預覽';

  @override
  String get messagePreviewDescription => '預覽新訊息通知中的訊息文字。';

  @override
  String get messages => '訊息';

  @override
  String get minimize => '最小化';

  @override
  String minute(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 分鐘',
      one: '$arg0 分鐘',
    );
    return '$_temp0';
  }

  @override
  String get mixinMessengerDesktop => 'Mixin Messenger 桌面';

  @override
  String get more => '更多';

  @override
  String get multisigTransaction => '多重簽名交易';

  @override
  String get mute => '靜音';

  @override
  String myMixinId(Object arg0) {
    return '我的 Mixin ID：$arg0';
  }

  @override
  String get myStickers => '我的表情';

  @override
  String get na => '暫無價格';

  @override
  String get name => '名稱';

  @override
  String get networkConnectionFailed => '網路連線失敗';

  @override
  String get networkError => '網路錯誤';

  @override
  String get newVersionAvailable => '發現新版本';

  @override
  String newVersionDescription(Object arg0, Object arg1) {
    return '發現新版本 Mixin Messenger $arg0，當前版本為 $arg1。是否要下載最新的版本？';
  }

  @override
  String get next => '下一步';

  @override
  String get nextConversation => '下一個會話';

  @override
  String get noAudio => '沒有音訊';

  @override
  String get noCamera => '沒有相機';

  @override
  String get noData => '沒有資料';

  @override
  String get noFiles => '沒有檔案';

  @override
  String get noLinks => '沒有連結';

  @override
  String get noMedia => '沒有媒體';

  @override
  String get noNetworkConnection => '無網路連線';

  @override
  String get noPosts => '沒有文章';

  @override
  String get noResults => '未找到相關結果';

  @override
  String get notFound => '沒有找到相應的訊息';

  @override
  String get notSupportBiometric => '此裝置不支援生物驗證';

  @override
  String get notificationContent => '啟用推送通知以即時更新價格警報和訊息。';

  @override
  String get notificationPermissionManually => '未允許通知，請到通知設定開啟。';

  @override
  String get notifications => '通知';

  @override
  String nowAnAddmin(Object arg0) {
    return '$arg0現在是管理員';
  }

  @override
  String get objects => '物件';

  @override
  String get oneByOneForward => '逐條轉發';

  @override
  String get oneHour => '1 小時';

  @override
  String get oneYear => '1 年';

  @override
  String get open => '開啟';

  @override
  String get openHomePage => '開啟主頁';

  @override
  String openLink(Object arg0) {
    return '開啟連結：$arg0';
  }

  @override
  String get openLogDirectory => '開啟日誌資料夾';

  @override
  String get openingBalance => '期初餘額';

  @override
  String get originalImage => '原圖';

  @override
  String get owner => '群主';

  @override
  String participantsCount(Object arg0) {
    return '$arg0 位群組成員';
  }

  @override
  String get passcodeIncorrect => '密碼不正確';

  @override
  String get password => '密碼';

  @override
  String pendingConfirmation(Object arg0, Object arg1, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0/$arg1 確認',
      one: '$arg0/$arg1 確認',
    );
    return '$_temp0';
  }

  @override
  String get phoneNumber => '手機號碼';

  @override
  String get photos => '照片';

  @override
  String get pickAConversation => '選擇一個對話，開始傳送資訊';

  @override
  String get picturesAndVideos => '影像 & 影片';

  @override
  String get pinTitle => '置頂';

  @override
  String pinnedMessageTitle(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 條置頂訊息',
      one: '$arg0 條置頂訊息',
    );
    return '$_temp0';
  }

  @override
  String get port => '埠號';

  @override
  String get post => '文章';

  @override
  String get preferences => '偏好設定';

  @override
  String get previousConversation => '上一個會話';

  @override
  String get privacyPolicy => '隱私政策';

  @override
  String get proxy => '代理';

  @override
  String get proxyAuth => '驗證（可選）';

  @override
  String get proxyConnection => '連線';

  @override
  String get proxyType => '代理型別';

  @override
  String get qrCodeExpiredDesc => '二維碼過期，請重試';

  @override
  String get quickSearch => '快速搜尋';

  @override
  String get quitMixin => '退出 Mixin';

  @override
  String get raw => '其他';

  @override
  String get rebate => '退款';

  @override
  String get recaptchaTimeout => '驗證超時';

  @override
  String get receiver => '至';

  @override
  String get recentChats => '最近聊天';

  @override
  String get reedit => '重新編輯';

  @override
  String get refresh => '重新整理';

  @override
  String get removeBot => '刪除機器人';

  @override
  String get removeChatFromCircle => '從圈子裡移除對話';

  @override
  String get removeContact => '刪除聯絡人';

  @override
  String get removeStickers => '移除所有表情';

  @override
  String get reply => '回覆';

  @override
  String get report => '舉報';

  @override
  String get reportAndBlock => '舉報並封鎖？';

  @override
  String get reportTitle => '給開發人員傳送聊天日誌？';

  @override
  String get resendCode => '重發驗證碼';

  @override
  String resendCodeIn(Object arg0) {
    return '$arg0 秒後重新發送驗證碼';
  }

  @override
  String get reset => '重置';

  @override
  String get resetLink => '重置邀請連結';

  @override
  String get restoreChat => '恢復聊天記錄';

  @override
  String get restoreChatTip => '從其他裝置恢復聊天記錄。請確保兩臺裝置連線到同一個 Wi-Fi 或熱點。';

  @override
  String get restoreFromOtherDevice => '從其他裝置恢復';

  @override
  String get retry => '重試';

  @override
  String get retryUploadFailed => '重新上傳失敗。';

  @override
  String get revokeMultisigTransaction => '撤銷多重簽名交易';

  @override
  String get save => '儲存';

  @override
  String get saveAs => '另存為';

  @override
  String get saveToCameraRoll => '儲存到相簿';

  @override
  String get sayHi => '打招呼';

  @override
  String get scamWarning => '警告：此賬號被大量使用者舉報，請謹防網路詐騙，注意個人財產安全';

  @override
  String get screenPasscode => '鎖屏密碼';

  @override
  String get search => '搜尋';

  @override
  String get searchContact => '搜尋使用者';

  @override
  String get searchConversation => '搜尋聊天記錄';

  @override
  String get searchEmpty => '找不到聯絡人或訊息。';

  @override
  String get searchPlaceholderNumber => '搜尋 Mixin ID 或手機號碼：';

  @override
  String searchRelatedMessage(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 條相關訊息',
      one: '$arg0 條相關訊息',
    );
    return '$_temp0';
  }

  @override
  String get searchUnread => '搜尋未讀會話';

  @override
  String get security => '安全';

  @override
  String get select => '選擇';

  @override
  String get send => '傳送';

  @override
  String get sendArchived => '打包成 zip 傳送';

  @override
  String get sendQuickly => '快速傳送';

  @override
  String get sendToDeveloper => '把日誌發給開發者';

  @override
  String get sendWithoutCompression => '傳送原始檔案';

  @override
  String get sendWithoutSound => '靜音傳送';

  @override
  String get set => '設定';

  @override
  String setDisappearingMessageTimeTo(Object arg0, Object arg1) {
    return '$arg0將限時訊息設定為 $arg1';
  }

  @override
  String get setPasscodeDesc => '設定解鎖密碼';

  @override
  String get settingAuthSearchHint => 'Mixin ID, 暱稱';

  @override
  String get settingBackupTips =>
      '備份你的聊天記錄到 iCloud。如果你丟失或者更換手機，你可以在重新安裝 Mixin Messenger 時恢復你的聊天記錄。注意備份到 iCloud 中的聊天記錄不受端對端加密保護！';

  @override
  String settingDeleteAccountPinContent(Object arg0) {
    return '如果您繼續，您的個人資料和賬戶資訊將在$arg0被刪除。閱讀我們的檔案以**瞭解更多**。';
  }

  @override
  String get settingDeleteAccountUrl =>
      'https://support.mixin.one/zh/article/5aac5l2v5yig6zmk6lsm5y377yf-1uteq30';

  @override
  String get share => '分享';

  @override
  String get shareApps => '分享的應用';

  @override
  String get shareContact => '分享聯絡人';

  @override
  String get shareError => '分享出錯';

  @override
  String get shareLink => '分享邀請連結';

  @override
  String shareMessageDescription(Object arg0, Object arg1) {
    return '你確定要傳送來自$arg0的$arg1？';
  }

  @override
  String shareMessageDescriptionEmpty(Object arg0) {
    return '你確定要傳送該$arg0？';
  }

  @override
  String get sharedMedia => '媒體內容';

  @override
  String get show => '顯示';

  @override
  String get showAvatar => '顯示頭像';

  @override
  String get showIdentityNumber => '顯示 ID';

  @override
  String get showMixin => '顯示 Mixin';

  @override
  String get signIn => '登入';

  @override
  String get signOut => '退出登入';

  @override
  String get signWithMobileNumber => '透過手機號登入';

  @override
  String get signWithQrcode => '透過二維碼登入';

  @override
  String get smileysAndPeople => '表情符號與人物';

  @override
  String get snapshotHash => '快照雜湊';

  @override
  String get status => '交易狀態';

  @override
  String get sticker => '貼紙';

  @override
  String get stickerAddInvalidSize =>
      '貼紙要求大於 1KB 且小於 1MB，寬高大於 128 畫素且小於 1024 畫素。';

  @override
  String get stickerAlbumDetail => '表情詳情';

  @override
  String get stickerStore => '表情商店';

  @override
  String get storageAutoDownloadDescription => '更改媒體的自動下載設定。';

  @override
  String get storageUsage => '儲存空間';

  @override
  String get strangerHint => '對方不是你的聯絡人';

  @override
  String get strangers => '陌生人';

  @override
  String get successful => '成功';

  @override
  String get symbols => '符號';

  @override
  String get syncFromOtherDevice => '從其他裝置同步';

  @override
  String get syncToOtherDevice => '同步到其他裝置';

  @override
  String get termsOfService => '服務條款';

  @override
  String get text => '文字';

  @override
  String get theme => '主題';

  @override
  String get thisMessageWasDeleted => '此訊息已撤回';

  @override
  String get time => '時間';

  @override
  String get to => '至';

  @override
  String get today => '今天';

  @override
  String get toggleChatInfo => '展開/關閉會話資訊';

  @override
  String get transactionHash => '交易雜湊';

  @override
  String get transactionId => '交易編號';

  @override
  String get transactionType => '交易型別';

  @override
  String get transactions => '交易記錄';

  @override
  String get transactionsCannotBeDeleted => '交易記錄不會被刪除';

  @override
  String get transcript => '聊天記錄';

  @override
  String get transfer => '轉賬';

  @override
  String get transferCompleted => '同步完成';

  @override
  String get transferProtocolVersionNotMatched => '版本不匹配，無法同步資料，請先升級應用。';

  @override
  String get transferringChats => '同步聊天記錄中';

  @override
  String get transferringChatsTips => '同步時請不要關閉螢幕並保持 Mixin 在前臺執行。';

  @override
  String get travelAndPlaces => '旅行與地點';

  @override
  String get typeMessage => '輸入訊息';

  @override
  String unableToOpenFile(Object arg0) {
    return '無法開啟檔案：$arg0';
  }

  @override
  String get unblock => '解除封鎖';

  @override
  String unitDay(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '天',
      one: '天',
    );
    return '$_temp0';
  }

  @override
  String unitHour(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '小時',
      one: '小時',
    );
    return '$_temp0';
  }

  @override
  String unitMinute(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '分',
      one: '分',
    );
    return '$_temp0';
  }

  @override
  String unitSecond(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '秒',
      one: '秒',
    );
    return '$_temp0';
  }

  @override
  String unitWeek(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '週',
      one: '週',
    );
    return '$_temp0';
  }

  @override
  String get unknowError => '未知錯誤';

  @override
  String get unlockMixinMessenger => '解鎖 Mixin Messenger';

  @override
  String get unlockWithWasscode => '輸入密碼解鎖 Mixin Messenger';

  @override
  String get unmute => '取消靜音';

  @override
  String get unpin => '取消置頂';

  @override
  String get unpinAllMessages => '取消所有置頂訊息';

  @override
  String get unpinAllMessagesConfirmation => '確定取消置頂所有訊息麼？';

  @override
  String get unreadMessages => '未讀訊息';

  @override
  String get updateMixin => '升級 Mixin';

  @override
  String updateMixinDescription(Object arg0) {
    return '當前版本（$arg0）不再可用!\n請點選下面的“更新”按鈕以更新到最新版本。';
  }

  @override
  String get upgrade => '升級';

  @override
  String get upgrading => '升級中...';

  @override
  String get useBiometric => '使用生物識別';

  @override
  String get userDeleteHint => '該使用者已經刪除了賬號。';

  @override
  String get userNotFound => '找不到這個使用者';

  @override
  String get username => '使用者名稱';

  @override
  String valueNow(Object arg0) {
    return '價值 $arg0';
  }

  @override
  String valueThen(Object arg0) {
    return '當時價值 $arg0';
  }

  @override
  String get verifyPin => '驗證 PIN';

  @override
  String get video => '影片';

  @override
  String get videos => '影片';

  @override
  String get waitingForThisMessage => '正在等待這條訊息。';

  @override
  String get waitingOtherDeviceConnection => '等待其他裝置連線。';

  @override
  String get webview2RuntimeInstallDescription =>
      '該裝置暫未安裝 WebView2 元件，請先下載並安裝 WebView2 Runtime。';

  @override
  String get webviewRuntimeUnavailable => 'WebView2 元件不可用';

  @override
  String get window => '視窗';

  @override
  String get withdrawal => '提現';

  @override
  String get withdrawalHash => '提現雜湊';

  @override
  String get you => '你';

  @override
  String get youDeletedThisMessage => '你撤回了一條訊息';

  @override
  String get zoom => '縮放';
}
