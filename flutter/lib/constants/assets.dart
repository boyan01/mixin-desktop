abstract final class MixinAssets {
  static const chat = 'assets/images/chat.svg';
  static const contacts = 'assets/images/slide_contacts.svg';
  static const groups = 'assets/images/group.svg';
  static const bots = 'assets/images/bot.svg';
  static const strangers = 'assets/images/strangers.svg';
  static const circle = 'assets/images/circle.svg';
  static const collapse = 'assets/images/collapse.svg';
  static const expanded = 'assets/images/expanded.svg';
  static const filterUnseen = 'assets/images/filter_unseen.svg';
  static const add = 'assets/images/ic_add.svg';
  static const search = 'assets/images/ic_search_small.svg';
  static const chatSearch = 'assets/images/ic_search.svg';
  static const chatInfo = 'assets/images/ic_screen.svg';
  static const back = 'assets/images/ic_back.svg';
  static const close = 'assets/images/ic_close.svg';
  static const send = 'assets/images/ic_send.svg';
  static const chatBackground = 'assets/images/chat_background.png';
  static const empty = 'assets/images/empty_file.svg';
  static const logo = 'assets/images/logo.png';
  static const retry = 'assets/images/ic_retry.svg';
  static const mute = 'assets/images/mute.svg';
  static const verified = 'assets/images/verified.svg';
  static const sent = 'assets/images/sent.svg';
  static const delivered = 'assets/images/delivered.svg';
  static const read = 'assets/images/read.svg';
  static const failed = 'assets/images/failed.svg';
  static const messagePin = 'assets/images/message_pin.svg';
  static const messageSecret = 'assets/images/message_secret.svg';
  static const messageRepresentative =
      'assets/images/message_representative.svg';
  static const profile = 'assets/images/ic_profile.svg';
  static const account = 'assets/images/account.svg';
  static const notification = 'assets/images/ic_notification.svg';
  static const storageUsage = 'assets/images/ic_storage_usage.svg';
  static const shield = 'assets/images/shield.svg';
  static const proxy = 'assets/images/proxy.svg';
  static const appearance = 'assets/images/ic_appearance.svg';
  static const about = 'assets/images/ic_about.svg';
  static const signOut = 'assets/images/ic_sign_out.svg';
  static const arrowRight = 'assets/images/ic_arrow_right.svg';
  static const selected = 'assets/images/selected.svg';
  static const warning = 'assets/images/triangle_warning.svg';
  static const aboutLogo = 'assets/images/about_logo.png';
  static const externalLink = 'assets/images/external_link.svg';
  static const linkSend = 'assets/images/link_send.svg';

  static String? messageIcon(String? category) {
    if (category == null) return null;
    if (category.contains('IMAGE')) return 'assets/images/image.svg';
    if (category.contains('VIDEO')) return 'assets/images/video.svg';
    if (category.contains('LIVE')) return 'assets/images/video.svg';
    if (category.contains('AUDIO')) return 'assets/images/audio.svg';
    if (category.contains('STICKER')) return 'assets/images/sticker.svg';
    if (category.contains('DATA') ||
        category.contains('POST') ||
        category.contains('TRANSCRIPT')) {
      return 'assets/images/file.svg';
    }
    if (category.contains('CONTACT')) return 'assets/images/contact.svg';
    if (category.contains('SNAPSHOT') || category.contains('TRANSFER')) {
      return 'assets/images/transfer.svg';
    }
    if (category.contains('LOCATION')) return 'assets/images/location.svg';
    if (category == 'APP_CARD' || category == 'APP_BUTTON_GROUP') {
      return 'assets/images/app_button.svg';
    }
    if (category.contains('RECALL')) return 'assets/images/recall.svg';
    if (category.contains('CALL')) return 'assets/images/video_call.svg';
    return null;
  }
}
