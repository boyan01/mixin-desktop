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
  static const empty = 'assets/images/empty_file.svg';
  static const mute = 'assets/images/mute.svg';
  static const verified = 'assets/images/verified.svg';

  static String? messageIcon(String? category) {
    if (category == null) return null;
    if (category.contains('IMAGE')) return 'assets/images/image.svg';
    if (category.contains('VIDEO')) return 'assets/images/video.svg';
    if (category.contains('AUDIO')) return 'assets/images/audio.svg';
    if (category.contains('STICKER')) return 'assets/images/sticker.svg';
    if (category.contains('DATA') || category.contains('POST')) {
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
