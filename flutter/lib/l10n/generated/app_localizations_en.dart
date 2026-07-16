// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get aMessage => 'a message';

  @override
  String get about => 'About';

  @override
  String get account => 'Account';

  @override
  String get activity => 'Activity';

  @override
  String get add => 'Add';

  @override
  String get addACaption => 'Add a caption';

  @override
  String get addBotWithPlus => '+ Add Bot';

  @override
  String get addContact => 'Add Contact';

  @override
  String get addContactWithPlus => '+ Add Contact';

  @override
  String get addFile => 'Add File';

  @override
  String get addGroupDescription => 'Add group description';

  @override
  String get addParticipants => 'Add Participants';

  @override
  String get addPeopleSearchHint => 'Mixin ID or Phone number';

  @override
  String get addProxy => 'Add Proxy';

  @override
  String get addSticker => 'Add Sticker';

  @override
  String get addStickerFailed => 'Failed to add sticker';

  @override
  String get addStickers => 'Add Stickers';

  @override
  String get addToCircle => 'Add to Circle';

  @override
  String get added => 'Added';

  @override
  String get address => 'Address';

  @override
  String get admin => 'Admin';

  @override
  String get alertKeyContactContactMessage => 'sent you a contact';

  @override
  String get allChats => 'Chats';

  @override
  String get animalsAndNature => 'Animals & Nature';

  @override
  String get anonymous => 'Anonymous';

  @override
  String get anonymousNumber => 'Anonymous Number';

  @override
  String get appCardShareDisallow => 'This URL cannot be shared.';

  @override
  String get appearance => 'Appearance';

  @override
  String get archivedFolder => 'Archived Folder';

  @override
  String get assetType => 'Asset Type';

  @override
  String get audio => 'Audio';

  @override
  String get audios => 'Audio';

  @override
  String get autoBackup => 'Auto Backup';

  @override
  String get autoLock => 'Auto Lock';

  @override
  String get avatar => 'Avatar';

  @override
  String get backup => 'Backup';

  @override
  String get backupChat => 'Backup Chat';

  @override
  String get backupToOtherDevice => 'Backup to Other Device';

  @override
  String get backupToOtherDeviceTips =>
      'Back up your chat history to another device. Make sure both devices are connected to the same Wi-Fi or hotspot.';

  @override
  String get backupWaitingOtherDevice =>
      'Open Mixin on your other device and start restore there.';

  @override
  String get biography => 'Biography';

  @override
  String get biometric => 'Biometric';

  @override
  String get block => 'Block';

  @override
  String get botNotFound => 'Bot not found';

  @override
  String get bots => 'BOTS';

  @override
  String get botsTitle => 'Bots';

  @override
  String get bringAllToFront => 'Bring All to Front';

  @override
  String get canNotRecognizeQrCode => 'Cannot recognize the QR code';

  @override
  String get cancel => 'Cancel';

  @override
  String get card => 'Card';

  @override
  String get change => 'Change';

  @override
  String get changeNumber => 'Change Number';

  @override
  String get changeNumberInstead => 'Change Number Instead';

  @override
  String changedDisappearingMessageSettings(Object arg0) {
    return '$arg0 changed disappearing message settings.';
  }

  @override
  String get chatBackup => 'Chat Backup';

  @override
  String get chatBackupAndRestore => 'Chat Backup and Restore';

  @override
  String get chatBotReceptionTitle => 'Tap the button to interact with the bot';

  @override
  String chatDecryptionFailedHint(Object arg0) {
    return 'Waiting for $arg0 to get online and establish an encrypted session.';
  }

  @override
  String chatDeleteMessage(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Delete $arg0 messages?',
      one: 'Delete $arg0 message?',
    );
    return '$_temp0';
  }

  @override
  String chatGroupAdd(Object arg0, Object arg1) {
    return '$arg0 added $arg1';
  }

  @override
  String chatGroupExit(Object arg0) {
    return '$arg0 left';
  }

  @override
  String chatGroupJoin(Object arg0) {
    return '$arg0 joined the group via invite link';
  }

  @override
  String chatGroupRemove(Object arg0, Object arg1) {
    return '$arg0 removed $arg1';
  }

  @override
  String get chatHintE2e => 'End-to-end encrypted';

  @override
  String get chatNotSupportUriOnPhone =>
      'This type of URL is not supported. Please check it on your phone.';

  @override
  String get chatNotSupportUrl =>
      'https://support.mixin.one/en/article/how-to-do-when-you-receive-a-message-like-this-this-type-of-message-is-not-supported-please-upgrade-mixin-17j1t3p';

  @override
  String get chatNotSupportViewOnPhone =>
      'This type of message is not supported. Please check it on your phone.';

  @override
  String chatPinMessage(Object arg0, Object arg1) {
    return '$arg0 pinned $arg1';
  }

  @override
  String get chatTextSize => 'Chat Text Size';

  @override
  String get chats => 'Chats';

  @override
  String get checkNewVersion => 'Check for updates';

  @override
  String circleSubtitle(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 Conversations',
      one: '$arg0 Conversation',
    );
    return '$_temp0';
  }

  @override
  String circleTitle(Object arg0) {
    return '$arg0\'s Circles';
  }

  @override
  String get circles => 'Circles';

  @override
  String get clear => 'Clear';

  @override
  String get clearChat => 'Clear Chat';

  @override
  String get clearFilter => 'Clear filter';

  @override
  String get clickToReloadQrcode => 'Click to reload the QR code';

  @override
  String get close => 'Close';

  @override
  String get closeWindow => 'Close window';

  @override
  String get closingBalance => 'Closing Balance';

  @override
  String get collapse => 'Collapse';

  @override
  String get collectible => 'Collectible';

  @override
  String get collectibles => 'Collectibles';

  @override
  String get collection => 'Collection';

  @override
  String get combineAndForward => 'Combine and forward';

  @override
  String get confirm => 'Confirm';

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
  String get contact => 'Contact';

  @override
  String contactMixinId(Object arg0) {
    return 'Mixin ID: $arg0';
  }

  @override
  String get contactMuteTitle => 'Mute notifications for…';

  @override
  String get contactTitle => 'Contacts';

  @override
  String get contentTooLong => 'Content too long';

  @override
  String get contentVoice => '[Voice call]';

  @override
  String get continueText => 'Continue';

  @override
  String get conversation => 'Conversation';

  @override
  String conversationDeleteTitle(Object arg0) {
    return 'Delete chat: $arg0';
  }

  @override
  String get copy => 'Copy';

  @override
  String get copyImage => 'Copy Image';

  @override
  String get copyInvite => 'Copy Invite Link';

  @override
  String get copyLink => 'Copy Link';

  @override
  String get copySelectedText => 'Copy Selected Text';

  @override
  String get copyText => 'Copy Text';

  @override
  String get create => 'Create';

  @override
  String get createCircle => 'New Circle';

  @override
  String get createConversation => 'New Conversation';

  @override
  String get createGroup => 'New Group';

  @override
  String createdAt(Object arg0) {
    return 'Created $arg0';
  }

  @override
  String createdThisGroup(Object arg0) {
    return '$arg0 created this group';
  }

  @override
  String get customTime => 'Custom Time';

  @override
  String get dark => 'Dark';

  @override
  String get dataAndStorageUsage => 'Data and Storage Usage';

  @override
  String get dataError => 'Data error';

  @override
  String get dataLoading => 'Loading data, please wait...';

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
  String get databaseUpgradeTips =>
      'The database is being upgraded. This may take several minutes. Please do not close this app.';

  @override
  String get delete => 'Delete';

  @override
  String get deleteAccountDetailHint =>
      'Local messages and iCloud Backups will not be deleted automatically';

  @override
  String get deleteAccountHint => 'Delete your account info and profile photo';

  @override
  String get deleteChat => 'Delete Chat';

  @override
  String get deleteChatDescription =>
      'Deleting this chat will remove messages from this device only. They will not be removed from other devices.';

  @override
  String get deleteCircle => 'Delete Circle';

  @override
  String get deleteForEveryone => 'Delete for Everyone';

  @override
  String get deleteForMe => 'Delete for me';

  @override
  String get deleteGroup => 'Delete Group';

  @override
  String get deleteMyAccount => 'Delete My Account';

  @override
  String deleteTheCircle(Object arg0) {
    return 'Do you want to delete the $arg0 circle?';
  }

  @override
  String get deposit => 'Deposit';

  @override
  String get depositHash => 'Deposit Hash';

  @override
  String get developer => 'Developer';

  @override
  String get deviceTransferFailed => 'Transfer failed';

  @override
  String disableDisappearingMessage(Object arg0) {
    return '$arg0 disabled disappearing message';
  }

  @override
  String get disabled => 'Disabled';

  @override
  String disappearingCustomTimeMaxWarning(Object arg0) {
    return 'The maximum time is $arg0.';
  }

  @override
  String get disappearingMessage => 'Disappearing Messages';

  @override
  String get disappearingMessageHint =>
      'When enabled, new messages sent and received in this chat will disappear after they have been seen. Read the document to **learn more**.';

  @override
  String get discard => 'Discard';

  @override
  String get discardRecordingWarning =>
      'Are you sure you want to stop recording and discard your voice message?';

  @override
  String get dismissAsAdmin => 'Dismiss as Admin';

  @override
  String get done => 'Done';

  @override
  String get download => 'Download';

  @override
  String get downloadLink => 'Download Link:';

  @override
  String get draft => 'Draft';

  @override
  String get dragAndDropFileHere => 'Drag and drop files here';

  @override
  String get durationIsTooShort => 'Duration is too short';

  @override
  String get edit => 'Edit';

  @override
  String get editCircleName => 'Edit Circle Name';

  @override
  String get editConversations => 'Edit Conversations';

  @override
  String get editGroupDescription => 'Edit Group Description';

  @override
  String get editGroupName => 'Edit Group Name';

  @override
  String get editImageClearWarning =>
      'All changes will be lost. Are you sure you want to exit?';

  @override
  String get editName => 'Edit Name';

  @override
  String get editProfile => 'Edit Profile';

  @override
  String get enablePushNotification => 'Enable push notifications';

  @override
  String get encryptZipFileWithPassword =>
      'Encrypt the ZIP file with a password';

  @override
  String get enterPinToDeleteAccount => 'Enter your PIN to delete your account';

  @override
  String get enterToSend => 'Return/Enter ⏎ to Send';

  @override
  String get enterYourPhoneNumber => 'Enter your phone number';

  @override
  String get enterYourPinToContinue => 'Enter your PIN to continue';

  @override
  String get errorAccessLimited => 'ERROR 403: Access Limited';

  @override
  String get errorAddressExists =>
      'The address does not exist. Please make sure it was added successfully.';

  @override
  String get errorAddressNotSync => 'Address refresh failed, please try again';

  @override
  String get errorAlreadyBondedReferralCode =>
      'ERROR 10731: This account has already applied a referral code';

  @override
  String get errorAssetExists => 'Asset does not exist';

  @override
  String get errorAuthentication => 'ERROR 401: Sign in to continue';

  @override
  String get errorBadData =>
      'ERROR 10002: The request data has an invalid field';

  @override
  String get errorBlockchain =>
      'ERROR 30100: Blockchain not in sync, please try again later.';

  @override
  String get errorConnectionTimeout =>
      'Network connection timeout, please try again';

  @override
  String get errorFullGroup => 'ERROR 20116: The group chat is full.';

  @override
  String get errorInsufficientBalance => 'ERROR 20117: Insufficient balance';

  @override
  String errorInsufficientTransactionFeeWithAmount(Object arg0) {
    return 'ERROR 20124: Insufficient transaction fee. Please make sure your wallet has $arg0 as fee';
  }

  @override
  String errorInvalidAddress(Object arg0, Object arg1) {
    return 'ERROR 30102: Invalid address format. Please enter the correct $arg0 $arg1 address!';
  }

  @override
  String get errorInvalidAddressPlain => 'ERROR 30102: Invalid address format.';

  @override
  String get errorInvalidCodeTooFrequent =>
      'ERROR 20129: Verification codes are being sent too frequently. Please try again later.';

  @override
  String get errorInvalidEmergencyContact =>
      'ERROR 20130: Invalid recovery contact';

  @override
  String get errorInvalidPinFormat => 'ERROR 20118: Invalid PIN format.';

  @override
  String get errorInviterPlanExpired =>
      'ERROR 10737: The inviter has no valid plan';

  @override
  String get errorLegacyPin =>
      'ERROR 20118: To enhance the security of the Mixin network, Mixin API has temporarily suspended the upgrading from D3M-PIN to TIP. Please refer to the documentation for details and register for processing.';

  @override
  String get errorNetworkTaskFailed =>
      'Network connection failed. Check or switch your network and try again';

  @override
  String get errorNoPinToken =>
      'No token. Please sign in again and try this feature again.';

  @override
  String get errorNotFound => 'ERROR 404: Not found';

  @override
  String get errorNotSupportedAudioFormat =>
      'Unsupported audio format. Please open it with another app.';

  @override
  String get errorNumberReachedLimit =>
      'ERROR 20132: The number has reached the limit.';

  @override
  String errorOldVersion(Object arg0) {
    return 'ERROR 10006: Please update Mixin ($arg0) to continue using the service.';
  }

  @override
  String get errorOpenLocation => 'Can\'t find a map app';

  @override
  String get errorPermission => 'Please open the necessary permissions';

  @override
  String get errorPhoneInvalidFormat => 'ERROR 20110: Invalid phone number';

  @override
  String get errorPhoneSmsDelivery => 'ERROR 10003: Failed to deliver SMS';

  @override
  String get errorPhoneVerificationCodeExpired =>
      'ERROR 20114: Expired phone verification code';

  @override
  String get errorPhoneVerificationCodeInvalid =>
      'ERROR 20113: Invalid phone verification code';

  @override
  String get errorPinCheckTooManyRequest =>
      'You have tried more than 5 times, please wait at least 24 hours to try again.';

  @override
  String get errorPinIncorrect => 'ERROR 20119: PIN incorrect';

  @override
  String errorPinIncorrectWithTimes(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'ERROR 20119: PIN incorrect. You still have $arg0 chances. Please wait for 24 hours to retry later.',
      one:
          'ERROR 20119: PIN incorrect. You still have $arg0 chance. Please wait for 24 hours to retry later.',
    );
    return '$_temp0';
  }

  @override
  String get errorRecaptchaIsInvalid => 'ERROR 10004: Recaptcha is invalid';

  @override
  String errorServer5xxCode(Object arg0) {
    return 'Server is under maintenance: $arg0';
  }

  @override
  String get errorTooManyRequest => 'ERROR 429: Rate limit exceeded';

  @override
  String get errorTooManyStickers => 'ERROR 20126: Too many stickers';

  @override
  String get errorTooSmallTransferAmount =>
      'ERROR 20120: Transfer amount is too small';

  @override
  String get errorTooSmallWithdrawAmount =>
      'ERROR 20127: Withdraw amount too small';

  @override
  String get errorTranscriptForward =>
      'Please forward all attachments after they have been downloaded';

  @override
  String get errorTransferToDeactivatedUser =>
      'ERROR 20160: Transfers cannot be made to a deactivated user';

  @override
  String get errorUnableToOpenMedia =>
      'Can\'t find an app that can open this media.';

  @override
  String errorUnknownWithCode(Object arg0) {
    return 'ERROR: $arg0';
  }

  @override
  String errorUnknownWithMessage(Object arg0) {
    return 'ERROR: $arg0';
  }

  @override
  String get errorUploadAttachmentFailed =>
      'Failed to upload message attachment';

  @override
  String get errorUsedPhone =>
      'ERROR 20122: This phone number is already associated with another account.';

  @override
  String get errorUserInvalidFormat => 'Invalid user id';

  @override
  String get errorWithdrawalMemoFormatIncorrect =>
      'ERROR 20131: Withdrawal memo format incorrect.';

  @override
  String get errorWithdrawalSuspend =>
      'ERROR 20137: Withdrawals are suspended.';

  @override
  String get exit => 'Exit';

  @override
  String get exitGroup => 'Exit Group';

  @override
  String get failed => 'Failed';

  @override
  String get failedToOpenDatabase =>
      'An error occurred while opening the database.';

  @override
  String get fee => 'Fee';

  @override
  String get file => 'File';

  @override
  String get fileChooserError => 'File chooser error';

  @override
  String get fileDoesNotExist => 'File does not exist';

  @override
  String get fileError => 'File error';

  @override
  String get files => 'Files';

  @override
  String get flags => 'Flags';

  @override
  String get followSystem => 'Follow System';

  @override
  String get followUsOnFacebook => 'Follow us on Facebook';

  @override
  String get followUsOnX => 'Follow us on X';

  @override
  String get foodAndDrink => 'Food & Drink';

  @override
  String get formatNotSupported => 'Format not supported';

  @override
  String get forward => 'Forward';

  @override
  String get from => 'From';

  @override
  String get fromWithColon => 'From:';

  @override
  String get generateQrcode => 'Generate QR Code';

  @override
  String get groupAlreadyIn => 'You are already in the group.';

  @override
  String get groupCantSend =>
      'You can\'t send messages to this group because you\'re no longer a participant.';

  @override
  String get groupName => 'Group Name';

  @override
  String get groupParticipants => 'Participants';

  @override
  String groupPopMenuMessage(Object arg0) {
    return 'Message $arg0';
  }

  @override
  String groupPopMenuRemove(Object arg0) {
    return 'Remove $arg0';
  }

  @override
  String get groups => 'Groups';

  @override
  String get groupsInCommon => 'Groups in Common';

  @override
  String get hash => 'HASH';

  @override
  String get help => 'Help';

  @override
  String get helpCenter => 'Help Center';

  @override
  String get hideMixin => 'Hide Mixin';

  @override
  String get host => 'Host';

  @override
  String hour(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 Hours',
      one: '$arg0 Hour',
    );
    return '$_temp0';
  }

  @override
  String get howAreYou => 'Hi, how are you?';

  @override
  String get iAmGood => 'I’m good.';

  @override
  String get id => 'ID';

  @override
  String get ignoreThisVersion => 'Ignore the new version';

  @override
  String get image => 'Image';

  @override
  String get includeFiles => 'Include Files';

  @override
  String get includeVideos => 'Include Videos';

  @override
  String get initializing => 'Initializing…';

  @override
  String get invalidStickerFormat => 'Invalid sticker format';

  @override
  String get inviteInfo =>
      'Anyone with Mixin can follow this link to join this group. Only share it with people you trust.';

  @override
  String get inviteToGroupViaLink => 'Invite to Group via Link';

  @override
  String get joinGroupWithPlus => '+ Join group';

  @override
  String joinedIn(Object arg0) {
    return 'Joined on $arg0';
  }

  @override
  String landingDeleteContent(Object arg0, Object arg1) {
    return 'You requested to delete your account on $arg0. The account will be deleted on $arg1. If you continue to log in, your account deletion will be cancelled.';
  }

  @override
  String landingInvitationDialogContent(Object arg0) {
    return 'We will send a 4-digit code to your phone number $arg0. Please enter the code on the next screen.';
  }

  @override
  String landingValidationTitle(Object arg0) {
    return 'Enter the 4-digit code sent to you at $arg0';
  }

  @override
  String get learnMore => 'Learn More';

  @override
  String get less => 'less';

  @override
  String get light => 'Light';

  @override
  String get linkedDevice => 'linked device';

  @override
  String get live => 'Live';

  @override
  String get loading => 'Loading...';

  @override
  String get loadingTime =>
      'The system time appears incorrect. Please correct it and try again.';

  @override
  String get locateToChat => 'Locate in Chat';

  @override
  String get location => 'Location';

  @override
  String get lock => 'Lock';

  @override
  String get logIn => 'Log in';

  @override
  String get loginAndAbortAccountDeletion =>
      'Continue to log in and abort account deletion';

  @override
  String get loginByQrcode => 'Log in to Mixin Messenger with a QR code';

  @override
  String get loginByQrcodeTips1 => 'Open Mixin Messenger on your phone.';

  @override
  String get loginByQrcodeTips2 =>
      'Scan the QR code on the screen and confirm your sign-in.';

  @override
  String get makeGroupAdmin => 'Make group admin';

  @override
  String get media => 'Media';

  @override
  String get memo => 'Memo';

  @override
  String get messageE2ee =>
      'Messages in this conversation are end-to-end encrypted. Tap for more info.';

  @override
  String get messageNotFound => 'Message not found';

  @override
  String get messageNotSupport =>
      'This type of message is not supported, please upgrade Mixin to the latest version.';

  @override
  String get messagePreview => 'Message Preview';

  @override
  String get messagePreviewDescription =>
      'Preview message text inside new message notifications.';

  @override
  String get messages => 'Messages';

  @override
  String get minimize => 'Minimize';

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
  String get mixinMessengerDesktop => 'Mixin Messenger Desktop';

  @override
  String get more => 'More';

  @override
  String get multisigTransaction => 'Multisig Transaction';

  @override
  String get mute => 'Mute';

  @override
  String myMixinId(Object arg0) {
    return 'My Mixin ID: $arg0';
  }

  @override
  String get myStickers => 'My Stickers';

  @override
  String get na => 'N/A';

  @override
  String get name => 'Name';

  @override
  String get networkConnectionFailed => 'Network connection failed';

  @override
  String get networkError => 'Network error';

  @override
  String get newVersionAvailable => 'New version available';

  @override
  String newVersionDescription(Object arg0, Object arg1) {
    return 'Mixin Messenger $arg0 is now available, you have $arg1. Would you like to download it now?';
  }

  @override
  String get next => 'Next';

  @override
  String get nextConversation => 'Next conversation';

  @override
  String get noAudio => 'NO AUDIO';

  @override
  String get noCamera => 'No camera';

  @override
  String get noData => 'No Data';

  @override
  String get noFiles => 'NO FILES';

  @override
  String get noLinks => 'NO LINKS';

  @override
  String get noMedia => 'NO MEDIA';

  @override
  String get noNetworkConnection => 'No network connection';

  @override
  String get noPosts => 'NO POSTS';

  @override
  String get noResults => 'NO RESULTS';

  @override
  String get notFound => 'Not found';

  @override
  String get notSupportBiometric =>
      'This device does not support biometric authentication';

  @override
  String get notificationContent =>
      'Enable push notifications to stay updated on price alerts and messages in real time.';

  @override
  String get notificationPermissionManually =>
      'Notifications are disabled. Please go to Notification Settings to turn them on.';

  @override
  String get notifications => 'Notifications';

  @override
  String nowAnAddmin(Object arg0) {
    return '$arg0 now an admin';
  }

  @override
  String get objects => 'Objects';

  @override
  String get oneByOneForward => 'One-by-One Forward';

  @override
  String get oneHour => '1 Hour';

  @override
  String get oneYear => '1 Year';

  @override
  String get open => 'Open';

  @override
  String get openHomePage => 'Open Homepage';

  @override
  String openLink(Object arg0) {
    return 'Open Link: $arg0';
  }

  @override
  String get openLogDirectory => 'Open Log Directory';

  @override
  String get openingBalance => 'Opening Balance';

  @override
  String get originalImage => 'Original';

  @override
  String get owner => 'Owner';

  @override
  String participantsCount(Object arg0) {
    return '$arg0 PARTICIPANTS';
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
      other: '$arg0/$arg1 confirmations',
      one: '$arg0/$arg1 confirmation',
    );
    return '$_temp0';
  }

  @override
  String get phoneNumber => 'Phone Number';

  @override
  String get photos => 'Photos';

  @override
  String get pickAConversation =>
      'Select a conversation and start sending a message';

  @override
  String get picturesAndVideos => 'Pictures & Videos';

  @override
  String get pinTitle => 'Pin';

  @override
  String pinnedMessageTitle(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 Pinned Messages',
      one: '$arg0 Pinned Message',
    );
    return '$_temp0';
  }

  @override
  String get port => 'Port';

  @override
  String get post => 'Post';

  @override
  String get preferences => 'Preferences';

  @override
  String get previousConversation => 'Previous conversation';

  @override
  String get privacyPolicy => 'Privacy Policy';

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
  String get quickSearch => 'Quick Search';

  @override
  String get quitMixin => 'Quit Mixin';

  @override
  String get raw => 'Raw';

  @override
  String get rebate => 'Rebate';

  @override
  String get recaptchaTimeout => 'Recaptcha timeout';

  @override
  String get receiver => 'Receiver';

  @override
  String get recentChats => 'CHATS';

  @override
  String get reedit => 'Re-edit';

  @override
  String get refresh => 'Refresh';

  @override
  String get removeBot => 'Remove Bot';

  @override
  String get removeChatFromCircle => 'Remove Chat from circle';

  @override
  String get removeContact => 'Remove Contact';

  @override
  String get removeStickers => 'Remove Stickers';

  @override
  String get reply => 'Reply';

  @override
  String get report => 'Report';

  @override
  String get reportAndBlock => 'Report and block?';

  @override
  String get reportTitle => 'Send the conversation log to developers?';

  @override
  String get resendCode => 'Resend code';

  @override
  String resendCodeIn(Object arg0) {
    return 'Resend code in $arg0 s';
  }

  @override
  String get reset => 'Reset';

  @override
  String get resetLink => 'Reset Link';

  @override
  String get restoreChat => 'Restore Chat';

  @override
  String get restoreChatTip =>
      'Restore your chat history from another device. Make sure both devices are connected to the same Wi-Fi or hotspot.';

  @override
  String get restoreFromOtherDevice => 'Restore from Other Device';

  @override
  String get retry => 'Retry';

  @override
  String get retryUploadFailed => 'Retry upload failed.';

  @override
  String get revokeMultisigTransaction => 'Revoke Multisig Transaction';

  @override
  String get save => 'Save';

  @override
  String get saveAs => 'Save as';

  @override
  String get saveToCameraRoll => 'Save to Camera Roll';

  @override
  String get sayHi => 'Say Hi';

  @override
  String get scamWarning =>
      'Warning: Many users reported this account as a scam. Please be careful, especially if it asks you for money';

  @override
  String get screenPasscode => 'Screen Passcode';

  @override
  String get search => 'Search';

  @override
  String get searchContact => 'Search Contacts';

  @override
  String get searchConversation => 'Search Conversation';

  @override
  String get searchEmpty => 'No chats, contacts or messages found.';

  @override
  String get searchPlaceholderNumber => 'Search Mixin ID or phone number:';

  @override
  String searchRelatedMessage(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 related messages',
      one: '$arg0 related message',
    );
    return '$_temp0';
  }

  @override
  String get searchUnread => 'Search Unread';

  @override
  String get secretUrl => 'https://mixin.one/pages/1000007';

  @override
  String get security => 'Security';

  @override
  String get select => 'Select';

  @override
  String get send => 'Send';

  @override
  String get sendArchived => 'Send as ZIP';

  @override
  String get sendQuickly => 'Send quickly';

  @override
  String get sendToDeveloper => 'Send to Developer';

  @override
  String get sendWithoutCompression => 'Send without compression';

  @override
  String get sendWithoutSound => 'Send Without Sound';

  @override
  String get set => 'Set';

  @override
  String setDisappearingMessageTimeTo(Object arg0, Object arg1) {
    return '$arg0 set disappearing message time to $arg1';
  }

  @override
  String get setPasscodeDesc => 'Set a passcode to unlock Mixin Messenger';

  @override
  String get settingAuthSearchHint => 'Mixin ID, Name';

  @override
  String get settingBackupTips =>
      'Back up your chat history to iCloud. If you lose your iPhone or switch to a new one, you can restore your chat history when you reinstall Mixin Messenger. Messages you back up are not protected by Mixin Messenger end-to-end encryption while in iCloud.';

  @override
  String settingDeleteAccountPinContent(Object arg0) {
    return 'If you continue, your profile and account details will be deleted on $arg0. Read our document to **learn more**.';
  }

  @override
  String get settingDeleteAccountUrl =>
      'https://support.mixin.one/en/article/how-to-delete-my-account-19fkagl';

  @override
  String get share => 'Share';

  @override
  String get shareApps => 'Shared Apps';

  @override
  String get shareContact => 'Share Contact';

  @override
  String get shareError => 'Share error.';

  @override
  String get shareLink => 'Share Link';

  @override
  String shareMessageDescription(Object arg0, Object arg1) {
    return 'Are you sure you want to send $arg0 from $arg1?';
  }

  @override
  String shareMessageDescriptionEmpty(Object arg0) {
    return 'Are you sure you want to send the $arg0?';
  }

  @override
  String get sharedMedia => 'Shared Media';

  @override
  String get show => 'Show';

  @override
  String get showAvatar => 'Show avatar';

  @override
  String get showIdentityNumber => 'Show Identity Number';

  @override
  String get showMixin => 'Show Mixin';

  @override
  String get signIn => 'Sign in';

  @override
  String get signOut => 'Sign Out';

  @override
  String get signWithMobileNumber => 'Sign in with mobile number';

  @override
  String get signWithQrcode => 'Sign in with QR code';

  @override
  String get smileysAndPeople => 'Smileys & People';

  @override
  String get snapshotHash => 'Snapshot Hash';

  @override
  String get status => 'Status';

  @override
  String get sticker => 'Sticker';

  @override
  String get stickerAddInvalidSize =>
      'Sticker files must be larger than 1 KB and smaller than 1 MB, with width and height between 128 px and 1024 px.';

  @override
  String get stickerAlbumDetail => 'Sticker Album Details';

  @override
  String get stickerStore => 'Sticker Store';

  @override
  String get storageAutoDownloadDescription =>
      'Change auto-download settings for media.';

  @override
  String get storageUsage => 'Storage Usage';

  @override
  String get strangerHint => 'This sender is not in your contacts';

  @override
  String get strangers => 'Strangers';

  @override
  String get successful => 'Successful';

  @override
  String get symbols => 'Symbols';

  @override
  String get syncFromOtherDevice => 'Sync from Other Device';

  @override
  String get syncToOtherDevice => 'Sync to Other Device';

  @override
  String get termsOfService => 'Terms of Service';

  @override
  String get text => 'Text';

  @override
  String get theme => 'Theme';

  @override
  String get thisMessageWasDeleted => 'This message was deleted';

  @override
  String get time => 'Time';

  @override
  String get to => 'To';

  @override
  String get today => 'Today';

  @override
  String get toggleChatInfo => 'Toggle chat info';

  @override
  String get trace => 'Trace';

  @override
  String get transactionHash => 'Transaction Hash';

  @override
  String get transactionId => 'Transaction Id';

  @override
  String get transactionType => 'Transaction Type';

  @override
  String get transactions => 'Transactions';

  @override
  String get transactionsCannotBeDeleted => 'Transactions cannot be deleted';

  @override
  String get transcript => 'Transcript';

  @override
  String get transfer => 'Transfer';

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
  String get typeMessage => 'Type message';

  @override
  String unableToOpenFile(Object arg0) {
    return 'Unable to open file: $arg0';
  }

  @override
  String get unblock => 'Unblock';

  @override
  String unitDay(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'days',
      one: 'day',
    );
    return '$_temp0';
  }

  @override
  String unitHour(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'hours',
      one: 'hour',
    );
    return '$_temp0';
  }

  @override
  String unitMinute(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'minutes',
      one: 'minute',
    );
    return '$_temp0';
  }

  @override
  String unitSecond(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'seconds',
      one: 'second',
    );
    return '$_temp0';
  }

  @override
  String unitWeek(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'weeks',
      one: 'week',
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
  String get unmute => 'Unmute';

  @override
  String get unpin => 'Unpin';

  @override
  String get unpinAllMessages => 'Unpin All Messages';

  @override
  String get unpinAllMessagesConfirmation =>
      'Are you sure you want to unpin all messages?';

  @override
  String get unreadMessages => 'Unread messages';

  @override
  String get updateMixin => 'Update Mixin';

  @override
  String updateMixinDescription(Object arg0) {
    return 'The current version ($arg0) is no longer available!\nPlease click \"Update\" below to update to the latest version.';
  }

  @override
  String get upgrade => 'Upgrade';

  @override
  String get upgrading => 'Upgrading';

  @override
  String get useBiometric => 'Use Biometric';

  @override
  String get userDeleteHint => 'This user has deleted their account.';

  @override
  String get userNotFound => 'User not found';

  @override
  String get username => 'Username';

  @override
  String valueNow(Object arg0) {
    return 'value now $arg0';
  }

  @override
  String valueThen(Object arg0) {
    return 'value then $arg0';
  }

  @override
  String get verifyPin => 'Verify PIN';

  @override
  String get video => 'Video';

  @override
  String get videos => 'Videos';

  @override
  String get waitingForThisMessage => 'Waiting for this message.';

  @override
  String get waitingOtherDeviceConnection =>
      'Waiting for the other device to connect.';

  @override
  String get webview2RuntimeInstallDescription =>
      'The device has not installed the WebView2 Runtime component. Please download and install WebView2 Runtime first.';

  @override
  String get webviewRuntimeUnavailable => 'WebView runtime is unavailable';

  @override
  String get window => 'Window';

  @override
  String get withdrawal => 'Withdraw';

  @override
  String get withdrawalHash => 'Withdrawal Hash';

  @override
  String get you => 'You';

  @override
  String get youDeletedThisMessage => 'You deleted this message';

  @override
  String get zoom => 'Zoom';
}
