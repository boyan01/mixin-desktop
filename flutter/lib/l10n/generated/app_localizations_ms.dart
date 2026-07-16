// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Malay (`ms`).
class AppLocalizationsMs extends AppLocalizations {
  AppLocalizationsMs([String locale = 'ms']) : super(locale);

  @override
  String get aMessage => 'a message';

  @override
  String get about => 'Mengenai';

  @override
  String get account => 'Akaun';

  @override
  String get activity => 'Activity';

  @override
  String get add => 'Add';

  @override
  String get addACaption => 'Add a caption';

  @override
  String get addBotWithPlus => '+ Add Bot';

  @override
  String get addContact => 'Tambah Kenalan';

  @override
  String get addContactWithPlus => '+ Add Contact';

  @override
  String get addFile => 'Add File';

  @override
  String get addGroupDescription => 'Tambahkan keterangan kumpulan';

  @override
  String get addParticipants => 'Tambah Peserta';

  @override
  String get addPeopleSearchHint => 'Mixin ID or Phone number';

  @override
  String get addProxy => 'Add Proxy';

  @override
  String get addSticker => 'Add Sticker';

  @override
  String get addStickerFailed => 'Penambahan pelekat gagal';

  @override
  String get addStickers => 'Add Stickers';

  @override
  String get addToCircle => 'Add to Circle';

  @override
  String get added => 'Added';

  @override
  String get address => 'Alamat';

  @override
  String get admin => 'pentadbir';

  @override
  String get alertKeyContactContactMessage => 'berkongsi kenalan';

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
  String get appearance => 'Penampilan';

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
  String get backupChat => 'Sandarkan Sembang';

  @override
  String get backupToOtherDevice => 'Sandarkan ke peranti lain';

  @override
  String get backupToOtherDeviceTips =>
      'Sandarkan sejarah sembang anda ke peranti lain. Pastikan kedua-dua peranti disambungkan ke Wi-Fi atau hotspot yang sama.';

  @override
  String get backupWaitingOtherDevice =>
      'Buka Mixin pada peranti anda yang satu lagi dan mulakan pemulihan di sana.';

  @override
  String get biography => 'Biography';

  @override
  String get biometric => 'Biometric';

  @override
  String get block => 'Sekat';

  @override
  String get botNotFound => 'Aplikasi tidak dijumpai';

  @override
  String get bots => 'BOT';

  @override
  String get botsTitle => 'Bot';

  @override
  String get bringAllToFront => 'Bring All to Front';

  @override
  String get canNotRecognizeQrCode => 'Tidak dapat mengenali kod QR';

  @override
  String get cancel => 'Batal';

  @override
  String get card => 'Kad';

  @override
  String get change => 'Ubah';

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
  String get chatBackupAndRestore => 'Sandaran dan Pemulihan Sembang';

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
      other: 'Padamkan $arg0 mesej?',
      one: 'null',
    );
    return '$_temp0';
  }

  @override
  String chatGroupAdd(Object arg0, Object arg1) {
    return '$arg0 menambahkan $arg1';
  }

  @override
  String chatGroupExit(Object arg0) {
    return 'Tinggal $arg0';
  }

  @override
  String chatGroupJoin(Object arg0) {
    return '$arg0 menyertai kumpulan melalui pautan jemputan';
  }

  @override
  String chatGroupRemove(Object arg0, Object arg1) {
    return '$arg0 mengalih keluar $arg1';
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
      other: '$arg0 Perbualan',
      one: 'null',
    );
    return '$_temp0';
  }

  @override
  String circleTitle(Object arg0) {
    return 'Lingkaran $arg0';
  }

  @override
  String get circles => 'Lingkaran';

  @override
  String get clear => 'Kosong';

  @override
  String get clearChat => 'Kosongkan Sembang';

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
  String get confirm => 'Sahkan';

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
  String get contact => 'Kenalan';

  @override
  String contactMixinId(Object arg0) {
    return 'Mixin ID: $arg0';
  }

  @override
  String get contactMuteTitle => 'Senyapkan pemberitahuan untuk…';

  @override
  String get contactTitle => 'Contacts';

  @override
  String get contentTooLong => 'Kandungan terlalu lama';

  @override
  String get contentVoice => '[Panggilan suara]';

  @override
  String get continueText => 'Teruskan';

  @override
  String get conversation => 'Perbualan';

  @override
  String conversationDeleteTitle(Object arg0) {
    return 'Delete chat: $arg0';
  }

  @override
  String get copy => 'Salinan';

  @override
  String get copyImage => 'Copy Image';

  @override
  String get copyInvite => 'Salin pautan';

  @override
  String get copyLink => 'Salin pautan';

  @override
  String get copySelectedText => 'Copy Selected Text';

  @override
  String get copyText => 'Copy Text';

  @override
  String get create => 'Buat';

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
  String get dataAndStorageUsage => 'Penggunaan Data dan Storan';

  @override
  String get dataError => 'Kesalahan data';

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
      'Pangkalan data sedang ditingkatkan, mungkin memerlukan beberapa minit, jangan tutup Aplikasi ini.';

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
  String get deleteForEveryone => 'Padamkan untuk Semua Orang';

  @override
  String get deleteForMe => 'Padamkan untuk saya';

  @override
  String get deleteGroup => 'Padam Kumpulan';

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
  String get developer => 'Pemaju';

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
  String get dismissAsAdmin => 'Ketepikan pentadbir';

  @override
  String get done => 'Selesai';

  @override
  String get download => 'Download';

  @override
  String get downloadLink => 'Download Link:';

  @override
  String get draft => 'Draft';

  @override
  String get dragAndDropFileHere => 'Drag and drop files here';

  @override
  String get durationIsTooShort => 'Jangka masa terlalu pendek';

  @override
  String get edit => 'Edit';

  @override
  String get editCircleName => 'Edit Nama Lingkaran';

  @override
  String get editConversations => 'Edit Perbualan';

  @override
  String get editGroupDescription => 'Edit keterangan kumpulan';

  @override
  String get editGroupName => 'Edit Nama';

  @override
  String get editImageClearWarning =>
      'All changes will be lost. Are you sure you want to exit?';

  @override
  String get editName => 'Edit Nama';

  @override
  String get editProfile => 'Edit Profile';

  @override
  String get enablePushNotification => 'Hidupkan Pemberitahuan';

  @override
  String get encryptZipFileWithPassword =>
      'Encrypt the ZIP file with a password';

  @override
  String get enterPinToDeleteAccount => 'Enter your PIN to delete your account';

  @override
  String get enterToSend => 'Return/Enter ⏎ to Send';

  @override
  String get enterYourPhoneNumber => 'Masukkan nombor telefon bimbit anda';

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
  String get errorAuthentication => 'RALAT 401: Log masuk untuk meneruskan';

  @override
  String get errorBadData =>
      'RALAT 10002: Data permintaan mempunyai medan yang tidak sah';

  @override
  String get errorBlockchain =>
      'RALAT 30100: Rantai blok tidak diselaraskan, sila cuba sebentar lagi.';

  @override
  String get errorConnectionTimeout => 'Tamat masa sambungan rangkaian';

  @override
  String get errorFullGroup => 'RALAT 20116: Kumpulan sembang penuh.';

  @override
  String get errorInsufficientBalance => 'RALAT 20117: Baki tidak mencukupi';

  @override
  String errorInsufficientTransactionFeeWithAmount(Object arg0) {
    return 'ERROR 20124: Insufficient transaction fee. Please make sure your wallet has $arg0 as fee';
  }

  @override
  String errorInvalidAddress(Object arg0, Object arg1) {
    return 'ERROR 30102: Invalid address format. Please enter the correct $arg0 $arg1 address!';
  }

  @override
  String get errorInvalidAddressPlain =>
      'RALAT 30102: Format alamat tidak sah.';

  @override
  String get errorInvalidCodeTooFrequent =>
      'RALAT 20129: Hantar kod pengesahan terlalu kerap, sila cuba sebentar lagi.';

  @override
  String get errorInvalidEmergencyContact =>
      'ERROR 20130: Invalid recovery contact';

  @override
  String get errorInvalidPinFormat => 'RALAT 20118: Format PIN tidak sah';

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
  String get errorNotFound => 'RALAT 404: Tidak ditemui';

  @override
  String get errorNotSupportedAudioFormat =>
      'Tidak disokong format audio, sila buka oleh aplikasi lain.';

  @override
  String get errorNumberReachedLimit =>
      'RALAT 20132: Angka telah mencapai had.';

  @override
  String errorOldVersion(Object arg0) {
    return 'RALAT 10006: Sila kemas kini Mixin($arg0) untuk terus menggunakan perkhidmatan ini.';
  }

  @override
  String get errorOpenLocation => 'Tidak dapat mencari aplikasi peta';

  @override
  String get errorPermission => 'Sila buka kebenaran yang diperlukan';

  @override
  String get errorPhoneInvalidFormat => 'RALAT 20110: Nombor telefon tidak sah';

  @override
  String get errorPhoneSmsDelivery => 'RALAT 10003: Gagal menghantar SMS';

  @override
  String get errorPhoneVerificationCodeExpired =>
      'RALAT 20114: Kod pengesahan telefon yang telah tamat tempoh';

  @override
  String get errorPhoneVerificationCodeInvalid =>
      'RALAT 20113: Kod pengesahan telefon tidak sah';

  @override
  String get errorPinCheckTooManyRequest =>
      'Anda telah mencuba lebih dari 5 kali, sila tunggu sekurang-kurangnya 24 jam untuk mencuba lagi.';

  @override
  String get errorPinIncorrect => 'RALAT 20119: PIN tidak betul';

  @override
  String errorPinIncorrectWithTimes(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'RALAT 20119: PIN tidak betul. Anda masih mempunyai $arg0 peluang. Sila tunggu selama 24 jam untuk cuba lagi kemudian.',
      one: 'null',
    );
    return '$_temp0';
  }

  @override
  String get errorRecaptchaIsInvalid => 'RALAT 10004: Recaptcha tidak sah';

  @override
  String errorServer5xxCode(Object arg0) {
    return 'Pelayan sedang dalam penyelenggaraan: $arg0';
  }

  @override
  String get errorTooManyRequest => 'RALAT 429: Had kadar melebihi';

  @override
  String get errorTooManyStickers => 'RALAT 20126: Terlalu banyak pelekat';

  @override
  String get errorTooSmallTransferAmount =>
      'RALAT 20120: Jumlahnya terlalu kecil';

  @override
  String get errorTooSmallWithdrawAmount =>
      'RALAT 20127: Jumlah penarikan terlalu kecil';

  @override
  String get errorTranscriptForward =>
      'Please forward all attachments after they have been downloaded';

  @override
  String get errorTransferToDeactivatedUser =>
      'ERROR 20160: Transfers cannot be made to a deactivated user';

  @override
  String get errorUnableToOpenMedia =>
      'Tidak dapat mencari aplikasi yang dapat buka media ini.';

  @override
  String errorUnknownWithCode(Object arg0) {
    return 'RALAT: $arg0';
  }

  @override
  String errorUnknownWithMessage(Object arg0) {
    return 'RALAT: $arg0';
  }

  @override
  String get errorUploadAttachmentFailed =>
      'Failed to upload message attachment';

  @override
  String get errorUsedPhone =>
      'RALAT 20122: Telefon digunakan oleh orang lain.';

  @override
  String get errorUserInvalidFormat => 'Id pengguna tidak sah';

  @override
  String get errorWithdrawalMemoFormatIncorrect =>
      'RALAT 20131: Penarikan format memo tidak betul.';

  @override
  String get errorWithdrawalSuspend =>
      'ERROR 20137: Withdrawals are suspended.';

  @override
  String get exit => 'Keluar';

  @override
  String get exitGroup => 'Keluar Kumpulan';

  @override
  String get failed => 'Failed';

  @override
  String get failedToOpenDatabase =>
      'An error occurred while opening the database.';

  @override
  String get fee => 'Bayaran';

  @override
  String get file => 'Fail';

  @override
  String get fileChooserError => 'Ralat pemilih fail';

  @override
  String get fileDoesNotExist => 'Fail tidak wujud';

  @override
  String get fileError => 'Ralat fail';

  @override
  String get files => 'Fail';

  @override
  String get flags => 'Flags';

  @override
  String get followSystem => 'Ikut Sistem';

  @override
  String get followUsOnFacebook => 'Ikuti kami di Facebook';

  @override
  String get followUsOnX => 'Ikuti kami di X';

  @override
  String get foodAndDrink => 'Food & Drink';

  @override
  String get formatNotSupported => 'Format tidak disokong';

  @override
  String get forward => 'Ke hadapan';

  @override
  String get from => 'From';

  @override
  String get fromWithColon => 'From:';

  @override
  String get generateQrcode => 'Generate QR Code';

  @override
  String get groupAlreadyIn => 'Anda sudah berada dalam kumpulan';

  @override
  String get groupCantSend =>
      'Anda tidak dapat menghantar mesej kepada kumpulan ini kerana anda bukan lagi peserta.';

  @override
  String get groupName => 'Nama kumpulan';

  @override
  String get groupParticipants => 'Participants';

  @override
  String groupPopMenuMessage(Object arg0) {
    return 'Mesej $arg0';
  }

  @override
  String groupPopMenuRemove(Object arg0) {
    return 'Alih keluar $arg0';
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
  String get helpCenter => 'Pusat bantuan';

  @override
  String get hideMixin => 'Hide Mixin';

  @override
  String get host => 'Host';

  @override
  String hour(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 Jam',
      one: 'null',
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
  String get initializing => 'Memulakan…';

  @override
  String get invalidStickerFormat => 'Format pelekat tidak sah';

  @override
  String get inviteInfo =>
      'Sesiapa sahaja yang mempunyai Mixin boleh mengikuti pautan ini untuk menyertai kumpulan ini. Kongsi sahaja dengan orang yang anda percayai.';

  @override
  String get inviteToGroupViaLink => 'Jemput ke Kumpulan melalui Pautan';

  @override
  String get joinGroupWithPlus => '+ Join group';

  @override
  String joinedIn(Object arg0) {
    return 'Menyertai $arg0';
  }

  @override
  String landingDeleteContent(Object arg0, Object arg1) {
    return 'You requested to delete your account on $arg0. The account will be deleted on $arg1. If you continue to log in, your account deletion will be cancelled.';
  }

  @override
  String landingInvitationDialogContent(Object arg0) {
    return 'Kami akan menghantar kod 4 digit ke nombor telefon anda $arg0, sila masukkan kod di skrin seterusnya.';
  }

  @override
  String landingValidationTitle(Object arg0) {
    return 'Masukkan kod 4 digit yang dihantar kepada anda di $arg0';
  }

  @override
  String get learnMore => 'Ketahui Lebih Lanjut';

  @override
  String get less => 'less';

  @override
  String get light => 'Light';

  @override
  String get linkedDevice => 'linked device';

  @override
  String get live => 'Langsung';

  @override
  String get loading => 'Loading...';

  @override
  String get loadingTime =>
      'Waktu sistem tidak biasa, sila terus gunakan lagi selepas pembetulan';

  @override
  String get locateToChat => 'Locate in Chat';

  @override
  String get location => 'Lokasi';

  @override
  String get lock => 'Lock';

  @override
  String get logIn => 'Log masuk';

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
  String get makeGroupAdmin => 'Buat pentadbir kumpulan';

  @override
  String get media => 'Media';

  @override
  String get memo => 'Memo';

  @override
  String get messageE2ee =>
      'Mesej ke perbualan ini disulitkan dari hujung ke hujung, ketuk untuk maklumat lebih lanjut.';

  @override
  String get messageNotFound => 'Mesej tidak dijumpai';

  @override
  String get messageNotSupport =>
      'Mesej jenis ini tidak disokong, sila tingkatkan Mixin ke versi terkini.';

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
  String get mixinMessengerDesktop => 'Desktop Mixin';

  @override
  String get more => 'Lebih banyak lagi';

  @override
  String get multisigTransaction => 'Transaksi Multisig';

  @override
  String get mute => 'Mute';

  @override
  String myMixinId(Object arg0) {
    return 'ID Mixin Saya: $arg0';
  }

  @override
  String get myStickers => 'My Stickers';

  @override
  String get na => 'N/A';

  @override
  String get name => 'Nama';

  @override
  String get networkConnectionFailed => 'Network connection failed';

  @override
  String get networkError => 'Ralat rangkaian';

  @override
  String get newVersionAvailable => 'New version available';

  @override
  String newVersionDescription(Object arg0, Object arg1) {
    return 'Mixin Messenger $arg0 is now available, you have $arg1. Would you like to download it now?';
  }

  @override
  String get next => 'Seterusnya';

  @override
  String get nextConversation => 'Next conversation';

  @override
  String get noAudio => 'TIADA AUDIO';

  @override
  String get noCamera => 'Tiada kamera';

  @override
  String get noData => 'No Data';

  @override
  String get noFiles => 'TIADA FAIL';

  @override
  String get noLinks => 'TIADA Pautan';

  @override
  String get noMedia => 'TIADA MEDIA';

  @override
  String get noNetworkConnection => 'Tiada sambungan rangkaian';

  @override
  String get noPosts => 'TIADA POST';

  @override
  String get noResults => 'Tiada keputusan';

  @override
  String get notFound => 'Tidak ditemui';

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
  String get notifications => 'Pemberitahuan';

  @override
  String nowAnAddmin(Object arg0) {
    return '$arg0 now an admin';
  }

  @override
  String get objects => 'Objects';

  @override
  String get oneByOneForward => 'One-by-One Forward';

  @override
  String get oneHour => '1 jam';

  @override
  String get oneYear => '1 tahun';

  @override
  String get open => 'Open';

  @override
  String get openHomePage => 'Buka laman Utama';

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
  String get owner => 'pemilik';

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
      other: '$arg0/$arg1 pengesahan',
      one: 'null',
    );
    return '$_temp0';
  }

  @override
  String get phoneNumber => 'Nombor telefon';

  @override
  String get photos => 'Foto';

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
  String get post => 'Kirim';

  @override
  String get preferences => 'Preferences';

  @override
  String get previousConversation => 'Previous conversation';

  @override
  String get privacyPolicy => 'Dasar Privasi';

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
  String get raw => 'Mentah';

  @override
  String get rebate => 'Rebat';

  @override
  String get recaptchaTimeout => 'Tamat masa Recaptcha';

  @override
  String get receiver => 'Penerima';

  @override
  String get recentChats => 'SEMBANG';

  @override
  String get reedit => 'Re-edit';

  @override
  String get refresh => 'Segarkan';

  @override
  String get removeBot => 'Keluarkan Bot';

  @override
  String get removeChatFromCircle => 'Remove Chat from circle';

  @override
  String get removeContact => 'Buang kenalan';

  @override
  String get removeStickers => 'Remove Stickers';

  @override
  String get reply => 'Reply';

  @override
  String get report => 'Lapor';

  @override
  String get reportAndBlock => 'Report and block?';

  @override
  String get reportTitle => 'Send the conversation log to developers?';

  @override
  String get resendCode => 'Hantar semula kod';

  @override
  String resendCodeIn(Object arg0) {
    return 'Hantar semula kod dalam $arg0 s';
  }

  @override
  String get reset => 'Reset';

  @override
  String get resetLink => 'Reset Link';

  @override
  String get restoreChat => 'Pulihkan Sembang';

  @override
  String get restoreChatTip =>
      'Pulihkan sejarah sembang anda daripada peranti lain. Pastikan kedua-dua peranti disambungkan ke Wi-Fi atau hotspot yang sama.';

  @override
  String get restoreFromOtherDevice => 'Pulihkan daripada peranti lain';

  @override
  String get retry => 'CUBA SEMULA';

  @override
  String get retryUploadFailed => 'Gagal memuat naik semula.';

  @override
  String get revokeMultisigTransaction => 'Batalkan Urus Niaga Multisig';

  @override
  String get save => 'Jimat';

  @override
  String get saveAs => 'Save as';

  @override
  String get saveToCameraRoll => 'Save to Camera Roll';

  @override
  String get sayHi => 'Ucap hai';

  @override
  String get scamWarning =>
      'Amaran: Ramai pengguna melaporkan akaun ini sebagai penipuan. Sila berhati-hati, terutamanya jika ia meminta wang kepada anda';

  @override
  String get screenPasscode => 'Screen Passcode';

  @override
  String get search => 'Cari';

  @override
  String get searchContact => 'Search Contacts';

  @override
  String get searchConversation => 'Cari Perbualan';

  @override
  String get searchEmpty => 'No chats, contacts or messages found.';

  @override
  String get searchPlaceholderNumber => 'Search Mixin ID or phone number:';

  @override
  String searchRelatedMessage(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 mesej berkaitan',
      one: 'null',
    );
    return '$_temp0';
  }

  @override
  String get searchUnread => 'Search Unread';

  @override
  String get secretUrl => 'https://mixin.one/pages/1000007';

  @override
  String get security => 'Keselamatan';

  @override
  String get select => 'Pilih';

  @override
  String get send => 'Hantar';

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
  String get settingAuthSearchHint => 'Mixin ID, Nama';

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
  String get share => 'Berkongsi';

  @override
  String get shareApps => 'Shared Apps';

  @override
  String get shareContact => 'Share Contact';

  @override
  String get shareError => 'Kongsi ralat';

  @override
  String get shareLink => 'Kongsi pautan';

  @override
  String shareMessageDescription(Object arg0, Object arg1) {
    return 'Adakah anda pasti mahu menghantar $arg0 dari $arg1?';
  }

  @override
  String shareMessageDescriptionEmpty(Object arg0) {
    return 'Adakah anda pasti mahu menghantar $arg0?';
  }

  @override
  String get sharedMedia => 'Media Berkongsi';

  @override
  String get show => 'Tunjuk';

  @override
  String get showAvatar => 'Show avatar';

  @override
  String get showIdentityNumber => 'Show Identity Number';

  @override
  String get showMixin => 'Show Mixin';

  @override
  String get signIn => 'Log masuk';

  @override
  String get signOut => 'Sign Out';

  @override
  String get signWithMobileNumber => 'Log masuk dengan nombor telefon';

  @override
  String get signWithQrcode => 'Sign in with QR code';

  @override
  String get smileysAndPeople => 'Smileys & People';

  @override
  String get snapshotHash => 'Snapshot Hash';

  @override
  String get status => 'Status';

  @override
  String get sticker => 'Pelekat';

  @override
  String get stickerAddInvalidSize =>
      'Memerlukan saiz fail pelekat lebih besar daripada 1KB dan kurang dari 1MB, lebar dan tinggi antara 128px dan 1024px.';

  @override
  String get stickerAlbumDetail => 'Sticker Album Details';

  @override
  String get stickerStore => 'Sticker Store';

  @override
  String get storageAutoDownloadDescription =>
      'Change auto-download settings for media.';

  @override
  String get storageUsage => 'Penggunaan Storan';

  @override
  String get strangerHint => 'This sender is not in your contacts';

  @override
  String get strangers => 'Strangers';

  @override
  String get successful => 'Berjaya';

  @override
  String get symbols => 'Symbols';

  @override
  String get syncFromOtherDevice => 'Segerakkan daripada peranti lain';

  @override
  String get syncToOtherDevice => 'Segerakkan ke peranti lain';

  @override
  String get termsOfService => 'Terma Perkhidmatan';

  @override
  String get text => 'Text';

  @override
  String get theme => 'Tema';

  @override
  String get thisMessageWasDeleted => 'Mesej ini telah dipadamkan';

  @override
  String get time => 'Masa';

  @override
  String get to => 'To';

  @override
  String get today => 'Hari ini';

  @override
  String get toggleChatInfo => 'Toggle chat info';

  @override
  String get trace => 'Jejak';

  @override
  String get transactionHash => 'Urus niaga Cincangan';

  @override
  String get transactionId => 'Id Urus Niaga';

  @override
  String get transactionType => 'Jenis Transaksi';

  @override
  String get transactions => 'Urus Niaga';

  @override
  String get transactionsCannotBeDeleted => 'Transactions cannot be deleted';

  @override
  String get transcript => 'Transcript';

  @override
  String get transfer => 'Pindah';

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
  String get typeMessage => 'Taipkan mesej';

  @override
  String unableToOpenFile(Object arg0) {
    return 'Unable to open file: $arg0';
  }

  @override
  String get unblock => 'Buka sekatan';

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
  String get unmute => 'Nyahsenyap';

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
  String get updateMixin => 'Kemas kini Mixin';

  @override
  String updateMixinDescription(Object arg0) {
    return 'Versi semasa ($arg0) tidak lagi tersedia!\nSila klik Kemas kini di bawah untuk mengemas kini ke versi terbaharu dari Google Play.';
  }

  @override
  String get upgrade => 'Upgrade';

  @override
  String get upgrading => 'Menaik taraf';

  @override
  String get useBiometric => 'Use Biometric';

  @override
  String get userDeleteHint => 'This user has deleted their account.';

  @override
  String get userNotFound => 'Pengguna tidak ditemui';

  @override
  String get username => 'Username';

  @override
  String valueNow(Object arg0) {
    return 'nilai sekarang $arg0';
  }

  @override
  String valueThen(Object arg0) {
    return 'nilai maka $arg0';
  }

  @override
  String get verifyPin => 'Sahkan PIN';

  @override
  String get video => 'Video';

  @override
  String get videos => 'Video';

  @override
  String get waitingForThisMessage => 'Waiting for this message.';

  @override
  String get waitingOtherDeviceConnection =>
      'Menunggu peranti lain untuk bersambung.';

  @override
  String get webview2RuntimeInstallDescription =>
      'The device has not installed the WebView2 Runtime component. Please download and install WebView2 Runtime first.';

  @override
  String get webviewRuntimeUnavailable => 'WebView runtime is unavailable';

  @override
  String get window => 'Window';

  @override
  String get withdrawal => 'Pengeluaran';

  @override
  String get withdrawalHash => 'Withdrawal Hash';

  @override
  String get you => 'Anda';

  @override
  String get youDeletedThisMessage => 'Anda memadamkan mesej ini';

  @override
  String get zoom => 'Zoom';
}
