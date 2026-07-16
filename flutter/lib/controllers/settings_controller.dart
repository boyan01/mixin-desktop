import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

class SettingsController extends ChangeNotifier {
  static const _themeModeKey = 'settings.themeMode';
  static const _messageShowAvatarKey = 'settings.messageShowAvatar';
  static const _messageShowIdentityNumberKey =
      'settings.messageShowIdentityNumber';
  static const _messagePreviewKey = 'settings.messagePreview';
  static const _photoAutoDownloadKey = 'settings.photoAutoDownload';
  static const _videoAutoDownloadKey = 'settings.videoAutoDownload';
  static const _fileAutoDownloadKey = 'settings.fileAutoDownload';
  static const _chatFontSizeDeltaKey = 'settings.chatFontSizeDelta';

  SharedPreferences? _preferences;

  ThemeMode themeMode = ThemeMode.system;
  bool messageShowAvatar = true;
  bool messageShowIdentityNumber = false;
  bool messagePreview = true;
  bool photoAutoDownload = true;
  bool videoAutoDownload = true;
  bool fileAutoDownload = true;
  double chatFontSizeDelta = 0;

  Future<void> initialize() async {
    final preferences = await SharedPreferences.getInstance();
    _preferences = preferences;
    themeMode = _themeModeFromName(preferences.getString(_themeModeKey));
    messageShowAvatar =
        preferences.getBool(_messageShowAvatarKey) ?? messageShowAvatar;
    messageShowIdentityNumber =
        preferences.getBool(_messageShowIdentityNumberKey) ??
        messageShowIdentityNumber;
    messagePreview = preferences.getBool(_messagePreviewKey) ?? messagePreview;
    photoAutoDownload =
        preferences.getBool(_photoAutoDownloadKey) ?? photoAutoDownload;
    videoAutoDownload =
        preferences.getBool(_videoAutoDownloadKey) ?? videoAutoDownload;
    fileAutoDownload =
        preferences.getBool(_fileAutoDownloadKey) ?? fileAutoDownload;
    chatFontSizeDelta =
        preferences.getDouble(_chatFontSizeDeltaKey) ?? chatFontSizeDelta;
  }

  void setThemeMode(ThemeMode value) {
    if (themeMode == value) return;
    themeMode = value;
    _preferences?.setString(_themeModeKey, value.name);
    notifyListeners();
  }

  void setMessageShowAvatar(bool value) {
    if (messageShowAvatar == value) return;
    messageShowAvatar = value;
    _preferences?.setBool(_messageShowAvatarKey, value);
    notifyListeners();
  }

  void setMessageShowIdentityNumber(bool value) {
    if (messageShowIdentityNumber == value) return;
    messageShowIdentityNumber = value;
    _preferences?.setBool(_messageShowIdentityNumberKey, value);
    notifyListeners();
  }

  void setMessagePreview(bool value) {
    if (messagePreview == value) return;
    messagePreview = value;
    _preferences?.setBool(_messagePreviewKey, value);
    notifyListeners();
  }

  void setPhotoAutoDownload(bool value) {
    if (photoAutoDownload == value) return;
    photoAutoDownload = value;
    _preferences?.setBool(_photoAutoDownloadKey, value);
    notifyListeners();
  }

  void setVideoAutoDownload(bool value) {
    if (videoAutoDownload == value) return;
    videoAutoDownload = value;
    _preferences?.setBool(_videoAutoDownloadKey, value);
    notifyListeners();
  }

  void setFileAutoDownload(bool value) {
    if (fileAutoDownload == value) return;
    fileAutoDownload = value;
    _preferences?.setBool(_fileAutoDownloadKey, value);
    notifyListeners();
  }

  void setChatFontSizeDelta(double value) {
    if (chatFontSizeDelta == value) return;
    chatFontSizeDelta = value;
    _preferences?.setDouble(_chatFontSizeDeltaKey, value);
    notifyListeners();
  }

  ThemeMode _themeModeFromName(String? value) {
    for (final mode in ThemeMode.values) {
      if (mode.name == value) return mode;
    }
    return ThemeMode.system;
  }
}
