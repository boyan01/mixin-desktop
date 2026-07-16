import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_es.dart';
import 'app_localizations_id.dart';
import 'app_localizations_ja.dart';
import 'app_localizations_ms.dart';
import 'app_localizations_ru.dart';
import 'app_localizations_zh.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'generated/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('es'),
    Locale('id'),
    Locale('ja'),
    Locale('ms'),
    Locale('ru'),
    Locale('zh', 'HK'),
    Locale('zh', 'TW'),
    Locale('zh'),
  ];

  /// No description provided for @aMessage.
  ///
  /// In en, this message translates to:
  /// **'a message'**
  String get aMessage;

  /// No description provided for @about.
  ///
  /// In en, this message translates to:
  /// **'About'**
  String get about;

  /// No description provided for @account.
  ///
  /// In en, this message translates to:
  /// **'Account'**
  String get account;

  /// No description provided for @activity.
  ///
  /// In en, this message translates to:
  /// **'Activity'**
  String get activity;

  /// No description provided for @add.
  ///
  /// In en, this message translates to:
  /// **'Add'**
  String get add;

  /// No description provided for @addACaption.
  ///
  /// In en, this message translates to:
  /// **'Add a caption'**
  String get addACaption;

  /// No description provided for @addBotWithPlus.
  ///
  /// In en, this message translates to:
  /// **'+ Add Bot'**
  String get addBotWithPlus;

  /// No description provided for @addContact.
  ///
  /// In en, this message translates to:
  /// **'Add Contact'**
  String get addContact;

  /// No description provided for @addContactWithPlus.
  ///
  /// In en, this message translates to:
  /// **'+ Add Contact'**
  String get addContactWithPlus;

  /// No description provided for @addFile.
  ///
  /// In en, this message translates to:
  /// **'Add File'**
  String get addFile;

  /// No description provided for @addGroupDescription.
  ///
  /// In en, this message translates to:
  /// **'Add group description'**
  String get addGroupDescription;

  /// No description provided for @addParticipants.
  ///
  /// In en, this message translates to:
  /// **'Add Participants'**
  String get addParticipants;

  /// No description provided for @addPeopleSearchHint.
  ///
  /// In en, this message translates to:
  /// **'Mixin ID or Phone number'**
  String get addPeopleSearchHint;

  /// No description provided for @addProxy.
  ///
  /// In en, this message translates to:
  /// **'Add Proxy'**
  String get addProxy;

  /// No description provided for @addSticker.
  ///
  /// In en, this message translates to:
  /// **'Add Sticker'**
  String get addSticker;

  /// No description provided for @addStickerFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to add sticker'**
  String get addStickerFailed;

  /// No description provided for @addStickers.
  ///
  /// In en, this message translates to:
  /// **'Add Stickers'**
  String get addStickers;

  /// No description provided for @addToCircle.
  ///
  /// In en, this message translates to:
  /// **'Add to Circle'**
  String get addToCircle;

  /// No description provided for @added.
  ///
  /// In en, this message translates to:
  /// **'Added'**
  String get added;

  /// No description provided for @address.
  ///
  /// In en, this message translates to:
  /// **'Address'**
  String get address;

  /// No description provided for @admin.
  ///
  /// In en, this message translates to:
  /// **'Admin'**
  String get admin;

  /// No description provided for @alertKeyContactContactMessage.
  ///
  /// In en, this message translates to:
  /// **'sent you a contact'**
  String get alertKeyContactContactMessage;

  /// No description provided for @allChats.
  ///
  /// In en, this message translates to:
  /// **'Chats'**
  String get allChats;

  /// No description provided for @animalsAndNature.
  ///
  /// In en, this message translates to:
  /// **'Animals & Nature'**
  String get animalsAndNature;

  /// No description provided for @anonymous.
  ///
  /// In en, this message translates to:
  /// **'Anonymous'**
  String get anonymous;

  /// No description provided for @anonymousNumber.
  ///
  /// In en, this message translates to:
  /// **'Anonymous Number'**
  String get anonymousNumber;

  /// No description provided for @appCardShareDisallow.
  ///
  /// In en, this message translates to:
  /// **'This URL cannot be shared.'**
  String get appCardShareDisallow;

  /// No description provided for @appearance.
  ///
  /// In en, this message translates to:
  /// **'Appearance'**
  String get appearance;

  /// No description provided for @archivedFolder.
  ///
  /// In en, this message translates to:
  /// **'Archived Folder'**
  String get archivedFolder;

  /// No description provided for @assetType.
  ///
  /// In en, this message translates to:
  /// **'Asset Type'**
  String get assetType;

  /// No description provided for @audio.
  ///
  /// In en, this message translates to:
  /// **'Audio'**
  String get audio;

  /// No description provided for @audios.
  ///
  /// In en, this message translates to:
  /// **'Audio'**
  String get audios;

  /// No description provided for @autoBackup.
  ///
  /// In en, this message translates to:
  /// **'Auto Backup'**
  String get autoBackup;

  /// No description provided for @autoLock.
  ///
  /// In en, this message translates to:
  /// **'Auto Lock'**
  String get autoLock;

  /// No description provided for @avatar.
  ///
  /// In en, this message translates to:
  /// **'Avatar'**
  String get avatar;

  /// No description provided for @backup.
  ///
  /// In en, this message translates to:
  /// **'Backup'**
  String get backup;

  /// No description provided for @backupChat.
  ///
  /// In en, this message translates to:
  /// **'Backup Chat'**
  String get backupChat;

  /// No description provided for @backupToOtherDevice.
  ///
  /// In en, this message translates to:
  /// **'Backup to Other Device'**
  String get backupToOtherDevice;

  /// No description provided for @backupToOtherDeviceTips.
  ///
  /// In en, this message translates to:
  /// **'Back up your chat history to another device. Make sure both devices are connected to the same Wi-Fi or hotspot.'**
  String get backupToOtherDeviceTips;

  /// No description provided for @backupWaitingOtherDevice.
  ///
  /// In en, this message translates to:
  /// **'Open Mixin on your other device and start restore there.'**
  String get backupWaitingOtherDevice;

  /// No description provided for @biography.
  ///
  /// In en, this message translates to:
  /// **'Biography'**
  String get biography;

  /// No description provided for @biometric.
  ///
  /// In en, this message translates to:
  /// **'Biometric'**
  String get biometric;

  /// No description provided for @block.
  ///
  /// In en, this message translates to:
  /// **'Block'**
  String get block;

  /// No description provided for @botNotFound.
  ///
  /// In en, this message translates to:
  /// **'Bot not found'**
  String get botNotFound;

  /// No description provided for @bots.
  ///
  /// In en, this message translates to:
  /// **'BOTS'**
  String get bots;

  /// No description provided for @botsTitle.
  ///
  /// In en, this message translates to:
  /// **'Bots'**
  String get botsTitle;

  /// No description provided for @bringAllToFront.
  ///
  /// In en, this message translates to:
  /// **'Bring All to Front'**
  String get bringAllToFront;

  /// No description provided for @canNotRecognizeQrCode.
  ///
  /// In en, this message translates to:
  /// **'Cannot recognize the QR code'**
  String get canNotRecognizeQrCode;

  /// No description provided for @cancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get cancel;

  /// No description provided for @card.
  ///
  /// In en, this message translates to:
  /// **'Card'**
  String get card;

  /// No description provided for @change.
  ///
  /// In en, this message translates to:
  /// **'Change'**
  String get change;

  /// No description provided for @changeNumber.
  ///
  /// In en, this message translates to:
  /// **'Change Number'**
  String get changeNumber;

  /// No description provided for @changeNumberInstead.
  ///
  /// In en, this message translates to:
  /// **'Change Number Instead'**
  String get changeNumberInstead;

  /// No description provided for @changedDisappearingMessageSettings.
  ///
  /// In en, this message translates to:
  /// **'{arg0} changed disappearing message settings.'**
  String changedDisappearingMessageSettings(Object arg0);

  /// No description provided for @chatBackup.
  ///
  /// In en, this message translates to:
  /// **'Chat Backup'**
  String get chatBackup;

  /// No description provided for @chatBackupAndRestore.
  ///
  /// In en, this message translates to:
  /// **'Chat Backup and Restore'**
  String get chatBackupAndRestore;

  /// No description provided for @chatBotReceptionTitle.
  ///
  /// In en, this message translates to:
  /// **'Tap the button to interact with the bot'**
  String get chatBotReceptionTitle;

  /// No description provided for @chatDecryptionFailedHint.
  ///
  /// In en, this message translates to:
  /// **'Waiting for {arg0} to get online and establish an encrypted session.'**
  String chatDecryptionFailedHint(Object arg0);

  /// No description provided for @chatDeleteMessage.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{Delete {arg0} message?} other{Delete {arg0} messages?}}'**
  String chatDeleteMessage(Object arg0, num count);

  /// No description provided for @chatGroupAdd.
  ///
  /// In en, this message translates to:
  /// **'{arg0} added {arg1}'**
  String chatGroupAdd(Object arg0, Object arg1);

  /// No description provided for @chatGroupExit.
  ///
  /// In en, this message translates to:
  /// **'{arg0} left'**
  String chatGroupExit(Object arg0);

  /// No description provided for @chatGroupJoin.
  ///
  /// In en, this message translates to:
  /// **'{arg0} joined the group via invite link'**
  String chatGroupJoin(Object arg0);

  /// No description provided for @chatGroupRemove.
  ///
  /// In en, this message translates to:
  /// **'{arg0} removed {arg1}'**
  String chatGroupRemove(Object arg0, Object arg1);

  /// No description provided for @chatHintE2e.
  ///
  /// In en, this message translates to:
  /// **'End-to-end encrypted'**
  String get chatHintE2e;

  /// No description provided for @chatNotSupportUriOnPhone.
  ///
  /// In en, this message translates to:
  /// **'This type of URL is not supported. Please check it on your phone.'**
  String get chatNotSupportUriOnPhone;

  /// No description provided for @chatNotSupportUrl.
  ///
  /// In en, this message translates to:
  /// **'https://support.mixin.one/en/article/how-to-do-when-you-receive-a-message-like-this-this-type-of-message-is-not-supported-please-upgrade-mixin-17j1t3p'**
  String get chatNotSupportUrl;

  /// No description provided for @chatNotSupportViewOnPhone.
  ///
  /// In en, this message translates to:
  /// **'This type of message is not supported. Please check it on your phone.'**
  String get chatNotSupportViewOnPhone;

  /// No description provided for @chatPinMessage.
  ///
  /// In en, this message translates to:
  /// **'{arg0} pinned {arg1}'**
  String chatPinMessage(Object arg0, Object arg1);

  /// No description provided for @chatTextSize.
  ///
  /// In en, this message translates to:
  /// **'Chat Text Size'**
  String get chatTextSize;

  /// No description provided for @chats.
  ///
  /// In en, this message translates to:
  /// **'Chats'**
  String get chats;

  /// No description provided for @checkNewVersion.
  ///
  /// In en, this message translates to:
  /// **'Check for updates'**
  String get checkNewVersion;

  /// No description provided for @circleSubtitle.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{{arg0} Conversation} other{{arg0} Conversations}}'**
  String circleSubtitle(Object arg0, num count);

  /// No description provided for @circleTitle.
  ///
  /// In en, this message translates to:
  /// **'{arg0}\'s Circles'**
  String circleTitle(Object arg0);

  /// No description provided for @circles.
  ///
  /// In en, this message translates to:
  /// **'Circles'**
  String get circles;

  /// No description provided for @clear.
  ///
  /// In en, this message translates to:
  /// **'Clear'**
  String get clear;

  /// No description provided for @clearChat.
  ///
  /// In en, this message translates to:
  /// **'Clear Chat'**
  String get clearChat;

  /// No description provided for @clearFilter.
  ///
  /// In en, this message translates to:
  /// **'Clear filter'**
  String get clearFilter;

  /// No description provided for @clickToReloadQrcode.
  ///
  /// In en, this message translates to:
  /// **'Click to reload the QR code'**
  String get clickToReloadQrcode;

  /// No description provided for @close.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get close;

  /// No description provided for @closeWindow.
  ///
  /// In en, this message translates to:
  /// **'Close window'**
  String get closeWindow;

  /// No description provided for @closingBalance.
  ///
  /// In en, this message translates to:
  /// **'Closing Balance'**
  String get closingBalance;

  /// No description provided for @collapse.
  ///
  /// In en, this message translates to:
  /// **'Collapse'**
  String get collapse;

  /// No description provided for @collectible.
  ///
  /// In en, this message translates to:
  /// **'Collectible'**
  String get collectible;

  /// No description provided for @collectibles.
  ///
  /// In en, this message translates to:
  /// **'Collectibles'**
  String get collectibles;

  /// No description provided for @collection.
  ///
  /// In en, this message translates to:
  /// **'Collection'**
  String get collection;

  /// No description provided for @combineAndForward.
  ///
  /// In en, this message translates to:
  /// **'Combine and forward'**
  String get combineAndForward;

  /// No description provided for @confirm.
  ///
  /// In en, this message translates to:
  /// **'Confirm'**
  String get confirm;

  /// No description provided for @confirmPasscodeDesc.
  ///
  /// In en, this message translates to:
  /// **'Enter again to confirm the passcode'**
  String get confirmPasscodeDesc;

  /// No description provided for @confirmSyncChatsFromPhone.
  ///
  /// In en, this message translates to:
  /// **'Are you sure to sync the chat history from the phone?'**
  String get confirmSyncChatsFromPhone;

  /// No description provided for @confirmSyncChatsToPhone.
  ///
  /// In en, this message translates to:
  /// **'Are you sure to sync the chat history to the phone?'**
  String get confirmSyncChatsToPhone;

  /// No description provided for @confirmations.
  ///
  /// In en, this message translates to:
  /// **'Confirmations'**
  String get confirmations;

  /// No description provided for @contact.
  ///
  /// In en, this message translates to:
  /// **'Contact'**
  String get contact;

  /// No description provided for @contactMixinId.
  ///
  /// In en, this message translates to:
  /// **'Mixin ID: {arg0}'**
  String contactMixinId(Object arg0);

  /// No description provided for @contactMuteTitle.
  ///
  /// In en, this message translates to:
  /// **'Mute notifications for…'**
  String get contactMuteTitle;

  /// No description provided for @contactTitle.
  ///
  /// In en, this message translates to:
  /// **'Contacts'**
  String get contactTitle;

  /// No description provided for @contentTooLong.
  ///
  /// In en, this message translates to:
  /// **'Content too long'**
  String get contentTooLong;

  /// No description provided for @contentVoice.
  ///
  /// In en, this message translates to:
  /// **'[Voice call]'**
  String get contentVoice;

  /// No description provided for @continueText.
  ///
  /// In en, this message translates to:
  /// **'Continue'**
  String get continueText;

  /// No description provided for @conversation.
  ///
  /// In en, this message translates to:
  /// **'Conversation'**
  String get conversation;

  /// No description provided for @conversationDeleteTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete chat: {arg0}'**
  String conversationDeleteTitle(Object arg0);

  /// No description provided for @copy.
  ///
  /// In en, this message translates to:
  /// **'Copy'**
  String get copy;

  /// No description provided for @copyImage.
  ///
  /// In en, this message translates to:
  /// **'Copy Image'**
  String get copyImage;

  /// No description provided for @copyInvite.
  ///
  /// In en, this message translates to:
  /// **'Copy Invite Link'**
  String get copyInvite;

  /// No description provided for @copyLink.
  ///
  /// In en, this message translates to:
  /// **'Copy Link'**
  String get copyLink;

  /// No description provided for @copySelectedText.
  ///
  /// In en, this message translates to:
  /// **'Copy Selected Text'**
  String get copySelectedText;

  /// No description provided for @copyText.
  ///
  /// In en, this message translates to:
  /// **'Copy Text'**
  String get copyText;

  /// No description provided for @create.
  ///
  /// In en, this message translates to:
  /// **'Create'**
  String get create;

  /// No description provided for @createCircle.
  ///
  /// In en, this message translates to:
  /// **'New Circle'**
  String get createCircle;

  /// No description provided for @createConversation.
  ///
  /// In en, this message translates to:
  /// **'New Conversation'**
  String get createConversation;

  /// No description provided for @createGroup.
  ///
  /// In en, this message translates to:
  /// **'New Group'**
  String get createGroup;

  /// No description provided for @createdAt.
  ///
  /// In en, this message translates to:
  /// **'Created {arg0}'**
  String createdAt(Object arg0);

  /// No description provided for @createdThisGroup.
  ///
  /// In en, this message translates to:
  /// **'{arg0} created this group'**
  String createdThisGroup(Object arg0);

  /// No description provided for @customTime.
  ///
  /// In en, this message translates to:
  /// **'Custom Time'**
  String get customTime;

  /// No description provided for @dark.
  ///
  /// In en, this message translates to:
  /// **'Dark'**
  String get dark;

  /// No description provided for @dataAndStorageUsage.
  ///
  /// In en, this message translates to:
  /// **'Data and Storage Usage'**
  String get dataAndStorageUsage;

  /// No description provided for @dataError.
  ///
  /// In en, this message translates to:
  /// **'Data error'**
  String get dataError;

  /// No description provided for @dataLoading.
  ///
  /// In en, this message translates to:
  /// **'Loading data, please wait...'**
  String get dataLoading;

  /// No description provided for @databaseCorruptedTips.
  ///
  /// In en, this message translates to:
  /// **'The database is corrupted and cannot be recovered. Clicking continue will create a new database file.'**
  String get databaseCorruptedTips;

  /// No description provided for @databaseLockedTips.
  ///
  /// In en, this message translates to:
  /// **'The database file is locked and cannot be accessed. Please try restarting the application or the system and try again.'**
  String get databaseLockedTips;

  /// No description provided for @databaseNotADbTips.
  ///
  /// In en, this message translates to:
  /// **'Cannot open the database. The file is not a valid database file.'**
  String get databaseNotADbTips;

  /// No description provided for @databaseRecreateTips.
  ///
  /// In en, this message translates to:
  /// **'Create a new database file. The old file will be deleted.'**
  String get databaseRecreateTips;

  /// No description provided for @databaseUpgradeTips.
  ///
  /// In en, this message translates to:
  /// **'The database is being upgraded. This may take several minutes. Please do not close this app.'**
  String get databaseUpgradeTips;

  /// No description provided for @delete.
  ///
  /// In en, this message translates to:
  /// **'Delete'**
  String get delete;

  /// No description provided for @deleteAccountDetailHint.
  ///
  /// In en, this message translates to:
  /// **'Local messages and iCloud Backups will not be deleted automatically'**
  String get deleteAccountDetailHint;

  /// No description provided for @deleteAccountHint.
  ///
  /// In en, this message translates to:
  /// **'Delete your account info and profile photo'**
  String get deleteAccountHint;

  /// No description provided for @deleteChat.
  ///
  /// In en, this message translates to:
  /// **'Delete Chat'**
  String get deleteChat;

  /// No description provided for @deleteChatDescription.
  ///
  /// In en, this message translates to:
  /// **'Deleting this chat will remove messages from this device only. They will not be removed from other devices.'**
  String get deleteChatDescription;

  /// No description provided for @deleteCircle.
  ///
  /// In en, this message translates to:
  /// **'Delete Circle'**
  String get deleteCircle;

  /// No description provided for @deleteForEveryone.
  ///
  /// In en, this message translates to:
  /// **'Delete for Everyone'**
  String get deleteForEveryone;

  /// No description provided for @deleteForMe.
  ///
  /// In en, this message translates to:
  /// **'Delete for me'**
  String get deleteForMe;

  /// No description provided for @deleteGroup.
  ///
  /// In en, this message translates to:
  /// **'Delete Group'**
  String get deleteGroup;

  /// No description provided for @deleteMyAccount.
  ///
  /// In en, this message translates to:
  /// **'Delete My Account'**
  String get deleteMyAccount;

  /// No description provided for @deleteTheCircle.
  ///
  /// In en, this message translates to:
  /// **'Do you want to delete the {arg0} circle?'**
  String deleteTheCircle(Object arg0);

  /// No description provided for @deposit.
  ///
  /// In en, this message translates to:
  /// **'Deposit'**
  String get deposit;

  /// No description provided for @depositHash.
  ///
  /// In en, this message translates to:
  /// **'Deposit Hash'**
  String get depositHash;

  /// No description provided for @developer.
  ///
  /// In en, this message translates to:
  /// **'Developer'**
  String get developer;

  /// No description provided for @deviceTransferFailed.
  ///
  /// In en, this message translates to:
  /// **'Transfer failed'**
  String get deviceTransferFailed;

  /// No description provided for @disableDisappearingMessage.
  ///
  /// In en, this message translates to:
  /// **'{arg0} disabled disappearing message'**
  String disableDisappearingMessage(Object arg0);

  /// No description provided for @disabled.
  ///
  /// In en, this message translates to:
  /// **'Disabled'**
  String get disabled;

  /// No description provided for @disappearingCustomTimeMaxWarning.
  ///
  /// In en, this message translates to:
  /// **'The maximum time is {arg0}.'**
  String disappearingCustomTimeMaxWarning(Object arg0);

  /// No description provided for @disappearingMessage.
  ///
  /// In en, this message translates to:
  /// **'Disappearing Messages'**
  String get disappearingMessage;

  /// No description provided for @disappearingMessageHint.
  ///
  /// In en, this message translates to:
  /// **'When enabled, new messages sent and received in this chat will disappear after they have been seen. Read the document to **learn more**.'**
  String get disappearingMessageHint;

  /// No description provided for @discard.
  ///
  /// In en, this message translates to:
  /// **'Discard'**
  String get discard;

  /// No description provided for @discardRecordingWarning.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to stop recording and discard your voice message?'**
  String get discardRecordingWarning;

  /// No description provided for @dismissAsAdmin.
  ///
  /// In en, this message translates to:
  /// **'Dismiss as Admin'**
  String get dismissAsAdmin;

  /// No description provided for @done.
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get done;

  /// No description provided for @download.
  ///
  /// In en, this message translates to:
  /// **'Download'**
  String get download;

  /// No description provided for @downloadLink.
  ///
  /// In en, this message translates to:
  /// **'Download Link:'**
  String get downloadLink;

  /// No description provided for @draft.
  ///
  /// In en, this message translates to:
  /// **'Draft'**
  String get draft;

  /// No description provided for @dragAndDropFileHere.
  ///
  /// In en, this message translates to:
  /// **'Drag and drop files here'**
  String get dragAndDropFileHere;

  /// No description provided for @durationIsTooShort.
  ///
  /// In en, this message translates to:
  /// **'Duration is too short'**
  String get durationIsTooShort;

  /// No description provided for @edit.
  ///
  /// In en, this message translates to:
  /// **'Edit'**
  String get edit;

  /// No description provided for @editCircleName.
  ///
  /// In en, this message translates to:
  /// **'Edit Circle Name'**
  String get editCircleName;

  /// No description provided for @editConversations.
  ///
  /// In en, this message translates to:
  /// **'Edit Conversations'**
  String get editConversations;

  /// No description provided for @editGroupDescription.
  ///
  /// In en, this message translates to:
  /// **'Edit Group Description'**
  String get editGroupDescription;

  /// No description provided for @editGroupName.
  ///
  /// In en, this message translates to:
  /// **'Edit Group Name'**
  String get editGroupName;

  /// No description provided for @editImageClearWarning.
  ///
  /// In en, this message translates to:
  /// **'All changes will be lost. Are you sure you want to exit?'**
  String get editImageClearWarning;

  /// No description provided for @editName.
  ///
  /// In en, this message translates to:
  /// **'Edit Name'**
  String get editName;

  /// No description provided for @editProfile.
  ///
  /// In en, this message translates to:
  /// **'Edit Profile'**
  String get editProfile;

  /// No description provided for @enablePushNotification.
  ///
  /// In en, this message translates to:
  /// **'Enable push notifications'**
  String get enablePushNotification;

  /// No description provided for @encryptZipFileWithPassword.
  ///
  /// In en, this message translates to:
  /// **'Encrypt the ZIP file with a password'**
  String get encryptZipFileWithPassword;

  /// No description provided for @enterPinToDeleteAccount.
  ///
  /// In en, this message translates to:
  /// **'Enter your PIN to delete your account'**
  String get enterPinToDeleteAccount;

  /// No description provided for @enterToSend.
  ///
  /// In en, this message translates to:
  /// **'Return/Enter ⏎ to Send'**
  String get enterToSend;

  /// No description provided for @enterYourPhoneNumber.
  ///
  /// In en, this message translates to:
  /// **'Enter your phone number'**
  String get enterYourPhoneNumber;

  /// No description provided for @enterYourPinToContinue.
  ///
  /// In en, this message translates to:
  /// **'Enter your PIN to continue'**
  String get enterYourPinToContinue;

  /// No description provided for @errorAccessLimited.
  ///
  /// In en, this message translates to:
  /// **'ERROR 403: Access Limited'**
  String get errorAccessLimited;

  /// No description provided for @errorAddressExists.
  ///
  /// In en, this message translates to:
  /// **'The address does not exist. Please make sure it was added successfully.'**
  String get errorAddressExists;

  /// No description provided for @errorAddressNotSync.
  ///
  /// In en, this message translates to:
  /// **'Address refresh failed, please try again'**
  String get errorAddressNotSync;

  /// No description provided for @errorAlreadyBondedReferralCode.
  ///
  /// In en, this message translates to:
  /// **'ERROR 10731: This account has already applied a referral code'**
  String get errorAlreadyBondedReferralCode;

  /// No description provided for @errorAssetExists.
  ///
  /// In en, this message translates to:
  /// **'Asset does not exist'**
  String get errorAssetExists;

  /// No description provided for @errorAuthentication.
  ///
  /// In en, this message translates to:
  /// **'ERROR 401: Sign in to continue'**
  String get errorAuthentication;

  /// No description provided for @errorBadData.
  ///
  /// In en, this message translates to:
  /// **'ERROR 10002: The request data has an invalid field'**
  String get errorBadData;

  /// No description provided for @errorBlockchain.
  ///
  /// In en, this message translates to:
  /// **'ERROR 30100: Blockchain not in sync, please try again later.'**
  String get errorBlockchain;

  /// No description provided for @errorConnectionTimeout.
  ///
  /// In en, this message translates to:
  /// **'Network connection timeout, please try again'**
  String get errorConnectionTimeout;

  /// No description provided for @errorFullGroup.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20116: The group chat is full.'**
  String get errorFullGroup;

  /// No description provided for @errorInsufficientBalance.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20117: Insufficient balance'**
  String get errorInsufficientBalance;

  /// No description provided for @errorInsufficientTransactionFeeWithAmount.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20124: Insufficient transaction fee. Please make sure your wallet has {arg0} as fee'**
  String errorInsufficientTransactionFeeWithAmount(Object arg0);

  /// No description provided for @errorInvalidAddress.
  ///
  /// In en, this message translates to:
  /// **'ERROR 30102: Invalid address format. Please enter the correct {arg0} {arg1} address!'**
  String errorInvalidAddress(Object arg0, Object arg1);

  /// No description provided for @errorInvalidAddressPlain.
  ///
  /// In en, this message translates to:
  /// **'ERROR 30102: Invalid address format.'**
  String get errorInvalidAddressPlain;

  /// No description provided for @errorInvalidCodeTooFrequent.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20129: Verification codes are being sent too frequently. Please try again later.'**
  String get errorInvalidCodeTooFrequent;

  /// No description provided for @errorInvalidEmergencyContact.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20130: Invalid recovery contact'**
  String get errorInvalidEmergencyContact;

  /// No description provided for @errorInvalidPinFormat.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20118: Invalid PIN format.'**
  String get errorInvalidPinFormat;

  /// No description provided for @errorInviterPlanExpired.
  ///
  /// In en, this message translates to:
  /// **'ERROR 10737: The inviter has no valid plan'**
  String get errorInviterPlanExpired;

  /// No description provided for @errorLegacyPin.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20118: To enhance the security of the Mixin network, Mixin API has temporarily suspended the upgrading from D3M-PIN to TIP. Please refer to the documentation for details and register for processing.'**
  String get errorLegacyPin;

  /// No description provided for @errorNetworkTaskFailed.
  ///
  /// In en, this message translates to:
  /// **'Network connection failed. Check or switch your network and try again'**
  String get errorNetworkTaskFailed;

  /// No description provided for @errorNoPinToken.
  ///
  /// In en, this message translates to:
  /// **'No token. Please sign in again and try this feature again.'**
  String get errorNoPinToken;

  /// No description provided for @errorNotFound.
  ///
  /// In en, this message translates to:
  /// **'ERROR 404: Not found'**
  String get errorNotFound;

  /// No description provided for @errorNotSupportedAudioFormat.
  ///
  /// In en, this message translates to:
  /// **'Unsupported audio format. Please open it with another app.'**
  String get errorNotSupportedAudioFormat;

  /// No description provided for @errorNumberReachedLimit.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20132: The number has reached the limit.'**
  String get errorNumberReachedLimit;

  /// No description provided for @errorOldVersion.
  ///
  /// In en, this message translates to:
  /// **'ERROR 10006: Please update Mixin ({arg0}) to continue using the service.'**
  String errorOldVersion(Object arg0);

  /// No description provided for @errorOpenLocation.
  ///
  /// In en, this message translates to:
  /// **'Can\'t find a map app'**
  String get errorOpenLocation;

  /// No description provided for @errorPermission.
  ///
  /// In en, this message translates to:
  /// **'Please open the necessary permissions'**
  String get errorPermission;

  /// No description provided for @errorPhoneInvalidFormat.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20110: Invalid phone number'**
  String get errorPhoneInvalidFormat;

  /// No description provided for @errorPhoneSmsDelivery.
  ///
  /// In en, this message translates to:
  /// **'ERROR 10003: Failed to deliver SMS'**
  String get errorPhoneSmsDelivery;

  /// No description provided for @errorPhoneVerificationCodeExpired.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20114: Expired phone verification code'**
  String get errorPhoneVerificationCodeExpired;

  /// No description provided for @errorPhoneVerificationCodeInvalid.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20113: Invalid phone verification code'**
  String get errorPhoneVerificationCodeInvalid;

  /// No description provided for @errorPinCheckTooManyRequest.
  ///
  /// In en, this message translates to:
  /// **'You have tried more than 5 times, please wait at least 24 hours to try again.'**
  String get errorPinCheckTooManyRequest;

  /// No description provided for @errorPinIncorrect.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20119: PIN incorrect'**
  String get errorPinIncorrect;

  /// No description provided for @errorPinIncorrectWithTimes.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{ERROR 20119: PIN incorrect. You still have {arg0} chance. Please wait for 24 hours to retry later.} other{ERROR 20119: PIN incorrect. You still have {arg0} chances. Please wait for 24 hours to retry later.}}'**
  String errorPinIncorrectWithTimes(Object arg0, num count);

  /// No description provided for @errorRecaptchaIsInvalid.
  ///
  /// In en, this message translates to:
  /// **'ERROR 10004: Recaptcha is invalid'**
  String get errorRecaptchaIsInvalid;

  /// No description provided for @errorServer5xxCode.
  ///
  /// In en, this message translates to:
  /// **'Server is under maintenance: {arg0}'**
  String errorServer5xxCode(Object arg0);

  /// No description provided for @errorTooManyRequest.
  ///
  /// In en, this message translates to:
  /// **'ERROR 429: Rate limit exceeded'**
  String get errorTooManyRequest;

  /// No description provided for @errorTooManyStickers.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20126: Too many stickers'**
  String get errorTooManyStickers;

  /// No description provided for @errorTooSmallTransferAmount.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20120: Transfer amount is too small'**
  String get errorTooSmallTransferAmount;

  /// No description provided for @errorTooSmallWithdrawAmount.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20127: Withdraw amount too small'**
  String get errorTooSmallWithdrawAmount;

  /// No description provided for @errorTranscriptForward.
  ///
  /// In en, this message translates to:
  /// **'Please forward all attachments after they have been downloaded'**
  String get errorTranscriptForward;

  /// No description provided for @errorTransferToDeactivatedUser.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20160: Transfers cannot be made to a deactivated user'**
  String get errorTransferToDeactivatedUser;

  /// No description provided for @errorUnableToOpenMedia.
  ///
  /// In en, this message translates to:
  /// **'Can\'t find an app that can open this media.'**
  String get errorUnableToOpenMedia;

  /// No description provided for @errorUnknownWithCode.
  ///
  /// In en, this message translates to:
  /// **'ERROR: {arg0}'**
  String errorUnknownWithCode(Object arg0);

  /// No description provided for @errorUnknownWithMessage.
  ///
  /// In en, this message translates to:
  /// **'ERROR: {arg0}'**
  String errorUnknownWithMessage(Object arg0);

  /// No description provided for @errorUploadAttachmentFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to upload message attachment'**
  String get errorUploadAttachmentFailed;

  /// No description provided for @errorUsedPhone.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20122: This phone number is already associated with another account.'**
  String get errorUsedPhone;

  /// No description provided for @errorUserInvalidFormat.
  ///
  /// In en, this message translates to:
  /// **'Invalid user id'**
  String get errorUserInvalidFormat;

  /// No description provided for @errorWithdrawalMemoFormatIncorrect.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20131: Withdrawal memo format incorrect.'**
  String get errorWithdrawalMemoFormatIncorrect;

  /// No description provided for @errorWithdrawalSuspend.
  ///
  /// In en, this message translates to:
  /// **'ERROR 20137: Withdrawals are suspended.'**
  String get errorWithdrawalSuspend;

  /// No description provided for @exit.
  ///
  /// In en, this message translates to:
  /// **'Exit'**
  String get exit;

  /// No description provided for @exitGroup.
  ///
  /// In en, this message translates to:
  /// **'Exit Group'**
  String get exitGroup;

  /// No description provided for @failed.
  ///
  /// In en, this message translates to:
  /// **'Failed'**
  String get failed;

  /// No description provided for @failedToOpenDatabase.
  ///
  /// In en, this message translates to:
  /// **'An error occurred while opening the database.'**
  String get failedToOpenDatabase;

  /// No description provided for @fee.
  ///
  /// In en, this message translates to:
  /// **'Fee'**
  String get fee;

  /// No description provided for @file.
  ///
  /// In en, this message translates to:
  /// **'File'**
  String get file;

  /// No description provided for @fileChooserError.
  ///
  /// In en, this message translates to:
  /// **'File chooser error'**
  String get fileChooserError;

  /// No description provided for @fileDoesNotExist.
  ///
  /// In en, this message translates to:
  /// **'File does not exist'**
  String get fileDoesNotExist;

  /// No description provided for @fileError.
  ///
  /// In en, this message translates to:
  /// **'File error'**
  String get fileError;

  /// No description provided for @files.
  ///
  /// In en, this message translates to:
  /// **'Files'**
  String get files;

  /// No description provided for @flags.
  ///
  /// In en, this message translates to:
  /// **'Flags'**
  String get flags;

  /// No description provided for @followSystem.
  ///
  /// In en, this message translates to:
  /// **'Follow System'**
  String get followSystem;

  /// No description provided for @followUsOnFacebook.
  ///
  /// In en, this message translates to:
  /// **'Follow us on Facebook'**
  String get followUsOnFacebook;

  /// No description provided for @followUsOnX.
  ///
  /// In en, this message translates to:
  /// **'Follow us on X'**
  String get followUsOnX;

  /// No description provided for @foodAndDrink.
  ///
  /// In en, this message translates to:
  /// **'Food & Drink'**
  String get foodAndDrink;

  /// No description provided for @formatNotSupported.
  ///
  /// In en, this message translates to:
  /// **'Format not supported'**
  String get formatNotSupported;

  /// No description provided for @forward.
  ///
  /// In en, this message translates to:
  /// **'Forward'**
  String get forward;

  /// No description provided for @from.
  ///
  /// In en, this message translates to:
  /// **'From'**
  String get from;

  /// No description provided for @fromWithColon.
  ///
  /// In en, this message translates to:
  /// **'From:'**
  String get fromWithColon;

  /// No description provided for @generateQrcode.
  ///
  /// In en, this message translates to:
  /// **'Generate QR Code'**
  String get generateQrcode;

  /// No description provided for @groupAlreadyIn.
  ///
  /// In en, this message translates to:
  /// **'You are already in the group.'**
  String get groupAlreadyIn;

  /// No description provided for @groupCantSend.
  ///
  /// In en, this message translates to:
  /// **'You can\'t send messages to this group because you\'re no longer a participant.'**
  String get groupCantSend;

  /// No description provided for @groupName.
  ///
  /// In en, this message translates to:
  /// **'Group Name'**
  String get groupName;

  /// No description provided for @groupParticipants.
  ///
  /// In en, this message translates to:
  /// **'Participants'**
  String get groupParticipants;

  /// No description provided for @groupPopMenuMessage.
  ///
  /// In en, this message translates to:
  /// **'Message {arg0}'**
  String groupPopMenuMessage(Object arg0);

  /// No description provided for @groupPopMenuRemove.
  ///
  /// In en, this message translates to:
  /// **'Remove {arg0}'**
  String groupPopMenuRemove(Object arg0);

  /// No description provided for @groups.
  ///
  /// In en, this message translates to:
  /// **'Groups'**
  String get groups;

  /// No description provided for @groupsInCommon.
  ///
  /// In en, this message translates to:
  /// **'Groups in Common'**
  String get groupsInCommon;

  /// No description provided for @hash.
  ///
  /// In en, this message translates to:
  /// **'HASH'**
  String get hash;

  /// No description provided for @help.
  ///
  /// In en, this message translates to:
  /// **'Help'**
  String get help;

  /// No description provided for @helpCenter.
  ///
  /// In en, this message translates to:
  /// **'Help Center'**
  String get helpCenter;

  /// No description provided for @hideMixin.
  ///
  /// In en, this message translates to:
  /// **'Hide Mixin'**
  String get hideMixin;

  /// No description provided for @host.
  ///
  /// In en, this message translates to:
  /// **'Host'**
  String get host;

  /// No description provided for @hour.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{{arg0} Hour} other{{arg0} Hours}}'**
  String hour(Object arg0, num count);

  /// No description provided for @howAreYou.
  ///
  /// In en, this message translates to:
  /// **'Hi, how are you?'**
  String get howAreYou;

  /// No description provided for @iAmGood.
  ///
  /// In en, this message translates to:
  /// **'I’m good.'**
  String get iAmGood;

  /// No description provided for @id.
  ///
  /// In en, this message translates to:
  /// **'ID'**
  String get id;

  /// No description provided for @ignoreThisVersion.
  ///
  /// In en, this message translates to:
  /// **'Ignore the new version'**
  String get ignoreThisVersion;

  /// No description provided for @image.
  ///
  /// In en, this message translates to:
  /// **'Image'**
  String get image;

  /// No description provided for @includeFiles.
  ///
  /// In en, this message translates to:
  /// **'Include Files'**
  String get includeFiles;

  /// No description provided for @includeVideos.
  ///
  /// In en, this message translates to:
  /// **'Include Videos'**
  String get includeVideos;

  /// No description provided for @initializing.
  ///
  /// In en, this message translates to:
  /// **'Initializing…'**
  String get initializing;

  /// No description provided for @invalidStickerFormat.
  ///
  /// In en, this message translates to:
  /// **'Invalid sticker format'**
  String get invalidStickerFormat;

  /// No description provided for @inviteInfo.
  ///
  /// In en, this message translates to:
  /// **'Anyone with Mixin can follow this link to join this group. Only share it with people you trust.'**
  String get inviteInfo;

  /// No description provided for @inviteToGroupViaLink.
  ///
  /// In en, this message translates to:
  /// **'Invite to Group via Link'**
  String get inviteToGroupViaLink;

  /// No description provided for @joinGroupWithPlus.
  ///
  /// In en, this message translates to:
  /// **'+ Join group'**
  String get joinGroupWithPlus;

  /// No description provided for @joinedIn.
  ///
  /// In en, this message translates to:
  /// **'Joined on {arg0}'**
  String joinedIn(Object arg0);

  /// No description provided for @landingDeleteContent.
  ///
  /// In en, this message translates to:
  /// **'You requested to delete your account on {arg0}. The account will be deleted on {arg1}. If you continue to log in, your account deletion will be cancelled.'**
  String landingDeleteContent(Object arg0, Object arg1);

  /// No description provided for @landingInvitationDialogContent.
  ///
  /// In en, this message translates to:
  /// **'We will send a 4-digit code to your phone number {arg0}. Please enter the code on the next screen.'**
  String landingInvitationDialogContent(Object arg0);

  /// No description provided for @landingValidationTitle.
  ///
  /// In en, this message translates to:
  /// **'Enter the 4-digit code sent to you at {arg0}'**
  String landingValidationTitle(Object arg0);

  /// No description provided for @learnMore.
  ///
  /// In en, this message translates to:
  /// **'Learn More'**
  String get learnMore;

  /// No description provided for @less.
  ///
  /// In en, this message translates to:
  /// **'less'**
  String get less;

  /// No description provided for @light.
  ///
  /// In en, this message translates to:
  /// **'Light'**
  String get light;

  /// No description provided for @linkedDevice.
  ///
  /// In en, this message translates to:
  /// **'linked device'**
  String get linkedDevice;

  /// No description provided for @live.
  ///
  /// In en, this message translates to:
  /// **'Live'**
  String get live;

  /// No description provided for @loading.
  ///
  /// In en, this message translates to:
  /// **'Loading...'**
  String get loading;

  /// No description provided for @loadingTime.
  ///
  /// In en, this message translates to:
  /// **'The system time appears incorrect. Please correct it and try again.'**
  String get loadingTime;

  /// No description provided for @locateToChat.
  ///
  /// In en, this message translates to:
  /// **'Locate in Chat'**
  String get locateToChat;

  /// No description provided for @location.
  ///
  /// In en, this message translates to:
  /// **'Location'**
  String get location;

  /// No description provided for @lock.
  ///
  /// In en, this message translates to:
  /// **'Lock'**
  String get lock;

  /// No description provided for @logIn.
  ///
  /// In en, this message translates to:
  /// **'Log in'**
  String get logIn;

  /// No description provided for @loginAndAbortAccountDeletion.
  ///
  /// In en, this message translates to:
  /// **'Continue to log in and abort account deletion'**
  String get loginAndAbortAccountDeletion;

  /// No description provided for @loginByQrcode.
  ///
  /// In en, this message translates to:
  /// **'Log in to Mixin Messenger with a QR code'**
  String get loginByQrcode;

  /// No description provided for @loginByQrcodeTips1.
  ///
  /// In en, this message translates to:
  /// **'Open Mixin Messenger on your phone.'**
  String get loginByQrcodeTips1;

  /// No description provided for @loginByQrcodeTips2.
  ///
  /// In en, this message translates to:
  /// **'Scan the QR code on the screen and confirm your sign-in.'**
  String get loginByQrcodeTips2;

  /// No description provided for @makeGroupAdmin.
  ///
  /// In en, this message translates to:
  /// **'Make group admin'**
  String get makeGroupAdmin;

  /// No description provided for @media.
  ///
  /// In en, this message translates to:
  /// **'Media'**
  String get media;

  /// No description provided for @memo.
  ///
  /// In en, this message translates to:
  /// **'Memo'**
  String get memo;

  /// No description provided for @messageE2ee.
  ///
  /// In en, this message translates to:
  /// **'Messages in this conversation are end-to-end encrypted. Tap for more info.'**
  String get messageE2ee;

  /// No description provided for @messageNotFound.
  ///
  /// In en, this message translates to:
  /// **'Message not found'**
  String get messageNotFound;

  /// No description provided for @messageNotSupport.
  ///
  /// In en, this message translates to:
  /// **'This type of message is not supported, please upgrade Mixin to the latest version.'**
  String get messageNotSupport;

  /// No description provided for @messagePreview.
  ///
  /// In en, this message translates to:
  /// **'Message Preview'**
  String get messagePreview;

  /// No description provided for @messagePreviewDescription.
  ///
  /// In en, this message translates to:
  /// **'Preview message text inside new message notifications.'**
  String get messagePreviewDescription;

  /// No description provided for @messages.
  ///
  /// In en, this message translates to:
  /// **'Messages'**
  String get messages;

  /// No description provided for @minimize.
  ///
  /// In en, this message translates to:
  /// **'Minimize'**
  String get minimize;

  /// No description provided for @minute.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{{arg0} Minute} other{{arg0} Minutes}}'**
  String minute(Object arg0, num count);

  /// No description provided for @mixinMessengerDesktop.
  ///
  /// In en, this message translates to:
  /// **'Mixin Messenger Desktop'**
  String get mixinMessengerDesktop;

  /// No description provided for @more.
  ///
  /// In en, this message translates to:
  /// **'More'**
  String get more;

  /// No description provided for @multisigTransaction.
  ///
  /// In en, this message translates to:
  /// **'Multisig Transaction'**
  String get multisigTransaction;

  /// No description provided for @mute.
  ///
  /// In en, this message translates to:
  /// **'Mute'**
  String get mute;

  /// No description provided for @myMixinId.
  ///
  /// In en, this message translates to:
  /// **'My Mixin ID: {arg0}'**
  String myMixinId(Object arg0);

  /// No description provided for @myStickers.
  ///
  /// In en, this message translates to:
  /// **'My Stickers'**
  String get myStickers;

  /// No description provided for @na.
  ///
  /// In en, this message translates to:
  /// **'N/A'**
  String get na;

  /// No description provided for @name.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get name;

  /// No description provided for @networkConnectionFailed.
  ///
  /// In en, this message translates to:
  /// **'Network connection failed'**
  String get networkConnectionFailed;

  /// No description provided for @networkError.
  ///
  /// In en, this message translates to:
  /// **'Network error'**
  String get networkError;

  /// No description provided for @newVersionAvailable.
  ///
  /// In en, this message translates to:
  /// **'New version available'**
  String get newVersionAvailable;

  /// No description provided for @newVersionDescription.
  ///
  /// In en, this message translates to:
  /// **'Mixin Messenger {arg0} is now available, you have {arg1}. Would you like to download it now?'**
  String newVersionDescription(Object arg0, Object arg1);

  /// No description provided for @next.
  ///
  /// In en, this message translates to:
  /// **'Next'**
  String get next;

  /// No description provided for @nextConversation.
  ///
  /// In en, this message translates to:
  /// **'Next conversation'**
  String get nextConversation;

  /// No description provided for @noAudio.
  ///
  /// In en, this message translates to:
  /// **'NO AUDIO'**
  String get noAudio;

  /// No description provided for @noCamera.
  ///
  /// In en, this message translates to:
  /// **'No camera'**
  String get noCamera;

  /// No description provided for @noData.
  ///
  /// In en, this message translates to:
  /// **'No Data'**
  String get noData;

  /// No description provided for @noFiles.
  ///
  /// In en, this message translates to:
  /// **'NO FILES'**
  String get noFiles;

  /// No description provided for @noLinks.
  ///
  /// In en, this message translates to:
  /// **'NO LINKS'**
  String get noLinks;

  /// No description provided for @noMedia.
  ///
  /// In en, this message translates to:
  /// **'NO MEDIA'**
  String get noMedia;

  /// No description provided for @noNetworkConnection.
  ///
  /// In en, this message translates to:
  /// **'No network connection'**
  String get noNetworkConnection;

  /// No description provided for @noPosts.
  ///
  /// In en, this message translates to:
  /// **'NO POSTS'**
  String get noPosts;

  /// No description provided for @noResults.
  ///
  /// In en, this message translates to:
  /// **'NO RESULTS'**
  String get noResults;

  /// No description provided for @notFound.
  ///
  /// In en, this message translates to:
  /// **'Not found'**
  String get notFound;

  /// No description provided for @notSupportBiometric.
  ///
  /// In en, this message translates to:
  /// **'This device does not support biometric authentication'**
  String get notSupportBiometric;

  /// No description provided for @notificationContent.
  ///
  /// In en, this message translates to:
  /// **'Enable push notifications to stay updated on price alerts and messages in real time.'**
  String get notificationContent;

  /// No description provided for @notificationPermissionManually.
  ///
  /// In en, this message translates to:
  /// **'Notifications are disabled. Please go to Notification Settings to turn them on.'**
  String get notificationPermissionManually;

  /// No description provided for @notifications.
  ///
  /// In en, this message translates to:
  /// **'Notifications'**
  String get notifications;

  /// No description provided for @nowAnAddmin.
  ///
  /// In en, this message translates to:
  /// **'{arg0} now an admin'**
  String nowAnAddmin(Object arg0);

  /// No description provided for @objects.
  ///
  /// In en, this message translates to:
  /// **'Objects'**
  String get objects;

  /// No description provided for @oneByOneForward.
  ///
  /// In en, this message translates to:
  /// **'One-by-One Forward'**
  String get oneByOneForward;

  /// No description provided for @oneHour.
  ///
  /// In en, this message translates to:
  /// **'1 Hour'**
  String get oneHour;

  /// No description provided for @oneYear.
  ///
  /// In en, this message translates to:
  /// **'1 Year'**
  String get oneYear;

  /// No description provided for @open.
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get open;

  /// No description provided for @openHomePage.
  ///
  /// In en, this message translates to:
  /// **'Open Homepage'**
  String get openHomePage;

  /// No description provided for @openLink.
  ///
  /// In en, this message translates to:
  /// **'Open Link: {arg0}'**
  String openLink(Object arg0);

  /// No description provided for @openLogDirectory.
  ///
  /// In en, this message translates to:
  /// **'Open Log Directory'**
  String get openLogDirectory;

  /// No description provided for @openingBalance.
  ///
  /// In en, this message translates to:
  /// **'Opening Balance'**
  String get openingBalance;

  /// No description provided for @originalImage.
  ///
  /// In en, this message translates to:
  /// **'Original'**
  String get originalImage;

  /// No description provided for @owner.
  ///
  /// In en, this message translates to:
  /// **'Owner'**
  String get owner;

  /// No description provided for @participantsCount.
  ///
  /// In en, this message translates to:
  /// **'{arg0} PARTICIPANTS'**
  String participantsCount(Object arg0);

  /// No description provided for @passcodeIncorrect.
  ///
  /// In en, this message translates to:
  /// **'Passcode incorrect'**
  String get passcodeIncorrect;

  /// No description provided for @password.
  ///
  /// In en, this message translates to:
  /// **'Password'**
  String get password;

  /// No description provided for @pendingConfirmation.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{{arg0}/{arg1} confirmation} other{{arg0}/{arg1} confirmations}}'**
  String pendingConfirmation(Object arg0, Object arg1, num count);

  /// No description provided for @phoneNumber.
  ///
  /// In en, this message translates to:
  /// **'Phone Number'**
  String get phoneNumber;

  /// No description provided for @photos.
  ///
  /// In en, this message translates to:
  /// **'Photos'**
  String get photos;

  /// No description provided for @pickAConversation.
  ///
  /// In en, this message translates to:
  /// **'Select a conversation and start sending a message'**
  String get pickAConversation;

  /// No description provided for @picturesAndVideos.
  ///
  /// In en, this message translates to:
  /// **'Pictures & Videos'**
  String get picturesAndVideos;

  /// No description provided for @pinTitle.
  ///
  /// In en, this message translates to:
  /// **'Pin'**
  String get pinTitle;

  /// No description provided for @pinnedMessageTitle.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{{arg0} Pinned Message} other{{arg0} Pinned Messages}}'**
  String pinnedMessageTitle(Object arg0, num count);

  /// No description provided for @port.
  ///
  /// In en, this message translates to:
  /// **'Port'**
  String get port;

  /// No description provided for @post.
  ///
  /// In en, this message translates to:
  /// **'Post'**
  String get post;

  /// No description provided for @preferences.
  ///
  /// In en, this message translates to:
  /// **'Preferences'**
  String get preferences;

  /// No description provided for @previousConversation.
  ///
  /// In en, this message translates to:
  /// **'Previous conversation'**
  String get previousConversation;

  /// No description provided for @privacyPolicy.
  ///
  /// In en, this message translates to:
  /// **'Privacy Policy'**
  String get privacyPolicy;

  /// No description provided for @proxy.
  ///
  /// In en, this message translates to:
  /// **'Proxy'**
  String get proxy;

  /// No description provided for @proxyAuth.
  ///
  /// In en, this message translates to:
  /// **'Authentication (Optional)'**
  String get proxyAuth;

  /// No description provided for @proxyConnection.
  ///
  /// In en, this message translates to:
  /// **'Connection'**
  String get proxyConnection;

  /// No description provided for @proxyType.
  ///
  /// In en, this message translates to:
  /// **'Proxy Type'**
  String get proxyType;

  /// No description provided for @qrCodeExpiredDesc.
  ///
  /// In en, this message translates to:
  /// **'QR code expired. Please try again.'**
  String get qrCodeExpiredDesc;

  /// No description provided for @quickSearch.
  ///
  /// In en, this message translates to:
  /// **'Quick Search'**
  String get quickSearch;

  /// No description provided for @quitMixin.
  ///
  /// In en, this message translates to:
  /// **'Quit Mixin'**
  String get quitMixin;

  /// No description provided for @raw.
  ///
  /// In en, this message translates to:
  /// **'Raw'**
  String get raw;

  /// No description provided for @rebate.
  ///
  /// In en, this message translates to:
  /// **'Rebate'**
  String get rebate;

  /// No description provided for @recaptchaTimeout.
  ///
  /// In en, this message translates to:
  /// **'Recaptcha timeout'**
  String get recaptchaTimeout;

  /// No description provided for @receiver.
  ///
  /// In en, this message translates to:
  /// **'Receiver'**
  String get receiver;

  /// No description provided for @recentChats.
  ///
  /// In en, this message translates to:
  /// **'CHATS'**
  String get recentChats;

  /// No description provided for @reedit.
  ///
  /// In en, this message translates to:
  /// **'Re-edit'**
  String get reedit;

  /// No description provided for @refresh.
  ///
  /// In en, this message translates to:
  /// **'Refresh'**
  String get refresh;

  /// No description provided for @removeBot.
  ///
  /// In en, this message translates to:
  /// **'Remove Bot'**
  String get removeBot;

  /// No description provided for @removeChatFromCircle.
  ///
  /// In en, this message translates to:
  /// **'Remove Chat from circle'**
  String get removeChatFromCircle;

  /// No description provided for @removeContact.
  ///
  /// In en, this message translates to:
  /// **'Remove Contact'**
  String get removeContact;

  /// No description provided for @removeStickers.
  ///
  /// In en, this message translates to:
  /// **'Remove Stickers'**
  String get removeStickers;

  /// No description provided for @reply.
  ///
  /// In en, this message translates to:
  /// **'Reply'**
  String get reply;

  /// No description provided for @report.
  ///
  /// In en, this message translates to:
  /// **'Report'**
  String get report;

  /// No description provided for @reportAndBlock.
  ///
  /// In en, this message translates to:
  /// **'Report and block?'**
  String get reportAndBlock;

  /// No description provided for @reportTitle.
  ///
  /// In en, this message translates to:
  /// **'Send the conversation log to developers?'**
  String get reportTitle;

  /// No description provided for @resendCode.
  ///
  /// In en, this message translates to:
  /// **'Resend code'**
  String get resendCode;

  /// No description provided for @resendCodeIn.
  ///
  /// In en, this message translates to:
  /// **'Resend code in {arg0} s'**
  String resendCodeIn(Object arg0);

  /// No description provided for @reset.
  ///
  /// In en, this message translates to:
  /// **'Reset'**
  String get reset;

  /// No description provided for @resetLink.
  ///
  /// In en, this message translates to:
  /// **'Reset Link'**
  String get resetLink;

  /// No description provided for @restoreChat.
  ///
  /// In en, this message translates to:
  /// **'Restore Chat'**
  String get restoreChat;

  /// No description provided for @restoreChatTip.
  ///
  /// In en, this message translates to:
  /// **'Restore your chat history from another device. Make sure both devices are connected to the same Wi-Fi or hotspot.'**
  String get restoreChatTip;

  /// No description provided for @restoreFromOtherDevice.
  ///
  /// In en, this message translates to:
  /// **'Restore from Other Device'**
  String get restoreFromOtherDevice;

  /// No description provided for @retry.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get retry;

  /// No description provided for @retryUploadFailed.
  ///
  /// In en, this message translates to:
  /// **'Retry upload failed.'**
  String get retryUploadFailed;

  /// No description provided for @revokeMultisigTransaction.
  ///
  /// In en, this message translates to:
  /// **'Revoke Multisig Transaction'**
  String get revokeMultisigTransaction;

  /// No description provided for @save.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get save;

  /// No description provided for @saveAs.
  ///
  /// In en, this message translates to:
  /// **'Save as'**
  String get saveAs;

  /// No description provided for @saveToCameraRoll.
  ///
  /// In en, this message translates to:
  /// **'Save to Camera Roll'**
  String get saveToCameraRoll;

  /// No description provided for @sayHi.
  ///
  /// In en, this message translates to:
  /// **'Say Hi'**
  String get sayHi;

  /// No description provided for @scamWarning.
  ///
  /// In en, this message translates to:
  /// **'Warning: Many users reported this account as a scam. Please be careful, especially if it asks you for money'**
  String get scamWarning;

  /// No description provided for @screenPasscode.
  ///
  /// In en, this message translates to:
  /// **'Screen Passcode'**
  String get screenPasscode;

  /// No description provided for @search.
  ///
  /// In en, this message translates to:
  /// **'Search'**
  String get search;

  /// No description provided for @searchContact.
  ///
  /// In en, this message translates to:
  /// **'Search Contacts'**
  String get searchContact;

  /// No description provided for @searchConversation.
  ///
  /// In en, this message translates to:
  /// **'Search Conversation'**
  String get searchConversation;

  /// No description provided for @searchEmpty.
  ///
  /// In en, this message translates to:
  /// **'No chats, contacts or messages found.'**
  String get searchEmpty;

  /// No description provided for @searchPlaceholderNumber.
  ///
  /// In en, this message translates to:
  /// **'Search Mixin ID or phone number:'**
  String get searchPlaceholderNumber;

  /// No description provided for @searchRelatedMessage.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{{arg0} related message} other{{arg0} related messages}}'**
  String searchRelatedMessage(Object arg0, num count);

  /// No description provided for @searchUnread.
  ///
  /// In en, this message translates to:
  /// **'Search Unread'**
  String get searchUnread;

  /// No description provided for @secretUrl.
  ///
  /// In en, this message translates to:
  /// **'https://mixin.one/pages/1000007'**
  String get secretUrl;

  /// No description provided for @security.
  ///
  /// In en, this message translates to:
  /// **'Security'**
  String get security;

  /// No description provided for @select.
  ///
  /// In en, this message translates to:
  /// **'Select'**
  String get select;

  /// No description provided for @send.
  ///
  /// In en, this message translates to:
  /// **'Send'**
  String get send;

  /// No description provided for @sendArchived.
  ///
  /// In en, this message translates to:
  /// **'Send as ZIP'**
  String get sendArchived;

  /// No description provided for @sendQuickly.
  ///
  /// In en, this message translates to:
  /// **'Send quickly'**
  String get sendQuickly;

  /// No description provided for @sendToDeveloper.
  ///
  /// In en, this message translates to:
  /// **'Send to Developer'**
  String get sendToDeveloper;

  /// No description provided for @sendWithoutCompression.
  ///
  /// In en, this message translates to:
  /// **'Send without compression'**
  String get sendWithoutCompression;

  /// No description provided for @sendWithoutSound.
  ///
  /// In en, this message translates to:
  /// **'Send Without Sound'**
  String get sendWithoutSound;

  /// No description provided for @set.
  ///
  /// In en, this message translates to:
  /// **'Set'**
  String get set;

  /// No description provided for @setDisappearingMessageTimeTo.
  ///
  /// In en, this message translates to:
  /// **'{arg0} set disappearing message time to {arg1}'**
  String setDisappearingMessageTimeTo(Object arg0, Object arg1);

  /// No description provided for @setPasscodeDesc.
  ///
  /// In en, this message translates to:
  /// **'Set a passcode to unlock Mixin Messenger'**
  String get setPasscodeDesc;

  /// No description provided for @settingAuthSearchHint.
  ///
  /// In en, this message translates to:
  /// **'Mixin ID, Name'**
  String get settingAuthSearchHint;

  /// No description provided for @settingBackupTips.
  ///
  /// In en, this message translates to:
  /// **'Back up your chat history to iCloud. If you lose your iPhone or switch to a new one, you can restore your chat history when you reinstall Mixin Messenger. Messages you back up are not protected by Mixin Messenger end-to-end encryption while in iCloud.'**
  String get settingBackupTips;

  /// No description provided for @settingDeleteAccountPinContent.
  ///
  /// In en, this message translates to:
  /// **'If you continue, your profile and account details will be deleted on {arg0}. Read our document to **learn more**.'**
  String settingDeleteAccountPinContent(Object arg0);

  /// No description provided for @settingDeleteAccountUrl.
  ///
  /// In en, this message translates to:
  /// **'https://support.mixin.one/en/article/how-to-delete-my-account-19fkagl'**
  String get settingDeleteAccountUrl;

  /// No description provided for @share.
  ///
  /// In en, this message translates to:
  /// **'Share'**
  String get share;

  /// No description provided for @shareApps.
  ///
  /// In en, this message translates to:
  /// **'Shared Apps'**
  String get shareApps;

  /// No description provided for @shareContact.
  ///
  /// In en, this message translates to:
  /// **'Share Contact'**
  String get shareContact;

  /// No description provided for @shareError.
  ///
  /// In en, this message translates to:
  /// **'Share error.'**
  String get shareError;

  /// No description provided for @shareLink.
  ///
  /// In en, this message translates to:
  /// **'Share Link'**
  String get shareLink;

  /// No description provided for @shareMessageDescription.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to send {arg0} from {arg1}?'**
  String shareMessageDescription(Object arg0, Object arg1);

  /// No description provided for @shareMessageDescriptionEmpty.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to send the {arg0}?'**
  String shareMessageDescriptionEmpty(Object arg0);

  /// No description provided for @sharedMedia.
  ///
  /// In en, this message translates to:
  /// **'Shared Media'**
  String get sharedMedia;

  /// No description provided for @show.
  ///
  /// In en, this message translates to:
  /// **'Show'**
  String get show;

  /// No description provided for @showAvatar.
  ///
  /// In en, this message translates to:
  /// **'Show avatar'**
  String get showAvatar;

  /// No description provided for @showIdentityNumber.
  ///
  /// In en, this message translates to:
  /// **'Show Identity Number'**
  String get showIdentityNumber;

  /// No description provided for @showMixin.
  ///
  /// In en, this message translates to:
  /// **'Show Mixin'**
  String get showMixin;

  /// No description provided for @signIn.
  ///
  /// In en, this message translates to:
  /// **'Sign in'**
  String get signIn;

  /// No description provided for @signOut.
  ///
  /// In en, this message translates to:
  /// **'Sign Out'**
  String get signOut;

  /// No description provided for @signWithMobileNumber.
  ///
  /// In en, this message translates to:
  /// **'Sign in with mobile number'**
  String get signWithMobileNumber;

  /// No description provided for @signWithQrcode.
  ///
  /// In en, this message translates to:
  /// **'Sign in with QR code'**
  String get signWithQrcode;

  /// No description provided for @smileysAndPeople.
  ///
  /// In en, this message translates to:
  /// **'Smileys & People'**
  String get smileysAndPeople;

  /// No description provided for @snapshotHash.
  ///
  /// In en, this message translates to:
  /// **'Snapshot Hash'**
  String get snapshotHash;

  /// No description provided for @status.
  ///
  /// In en, this message translates to:
  /// **'Status'**
  String get status;

  /// No description provided for @sticker.
  ///
  /// In en, this message translates to:
  /// **'Sticker'**
  String get sticker;

  /// No description provided for @stickerAddInvalidSize.
  ///
  /// In en, this message translates to:
  /// **'Sticker files must be larger than 1 KB and smaller than 1 MB, with width and height between 128 px and 1024 px.'**
  String get stickerAddInvalidSize;

  /// No description provided for @stickerAlbumDetail.
  ///
  /// In en, this message translates to:
  /// **'Sticker Album Details'**
  String get stickerAlbumDetail;

  /// No description provided for @stickerStore.
  ///
  /// In en, this message translates to:
  /// **'Sticker Store'**
  String get stickerStore;

  /// No description provided for @storageAutoDownloadDescription.
  ///
  /// In en, this message translates to:
  /// **'Change auto-download settings for media.'**
  String get storageAutoDownloadDescription;

  /// No description provided for @storageUsage.
  ///
  /// In en, this message translates to:
  /// **'Storage Usage'**
  String get storageUsage;

  /// No description provided for @strangerHint.
  ///
  /// In en, this message translates to:
  /// **'This sender is not in your contacts'**
  String get strangerHint;

  /// No description provided for @strangers.
  ///
  /// In en, this message translates to:
  /// **'Strangers'**
  String get strangers;

  /// No description provided for @successful.
  ///
  /// In en, this message translates to:
  /// **'Successful'**
  String get successful;

  /// No description provided for @symbols.
  ///
  /// In en, this message translates to:
  /// **'Symbols'**
  String get symbols;

  /// No description provided for @syncFromOtherDevice.
  ///
  /// In en, this message translates to:
  /// **'Sync from Other Device'**
  String get syncFromOtherDevice;

  /// No description provided for @syncToOtherDevice.
  ///
  /// In en, this message translates to:
  /// **'Sync to Other Device'**
  String get syncToOtherDevice;

  /// No description provided for @termsOfService.
  ///
  /// In en, this message translates to:
  /// **'Terms of Service'**
  String get termsOfService;

  /// No description provided for @text.
  ///
  /// In en, this message translates to:
  /// **'Text'**
  String get text;

  /// No description provided for @theme.
  ///
  /// In en, this message translates to:
  /// **'Theme'**
  String get theme;

  /// No description provided for @thisMessageWasDeleted.
  ///
  /// In en, this message translates to:
  /// **'This message was deleted'**
  String get thisMessageWasDeleted;

  /// No description provided for @time.
  ///
  /// In en, this message translates to:
  /// **'Time'**
  String get time;

  /// No description provided for @to.
  ///
  /// In en, this message translates to:
  /// **'To'**
  String get to;

  /// No description provided for @today.
  ///
  /// In en, this message translates to:
  /// **'Today'**
  String get today;

  /// No description provided for @toggleChatInfo.
  ///
  /// In en, this message translates to:
  /// **'Toggle chat info'**
  String get toggleChatInfo;

  /// No description provided for @trace.
  ///
  /// In en, this message translates to:
  /// **'Trace'**
  String get trace;

  /// No description provided for @transactionHash.
  ///
  /// In en, this message translates to:
  /// **'Transaction Hash'**
  String get transactionHash;

  /// No description provided for @transactionId.
  ///
  /// In en, this message translates to:
  /// **'Transaction Id'**
  String get transactionId;

  /// No description provided for @transactionType.
  ///
  /// In en, this message translates to:
  /// **'Transaction Type'**
  String get transactionType;

  /// No description provided for @transactions.
  ///
  /// In en, this message translates to:
  /// **'Transactions'**
  String get transactions;

  /// No description provided for @transactionsCannotBeDeleted.
  ///
  /// In en, this message translates to:
  /// **'Transactions cannot be deleted'**
  String get transactionsCannotBeDeleted;

  /// No description provided for @transcript.
  ///
  /// In en, this message translates to:
  /// **'Transcript'**
  String get transcript;

  /// No description provided for @transfer.
  ///
  /// In en, this message translates to:
  /// **'Transfer'**
  String get transfer;

  /// No description provided for @transferCompleted.
  ///
  /// In en, this message translates to:
  /// **'Transfer completed'**
  String get transferCompleted;

  /// No description provided for @transferProtocolVersionNotMatched.
  ///
  /// In en, this message translates to:
  /// **'Protocol version does not match, transfer failed. Please upgrade the application first.'**
  String get transferProtocolVersionNotMatched;

  /// No description provided for @transferringChats.
  ///
  /// In en, this message translates to:
  /// **'Transferring Chats'**
  String get transferringChats;

  /// No description provided for @transferringChatsTips.
  ///
  /// In en, this message translates to:
  /// **'Please do not turn off the screen and keep the Mixin running in the foreground while syncing.'**
  String get transferringChatsTips;

  /// No description provided for @travelAndPlaces.
  ///
  /// In en, this message translates to:
  /// **'Travel & Places'**
  String get travelAndPlaces;

  /// No description provided for @typeMessage.
  ///
  /// In en, this message translates to:
  /// **'Type message'**
  String get typeMessage;

  /// No description provided for @unableToOpenFile.
  ///
  /// In en, this message translates to:
  /// **'Unable to open file: {arg0}'**
  String unableToOpenFile(Object arg0);

  /// No description provided for @unblock.
  ///
  /// In en, this message translates to:
  /// **'Unblock'**
  String get unblock;

  /// No description provided for @unitDay.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{day} other{days}}'**
  String unitDay(num count);

  /// No description provided for @unitHour.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{hour} other{hours}}'**
  String unitHour(num count);

  /// No description provided for @unitMinute.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{minute} other{minutes}}'**
  String unitMinute(num count);

  /// No description provided for @unitSecond.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{second} other{seconds}}'**
  String unitSecond(num count);

  /// No description provided for @unitWeek.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{week} other{weeks}}'**
  String unitWeek(num count);

  /// No description provided for @unknowError.
  ///
  /// In en, this message translates to:
  /// **'Unknow error'**
  String get unknowError;

  /// No description provided for @unlockMixinMessenger.
  ///
  /// In en, this message translates to:
  /// **'Unlock Mixin Messenger'**
  String get unlockMixinMessenger;

  /// No description provided for @unlockWithWasscode.
  ///
  /// In en, this message translates to:
  /// **'Enter Passcode to unlock Mixin Messenger'**
  String get unlockWithWasscode;

  /// No description provided for @unmute.
  ///
  /// In en, this message translates to:
  /// **'Unmute'**
  String get unmute;

  /// No description provided for @unpin.
  ///
  /// In en, this message translates to:
  /// **'Unpin'**
  String get unpin;

  /// No description provided for @unpinAllMessages.
  ///
  /// In en, this message translates to:
  /// **'Unpin All Messages'**
  String get unpinAllMessages;

  /// No description provided for @unpinAllMessagesConfirmation.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to unpin all messages?'**
  String get unpinAllMessagesConfirmation;

  /// No description provided for @unreadMessages.
  ///
  /// In en, this message translates to:
  /// **'Unread messages'**
  String get unreadMessages;

  /// No description provided for @updateMixin.
  ///
  /// In en, this message translates to:
  /// **'Update Mixin'**
  String get updateMixin;

  /// No description provided for @updateMixinDescription.
  ///
  /// In en, this message translates to:
  /// **'The current version ({arg0}) is no longer available!\nPlease click \"Update\" below to update to the latest version.'**
  String updateMixinDescription(Object arg0);

  /// No description provided for @upgrade.
  ///
  /// In en, this message translates to:
  /// **'Upgrade'**
  String get upgrade;

  /// No description provided for @upgrading.
  ///
  /// In en, this message translates to:
  /// **'Upgrading'**
  String get upgrading;

  /// No description provided for @useBiometric.
  ///
  /// In en, this message translates to:
  /// **'Use Biometric'**
  String get useBiometric;

  /// No description provided for @userDeleteHint.
  ///
  /// In en, this message translates to:
  /// **'This user has deleted their account.'**
  String get userDeleteHint;

  /// No description provided for @userNotFound.
  ///
  /// In en, this message translates to:
  /// **'User not found'**
  String get userNotFound;

  /// No description provided for @username.
  ///
  /// In en, this message translates to:
  /// **'Username'**
  String get username;

  /// No description provided for @valueNow.
  ///
  /// In en, this message translates to:
  /// **'value now {arg0}'**
  String valueNow(Object arg0);

  /// No description provided for @valueThen.
  ///
  /// In en, this message translates to:
  /// **'value then {arg0}'**
  String valueThen(Object arg0);

  /// No description provided for @verifyPin.
  ///
  /// In en, this message translates to:
  /// **'Verify PIN'**
  String get verifyPin;

  /// No description provided for @video.
  ///
  /// In en, this message translates to:
  /// **'Video'**
  String get video;

  /// No description provided for @videos.
  ///
  /// In en, this message translates to:
  /// **'Videos'**
  String get videos;

  /// No description provided for @waitingForThisMessage.
  ///
  /// In en, this message translates to:
  /// **'Waiting for this message.'**
  String get waitingForThisMessage;

  /// No description provided for @waitingOtherDeviceConnection.
  ///
  /// In en, this message translates to:
  /// **'Waiting for the other device to connect.'**
  String get waitingOtherDeviceConnection;

  /// No description provided for @webview2RuntimeInstallDescription.
  ///
  /// In en, this message translates to:
  /// **'The device has not installed the WebView2 Runtime component. Please download and install WebView2 Runtime first.'**
  String get webview2RuntimeInstallDescription;

  /// No description provided for @webviewRuntimeUnavailable.
  ///
  /// In en, this message translates to:
  /// **'WebView runtime is unavailable'**
  String get webviewRuntimeUnavailable;

  /// No description provided for @window.
  ///
  /// In en, this message translates to:
  /// **'Window'**
  String get window;

  /// No description provided for @withdrawal.
  ///
  /// In en, this message translates to:
  /// **'Withdraw'**
  String get withdrawal;

  /// No description provided for @withdrawalHash.
  ///
  /// In en, this message translates to:
  /// **'Withdrawal Hash'**
  String get withdrawalHash;

  /// No description provided for @you.
  ///
  /// In en, this message translates to:
  /// **'You'**
  String get you;

  /// No description provided for @youDeletedThisMessage.
  ///
  /// In en, this message translates to:
  /// **'You deleted this message'**
  String get youDeletedThisMessage;

  /// No description provided for @zoom.
  ///
  /// In en, this message translates to:
  /// **'Zoom'**
  String get zoom;
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) => <String>[
    'en',
    'es',
    'id',
    'ja',
    'ms',
    'ru',
    'zh',
  ].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when language+country codes are specified.
  switch (locale.languageCode) {
    case 'zh':
      {
        switch (locale.countryCode) {
          case 'HK':
            return AppLocalizationsZhHk();
          case 'TW':
            return AppLocalizationsZhTw();
        }
        break;
      }
  }

  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'es':
      return AppLocalizationsEs();
    case 'id':
      return AppLocalizationsId();
    case 'ja':
      return AppLocalizationsJa();
    case 'ms':
      return AppLocalizationsMs();
    case 'ru':
      return AppLocalizationsRu();
    case 'zh':
      return AppLocalizationsZh();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
