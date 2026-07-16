// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Spanish Castilian (`es`).
class AppLocalizationsEs extends AppLocalizations {
  AppLocalizationsEs([String locale = 'es']) : super(locale);

  @override
  String get aMessage => 'un mensaje';

  @override
  String get about => 'Acerca de';

  @override
  String get account => 'Cuenta';

  @override
  String get activity => 'Actividad';

  @override
  String get add => 'Añadir';

  @override
  String get addACaption => 'Add a caption';

  @override
  String get addBotWithPlus => '+ Añadir Bot';

  @override
  String get addContact => 'Añadir contacto';

  @override
  String get addContactWithPlus => '+ Añadir contacto';

  @override
  String get addFile => 'Añadir archivo';

  @override
  String get addGroupDescription => 'Añadir descripción del grupo';

  @override
  String get addParticipants => 'Añadir participantes';

  @override
  String get addPeopleSearchHint => 'ID de Mixin o Número de teléfono';

  @override
  String get addProxy => 'Add Proxy';

  @override
  String get addSticker => 'Añadir pegatina';

  @override
  String get addStickerFailed => 'No se ha podido añadir la pegatina';

  @override
  String get addStickers => 'Añadir pegatinas';

  @override
  String get addToCircle => 'Añadir al círculo';

  @override
  String get added => 'Añadido';

  @override
  String get address => 'Dirección';

  @override
  String get admin => 'Administrador';

  @override
  String get alertKeyContactContactMessage => 'te ha enviado un contacto';

  @override
  String get allChats => 'Chats';

  @override
  String get animalsAndNature => 'Animales y Naturaleza';

  @override
  String get anonymous => 'Anonymous';

  @override
  String get anonymousNumber => 'Anonymous Number';

  @override
  String get appCardShareDisallow => 'No se puede compartir esta URL';

  @override
  String get appearance => 'Apariencia';

  @override
  String get archivedFolder => 'Carpeta archivada';

  @override
  String get assetType => 'Tipo de activo';

  @override
  String get audio => 'Audio';

  @override
  String get audios => 'Audios';

  @override
  String get autoBackup => 'Copia de seguridad automática';

  @override
  String get autoLock => 'Auto Lock';

  @override
  String get avatar => 'Avatar';

  @override
  String get backup => 'Copia de seguridad';

  @override
  String get backupChat => 'Hacer copia de seguridad del chat';

  @override
  String get backupToOtherDevice => 'Copia de seguridad a otro dispositivo';

  @override
  String get backupToOtherDeviceTips =>
      'Haz una copia de seguridad de tu historial de chat en otro dispositivo. Asegúrate de que ambos dispositivos estén conectados a la misma red Wi-Fi o punto de acceso.';

  @override
  String get backupWaitingOtherDevice =>
      'Abre Mixin en tu otro dispositivo y empieza la restauración allí.';

  @override
  String get biography => 'Biografía';

  @override
  String get biometric => 'Biometric';

  @override
  String get block => 'Bloquear';

  @override
  String get botNotFound => 'Bot no encontrado';

  @override
  String get bots => 'BOTS';

  @override
  String get botsTitle => 'Bots';

  @override
  String get bringAllToFront => 'Traer todo al frente';

  @override
  String get canNotRecognizeQrCode => 'No se puede reconocer el código QR';

  @override
  String get cancel => 'Cancelar';

  @override
  String get card => 'Tarjeta';

  @override
  String get change => 'Cambiar';

  @override
  String get changeNumber => 'Cambiar número';

  @override
  String get changeNumberInstead => 'Cambiar número';

  @override
  String changedDisappearingMessageSettings(Object arg0) {
    return '$arg0 ha cambiado la configuración de los mensajes que desaparecen.';
  }

  @override
  String get chatBackup => 'Copia de seguridad de chat';

  @override
  String get chatBackupAndRestore =>
      'Copia de seguridad y restauración de chats';

  @override
  String get chatBotReceptionTitle =>
      'Toca el botón para interactuar con el bot';

  @override
  String chatDecryptionFailedHint(Object arg0) {
    return 'Esperando a que $arg0 se conecte y establezca una sesión cifrada.';
  }

  @override
  String chatDeleteMessage(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '¿Eliminar $arg0 mensajes?',
      one: '¿Eliminar $arg0 mensaje?',
    );
    return '$_temp0';
  }

  @override
  String chatGroupAdd(Object arg0, Object arg1) {
    return '$arg0 ha añadido $arg1';
  }

  @override
  String chatGroupExit(Object arg0) {
    return '$arg0 salió';
  }

  @override
  String chatGroupJoin(Object arg0) {
    return '$arg0 se ha unido al grupo mediante un enlace de invitación';
  }

  @override
  String chatGroupRemove(Object arg0, Object arg1) {
    return '$arg0 ha eliminado $arg1';
  }

  @override
  String get chatHintE2e => 'Cifrado de extremo a extremo';

  @override
  String get chatNotSupportUriOnPhone =>
      'Este tipo de URL no es compatible. Revísalo en tu teléfono.';

  @override
  String get chatNotSupportUrl =>
      'https://support.mixin.one/en/article/how-to-do-when-you-receive-a-message-like-this-this-type-of-message-is-not-supported-please-upgrade-mixin-17j1t3p';

  @override
  String get chatNotSupportViewOnPhone =>
      'Este tipo de mensaje no es compatible. Revísalo en tu teléfono.';

  @override
  String chatPinMessage(Object arg0, Object arg1) {
    return '$arg0 ha fijado $arg1';
  }

  @override
  String get chatTextSize => 'Tamaño del texto del chat';

  @override
  String get chats => 'Chats';

  @override
  String get checkNewVersion => 'Comprobar nueva versión';

  @override
  String circleSubtitle(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 Conversaciones',
      one: '$arg0 Conversación',
    );
    return '$_temp0';
  }

  @override
  String circleTitle(Object arg0) {
    return 'Círculos de $arg0';
  }

  @override
  String get circles => 'Círculos';

  @override
  String get clear => 'Borrar';

  @override
  String get clearChat => 'Vaciar la conversación';

  @override
  String get clearFilter => 'Limpiar filtro';

  @override
  String get clickToReloadQrcode => 'Haz clic para recargar el código QR';

  @override
  String get close => 'Cerrar';

  @override
  String get closeWindow => 'Cerrar ventana';

  @override
  String get closingBalance => 'Closing Balance';

  @override
  String get collapse => 'Colapsar';

  @override
  String get collectible => 'Collectible';

  @override
  String get collectibles => 'Collectibles';

  @override
  String get collection => 'Collection';

  @override
  String get combineAndForward => 'Combinar y reenviar';

  @override
  String get confirm => 'Confirmar';

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
  String get contact => 'Contacto';

  @override
  String contactMixinId(Object arg0) {
    return 'ID de Mixin: $arg0';
  }

  @override
  String get contactMuteTitle => 'Silenciar notificaciones para…';

  @override
  String get contactTitle => 'Contactos';

  @override
  String get contentTooLong => 'Contenido demasiado largo';

  @override
  String get contentVoice => '[Llamada de voz]';

  @override
  String get continueText => 'Continuar';

  @override
  String get conversation => 'Conversación';

  @override
  String conversationDeleteTitle(Object arg0) {
    return 'Eliminar chat: $arg0';
  }

  @override
  String get copy => 'Copiar';

  @override
  String get copyImage => 'Copy Image';

  @override
  String get copyInvite => 'Copiar enlace de invitación';

  @override
  String get copyLink => 'Copiar enlace';

  @override
  String get copySelectedText => 'Copy Selected Text';

  @override
  String get copyText => 'Copy Text';

  @override
  String get create => 'Crear';

  @override
  String get createCircle => 'Nuevo círculo';

  @override
  String get createConversation => 'Nueva conversación';

  @override
  String get createGroup => 'Nuevo grupo';

  @override
  String createdAt(Object arg0) {
    return 'Created $arg0';
  }

  @override
  String createdThisGroup(Object arg0) {
    return '$arg0 ha creado este grupo';
  }

  @override
  String get customTime => 'Tiempo personalizado';

  @override
  String get dark => 'Oscuro';

  @override
  String get dataAndStorageUsage => 'Uso de datos y almacenamiento';

  @override
  String get dataError => 'Error de datos';

  @override
  String get dataLoading => 'Cargando datos, por favor espera...';

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
      'La base de datos se está actualizando, puede tardar varios minutos, no cierres esta aplicación.';

  @override
  String get delete => 'Borrar';

  @override
  String get deleteAccountDetailHint =>
      'Los mensajes locales y las copias de seguridad de iCloud no se eliminarán automáticamente';

  @override
  String get deleteAccountHint =>
      'Eliminar la información de tu cuenta y la foto de perfil';

  @override
  String get deleteChat => 'Eliminar chat';

  @override
  String get deleteChatDescription =>
      'Eliminar el chat eliminará los mensajes de estos dispositivos únicamente. No se eliminarán de otros dispositivos.';

  @override
  String get deleteCircle => 'Eliminar círculo';

  @override
  String get deleteForEveryone => 'Eliminar para todos';

  @override
  String get deleteForMe => 'Eliminar para mí';

  @override
  String get deleteGroup => 'Eliminar grupo';

  @override
  String get deleteMyAccount => 'Borrar mi cuenta';

  @override
  String deleteTheCircle(Object arg0) {
    return '¿Quieres eliminar el círculo $arg0?';
  }

  @override
  String get deposit => 'Depósito';

  @override
  String get depositHash => 'Deposit Hash';

  @override
  String get developer => 'Desarrollador';

  @override
  String get deviceTransferFailed => 'Transfer failed';

  @override
  String disableDisappearingMessage(Object arg0) {
    return '$arg0 ha desactivado los mensajes que desaparecen';
  }

  @override
  String get disabled => 'Disabled';

  @override
  String disappearingCustomTimeMaxWarning(Object arg0) {
    return 'El tiempo máximo es $arg0.';
  }

  @override
  String get disappearingMessage => 'Mensajes que desaparecen';

  @override
  String get disappearingMessageHint =>
      'Cuando está habilitado, los nuevos mensajes enviados y recibidos en este chat desaparecerán después de que se hayan visto, lee el documento para **obtener más información**.';

  @override
  String get discard => 'Desechar';

  @override
  String get discardRecordingWarning =>
      '¿Estás seguro de que deseas detener la grabación y descartar tu mensaje de voz?';

  @override
  String get dismissAsAdmin => 'Quitar como administrador';

  @override
  String get done => 'Hecho';

  @override
  String get download => 'Descargar';

  @override
  String get downloadLink => 'Enlace de descarga:';

  @override
  String get draft => 'Borrador';

  @override
  String get dragAndDropFileHere => 'Arrastra y suelta archivos aquí';

  @override
  String get durationIsTooShort => 'La duración es demasiado corta';

  @override
  String get edit => 'Editar';

  @override
  String get editCircleName => 'Editar nombre del círculo';

  @override
  String get editConversations => 'Editar conversaciones';

  @override
  String get editGroupDescription => 'Editar descripción del grupo';

  @override
  String get editGroupName => 'Editar nombre de grupo';

  @override
  String get editImageClearWarning =>
      'Todos los cambios se perderán. ¿Estás seguro de que quieres salir?';

  @override
  String get editName => 'Editar nombre';

  @override
  String get editProfile => 'Editar perfil';

  @override
  String get enablePushNotification => 'Activar las notificaciones';

  @override
  String get encryptZipFileWithPassword =>
      'Encrypt the ZIP file with a password';

  @override
  String get enterPinToDeleteAccount =>
      'Introduce tu PIN para eliminar tu cuenta';

  @override
  String get enterToSend => 'Return/Enter ⏎ to Send';

  @override
  String get enterYourPhoneNumber => 'Introduce tu número telefónico';

  @override
  String get enterYourPinToContinue => 'Introduce tu PIN para continuar';

  @override
  String get errorAccessLimited => 'ERROR 403: Access Limited';

  @override
  String get errorAddressExists =>
      'La dirección no existe, asegúrate de que la dirección se haya añadido correctamente';

  @override
  String get errorAddressNotSync =>
      'No se ha podido actualizar la dirección, inténtalo de nuevo';

  @override
  String get errorAlreadyBondedReferralCode =>
      'ERROR 10731: This account has already applied a referral code';

  @override
  String get errorAssetExists => 'El activo no existe';

  @override
  String get errorAuthentication => 'ERROR 401: Iniciar sesión para continuar';

  @override
  String get errorBadData =>
      'ERROR 10002: Los datos de la solicitud tienen un campo no válido';

  @override
  String get errorBlockchain =>
      'ERROR 30100: Blockchain no está sincronizado, inténtalo de nuevo más tarde.';

  @override
  String get errorConnectionTimeout =>
      'Se ha agotado el tiempo de espera de la conexión de red, inténtalo de nuevo';

  @override
  String get errorFullGroup => 'ERROR 20116: El chat de grupo está lleno.';

  @override
  String get errorInsufficientBalance => 'ERROR 20117: Saldo insuficiente';

  @override
  String errorInsufficientTransactionFeeWithAmount(Object arg0) {
    return 'ERROR 20124: Tarifa de transacción insuficiente. Asegúrate de que tu cartera tenga $arg0 como tarifa';
  }

  @override
  String errorInvalidAddress(Object arg0, Object arg1) {
    return 'ERROR 30102: formato de dirección no válido. Introduce la dirección $arg0 $arg1 correcta.';
  }

  @override
  String get errorInvalidAddressPlain =>
      'ERROR 30102: formato de dirección no válido.';

  @override
  String get errorInvalidCodeTooFrequent =>
      'ERROR 20129: Los códigos de verificación se envían con demasiada frecuencia. Inténtalo de nuevo más tarde.';

  @override
  String get errorInvalidEmergencyContact =>
      'ERROR 20130: Invalid recovery contact';

  @override
  String get errorInvalidPinFormat => 'ERROR 20118: Formato de PIN no válido.';

  @override
  String get errorInviterPlanExpired =>
      'ERROR 10737: The inviter has no valid plan';

  @override
  String get errorLegacyPin =>
      'ERROR 20118: To enhance the security of the Mixin network, Mixin API has temporarily suspended the upgrading from D3M-PIN to TIP. Please refer to the documentation for details and register for processing.';

  @override
  String get errorNetworkTaskFailed =>
      'La conexión de red ha fallado. Comprueba o cambia tu red e inténtalo de nuevo';

  @override
  String get errorNoPinToken =>
      'Sin token. Vuelve a iniciar sesión e inténtalo de nuevo.';

  @override
  String get errorNotFound => 'ERROR 404: No encontrado';

  @override
  String get errorNotSupportedAudioFormat =>
      'Formato de audio no compatible, ábrelo con otra aplicación.';

  @override
  String get errorNumberReachedLimit =>
      'ERROR 20132: El número ha llegado al límite.';

  @override
  String errorOldVersion(Object arg0) {
    return 'ERROR 10006: Actualiza Mixin($arg0) para seguir usando el servicio.';
  }

  @override
  String get errorOpenLocation => 'No puedo encontrar una aplicación de mapas';

  @override
  String get errorPermission => 'Por favor abre los permisos necesarios';

  @override
  String get errorPhoneInvalidFormat =>
      'ERROR 20110: Número de teléfono no válido';

  @override
  String get errorPhoneSmsDelivery => 'ERROR 10003: Error al entregar SMS';

  @override
  String get errorPhoneVerificationCodeExpired =>
      'ERROR 20114: Código de verificación del teléfono caducado';

  @override
  String get errorPhoneVerificationCodeInvalid =>
      'ERROR 20113: Código de verificación del teléfono no válido';

  @override
  String get errorPinCheckTooManyRequest =>
      'Lo ha intentado más de 5 veces, espera al menos 24 horas para volver a intentarlo.';

  @override
  String get errorPinIncorrect => 'ERROR 20119: PIN incorrecto';

  @override
  String errorPinIncorrectWithTimes(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'ERROR 20119: PIN incorrecto. Todavía tienes $arg0 oportunidades. Espera 24 horas para volver a intentarlo más tarde.',
      one:
          'ERROR 20119: PIN incorrecto. Todavía tienes $arg0 oportunidad. Espera 24 horas para volver a intentarlo más tarde.',
    );
    return '$_temp0';
  }

  @override
  String get errorRecaptchaIsInvalid => 'ERROR 10004: Recaptcha no es válido';

  @override
  String errorServer5xxCode(Object arg0) {
    return 'El servidor está en mantenimiento: $arg0';
  }

  @override
  String get errorTooManyRequest => 'ERROR 429: Límite de tasa excedido';

  @override
  String get errorTooManyStickers => 'ERROR 20126: Demasiadas pegatinas';

  @override
  String get errorTooSmallTransferAmount =>
      'ERROR 20120: Importe de la transferencia demasiado pequeño';

  @override
  String get errorTooSmallWithdrawAmount =>
      'ERROR 20127: Cantidad de retiro demasiado pequeña';

  @override
  String get errorTranscriptForward =>
      'Reenvía todos los archivos adjuntos después de que se hayan descargado.';

  @override
  String get errorTransferToDeactivatedUser =>
      'ERROR 20160: Transfers cannot be made to a deactivated user';

  @override
  String get errorUnableToOpenMedia =>
      'No puedo encontrar una aplicación capaz de abrir este medio.';

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
      'No se ha podido cargar el archivo adjunto del mensaje';

  @override
  String get errorUsedPhone =>
      'ERROR 20122: Este número de teléfono ya está asociado a otra cuenta.';

  @override
  String get errorUserInvalidFormat => 'ID de usuario inválido';

  @override
  String get errorWithdrawalMemoFormatIncorrect =>
      'ERROR 20131: Formato de nota de retiro incorrecto.';

  @override
  String get errorWithdrawalSuspend =>
      'ERROR 20137: Withdrawals are suspended.';

  @override
  String get exit => 'Salir';

  @override
  String get exitGroup => 'Salir del grupo';

  @override
  String get failed => 'Fallido';

  @override
  String get failedToOpenDatabase =>
      'An error occurred while opening the database.';

  @override
  String get fee => 'Tarifa';

  @override
  String get file => 'Archivo';

  @override
  String get fileChooserError => 'Error del selector de archivos';

  @override
  String get fileDoesNotExist => 'El archivo no existe';

  @override
  String get fileError => 'Error de archivo';

  @override
  String get files => 'Archivos';

  @override
  String get flags => 'Banderas';

  @override
  String get followSystem => 'Seguir sistema';

  @override
  String get followUsOnFacebook => 'Síguenos en Facebook';

  @override
  String get followUsOnX => 'Síguenos en X';

  @override
  String get foodAndDrink => 'Comida y bebida';

  @override
  String get formatNotSupported => 'Formato no compatible';

  @override
  String get forward => 'Reenviar';

  @override
  String get from => 'De';

  @override
  String get fromWithColon => 'De:';

  @override
  String get generateQrcode => 'Generate QR Code';

  @override
  String get groupAlreadyIn => 'Ya estás en el grupo';

  @override
  String get groupCantSend =>
      'No puedes enviar mensajes a este grupo porque ya no eres un participante.';

  @override
  String get groupName => 'Nombre del grupo';

  @override
  String get groupParticipants => 'Participantes';

  @override
  String groupPopMenuMessage(Object arg0) {
    return 'Mensaje $arg0';
  }

  @override
  String groupPopMenuRemove(Object arg0) {
    return 'Quitar $arg0';
  }

  @override
  String get groups => 'Grupos';

  @override
  String get groupsInCommon => 'Grupos en común';

  @override
  String get hash => 'HASH';

  @override
  String get help => 'Ayuda';

  @override
  String get helpCenter => 'Centro de ayuda';

  @override
  String get hideMixin => 'Ocultar Mixin';

  @override
  String get host => 'Host';

  @override
  String hour(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 Horas',
      one: '$arg0 Hora',
    );
    return '$_temp0';
  }

  @override
  String get howAreYou => 'Hola, ¿cómo estás?';

  @override
  String get iAmGood => 'Estoy bien.';

  @override
  String get id => 'ID';

  @override
  String get ignoreThisVersion => 'Ignorar la nueva versión';

  @override
  String get image => 'imagen';

  @override
  String get includeFiles => 'Incluir archivos';

  @override
  String get includeVideos => 'Incluir vídeos';

  @override
  String get initializing => 'Inicializando…';

  @override
  String get invalidStickerFormat => 'Formato de etiqueta inválido';

  @override
  String get inviteInfo =>
      'Cualquier persona con Mixin puede seguir este enlace para unirse a este grupo. Solo compártelo con personas en las que confíes.';

  @override
  String get inviteToGroupViaLink => 'Invitar al grupo a través de un enlace';

  @override
  String get joinGroupWithPlus => '+ Unirse al grupo';

  @override
  String joinedIn(Object arg0) {
    return 'Se ha unido en $arg0';
  }

  @override
  String landingDeleteContent(Object arg0, Object arg1) {
    return 'You requested to delete your account on $arg0. The account will be deleted on $arg1. If you continue to log in, your account deletion will be cancelled.';
  }

  @override
  String landingInvitationDialogContent(Object arg0) {
    return 'Te enviaremos un código de 4 dígitos a tu número de teléfono $arg0. Introduce el código en la siguiente pantalla.';
  }

  @override
  String landingValidationTitle(Object arg0) {
    return 'Introduce el código de 4 dígitos que te enviamos a $arg0';
  }

  @override
  String get learnMore => 'Obtener más información';

  @override
  String get less => 'menos';

  @override
  String get light => 'Claro';

  @override
  String get linkedDevice => 'dispositivo vinculado';

  @override
  String get live => 'En vivo';

  @override
  String get loading => 'Cargando...';

  @override
  String get loadingTime =>
      'La hora del sistema es inusual, continúa usándola nuevamente después de la corrección';

  @override
  String get locateToChat => 'Ir al chat';

  @override
  String get location => 'Localización';

  @override
  String get lock => 'Lock';

  @override
  String get logIn => 'Iniciar sesión';

  @override
  String get loginAndAbortAccountDeletion =>
      'Continuar para iniciar sesión y cancelar la eliminación de la cuenta';

  @override
  String get loginByQrcode => 'Iniciar sesión en Mixin Messenger por código QR';

  @override
  String get loginByQrcodeTips1 => 'Abre Mixin Messenger en tu teléfono.';

  @override
  String get loginByQrcodeTips2 =>
      'Escanea el código QR en la pantalla y confirma tu inicio de sesión.';

  @override
  String get makeGroupAdmin => 'Hacer administrador de grupo';

  @override
  String get media => 'Medios de comunicación';

  @override
  String get memo => 'Memorándum';

  @override
  String get messageE2ee =>
      'Los mensajes de esta conversación están encriptados de extremo a extremo, toca para obtener más información.';

  @override
  String get messageNotFound => 'Mensaje no encontrado';

  @override
  String get messageNotSupport =>
      'Este tipo de mensaje no es compatible. Actualiza Mixin a la última versión.';

  @override
  String get messagePreview => 'Vista previa del mensaje';

  @override
  String get messagePreviewDescription =>
      'Obtén una vista previa del texto del mensaje dentro de las notificaciones de mensajes nuevos.';

  @override
  String get messages => 'Mensajes';

  @override
  String get minimize => 'Minimizar';

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
  String get mixinMessengerDesktop => 'Mixin Messenger de Escritorio';

  @override
  String get more => 'Más';

  @override
  String get multisigTransaction => 'Transacción multigrado';

  @override
  String get mute => 'Silenciar';

  @override
  String myMixinId(Object arg0) {
    return 'Mi ID de Mixin: $arg0';
  }

  @override
  String get myStickers => 'Mis pegatinas';

  @override
  String get na => 'N/D';

  @override
  String get name => 'Nombre';

  @override
  String get networkConnectionFailed => 'Conexión de red fallida';

  @override
  String get networkError => 'Error de red';

  @override
  String get newVersionAvailable => 'Nueva versión disponible';

  @override
  String newVersionDescription(Object arg0, Object arg1) {
    return 'Mixin Messenger $arg0 ya está disponible, tienes $arg1. ¿Te gustaría descargarlo ahora?';
  }

  @override
  String get next => 'Próximo';

  @override
  String get nextConversation => 'Próxima conversación';

  @override
  String get noAudio => 'SIN AUDIO';

  @override
  String get noCamera => 'Sin cámaras';

  @override
  String get noData => 'Sin datos';

  @override
  String get noFiles => 'SIN ARCHIVOS';

  @override
  String get noLinks => 'SIN ENLACES';

  @override
  String get noMedia => 'SIN MULTIMEDIA';

  @override
  String get noNetworkConnection => 'No hay conexion de red';

  @override
  String get noPosts => 'SIN PUBLICACIONES';

  @override
  String get noResults => 'SIN RESULTADOS';

  @override
  String get notFound => 'Extraviado';

  @override
  String get notSupportBiometric =>
      'This device does not support biometric authentication';

  @override
  String get notificationContent =>
      'Enable push notifications to stay updated on price alerts and messages in real time.';

  @override
  String get notificationPermissionManually =>
      'Las notificaciones no están permitidas, ve a Configuración de notificaciones para activarlas.';

  @override
  String get notifications => 'Notificaciones';

  @override
  String nowAnAddmin(Object arg0) {
    return '$arg0 ahora es administrador';
  }

  @override
  String get objects => 'Objetos';

  @override
  String get oneByOneForward => 'Adelante uno por uno';

  @override
  String get oneHour => '1 hora';

  @override
  String get oneYear => '1 año';

  @override
  String get open => 'Open';

  @override
  String get openHomePage => 'Abrir página de inicio';

  @override
  String openLink(Object arg0) {
    return 'Abrir enlace: $arg0';
  }

  @override
  String get openLogDirectory => 'abrir directorio de registro';

  @override
  String get openingBalance => 'Opening Balance';

  @override
  String get originalImage => 'Original';

  @override
  String get owner => 'Propietario';

  @override
  String participantsCount(Object arg0) {
    return '$arg0 PARTICIPANTES';
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
      other: '$arg0/$arg1 confirmaciones',
      one: '$arg0/$arg1 confirmación',
    );
    return '$_temp0';
  }

  @override
  String get phoneNumber => 'Número de teléfono';

  @override
  String get photos => 'Fotos';

  @override
  String get pickAConversation =>
      'Selecciona una conversación y comienza a enviar un mensaje';

  @override
  String get picturesAndVideos => 'Pictures & Videos';

  @override
  String get pinTitle => 'Fijar';

  @override
  String pinnedMessageTitle(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 Mensajes fijados',
      one: '$arg0 Mensaje fijado',
    );
    return '$_temp0';
  }

  @override
  String get port => 'Port';

  @override
  String get post => 'Publicar';

  @override
  String get preferences => 'Preferencias';

  @override
  String get previousConversation => 'Conversación anterior';

  @override
  String get privacyPolicy => 'Política de privacidad';

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
  String get quickSearch => 'Búsqueda rápida';

  @override
  String get quitMixin => 'Salir de Mixin';

  @override
  String get raw => 'Sin procesar';

  @override
  String get rebate => 'Reembolso';

  @override
  String get recaptchaTimeout => 'Tiempo de espera de recaptcha';

  @override
  String get receiver => 'Receptor';

  @override
  String get recentChats => 'CHATS';

  @override
  String get reedit => 'Reeditar';

  @override
  String get refresh => 'Actualizar';

  @override
  String get removeBot => 'Eliminar Bot';

  @override
  String get removeChatFromCircle => 'Eliminar chat del círculo';

  @override
  String get removeContact => 'Eliminar contacto';

  @override
  String get removeStickers => 'Eliminar pegatinas';

  @override
  String get reply => 'Responder';

  @override
  String get report => 'Informe';

  @override
  String get reportAndBlock => '¿Denunciar y bloquear?';

  @override
  String get reportTitle =>
      '¿Enviar el registro de conversación a los desarrolladores?';

  @override
  String get resendCode => 'Reenviar codigo';

  @override
  String resendCodeIn(Object arg0) {
    return 'Reenviar código en $arg0 s';
  }

  @override
  String get reset => 'Reiniciar';

  @override
  String get resetLink => 'Restablecer enlace';

  @override
  String get restoreChat => 'Restaurar chat';

  @override
  String get restoreChatTip =>
      'Restaura tu historial de chat desde otro dispositivo. Asegúrate de que ambos dispositivos estén conectados a la misma red Wi-Fi o punto de acceso.';

  @override
  String get restoreFromOtherDevice => 'Restaurar desde otro dispositivo';

  @override
  String get retry => 'Reintentar';

  @override
  String get retryUploadFailed => 'Reintentar carga fallida.';

  @override
  String get revokeMultisigTransaction => 'Revocar transacción multigrado';

  @override
  String get save => 'Guardar';

  @override
  String get saveAs => 'Guardar como';

  @override
  String get saveToCameraRoll => 'Guarda en el Rollo de la cámara';

  @override
  String get sayHi => 'Di hola';

  @override
  String get scamWarning =>
      'Advertencia: Muchos usuarios han reportado esta cuenta como una estafa. Ten cuidado, especialmente si te pide dinero.';

  @override
  String get screenPasscode => 'Screen Passcode';

  @override
  String get search => 'Buscar';

  @override
  String get searchContact => 'Buscar contacto';

  @override
  String get searchConversation => 'Buscar conversación';

  @override
  String get searchEmpty => 'No se han encontrado chats, contactos o mensajes.';

  @override
  String get searchPlaceholderNumber => 'Buscar Mixin ID o número de teléfono:';

  @override
  String searchRelatedMessage(Object arg0, num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$arg0 mensajes relacionados',
      one: '$arg0 mensaje relacionado',
    );
    return '$_temp0';
  }

  @override
  String get searchUnread => 'Buscar no leído';

  @override
  String get secretUrl => 'https://mixin.one/pages/1000007';

  @override
  String get security => 'Seguridad';

  @override
  String get select => 'Seleccionar';

  @override
  String get send => 'Enviar';

  @override
  String get sendArchived => 'Todos los archivos archivados en un archivo zip';

  @override
  String get sendQuickly => 'Enviar rápidamente';

  @override
  String get sendToDeveloper => 'Enviar al Desarrollador';

  @override
  String get sendWithoutCompression => 'Enviar sin compresión';

  @override
  String get sendWithoutSound => 'Enviar sin sonido';

  @override
  String get set => 'Establecer';

  @override
  String setDisappearingMessageTimeTo(Object arg0, Object arg1) {
    return '$arg0 ha establecido el tiempo de desaparición del mensaje a $arg1';
  }

  @override
  String get setPasscodeDesc => 'Set a passcode to unlock Mixin Messenger';

  @override
  String get settingAuthSearchHint => 'ID de Mixin, nombre';

  @override
  String get settingBackupTips =>
      'Haz una copia de seguridad de tu historial de chat en iCloud. si pierdes tu iPhone o cambias a uno nuevo, puedes restaurar tu historial de chat cuando reinstales Mixin Messenger. Los mensajes de los que realizas una copia de seguridad no están protegidos por el cifrado de extremo a extremo de Mixin Messenger mientras estás en iCloud.';

  @override
  String settingDeleteAccountPinContent(Object arg0) {
    return 'Si continúas, tu perfil y los detalles de tu cuenta se eliminarán el $arg0. le nuestro documento para **aprender más**.';
  }

  @override
  String get settingDeleteAccountUrl =>
      'https://support.mixin.one/en/article/how-to-delete-my-account-19fkagl';

  @override
  String get share => 'Compartir';

  @override
  String get shareApps => 'Aplicaciones compartidas';

  @override
  String get shareContact => 'Compartir contacto';

  @override
  String get shareError => 'Compartir error.';

  @override
  String get shareLink => 'Compartir enlace';

  @override
  String shareMessageDescription(Object arg0, Object arg1) {
    return '¿Estás seguro de que quieres enviar un $arg0 desde $arg1?';
  }

  @override
  String shareMessageDescriptionEmpty(Object arg0) {
    return '¿Estás seguro de que quieres enviar el $arg0?';
  }

  @override
  String get sharedMedia => 'Medios compartidos';

  @override
  String get show => 'Espectáculo';

  @override
  String get showAvatar => 'Mostrar avatar';

  @override
  String get showIdentityNumber => 'Show Identity Number';

  @override
  String get showMixin => 'Mostrar Mixin';

  @override
  String get signIn => 'Iniciar sesión';

  @override
  String get signOut => 'Cerrar sesión';

  @override
  String get signWithMobileNumber => 'Iniciar sesión con número de teléfono';

  @override
  String get signWithQrcode => 'Iniciar sesión con código QR';

  @override
  String get smileysAndPeople => 'Emoticonos y personas';

  @override
  String get snapshotHash => 'Snapshot Hash';

  @override
  String get status => 'Estado';

  @override
  String get sticker => 'Pegatina';

  @override
  String get stickerAddInvalidSize =>
      'Requiere un tamaño de archivo de pegatinas de más de 1 KB y menos de 1 MB, ancho y alto entre 128 px y 1024 px.';

  @override
  String get stickerAlbumDetail => 'Detalle del álbum de cromos';

  @override
  String get stickerStore => 'Tienda de pegatinas';

  @override
  String get storageAutoDownloadDescription =>
      'Cambia la configuración de descarga automática para medios.';

  @override
  String get storageUsage => 'Uso de almacenamiento';

  @override
  String get strangerHint => 'Este remitente no está en tus contactos';

  @override
  String get strangers => 'Extraños';

  @override
  String get successful => 'Exitoso';

  @override
  String get symbols => 'Simbolos';

  @override
  String get syncFromOtherDevice => 'Sincronizar desde otro dispositivo';

  @override
  String get syncToOtherDevice => 'Sincronizar a otro dispositivo';

  @override
  String get termsOfService => 'Términos de servicio';

  @override
  String get text => 'Texto';

  @override
  String get theme => 'Tema';

  @override
  String get thisMessageWasDeleted => 'Este mensaje ha sido eliminado';

  @override
  String get time => 'Tiempo';

  @override
  String get to => 'A';

  @override
  String get today => 'Hoy';

  @override
  String get toggleChatInfo => 'Alternar información de chat';

  @override
  String get trace => 'Rastro';

  @override
  String get transactionHash => 'Hash de transacción';

  @override
  String get transactionId => 'ID de transacción';

  @override
  String get transactionType => 'Tipo de transacción';

  @override
  String get transactions => 'Transacciones';

  @override
  String get transactionsCannotBeDeleted =>
      'Las transacciones NO se pueden eliminar';

  @override
  String get transcript => 'Transcripción';

  @override
  String get transfer => 'Transferir';

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
  String get travelAndPlaces => 'Viajes y lugares';

  @override
  String get typeMessage => 'Escribe el mensaje';

  @override
  String unableToOpenFile(Object arg0) {
    return 'No se puede abrir el archivo: $arg0';
  }

  @override
  String get unblock => 'Desbloquear';

  @override
  String unitDay(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'días',
      one: 'día',
    );
    return '$_temp0';
  }

  @override
  String unitHour(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'horas',
      one: 'hora',
    );
    return '$_temp0';
  }

  @override
  String unitMinute(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'minutos',
      one: 'minuto',
    );
    return '$_temp0';
  }

  @override
  String unitSecond(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'segundos',
      one: 'segundo',
    );
    return '$_temp0';
  }

  @override
  String unitWeek(num count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'semanas',
      one: 'semana',
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
  String get unmute => 'Desactivar silencio';

  @override
  String get unpin => 'Desanclar';

  @override
  String get unpinAllMessages => 'Desanclar todos los mensajes';

  @override
  String get unpinAllMessagesConfirmation =>
      '¿Estás seguro de que quieres desanclar todos los mensajes?';

  @override
  String get unreadMessages => 'Mensajes no leídos';

  @override
  String get updateMixin => 'Actualizar Mixin';

  @override
  String updateMixinDescription(Object arg0) {
    return '¡La versión actual ($arg0) ya no está disponible!\nHaz clic en Actualizar a continuación para actualizar a la última versión de Google Play.';
  }

  @override
  String get upgrade => 'Mejora';

  @override
  String get upgrading => 'Actualizando';

  @override
  String get useBiometric => 'Use Biometric';

  @override
  String get userDeleteHint => 'This user has deleted their account.';

  @override
  String get userNotFound => 'Usuario no encontrado';

  @override
  String get username => 'Username';

  @override
  String valueNow(Object arg0) {
    return 'valor ahora $arg0';
  }

  @override
  String valueThen(Object arg0) {
    return 'valor entonces $arg0';
  }

  @override
  String get verifyPin => 'Verificar PIN';

  @override
  String get video => 'Video';

  @override
  String get videos => 'Vídeos';

  @override
  String get waitingForThisMessage => 'Esperando este mensaje.';

  @override
  String get waitingOtherDeviceConnection =>
      'Esperando a que se conecte el otro dispositivo.';

  @override
  String get webview2RuntimeInstallDescription =>
      'El dispositivo no ha instalado el componente WebView2 Runtime. Primero descarga e instale WebView2 Runtime.';

  @override
  String get webviewRuntimeUnavailable =>
      'El tiempo de ejecución de WebView no está disponible';

  @override
  String get window => 'Ventana';

  @override
  String get withdrawal => 'Retirar';

  @override
  String get withdrawalHash => 'Withdrawal Hash';

  @override
  String get you => 'Tú';

  @override
  String get youDeletedThisMessage => 'Has borrado este mensaje';

  @override
  String get zoom => 'Zoom';
}
