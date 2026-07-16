// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Japanese (`ja`).
class AppLocalizationsJa extends AppLocalizations {
  AppLocalizationsJa([String locale = 'ja']) : super(locale);

  @override
  String get aMessage => 'メッセージ';

  @override
  String get about => 'Mixinについて';

  @override
  String get account => 'アカウント';

  @override
  String get activity => 'Activity';

  @override
  String get add => '追加';

  @override
  String get addACaption => 'Add a caption';

  @override
  String get addBotWithPlus => '+ Botを追加';

  @override
  String get addContact => '友だちを追加';

  @override
  String get addContactWithPlus => '友だちを追加';

  @override
  String get addFile => 'ファイルを追加';

  @override
  String get addGroupDescription => 'グループアナウンス';

  @override
  String get addParticipants => 'メンバーを追加';

  @override
  String get addPeopleSearchHint => 'Mixin IDまたは電話番号';

  @override
  String get addProxy => 'Add Proxy';

  @override
  String get addSticker => 'スタンプを追加';

  @override
  String get addStickerFailed => 'スタンプの追加に失敗しました';

  @override
  String get addStickers => 'スタンプを追加';

  @override
  String get addToCircle => 'Add to Circle';

  @override
  String get added => '追加済み';

  @override
  String get address => 'アドレス';

  @override
  String get admin => '管理者';

  @override
  String get alertKeyContactContactMessage => '連絡先が届きました';

  @override
  String get allChats => 'チャット';

  @override
  String get animalsAndNature => 'Animals & Nature';

  @override
  String get anonymous => 'Anonymous';

  @override
  String get anonymousNumber => '匿名番号';

  @override
  String get appCardShareDisallow => 'このURLは共有できません';

  @override
  String get appearance => '言語とテーマ';

  @override
  String get archivedFolder => 'アーカイブされたフォルダ';

  @override
  String get assetType => '資産タイプ';

  @override
  String get audio => '音声メッセージ';

  @override
  String get audios => '音声メッセージ';

  @override
  String get autoBackup => 'チャット履歴の自動バックアップ';

  @override
  String get autoLock => 'Auto Lock';

  @override
  String get avatar => 'アバター';

  @override
  String get backup => 'チャット履歴のバックアップ';

  @override
  String get backupChat => 'チャットをバックアップ';

  @override
  String get backupToOtherDevice => '別のデバイスへバックアップ';

  @override
  String get backupToOtherDeviceTips =>
      'チャット履歴を別のデバイスにバックアップします。両方のデバイスが同じWi-Fiまたはホットスポットに接続されていることを確認してください。';

  @override
  String get backupWaitingOtherDevice => '別のデバイスでMixinを開き、そこで復元を開始してください。';

  @override
  String get biography => '自己紹介文';

  @override
  String get biometric => 'Biometric';

  @override
  String get block => 'ブロック';

  @override
  String get botNotFound => 'ミニアプリが見つかりません';

  @override
  String get bots => 'ミニアプリ';

  @override
  String get botsTitle => 'Bot一覧';

  @override
  String get bringAllToFront => 'Bring All to Front';

  @override
  String get canNotRecognizeQrCode => 'QRコードを認識できません';

  @override
  String get cancel => 'キャンセル';

  @override
  String get card => 'カード';

  @override
  String get change => '変更';

  @override
  String get changeNumber => '電話番号を変更';

  @override
  String get changeNumberInstead => '電話番号を変更';

  @override
  String changedDisappearingMessageSettings(Object arg0) {
    return '$arg0が消えるメッセージを設定しました';
  }

  @override
  String get chatBackup => 'チャットのバックアップ';

  @override
  String get chatBackupAndRestore => 'チャットのバックアップと復元';

  @override
  String get chatBotReceptionTitle => 'ミニアプリを使用するためにボタンをタップしてください';

  @override
  String chatDecryptionFailedHint(Object arg0) {
    return '$arg0が参加し暗号化セッションが開始するまで待機しています...';
  }

  @override
  String chatDeleteMessage(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0のメッセージを削除しますか？',
      one: '$arg0のメッセージを削除しますか？',
    );
    return '$_temp0';
  }

  @override
  String chatGroupAdd(Object arg0, Object arg1) {
    return '$arg0が$arg1を追加しました';
  }

  @override
  String chatGroupExit(Object arg0) {
    return '$arg0が退出しました';
  }

  @override
  String chatGroupJoin(Object arg0) {
    return '$arg0が招待リンクから参加しました';
  }

  @override
  String chatGroupRemove(Object arg0, Object arg1) {
    return '$arg0が$arg1を退会させました';
  }

  @override
  String get chatHintE2e => 'E2E暗号化';

  @override
  String get chatNotSupportUriOnPhone => 'URLが読み込めません。お使いの携帯電話の設定をご確認ください';

  @override
  String get chatNotSupportUrl =>
      'https://support.mixin.one/en/article/how-to-do-when-you-receive-a-message-like-this-this-type-of-message-is-not-supported-please-upgrade-mixin-17j1t3p';

  @override
  String get chatNotSupportViewOnPhone =>
      'この種類のチャットは読み込めません。お使いの携帯電話の設定をご確認ください';

  @override
  String chatPinMessage(Object arg0, Object arg1) {
    return '$arg0は$arg1をピン留めしました';
  }

  @override
  String get chatTextSize => 'Chat Text Size';

  @override
  String get chats => 'Chats';

  @override
  String get checkNewVersion => 'アップデートを確認';

  @override
  String circleSubtitle(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0件の会話',
      one: '$arg0件の会話',
    );
    return '$_temp0';
  }

  @override
  String circleTitle(Object arg0) {
    return '$arg0のグループリスト';
  }

  @override
  String get circles => 'グループリスト';

  @override
  String get clear => 'クリア';

  @override
  String get clearChat => 'チャットを削除する';

  @override
  String get clearFilter => 'Clear filter';

  @override
  String get clickToReloadQrcode => 'リロード';

  @override
  String get close => '閉じる';

  @override
  String get closeWindow => 'ウィンドウを閉じる';

  @override
  String get closingBalance => 'Closing Balance';

  @override
  String get collapse => 'サイドバー';

  @override
  String get collectible => 'Collectible';

  @override
  String get collectibles => 'Collectibles';

  @override
  String get collection => 'Collection';

  @override
  String get combineAndForward => 'まとめて転送';

  @override
  String get confirm => '確認する';

  @override
  String get confirmPasscodeDesc => 'Enter again to confirm the passcode';

  @override
  String get confirmSyncChatsFromPhone =>
      'Are you sure to sync the chat history from the phone?';

  @override
  String get confirmSyncChatsToPhone =>
      'Are you sure to sync the chat history to the phone?';

  @override
  String get confirmations => 'Confirmations';

  @override
  String get contact => '連絡先';

  @override
  String contactMixinId(Object arg0) {
    return 'Mixin ID: $arg0';
  }

  @override
  String get contactMuteTitle => '通知をミュートする';

  @override
  String get contactTitle => '連絡先';

  @override
  String get contentTooLong => '文字数を減らしてください';

  @override
  String get contentVoice => '[音声通話]';

  @override
  String get continueText => '続ける';

  @override
  String get conversation => 'チャットルーム';

  @override
  String conversationDeleteTitle(Object arg0) {
    return 'チャットを削除する：$arg0';
  }

  @override
  String get copy => 'コピー';

  @override
  String get copyImage => 'Copy Image';

  @override
  String get copyInvite => '招待リンクをコピーする';

  @override
  String get copyLink => 'リンクをコピー';

  @override
  String get copySelectedText => 'Copy Selected Text';

  @override
  String get copyText => 'Copy Text';

  @override
  String get create => '作成';

  @override
  String get createCircle => '新しいグループリスト';

  @override
  String get createConversation => '新しいチャットルーム';

  @override
  String get createGroup => '新しいグループ';

  @override
  String createdAt(Object arg0) {
    return 'Created $arg0';
  }

  @override
  String createdThisGroup(Object arg0) {
    return '$arg0がグループを作成しました';
  }

  @override
  String get customTime => '日時';

  @override
  String get dark => 'ダーク';

  @override
  String get dataAndStorageUsage => 'ストレージ使用率';

  @override
  String get dataError => 'データエラー';

  @override
  String get dataLoading => 'ロード中...';

  @override
  String get databaseCorruptedTips =>
      'The database is corrupted and cannot be recovered. Clicking continue will create a new database file.';

  @override
  String get databaseLockedTips =>
      'The database file is locked and cannot be accessed. Please try restarting the application or the system and try again.';

  @override
  String get databaseNotADbTips =>
      'Cannot open the database. The file is not a valid database file.';

  @override
  String get databaseRecreateTips =>
      'Create a new database file. The old file will be deleted.';

  @override
  String get databaseUpgradeTips => 'データベースをアップグレードしています。アプリを閉じないでください。';

  @override
  String get delete => '削除';

  @override
  String get deleteAccountDetailHint => 'ローカルメッセージとiCloudにバックアップされた内容は削除されません';

  @override
  String get deleteAccountHint => 'アカウント情報とプロフィール画像を削除します';

  @override
  String get deleteChat => 'チャットを削除';

  @override
  String get deleteChatDescription =>
      'チャットを削除すると、この端末のみからメッセージが削除されます。他の端末からは削除されません。';

  @override
  String get deleteCircle => 'グループリストを削除';

  @override
  String get deleteForEveryone => '全員のチャットから削除';

  @override
  String get deleteForMe => 'あなたのチャットから削除';

  @override
  String get deleteGroup => 'グループを削除';

  @override
  String get deleteMyAccount => 'アカウント削除';

  @override
  String deleteTheCircle(Object arg0) {
    return '$arg0のグループリストを削除しますか？';
  }

  @override
  String get deposit => '入金';

  @override
  String get depositHash => 'Deposit Hash';

  @override
  String get developer => '開発者向け情報';

  @override
  String get deviceTransferFailed => 'Transfer failed';

  @override
  String disableDisappearingMessage(Object arg0) {
    return '$arg0が消えるメッセージを無効にしました';
  }

  @override
  String get disabled => 'Disabled';

  @override
  String disappearingCustomTimeMaxWarning(Object arg0) {
    return 'The maximum time is $arg0.';
  }

  @override
  String get disappearingMessage => '消えるメッセージ';

  @override
  String get disappearingMessageHint =>
      '有効にすると、このチャットで送受信された新しいメッセージは、見た後に消えます。詳しくは、こちらをお読みください。';

  @override
  String get discard => 'Discard';

  @override
  String get discardRecordingWarning =>
      'Are you sure you want to stop recording and discard your voice message?';

  @override
  String get dismissAsAdmin => '管理者権限を解除';

  @override
  String get done => '完了';

  @override
  String get download => 'ダウンロード';

  @override
  String get downloadLink => 'ダウンロードリンク:';

  @override
  String get draft => 'Draft';

  @override
  String get dragAndDropFileHere => 'ファイルをドラッグ＆ドロップ';

  @override
  String get durationIsTooShort => '期間が短すぎます';

  @override
  String get edit => '編集';

  @override
  String get editCircleName => 'グループリスト名を編集';

  @override
  String get editConversations => 'チャットルームを編集';

  @override
  String get editGroupDescription => 'グループアナウンスを編集';

  @override
  String get editGroupName => 'グループ名を編集';

  @override
  String get editImageClearWarning => 'すべての変更が失われます。本当に終了しますか？';

  @override
  String get editName => '名前を変更';

  @override
  String get editProfile => 'プロフィールを編集';

  @override
  String get enablePushNotification => '通知をオンにする';

  @override
  String get encryptZipFileWithPassword =>
      'Encrypt the ZIP file with a password';

  @override
  String get enterPinToDeleteAccount => 'アカウントを削除するためにPINコードを入力してください';

  @override
  String get enterToSend => 'Return/Enter ⏎ to Send';

  @override
  String get enterYourPhoneNumber => '電話番号を入力してください';

  @override
  String get enterYourPinToContinue => 'PINコードを入力して、続けてください';

  @override
  String get errorAccessLimited => 'ERROR 403: Access Limited';

  @override
  String get errorAddressExists => 'アドレスが存在しません。アドレスが正常に追加されていることを確認してください。';

  @override
  String get errorAddressNotSync => 'アドレスの更新に失敗しました。もう一度やり直してください。';

  @override
  String get errorAlreadyBondedReferralCode =>
      'ERROR 10731: This account has already applied a referral code';

  @override
  String get errorAssetExists => '資産がありません';

  @override
  String get errorAuthentication => 'エラー 401：サインインをして続ける';

  @override
  String get errorBadData => 'エラー 10002：リクエストデータが無効です';

  @override
  String get errorBlockchain => 'エラー 30100：ブロックチェーンが同期できていません。後程もう一度お試しください。';

  @override
  String get errorConnectionTimeout => 'ネットワーク接続がタイムアウトしました';

  @override
  String get errorFullGroup => 'エラー 20116：グループチャットが満員です';

  @override
  String get errorInsufficientBalance => 'エラー 20117：残高が不足しています';

  @override
  String errorInsufficientTransactionFeeWithAmount(Object arg0) {
    return 'エラー 20124：取引手数料が不足しています。ウォレットに手数料用に最低でも$arg0があることを確認してください。';
  }

  @override
  String errorInvalidAddress(Object arg0, Object arg1) {
    return 'エラー30102：無効なアドレス形式です。正しい$arg0 $arg1 アドレスを入力してください。';
  }

  @override
  String get errorInvalidAddressPlain => 'エラー30102：無効なアドレス形式です';

  @override
  String get errorInvalidCodeTooFrequent =>
      'エラー 20129：認証コードを送信する頻度が多すぎます。しばらくしてからもう一度お試しください。';

  @override
  String get errorInvalidEmergencyContact =>
      'ERROR 20130: Invalid recovery contact';

  @override
  String get errorInvalidPinFormat => 'エラー 20118：無効なPINフォーマットです';

  @override
  String get errorInviterPlanExpired =>
      'ERROR 10737: The inviter has no valid plan';

  @override
  String get errorLegacyPin =>
      'ERROR 20118: To enhance the security of the Mixin network, Mixin API has temporarily suspended the upgrading from D3M-PIN to TIP. Please refer to the documentation for details and register for processing.';

  @override
  String get errorNetworkTaskFailed =>
      'ネットワーク接続に失敗しました。ネットワーク接続状態を確認した後にもう一度試してください。';

  @override
  String get errorNoPinToken =>
      'No token. Please sign in again and try this feature again.';

  @override
  String get errorNotFound => 'エラー 404：結果なし';

  @override
  String get errorNotSupportedAudioFormat =>
      'サポートされていないオーディオ形式です。他のアプリで開いてください。';

  @override
  String get errorNumberReachedLimit => 'エラー 20132：数が上限に達しています';

  @override
  String errorOldVersion(Object arg0) {
    return 'エラー 10006：このサービスを引き続き使用するには、Mixin($arg0)をアップデートしてください。';
  }

  @override
  String get errorOpenLocation => '地図アプリがありません';

  @override
  String get errorPermission => '必要な権限を開いてください';

  @override
  String get errorPhoneInvalidFormat => 'エラー 20110：無効な電話番号です';

  @override
  String get errorPhoneSmsDelivery => 'エラー 10003：SMSの送信に失敗しました';

  @override
  String get errorPhoneVerificationCodeExpired =>
      'エラー 20114：電話番号認証コードの有効期限が切れています';

  @override
  String get errorPhoneVerificationCodeInvalid => 'エラー 20113：電話番号認証コードが無効です';

  @override
  String get errorPinCheckTooManyRequest =>
      '入力ミスが5回に達したため一時的にロックします。24時間後にもう一度試してください。';

  @override
  String get errorPinIncorrect => 'PINコードが違います';

  @override
  String errorPinIncorrectWithTimes(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'エラー20119：PINコードが間違っています。あと$arg0回入力可能です。24時間後に再試行してください。',
      one: 'エラー 20119：PINコードが間違っています。あと$arg0回入力可能です。24時間後に再試行してください。',
    );
    return '$_temp0';
  }

  @override
  String get errorRecaptchaIsInvalid => 'エラー 10004：Recaptchaが無効です';

  @override
  String errorServer5xxCode(Object arg0) {
    return 'サーバーメンテナンス中：$arg0';
  }

  @override
  String get errorTooManyRequest => 'エラー 429：レート制限を超過しています';

  @override
  String get errorTooManyStickers => 'エラー 20126：スタンプが多すぎます';

  @override
  String get errorTooSmallTransferAmount => '送金数量が小さすぎます';

  @override
  String get errorTooSmallWithdrawAmount => 'エラー 20127：出金額が小さすぎます';

  @override
  String get errorTranscriptForward => '添付ファイルはすべてダウンロード後、転送してください。';

  @override
  String get errorTransferToDeactivatedUser =>
      'ERROR 20160: Transfers cannot be made to a deactivated user';

  @override
  String get errorUnableToOpenMedia => 'メディアを開くことができるアプリがありません';

  @override
  String errorUnknownWithCode(Object arg0) {
    return 'エラー：$arg0';
  }

  @override
  String errorUnknownWithMessage(Object arg0) {
    return 'エラー：$arg0';
  }

  @override
  String get errorUploadAttachmentFailed =>
      'Failed to upload message attachment';

  @override
  String get errorUsedPhone => 'エラー20122：この電話番号はすでに他のアカウントと紐づけられています';

  @override
  String get errorUserInvalidFormat => '無効なユーザーIDです';

  @override
  String get errorWithdrawalMemoFormatIncorrect => 'エラー20131：出金メモのフォーマットが不正確です';

  @override
  String get errorWithdrawalSuspend =>
      'ERROR 20137: Withdrawals are suspended.';

  @override
  String get exit => '退出';

  @override
  String get exitGroup => 'グループから退出';

  @override
  String get failed => '失敗しました';

  @override
  String get failedToOpenDatabase =>
      'An error occurred while opening the database.';

  @override
  String get fee => '手数料';

  @override
  String get file => 'ファイル';

  @override
  String get fileChooserError => 'ファイル選択エラー';

  @override
  String get fileDoesNotExist => 'ファイルが存在しません';

  @override
  String get fileError => 'ファイルエラー';

  @override
  String get files => 'ファイル';

  @override
  String get flags => 'Flags';

  @override
  String get followSystem => 'システム設定に従う';

  @override
  String get followUsOnFacebook => 'FacebookでMixinをフォロー';

  @override
  String get followUsOnX => 'XでMixinをフォロー';

  @override
  String get foodAndDrink => 'Food & Drink';

  @override
  String get formatNotSupported => 'サポートされていないフォーマットです';

  @override
  String get forward => '転送';

  @override
  String get from => '送信元';

  @override
  String get fromWithColon => '送信元:';

  @override
  String get generateQrcode => 'Generate QR Code';

  @override
  String get groupAlreadyIn => 'すでにこのグループに参加しています。';

  @override
  String get groupCantSend => '参加者ではないため、このグループにメッセージを送ることができません。';

  @override
  String get groupName => 'グループ名';

  @override
  String get groupParticipants => '参加者';

  @override
  String groupPopMenuMessage(Object arg0) {
    return '$arg0へメッセージを送信';
  }

  @override
  String groupPopMenuRemove(Object arg0) {
    return '$arg0をグループから退会させる';
  }

  @override
  String get groups => 'グループ';

  @override
  String get groupsInCommon => '共通のグループ';

  @override
  String get hash => 'HASH';

  @override
  String get help => 'ヘルプ';

  @override
  String get helpCenter => 'ヘルプセンター';

  @override
  String get hideMixin => 'Mixinを非表示にする';

  @override
  String get host => 'Host';

  @override
  String hour(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0時間',
      one: '$arg0時間',
    );
    return '$_temp0';
  }

  @override
  String get howAreYou => 'こんにちは、調子はどうですか';

  @override
  String get iAmGood => 'いい気分';

  @override
  String get id => 'ID';

  @override
  String get ignoreThisVersion => '最新版を無視';

  @override
  String get image => '画像';

  @override
  String get includeFiles => 'ファイルが含まれています';

  @override
  String get includeVideos => '動画が含まれています';

  @override
  String get initializing => '初期化中…';

  @override
  String get invalidStickerFormat => 'スタンプのフォーマットが無効です';

  @override
  String get inviteInfo => 'リンクを知っている人はだれでもグループに参加可能です、信頼できる人だけに共有してください';

  @override
  String get inviteToGroupViaLink => 'リンクを使って招待する';

  @override
  String get joinGroupWithPlus => 'グループに参加';

  @override
  String joinedIn(Object arg0) {
    return '参加日 $arg0';
  }

  @override
  String landingDeleteContent(Object arg0, Object arg1) {
    return 'You requested to delete your account on $arg0. The account will be deleted on $arg1. If you continue to log in, your account deletion will be cancelled.';
  }

  @override
  String landingInvitationDialogContent(Object arg0) {
    return '4桁のコードを電話番号$arg0に送信します、次の画面でコードを入力してください';
  }

  @override
  String landingValidationTitle(Object arg0) {
    return '$arg0に送信された4桁のコードを入力してください';
  }

  @override
  String get learnMore => 'こちら';

  @override
  String get less => '少なく表示';

  @override
  String get light => 'ライト';

  @override
  String get linkedDevice => 'linked device';

  @override
  String get live => '配信';

  @override
  String get loading => 'ロード中...';

  @override
  String get loadingTime => 'システム時刻が異常です。修正後、使用してください';

  @override
  String get locateToChat => 'チャットを探す';

  @override
  String get location => '位置情報';

  @override
  String get lock => 'Lock';

  @override
  String get logIn => 'ログイン';

  @override
  String get loginAndAbortAccountDeletion => 'そのままログインし、アカウント削除をキャンセルします';

  @override
  String get loginByQrcode => 'QRコードでMixinにログインする';

  @override
  String get loginByQrcodeTips1 => '携帯でMixinを開き';

  @override
  String get loginByQrcodeTips2 => '画面に表示されるQRコードを読み取り、ログインします';

  @override
  String get makeGroupAdmin => '管理者権限を付与';

  @override
  String get media => 'メディア';

  @override
  String get memo => 'メモ';

  @override
  String get messageE2ee => 'チャットルームでのメッセージはE2Eで暗号化されています。詳細はタップしてください。';

  @override
  String get messageNotFound => 'メッセージが見つかりません';

  @override
  String get messageNotSupport => 'このメッセージは未対応であるため、Mixinを最新版にアップデートしてください。';

  @override
  String get messagePreview => 'Message Preview';

  @override
  String get messagePreviewDescription => '新着メッセージ通知内のメッセージテキストをプレビューします';

  @override
  String get messages => 'メッセージ';

  @override
  String get minimize => '最小化';

  @override
  String minute(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 Minutes',
      one: '$arg0 Minute',
    );
    return '$_temp0';
  }

  @override
  String get mixinMessengerDesktop => 'Mixin デスクトップ';

  @override
  String get more => 'もっとみる';

  @override
  String get multisigTransaction => 'マルチシグトランザクション';

  @override
  String get mute => 'Mute';

  @override
  String myMixinId(Object arg0) {
    return 'マイMixin ID:$arg0';
  }

  @override
  String get myStickers => 'マイスタンプ';

  @override
  String get na => 'なし';

  @override
  String get name => '名前';

  @override
  String get networkConnectionFailed => 'Network connection failed';

  @override
  String get networkError => 'ネットワークエラー';

  @override
  String get newVersionAvailable => '最新版の公開';

  @override
  String newVersionDescription(Object arg0, Object arg1) {
    return 'Mixin Messenger $arg0 が利用可能です。現在のバージョンは $arg1 です。今すぐダウンロードしますか？';
  }

  @override
  String get next => '次へ';

  @override
  String get nextConversation => '次のチャットルーム';

  @override
  String get noAudio => '音声メッセージがありません';

  @override
  String get noCamera => 'カメラを認識できません';

  @override
  String get noData => 'データがありません';

  @override
  String get noFiles => 'ファイルがありません';

  @override
  String get noLinks => 'リンクがありません';

  @override
  String get noMedia => 'メディアがありません';

  @override
  String get noNetworkConnection => 'ネットワーク接続がありません';

  @override
  String get noPosts => '投稿がありません';

  @override
  String get noResults => '結果なし';

  @override
  String get notFound => '見つかりません';

  @override
  String get notSupportBiometric =>
      'This device does not support biometric authentication';

  @override
  String get notificationContent =>
      'Enable push notifications to stay updated on price alerts and messages in real time.';

  @override
  String get notificationPermissionManually => '通知は許可されていませんので、通知設定から許可してください。';

  @override
  String get notifications => '通知';

  @override
  String nowAnAddmin(Object arg0) {
    return '$arg0は管理者です';
  }

  @override
  String get objects => 'Objects';

  @override
  String get oneByOneForward => 'それぞれ転送する';

  @override
  String get oneHour => '1時間';

  @override
  String get oneYear => '1年間';

  @override
  String get open => 'Open';

  @override
  String get openHomePage => 'ホームページを開く';

  @override
  String openLink(Object arg0) {
    return 'リンクを開く: $arg0';
  }

  @override
  String get openLogDirectory => 'ログディレクトリを開く';

  @override
  String get openingBalance => 'Opening Balance';

  @override
  String get originalImage => 'オリジナル';

  @override
  String get owner => 'オーナー';

  @override
  String participantsCount(Object arg0) {
    return '$arg0人のメンバー';
  }

  @override
  String get passcodeIncorrect => 'Passcode incorrect';

  @override
  String get password => 'Password';

  @override
  String pendingConfirmation(Object arg0, Object arg1, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0/$arg1 承認',
      one: '$arg0/$arg1 承認',
    );
    return '$_temp0';
  }

  @override
  String get phoneNumber => '電話番号';

  @override
  String get photos => '写真';

  @override
  String get pickAConversation => 'チャットルームを選択して、メッセージを送信してみましょう';

  @override
  String get picturesAndVideos => 'Pictures & Videos';

  @override
  String get pinTitle => 'ピン留め';

  @override
  String pinnedMessageTitle(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0個のピン留めされたメッセージ',
      one: '$arg0個のピン留めされたメッセージ',
    );
    return '$_temp0';
  }

  @override
  String get port => 'Port';

  @override
  String get post => '投稿';

  @override
  String get preferences => '環境設定';

  @override
  String get previousConversation => '過去のチャットルーム';

  @override
  String get privacyPolicy => 'プライバシーポリシー';

  @override
  String get proxy => 'Proxy';

  @override
  String get proxyAuth => 'Authentication (Optional)';

  @override
  String get proxyConnection => 'Connection';

  @override
  String get proxyType => 'Proxy Type';

  @override
  String get qrCodeExpiredDesc => 'QR code expired. Please try again.';

  @override
  String get quickSearch => 'クイック検索';

  @override
  String get quitMixin => '終了';

  @override
  String get raw => 'その他';

  @override
  String get rebate => '払い戻し';

  @override
  String get recaptchaTimeout => 'Recaptchaがタイムアウトしました';

  @override
  String get receiver => '受取人';

  @override
  String get recentChats => 'チャット';

  @override
  String get reedit => '再編集';

  @override
  String get refresh => '更新';

  @override
  String get removeBot => 'Myミニアプリから削除';

  @override
  String get removeChatFromCircle => 'グループリストからチャットを削除';

  @override
  String get removeContact => '連絡先を削除';

  @override
  String get removeStickers => 'スタンプの削除';

  @override
  String get reply => '返信';

  @override
  String get report => '報告';

  @override
  String get reportAndBlock => '報告してブロックしますか?';

  @override
  String get reportTitle => 'Mixinの開発者へ会話記録を送信しますか?';

  @override
  String get resendCode => 'コードを再送する';

  @override
  String resendCodeIn(Object arg0) {
    return '$arg0秒後にコードを再送';
  }

  @override
  String get reset => 'リセット';

  @override
  String get resetLink => 'リンクを取り消す';

  @override
  String get restoreChat => 'チャットを復元';

  @override
  String get restoreChatTip =>
      '別のデバイスからチャット履歴を復元します。両方のデバイスが同じWi-Fiまたはホットスポットに接続されていることを確認してください。';

  @override
  String get restoreFromOtherDevice => '別のデバイスから復元';

  @override
  String get retry => 'リトライ';

  @override
  String get retryUploadFailed => 'アップロードの再試行に失敗しました。';

  @override
  String get revokeMultisigTransaction => 'マルチシグトランザクションを取り消す';

  @override
  String get save => '保存';

  @override
  String get saveAs => '名前をつけて保存';

  @override
  String get saveToCameraRoll => 'カメラロールに保存する';

  @override
  String get sayHi => '挨拶をしましょう';

  @override
  String get scamWarning => '警告：たくさん報告されているユーザーです、詐欺に気をつけてください';

  @override
  String get screenPasscode => 'Screen Passcode';

  @override
  String get search => '検索';

  @override
  String get searchContact => '連絡先を検索';

  @override
  String get searchConversation => 'チャットルームを検索';

  @override
  String get searchEmpty => '一致する情報は見つかりませんでした';

  @override
  String get searchPlaceholderNumber => 'Mixin ID または電話番号を検索';

  @override
  String searchRelatedMessage(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0個の関連するメッセージ',
      one: '$arg0個の関連するメッセージ',
    );
    return '$_temp0';
  }

  @override
  String get searchUnread => 'Search Unread';

  @override
  String get secretUrl => 'https://mixin.one/pages/1000007';

  @override
  String get security => 'セキュリティ';

  @override
  String get select => '選択';

  @override
  String get send => '送る';

  @override
  String get sendArchived => '1つのZIPファイルにアーカイブ';

  @override
  String get sendQuickly => 'クイック送信';

  @override
  String get sendToDeveloper => '開発者へ送信';

  @override
  String get sendWithoutCompression => '圧縮せずに送信';

  @override
  String get sendWithoutSound => '通知音を鳴らさずに送信する';

  @override
  String get set => '設定';

  @override
  String setDisappearingMessageTimeTo(Object arg0, Object arg1) {
    return '1\$sは、消えるメッセージの有効時間を$arg0に設定しました。';
  }

  @override
  String get setPasscodeDesc => 'Set a passcode to unlock Mixin Messenger';

  @override
  String get settingAuthSearchHint => 'Mixin ID, 名前';

  @override
  String get settingBackupTips =>
      'iCloudにチャット履歴をバックアップします。 iPhoneを紛失または機種変更した場合にMixinを再インストールしてチャット履歴を復元できます。バックアップしたメッセージはMixinのE2E暗号によって保護されていません。';

  @override
  String settingDeleteAccountPinContent(Object arg0) {
    return '$arg0と紐付けられたプロフィールとアカウント情報が削除されます。詳細はこちらをご覧ください。';
  }

  @override
  String get settingDeleteAccountUrl =>
      'https://support.mixin.one/en/article/how-to-delete-my-account-19fkagl';

  @override
  String get share => '共有';

  @override
  String get shareApps => '共有ずみのアプリ';

  @override
  String get shareContact => 'Share Contact';

  @override
  String get shareError => 'エラーを共有';

  @override
  String get shareLink => 'リンクをシェアする';

  @override
  String shareMessageDescription(Object arg0, Object arg1) {
    return '$arg0から$arg1を送信しますか？';
  }

  @override
  String shareMessageDescriptionEmpty(Object arg0) {
    return '$arg0を送信しますか？';
  }

  @override
  String get sharedMedia => '共有されたメディア';

  @override
  String get show => '表示';

  @override
  String get showAvatar => 'アバターの表示';

  @override
  String get showIdentityNumber => 'Show Identity Number';

  @override
  String get showMixin => 'Mixinを表示';

  @override
  String get signIn => 'ログイン';

  @override
  String get signOut => 'サインアウト';

  @override
  String get signWithMobileNumber => '電話番号でログイン';

  @override
  String get signWithQrcode => 'QRコードでログイン';

  @override
  String get smileysAndPeople => 'Smileys & People';

  @override
  String get snapshotHash => 'Snapshot Hash';

  @override
  String get status => 'ステータス';

  @override
  String get sticker => 'スタンプ';

  @override
  String get stickerAddInvalidSize =>
      'スタンプのサイズは1KB以上1MB未満、 幅と高さは128ピクセルから1024ピクセルである必要があります';

  @override
  String get stickerAlbumDetail => 'スタンプアルバム詳細';

  @override
  String get stickerStore => 'スタンプストア';

  @override
  String get storageAutoDownloadDescription => 'メディアの自動ダウンロード設定を変更する';

  @override
  String get storageUsage => 'ストレージ使用率';

  @override
  String get strangerHint => '連絡先にない相手からのメッセージです';

  @override
  String get strangers => '連絡先にない相手';

  @override
  String get successful => '成功';

  @override
  String get symbols => 'Symbols';

  @override
  String get syncFromOtherDevice => '別のデバイスから同期';

  @override
  String get syncToOtherDevice => '別のデバイスへ同期';

  @override
  String get termsOfService => '利用規約';

  @override
  String get text => 'テキスト';

  @override
  String get theme => 'テーマ';

  @override
  String get thisMessageWasDeleted => 'このメッセージは削除されています';

  @override
  String get time => '日時';

  @override
  String get to => '宛先';

  @override
  String get today => '今日';

  @override
  String get toggleChatInfo => 'チャット情報のオン/オフ';

  @override
  String get trace => '記録情報';

  @override
  String get transactionHash => 'トランザクションハッシュ';

  @override
  String get transactionId => 'トランザクションID';

  @override
  String get transactionType => 'トランザクションタイプ';

  @override
  String get transactions => '取引';

  @override
  String get transactionsCannotBeDeleted => 'トランザクション履歴を削除することはできません';

  @override
  String get transcript => 'メッセージ履歴';

  @override
  String get transfer => '送金';

  @override
  String get transferCompleted => 'Transfer completed';

  @override
  String get transferProtocolVersionNotMatched =>
      'Protocol version does not match, transfer failed. Please upgrade the application first.';

  @override
  String get transferringChats => 'Transferring Chats';

  @override
  String get transferringChatsTips =>
      'Please do not turn off the screen and keep the Mixin running in the foreground while syncing.';

  @override
  String get travelAndPlaces => 'Travel & Places';

  @override
  String get typeMessage => 'メッセージを入力';

  @override
  String unableToOpenFile(Object arg0) {
    return 'ファイルを開くことができません: $arg0';
  }

  @override
  String get unblock => 'ブロックを解除';

  @override
  String unitDay(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '日間',
      one: '日',
    );
    return '$_temp0';
  }

  @override
  String unitHour(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '時間',
      one: '時',
    );
    return '$_temp0';
  }

  @override
  String unitMinute(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '分間',
      one: '分',
    );
    return '$_temp0';
  }

  @override
  String unitSecond(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '秒間',
      one: '秒',
    );
    return '$_temp0';
  }

  @override
  String unitWeek(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '週間',
      one: '週',
    );
    return '$_temp0';
  }

  @override
  String get unknowError => 'Unknow error';

  @override
  String get unlockMixinMessenger => 'Unlock Mixin Messenger';

  @override
  String get unlockWithWasscode => 'Enter Passcode to unlock Mixin Messenger';

  @override
  String get unmute => 'ミュート解除';

  @override
  String get unpin => 'ピン留めを止める';

  @override
  String get unpinAllMessages => '全てのメッセージのピン留めを解除する';

  @override
  String get unpinAllMessagesConfirmation => '全てのメッセージのピン留めを解除しますか？';

  @override
  String get unreadMessages => '新しいメッセージ';

  @override
  String get updateMixin => 'Mixinのアップデート';

  @override
  String updateMixinDescription(Object arg0) {
    return '現在のバージョン($arg0)は使用できなくなりました。\n「アップデート」をクリックして、Google Playから最新バージョンにアップデートしてください。';
  }

  @override
  String get upgrade => 'Upgrade';

  @override
  String get upgrading => 'アップデート中';

  @override
  String get useBiometric => 'Use Biometric';

  @override
  String get userDeleteHint => 'This user has deleted their account.';

  @override
  String get userNotFound => 'ユーザーが見つかりませんでした';

  @override
  String get username => 'Username';

  @override
  String valueNow(Object arg0) {
    return '現在価格 $arg0';
  }

  @override
  String valueThen(Object arg0) {
    return '当時の価格 $arg0';
  }

  @override
  String get verifyPin => 'PINコードを認証';

  @override
  String get video => '動画';

  @override
  String get videos => '動画';

  @override
  String get waitingForThisMessage => 'このメッセージを待っています。';

  @override
  String get waitingOtherDeviceConnection => '別のデバイスの接続を待っています。';

  @override
  String get webview2RuntimeInstallDescription =>
      'このデバイスには、WebView2 Runtimeコンポーネントがインストールされていません。先にWebView2 Runtimeをダウンロードし、インストールしてください。';

  @override
  String get webviewRuntimeUnavailable => 'WebView runtimeは利用できません';

  @override
  String get window => 'ウィンドウ';

  @override
  String get withdrawal => '出金';

  @override
  String get withdrawalHash => 'Withdrawal Hash';

  @override
  String get you => '自分';

  @override
  String get youDeletedThisMessage => 'このメッセージを削除しました。';

  @override
  String get zoom => 'Zoom';
}
