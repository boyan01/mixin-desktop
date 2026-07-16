// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Russian (`ru`).
class AppLocalizationsRu extends AppLocalizations {
  AppLocalizationsRu([String locale = 'ru']) : super(locale);

  @override
  String get aMessage => 'сообщение';

  @override
  String get about => 'О приложении';

  @override
  String get account => 'Аккаунт';

  @override
  String get activity => 'Activity';

  @override
  String get add => 'Добавить';

  @override
  String get addACaption => 'Add a caption';

  @override
  String get addBotWithPlus => '+ Добавить бота';

  @override
  String get addContact => 'Добавить контакт';

  @override
  String get addContactWithPlus => '+ Добавить контакт';

  @override
  String get addFile => 'Добавить файл';

  @override
  String get addGroupDescription => 'Добавить описание группы';

  @override
  String get addParticipants => 'Добавить участников';

  @override
  String get addPeopleSearchHint => 'Mixin ID или номер телефона';

  @override
  String get addProxy => 'Add Proxy';

  @override
  String get addSticker => 'Добавить наклейку';

  @override
  String get addStickerFailed => 'Не удалось добавить стикер';

  @override
  String get addStickers => 'Добавить стикеры';

  @override
  String get addToCircle => 'Add to Circle';

  @override
  String get added => 'Добавлен';

  @override
  String get address => 'Адрес';

  @override
  String get admin => 'Администратор';

  @override
  String get alertKeyContactContactMessage => 'отправил вам контакт';

  @override
  String get allChats => 'Чаты';

  @override
  String get animalsAndNature => 'Animals & Nature';

  @override
  String get anonymous => 'Anonymous';

  @override
  String get anonymousNumber => 'Anonymous Number';

  @override
  String get appCardShareDisallow => 'Этим URL нельзя поделиться';

  @override
  String get appearance => 'Вид';

  @override
  String get archivedFolder => 'Архивная папка';

  @override
  String get assetType => 'Тип актива';

  @override
  String get audio => 'Аудио';

  @override
  String get audios => 'Аудио';

  @override
  String get autoBackup => 'Автоматическое резервное копирование';

  @override
  String get autoLock => 'Auto Lock';

  @override
  String get avatar => 'Аватар';

  @override
  String get backup => 'Резервное копирование';

  @override
  String get backupChat => 'Создать резервную копию чатов';

  @override
  String get backupToOtherDevice =>
      'Резервное копирование на другое устройство';

  @override
  String get backupToOtherDeviceTips =>
      'Сделайте резервную копию истории чатов на другое устройство. Убедитесь, что оба устройства подключены к одной сети Wi-Fi или точке доступа.';

  @override
  String get backupWaitingOtherDevice =>
      'Откройте Mixin на другом устройстве и начните восстановление там.';

  @override
  String get biography => 'Биография';

  @override
  String get biometric => 'Biometric';

  @override
  String get block => 'Блокировать';

  @override
  String get botNotFound => 'Бот не найден';

  @override
  String get bots => 'БОТЫ';

  @override
  String get botsTitle => 'Боты';

  @override
  String get bringAllToFront => 'Bring All to Front';

  @override
  String get canNotRecognizeQrCode => 'Не удается распознать QR-код';

  @override
  String get cancel => 'Отменить';

  @override
  String get card => 'Карта';

  @override
  String get change => 'Изменить';

  @override
  String get changeNumber => 'Изменить номер';

  @override
  String get changeNumberInstead => 'Вместо этого изменить номер';

  @override
  String changedDisappearingMessageSettings(Object arg0) {
    return '$arg0 изменил настройки исчезающих сообщений.';
  }

  @override
  String get chatBackup => 'Резервное копирование чата';

  @override
  String get chatBackupAndRestore =>
      'Резервное копирование и восстановление чатов';

  @override
  String get chatBotReceptionTitle =>
      'Нажмите кнопку, чтобы взаимодействовать с ботом';

  @override
  String chatDecryptionFailedHint(Object arg0) {
    return 'Ожидание подключения $arg0 к сети и установления зашифрованного сеанса.';
  }

  @override
  String chatDeleteMessage(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Удалить $arg0 сообщения?',
      one: 'Удалить $arg0 сообщение?',
    );
    return '$_temp0';
  }

  @override
  String chatGroupAdd(Object arg0, Object arg1) {
    return '$arg0 добавил $arg1';
  }

  @override
  String chatGroupExit(Object arg0) {
    return '$arg0 покинул(а) группу';
  }

  @override
  String chatGroupJoin(Object arg0) {
    return '$arg0 присоединился к группе по ссылке-приглашению';
  }

  @override
  String chatGroupRemove(Object arg0, Object arg1) {
    return '$arg0 удалил $arg1';
  }

  @override
  String get chatHintE2e => 'Сквозное шифрование';

  @override
  String get chatNotSupportUriOnPhone =>
      'Этот тип URL не поддерживается. Проверьте его на телефоне.';

  @override
  String get chatNotSupportUrl =>
      'https://support.mixin.one/en/article/how-to-do-when-you-receive-a-message-like-this-this-type-of-message-is-not-supported-please-upgrade-mixin-17j1t3p';

  @override
  String get chatNotSupportViewOnPhone =>
      'Этот тип сообщений не поддерживается. Проверьте его на телефоне.';

  @override
  String chatPinMessage(Object arg0, Object arg1) {
    return '$arg0 закрепил $arg1';
  }

  @override
  String get chatTextSize => 'Chat Text Size';

  @override
  String get chats => 'Chats';

  @override
  String get checkNewVersion => 'Проверить новую версию';

  @override
  String circleSubtitle(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 чаты',
      one: '$arg0 чат',
    );
    return '$_temp0';
  }

  @override
  String circleTitle(Object arg0) {
    return 'Круги пользователя $arg0';
  }

  @override
  String get circles => 'Круги';

  @override
  String get clear => 'Очистить';

  @override
  String get clearChat => 'Очистить чат';

  @override
  String get clearFilter => 'Clear filter';

  @override
  String get clickToReloadQrcode => 'Нажмите, чтобы перезагрузить QR-код';

  @override
  String get close => 'Закрыть';

  @override
  String get closeWindow => 'Закрыть окно';

  @override
  String get closingBalance => 'Closing Balance';

  @override
  String get collapse => 'Свернуть';

  @override
  String get collectible => 'Collectible';

  @override
  String get collectibles => 'Collectibles';

  @override
  String get collection => 'Collection';

  @override
  String get combineAndForward => 'Объединить и переслать';

  @override
  String get confirm => 'Подтвердить';

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
  String get contact => 'Контакт';

  @override
  String contactMixinId(Object arg0) {
    return 'Идентификатор Mixin: $arg0';
  }

  @override
  String get contactMuteTitle => 'Отключить уведомления для…';

  @override
  String get contactTitle => 'Контакты';

  @override
  String get contentTooLong => 'Слишком длинный контент';

  @override
  String get contentVoice => '[Голосовой вызов]';

  @override
  String get continueText => 'Продолжить';

  @override
  String get conversation => 'Беседа';

  @override
  String conversationDeleteTitle(Object arg0) {
    return 'Удалить чат: $arg0';
  }

  @override
  String get copy => 'Копировать';

  @override
  String get copyImage => 'Copy Image';

  @override
  String get copyInvite => 'Скопировать пригласительную ссылку';

  @override
  String get copyLink => 'Копировать ссылку';

  @override
  String get copySelectedText => 'Copy Selected Text';

  @override
  String get copyText => 'Copy Text';

  @override
  String get create => 'Создавать';

  @override
  String get createCircle => 'Новый круг';

  @override
  String get createConversation => 'Новый разговор';

  @override
  String get createGroup => 'Новая группа';

  @override
  String createdAt(Object arg0) {
    return 'Created $arg0';
  }

  @override
  String createdThisGroup(Object arg0) {
    return '$arg0 создал эту группу';
  }

  @override
  String get customTime => 'Произвольное время';

  @override
  String get dark => 'Темное';

  @override
  String get dataAndStorageUsage => 'Использование данных и хранилища';

  @override
  String get dataError => 'Ошибка данных';

  @override
  String get dataLoading => 'Загрузка данных, пожалуйста, подождите...';

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
      'База данных обновляется, это может занять несколько минут, пожалуйста, не закрывайте это приложение.';

  @override
  String get delete => 'Удалить';

  @override
  String get deleteAccountDetailHint =>
      'Локальные сообщения и резервные копии iCloud не будут удаляться автоматически';

  @override
  String get deleteAccountHint =>
      'Удалить информацию об учетной записи и фото профиля';

  @override
  String get deleteChat => 'Удалить чат';

  @override
  String get deleteChatDescription =>
      'Удаление чата приведет к удалению сообщений только с этих устройств. Они не будут удалены с других устройств.';

  @override
  String get deleteCircle => 'Удалить круг';

  @override
  String get deleteForEveryone => 'Удалить для всех';

  @override
  String get deleteForMe => 'Удалить для меня';

  @override
  String get deleteGroup => 'Удалить группу';

  @override
  String get deleteMyAccount => 'Удалить мой аккаунт';

  @override
  String deleteTheCircle(Object arg0) {
    return 'Удалить $arg0 круг?';
  }

  @override
  String get deposit => 'Депозит';

  @override
  String get depositHash => 'Deposit Hash';

  @override
  String get developer => 'Разработчик';

  @override
  String get deviceTransferFailed => 'Transfer failed';

  @override
  String disableDisappearingMessage(Object arg0) {
    return '$arg0 отключил исчезающие сообщения';
  }

  @override
  String get disabled => 'Disabled';

  @override
  String disappearingCustomTimeMaxWarning(Object arg0) {
    return 'The maximum time is $arg0.';
  }

  @override
  String get disappearingMessage => 'Исчезающие сообщения';

  @override
  String get disappearingMessageHint =>
      'Если эта функция включена, отправленные и полученные новые сообщения в этом чате исчезнут после того, как они будут просмотрены. Прочтите документ, чтобы **узнать больше**';

  @override
  String get discard => 'Discard';

  @override
  String get discardRecordingWarning =>
      'Are you sure you want to stop recording and discard your voice message?';

  @override
  String get dismissAsAdmin => 'Снять права администратора';

  @override
  String get done => 'Сделано';

  @override
  String get download => 'Скачать';

  @override
  String get downloadLink => 'Ссылка на скачивание:';

  @override
  String get draft => 'Draft';

  @override
  String get dragAndDropFileHere => 'Перетащите файлы сюда';

  @override
  String get durationIsTooShort => 'Продолжительность слишком мала';

  @override
  String get edit => 'Редактировать';

  @override
  String get editCircleName => 'Изменить название круга';

  @override
  String get editConversations => 'Редактировать беседы';

  @override
  String get editGroupDescription => 'Изменить описание группы';

  @override
  String get editGroupName => 'Изменить имя группы';

  @override
  String get editImageClearWarning =>
      'Все изменения будут потеряны. Вы уверены, что хотите выйти?';

  @override
  String get editName => 'Редактировать название';

  @override
  String get editProfile => 'Редактировать профиль';

  @override
  String get enablePushNotification => 'Включить уведомления';

  @override
  String get encryptZipFileWithPassword =>
      'Encrypt the ZIP file with a password';

  @override
  String get enterPinToDeleteAccount =>
      'Введите свой PIN-код, чтобы удалить свою учетную запись';

  @override
  String get enterToSend => 'Return/Enter ⏎ to Send';

  @override
  String get enterYourPhoneNumber => 'Введите свой номер телефона';

  @override
  String get enterYourPinToContinue => 'Введите PIN-код, чтобы продолжить';

  @override
  String get errorAccessLimited => 'ERROR 403: Access Limited';

  @override
  String get errorAddressExists =>
      'Адрес не существует, убедитесь, что адрес успешно добавлен';

  @override
  String get errorAddressNotSync =>
      'Не удалось обновить адрес. Повторите попытку.';

  @override
  String get errorAlreadyBondedReferralCode =>
      'ERROR 10731: This account has already applied a referral code';

  @override
  String get errorAssetExists => 'Актив не существует';

  @override
  String get errorAuthentication => 'ОШИБКА 401: Войдите, чтобы продолжить';

  @override
  String get errorBadData =>
      'ОШИБКА 10002: Данные запроса содержат недопустимое поле';

  @override
  String get errorBlockchain =>
      'ОШИБКА 30100: Блокчейн не синхронизирован, повторите попытку позже.';

  @override
  String get errorConnectionTimeout =>
      'Тайм-аут подключения к сети, повторите попытку.';

  @override
  String get errorFullGroup => 'ОШИБКА 20116: Групповой чат заполнен.';

  @override
  String get errorInsufficientBalance => 'ОШИБКА 20117: Недостаточный баланс';

  @override
  String errorInsufficientTransactionFeeWithAmount(Object arg0) {
    return 'ОШИБКА 20124: Недостаточная комиссия за транзакцию. Убедитесь, что в вашем кошельке есть $arg0 в качестве комиссии.';
  }

  @override
  String errorInvalidAddress(Object arg0, Object arg1) {
    return 'ОШИБКА 30102: Недопустимый формат адреса. Пожалуйста, введите правильный адрес $arg0 $arg1!';
  }

  @override
  String get errorInvalidAddressPlain =>
      'ОШИБКА 30102: Недопустимый формат адреса.';

  @override
  String get errorInvalidCodeTooFrequent =>
      'ОШИБКА 20129: Код подтверждения отправляется слишком часто. Повторите попытку позже.';

  @override
  String get errorInvalidEmergencyContact =>
      'ERROR 20130: Invalid recovery contact';

  @override
  String get errorInvalidPinFormat => 'ОШИБКА 20118: Неверный формат PIN-кода.';

  @override
  String get errorInviterPlanExpired =>
      'ERROR 10737: The inviter has no valid plan';

  @override
  String get errorLegacyPin =>
      'ERROR 20118: To enhance the security of the Mixin network, Mixin API has temporarily suspended the upgrading from D3M-PIN to TIP. Please refer to the documentation for details and register for processing.';

  @override
  String get errorNetworkTaskFailed =>
      'Не удалось подключиться к сети. Проверьте или переключите сеть и повторите попытку.';

  @override
  String get errorNoPinToken =>
      'No token. Please sign in again and try this feature again.';

  @override
  String get errorNotFound => 'ОШИБКА 404: Не найдено';

  @override
  String get errorNotSupportedAudioFormat =>
      'Аудиоформат не поддерживается, откройте его в другом приложении.';

  @override
  String get errorNumberReachedLimit => 'ОШИБКА 20132: число достигло предела.';

  @override
  String errorOldVersion(Object arg0) {
    return 'ОШИБКА 10006: Обновите Mixin($arg0), чтобы продолжить использование сервиса.';
  }

  @override
  String get errorOpenLocation => 'Не могу найти приложение карты';

  @override
  String get errorPermission => 'Пожалуйста, Откройте необходимые разрешения';

  @override
  String get errorPhoneInvalidFormat => 'ОШИБКА 20110: Неверный номер телефона';

  @override
  String get errorPhoneSmsDelivery => 'ОШИБКА 10003: Не удалось доставить SMS';

  @override
  String get errorPhoneVerificationCodeExpired =>
      'ОШИБКА 20114: Срок действия кода подтверждения телефона истек';

  @override
  String get errorPhoneVerificationCodeInvalid =>
      'ОШИБКА 20113: Неверный код подтверждения телефона';

  @override
  String get errorPinCheckTooManyRequest =>
      'Вы пытались более 5 раз, пожалуйста, подождите не менее 24 часов, чтобы повторить попытку.';

  @override
  String get errorPinIncorrect => 'ОШИБКА 20119: Неверный PIN-код';

  @override
  String errorPinIncorrectWithTimes(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'ОШИБКА 20119: Неверный PIN-код. У вас еще есть $arg0 шансов. Пожалуйста, подождите 24 часа, чтобы повторить попытку позже.',
      one:
          'ОШИБКА 20119: Неверный PIN-код. У вас еще есть шанс $arg0. Пожалуйста, подождите 24 часа, чтобы повторить попытку позже.',
    );
    return '$_temp0';
  }

  @override
  String get errorRecaptchaIsInvalid =>
      'ОШИБКА 10004: Recaptcha недействительна';

  @override
  String errorServer5xxCode(Object arg0) {
    return 'Сервер находится на обслуживании: $arg0';
  }

  @override
  String get errorTooManyRequest => 'ОШИБКА 429: Превышен предел скорости';

  @override
  String get errorTooManyStickers => 'ОШИБКА 20126: Слишком много наклеек';

  @override
  String get errorTooSmallTransferAmount =>
      'ОШИБКА 20120: Сумма перевода слишком мала';

  @override
  String get errorTooSmallWithdrawAmount =>
      'ОШИБКА 20127: Слишком маленькая сумма вывода';

  @override
  String get errorTranscriptForward =>
      'Пожалуйста, пересылайте все вложения после их загрузки';

  @override
  String get errorTransferToDeactivatedUser =>
      'ERROR 20160: Transfers cannot be made to a deactivated user';

  @override
  String get errorUnableToOpenMedia =>
      'Не удается найти приложение, способное открыть этот носитель.';

  @override
  String errorUnknownWithCode(Object arg0) {
    return 'ОШИБКА: $arg0';
  }

  @override
  String errorUnknownWithMessage(Object arg0) {
    return 'ОШИБКА: $arg0';
  }

  @override
  String get errorUploadAttachmentFailed =>
      'Failed to upload message attachment';

  @override
  String get errorUsedPhone =>
      'ОШИБКА 20122: этот номер телефона уже связан с другой учетной записью.';

  @override
  String get errorUserInvalidFormat =>
      'Недопустимый идентификатор пользователя';

  @override
  String get errorWithdrawalMemoFormatIncorrect =>
      'ОШИБКА 20131: Неверный формат уведомления о снятии средств.';

  @override
  String get errorWithdrawalSuspend =>
      'ERROR 20137: Withdrawals are suspended.';

  @override
  String get exit => 'Выход';

  @override
  String get exitGroup => 'Выйти из группы';

  @override
  String get failed => 'Не удалось';

  @override
  String get failedToOpenDatabase =>
      'An error occurred while opening the database.';

  @override
  String get fee => 'Комиссия';

  @override
  String get file => 'Файл';

  @override
  String get fileChooserError => 'Ошибка выбора файла';

  @override
  String get fileDoesNotExist => 'Файл не существует';

  @override
  String get fileError => 'Ошибка файла';

  @override
  String get files => 'Файлы';

  @override
  String get flags => 'Flags';

  @override
  String get followSystem => 'Следуйте системе';

  @override
  String get followUsOnFacebook => 'Следите за нами на Фейсбуке';

  @override
  String get followUsOnX => 'Следите за нами на X';

  @override
  String get foodAndDrink => 'Food & Drink';

  @override
  String get formatNotSupported => 'Формат не поддерживается';

  @override
  String get forward => 'Переслать';

  @override
  String get from => 'От';

  @override
  String get fromWithColon => 'От:';

  @override
  String get generateQrcode => 'Generate QR Code';

  @override
  String get groupAlreadyIn => 'Вы уже в этой группе';

  @override
  String get groupCantSend =>
      'Вы не можете отправлять сообщения в эту группу, потому что вы больше не являетесь ее участником.';

  @override
  String get groupName => 'Название группы';

  @override
  String get groupParticipants => 'Участники';

  @override
  String groupPopMenuMessage(Object arg0) {
    return 'Сообщение $arg0';
  }

  @override
  String groupPopMenuRemove(Object arg0) {
    return 'Удалить $arg0';
  }

  @override
  String get groups => 'Группы';

  @override
  String get groupsInCommon => 'Общие группы';

  @override
  String get hash => 'HASH';

  @override
  String get help => 'Помощь';

  @override
  String get helpCenter => 'Центр помощи';

  @override
  String get hideMixin => 'Скрыть Mixin';

  @override
  String get host => 'Host';

  @override
  String hour(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 часы',
      one: '$arg0 час',
    );
    return '$_temp0';
  }

  @override
  String get howAreYou => 'Привет, как дела?';

  @override
  String get iAmGood => 'Я в порядке.';

  @override
  String get id => 'ID';

  @override
  String get ignoreThisVersion => 'Игнорировать новую версию';

  @override
  String get image => 'изображение';

  @override
  String get includeFiles => 'Включить файлы';

  @override
  String get includeVideos => 'Включить видео';

  @override
  String get initializing => 'Инициализация…';

  @override
  String get invalidStickerFormat => 'Неверный формат стикера';

  @override
  String get inviteInfo =>
      'Любой, у кого есть Mixin, может перейти по этой ссылке, чтобы присоединиться к этой группе. Делитесь ею только с теми, кому вы доверяете.';

  @override
  String get inviteToGroupViaLink => 'Пригласить в группу по ссылке';

  @override
  String get joinGroupWithPlus => '+ Присоединиться к группе';

  @override
  String joinedIn(Object arg0) {
    return 'Присоединился к $arg0';
  }

  @override
  String landingDeleteContent(Object arg0, Object arg1) {
    return 'You requested to delete your account on $arg0. The account will be deleted on $arg1. If you continue to log in, your account deletion will be cancelled.';
  }

  @override
  String landingInvitationDialogContent(Object arg0) {
    return 'Мы отправим 4-значный код на ваш номер телефона $arg0. Введите код на следующем экране.';
  }

  @override
  String landingValidationTitle(Object arg0) {
    return 'Введите 4-значный код, отправленный вам на $arg0.';
  }

  @override
  String get learnMore => 'Узнать больше';

  @override
  String get less => 'меньше';

  @override
  String get light => 'Светлое';

  @override
  String get linkedDevice => 'linked device';

  @override
  String get live => 'Вживую';

  @override
  String get loading => 'Загрузка...';

  @override
  String get loadingTime =>
      'Системное время необычно, пожалуйста, продолжайте использовать его снова после исправления';

  @override
  String get locateToChat => 'Перейти к чату';

  @override
  String get location => 'Расположение';

  @override
  String get lock => 'Lock';

  @override
  String get logIn => 'Авторизоваться';

  @override
  String get loginAndAbortAccountDeletion =>
      'Продолжить вход и отменить удаление учетной записи';

  @override
  String get loginByQrcode => 'Войдите в Mixin Messenger по QR-коду';

  @override
  String get loginByQrcodeTips1 => 'Open Mixin Messenger on your phone.';

  @override
  String get loginByQrcodeTips2 =>
      'Scan the QR code on the screen and confirm your sign-in.';

  @override
  String get makeGroupAdmin => 'Сделать администратором группы';

  @override
  String get media => 'Медиа';

  @override
  String get memo => 'Памятка';

  @override
  String get messageE2ee =>
      'Сообщения в этом чате полностью зашифрованы. Нажмите, чтобы узнать больше.';

  @override
  String get messageNotFound => 'Сообщение не найдено';

  @override
  String get messageNotSupport =>
      'Этот тип сообщений не поддерживается. Обновите Mixin до последней версии.';

  @override
  String get messagePreview => 'Предварительный просмотр сообщения';

  @override
  String get messagePreviewDescription =>
      'Предварительный просмотр текста сообщения в уведомлениях о новых сообщениях.';

  @override
  String get messages => 'Сообщения';

  @override
  String get minimize => 'Свести к минимуму';

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
  String get mixinMessengerDesktop => 'Рабочий стол Mixin Messenger';

  @override
  String get more => 'Больше';

  @override
  String get multisigTransaction => 'Мультиподписная транзакция';

  @override
  String get mute => 'Беззвучный';

  @override
  String myMixinId(Object arg0) {
    return 'Мой Mixin ID: $arg0';
  }

  @override
  String get myStickers => 'Мои стикеры';

  @override
  String get na => 'Нет данных';

  @override
  String get name => 'Имя';

  @override
  String get networkConnectionFailed => 'Network connection failed';

  @override
  String get networkError => 'Сетевая ошибка';

  @override
  String get newVersionAvailable => 'Доступна новая версия';

  @override
  String newVersionDescription(Object arg0, Object arg1) {
    return 'Доступен Mixin Messenger $arg0; у вас установлена $arg1. Загрузить сейчас?';
  }

  @override
  String get next => 'Следующий';

  @override
  String get nextConversation => 'Следующий разговор';

  @override
  String get noAudio => 'НЕТ АУДИО';

  @override
  String get noCamera => 'Нет камеры';

  @override
  String get noData => 'Нет данных';

  @override
  String get noFiles => 'ФАЙЛОВ НЕТ';

  @override
  String get noLinks => 'НЕТ ССЫЛОК';

  @override
  String get noMedia => 'НЕТ МЕДИА';

  @override
  String get noNetworkConnection => 'Нет подключения к сети';

  @override
  String get noPosts => 'НЕТ СООБЩЕНИЙ';

  @override
  String get noResults => 'НЕТ РЕЗУЛЬТАТОВ';

  @override
  String get notFound => 'Не найден';

  @override
  String get notSupportBiometric =>
      'This device does not support biometric authentication';

  @override
  String get notificationContent =>
      'Enable push notifications to stay updated on price alerts and messages in real time.';

  @override
  String get notificationPermissionManually =>
      'Уведомления запрещены. Чтобы включить их, перейдите в настройки уведомлений.';

  @override
  String get notifications => 'Уведомления';

  @override
  String nowAnAddmin(Object arg0) {
    return '$arg0 теперь администратор';
  }

  @override
  String get objects => 'Objects';

  @override
  String get oneByOneForward => 'Один за другим вперед';

  @override
  String get oneHour => '1 час';

  @override
  String get oneYear => '1 год';

  @override
  String get open => 'Open';

  @override
  String get openHomePage => 'Открыть домашнюю страницу';

  @override
  String openLink(Object arg0) {
    return 'Открыть ссылку: $arg0';
  }

  @override
  String get openLogDirectory => 'открыть каталог журналов';

  @override
  String get openingBalance => 'Opening Balance';

  @override
  String get originalImage => 'Оригинал';

  @override
  String get owner => 'Владелец';

  @override
  String participantsCount(Object arg0) {
    return '$arg0 УЧАСТНИКОВ';
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
      other: '$arg0/$arg1 подтверждения',
      one: '$arg0/$arg1 подтверждение',
    );
    return '$_temp0';
  }

  @override
  String get phoneNumber => 'Номер телефона';

  @override
  String get photos => 'Фото';

  @override
  String get pickAConversation =>
      'Выберите беседу и начните отправлять сообщение';

  @override
  String get picturesAndVideos => 'Pictures & Videos';

  @override
  String get pinTitle => 'Закрепить';

  @override
  String pinnedMessageTitle(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 закрепленных сообщений',
      one: '$arg0 закрепленное сообщение',
    );
    return '$_temp0';
  }

  @override
  String get port => 'Port';

  @override
  String get post => 'Публикация';

  @override
  String get preferences => 'Настройки';

  @override
  String get previousConversation => 'Предыдущий разговор';

  @override
  String get privacyPolicy => 'Политика конфиденциальности';

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
  String get quickSearch => 'Быстрый поиск';

  @override
  String get quitMixin => 'Выйти из Mixin';

  @override
  String get raw => 'Исходный';

  @override
  String get rebate => 'Скидка';

  @override
  String get recaptchaTimeout => 'Тайм-аут рекапчи';

  @override
  String get receiver => 'Получатель';

  @override
  String get recentChats => 'ЧАТЫ';

  @override
  String get reedit => 'Редактировать повторно';

  @override
  String get refresh => 'Обновить';

  @override
  String get removeBot => 'Удалить бота';

  @override
  String get removeChatFromCircle => 'Удалить чат из круга';

  @override
  String get removeContact => 'Удалить контакт';

  @override
  String get removeStickers => 'Удалить наклейки';

  @override
  String get reply => 'Ответить';

  @override
  String get report => 'Отчет';

  @override
  String get reportAndBlock => 'Пожаловаться и заблокировать?';

  @override
  String get reportTitle => 'Отправить журнал бесед разработчикам?';

  @override
  String get resendCode => 'Отправить код еще раз';

  @override
  String resendCodeIn(Object arg0) {
    return 'Повторно отправить код через $arg0 с.';
  }

  @override
  String get reset => 'Перезагрузить';

  @override
  String get resetLink => 'Сбросить ссылку';

  @override
  String get restoreChat => 'Восстановить чаты';

  @override
  String get restoreChatTip =>
      'Восстановите историю чатов с другого устройства. Убедитесь, что оба устройства подключены к одной сети Wi-Fi или точке доступа.';

  @override
  String get restoreFromOtherDevice => 'Восстановить с другого устройства';

  @override
  String get retry => 'Повторить';

  @override
  String get retryUploadFailed => 'Повторная загрузка не удалась.';

  @override
  String get revokeMultisigTransaction =>
      'Отозвать транзакцию с мультиподписью';

  @override
  String get save => 'Сохранить';

  @override
  String get saveAs => 'Сохранить как';

  @override
  String get saveToCameraRoll => 'Сохранить в ленту камеры';

  @override
  String get sayHi => 'Скажи привет';

  @override
  String get scamWarning =>
      'Предупреждение: многие пользователи сообщали об этой учетной записи как о мошенничестве. Пожалуйста, будьте осторожны, особенно если он просит у вас деньги';

  @override
  String get screenPasscode => 'Screen Passcode';

  @override
  String get search => 'Поиск';

  @override
  String get searchContact => 'Поиск контакта';

  @override
  String get searchConversation => 'Поиск беседы';

  @override
  String get searchEmpty => 'Чаты, контакты или сообщения не найдены.';

  @override
  String get searchPlaceholderNumber => 'Найдите Mixin ID или номер телефона:';

  @override
  String searchRelatedMessage(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 похожие сообщения',
      one: '$arg0 связанное сообщение',
    );
    return '$_temp0';
  }

  @override
  String get searchUnread => 'Search Unread';

  @override
  String get secretUrl => 'https://mixin.one/pages/1000007';

  @override
  String get security => 'Безопасность';

  @override
  String get select => 'Выбрать';

  @override
  String get send => 'Отправить';

  @override
  String get sendArchived => 'Заархивированы все файлы в один zip файл';

  @override
  String get sendQuickly => 'Отправить быстро';

  @override
  String get sendToDeveloper => 'Отправить разработчику';

  @override
  String get sendWithoutCompression => 'Отправить без сжатия';

  @override
  String get sendWithoutSound => 'Отправить без звука';

  @override
  String get set => 'Установлен';

  @override
  String setDisappearingMessageTimeTo(Object arg0, Object arg1) {
    return '$arg0 установил время исчезновения сообщения на $arg1.';
  }

  @override
  String get setPasscodeDesc => 'Set a passcode to unlock Mixin Messenger';

  @override
  String get settingAuthSearchHint => 'Mixin ID, имя';

  @override
  String get settingBackupTips =>
      'Создайте резервную копию истории чата в iCloud. Если вы потеряете свой iPhone или переключитесь на новый, вы сможете восстановить историю чата при переустановке Mixin Messenger. Сообщения, которые вы резервируете, не защищены сквозным шифрованием Mixin Messenger в iCloud.';

  @override
  String settingDeleteAccountPinContent(Object arg0) {
    return 'Если вы продолжите, ваш профиль и данные аккаунта будут удалены $arg0. прочтите наш документ, чтобы **Узнать больше**.';
  }

  @override
  String get settingDeleteAccountUrl =>
      'https://support.mixin.one/en/article/how-to-delete-my-account-19fkagl';

  @override
  String get share => 'Поделиться';

  @override
  String get shareApps => 'Общие приложения';

  @override
  String get shareContact => 'Поделиться контактом';

  @override
  String get shareError => 'Поделитесь ошибкой.';

  @override
  String get shareLink => 'Поделиться ссылкой';

  @override
  String shareMessageDescription(Object arg0, Object arg1) {
    return 'Вы уверены, что хотите отправить $arg0 от $arg1?';
  }

  @override
  String shareMessageDescriptionEmpty(Object arg0) {
    return 'Вы уверены, что хотите отправить $arg0?';
  }

  @override
  String get sharedMedia => 'Общие медиа';

  @override
  String get show => 'Показать';

  @override
  String get showAvatar => 'Показать аватар';

  @override
  String get showIdentityNumber => 'Show Identity Number';

  @override
  String get showMixin => 'Показать Mixin';

  @override
  String get signIn => 'Войти';

  @override
  String get signOut => 'Выход';

  @override
  String get signWithMobileNumber => 'Войти через номер телефона';

  @override
  String get signWithQrcode => 'Войти с QrCode';

  @override
  String get smileysAndPeople => 'Smileys & People';

  @override
  String get snapshotHash => 'Snapshot Hash';

  @override
  String get status => 'Статус';

  @override
  String get sticker => 'Наклейка';

  @override
  String get stickerAddInvalidSize =>
      'Требуется размер файла наклеек от 1 КБ до 1 МБ, ширина и высота от 128 до 1024 пикселей.';

  @override
  String get stickerAlbumDetail => 'Деталь альбома стикеров';

  @override
  String get stickerStore => 'Магазин наклеек';

  @override
  String get storageAutoDownloadDescription =>
      'Измените настройки автоматической загрузки для медиафайлов.';

  @override
  String get storageUsage => 'Использование хранилища';

  @override
  String get strangerHint => 'Этого отправителя нет в ваших контактах';

  @override
  String get strangers => 'Незнакомцы';

  @override
  String get successful => 'Успешный';

  @override
  String get symbols => 'Symbols';

  @override
  String get syncFromOtherDevice => 'Синхронизировать с другого устройства';

  @override
  String get syncToOtherDevice => 'Синхронизировать на другое устройство';

  @override
  String get termsOfService => 'Условия обслуживания';

  @override
  String get text => 'Текст';

  @override
  String get theme => 'Тема';

  @override
  String get thisMessageWasDeleted => 'Это сообщение было удалено';

  @override
  String get time => 'Время';

  @override
  String get to => 'В';

  @override
  String get today => 'Сегодня';

  @override
  String get toggleChatInfo => 'Переключить информацию о чате';

  @override
  String get trace => 'След';

  @override
  String get transactionHash => 'Хэш транзакции';

  @override
  String get transactionId => 'ID транзакции';

  @override
  String get transactionType => 'Тип операции';

  @override
  String get transactions => 'Транзакции';

  @override
  String get transactionsCannotBeDeleted => 'Транзакции НЕ МОГУТ быть удалены';

  @override
  String get transcript => 'Стенограмма';

  @override
  String get transfer => 'Передача';

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
  String get typeMessage => 'Введите сообщение';

  @override
  String unableToOpenFile(Object arg0) {
    return 'Не удалось открыть файл: $arg0';
  }

  @override
  String get unblock => 'Разблокировать';

  @override
  String unitDay(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'дни',
      one: 'день',
    );
    return '$_temp0';
  }

  @override
  String unitHour(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'часы',
      one: 'час',
    );
    return '$_temp0';
  }

  @override
  String unitMinute(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'минуты',
      one: 'минута',
    );
    return '$_temp0';
  }

  @override
  String unitSecond(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'секунды',
      one: 'секунда',
    );
    return '$_temp0';
  }

  @override
  String unitWeek(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'недели',
      one: 'неделя',
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
  String get unmute => 'Включить звук';

  @override
  String get unpin => 'Открепить';

  @override
  String get unpinAllMessages => 'Открепить все сообщения';

  @override
  String get unpinAllMessagesConfirmation =>
      'Вы уверены, что хотите открепить все сообщения?';

  @override
  String get unreadMessages => 'Непрочитанные сообщения';

  @override
  String get updateMixin => 'Обновить Mixin';

  @override
  String updateMixinDescription(Object arg0) {
    return 'Текущая версия ($arg0) больше не доступна!\nНажмите «Обновить» ниже, чтобы выполнить обновление до последней версии из Google Play.';
  }

  @override
  String get upgrade => 'Upgrade';

  @override
  String get upgrading => 'Обновление';

  @override
  String get useBiometric => 'Use Biometric';

  @override
  String get userDeleteHint => 'This user has deleted their account.';

  @override
  String get userNotFound => 'Пользователь не найден';

  @override
  String get username => 'Username';

  @override
  String valueNow(Object arg0) {
    return 'значение сейчас $arg0';
  }

  @override
  String valueThen(Object arg0) {
    return 'значение затем $arg0';
  }

  @override
  String get verifyPin => 'Подтвердить PIN-код';

  @override
  String get video => 'Видео';

  @override
  String get videos => 'Видео';

  @override
  String get waitingForThisMessage => 'Ждем этого сообщения.';

  @override
  String get waitingOtherDeviceConnection =>
      'Ожидание подключения другого устройства.';

  @override
  String get webview2RuntimeInstallDescription =>
      'На устройстве не установлен компонент WebView2 Runtime. Сначала загрузите и установите WebView2 Runtime.';

  @override
  String get webviewRuntimeUnavailable => 'Среда выполнения WebView недоступна';

  @override
  String get window => 'Окно';

  @override
  String get withdrawal => 'Вывести';

  @override
  String get withdrawalHash => 'Withdrawal Hash';

  @override
  String get you => 'Вы';

  @override
  String get youDeletedThisMessage => 'Вы удалили это сообщение';

  @override
  String get zoom => 'Zoom';
}
