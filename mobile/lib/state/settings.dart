// App-wide settings state: language + UI prefs, persisted to disk.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../core/language.dart';

class SettingsState {
  final AppLanguage language;
  const SettingsState({this.language = AppLanguage.en});
  SettingsState copyWith({AppLanguage? language}) =>
      SettingsState(language: language ?? this.language);
}

class SettingsNotifier extends StateNotifier<SettingsState> {
  SettingsNotifier() : super(const SettingsState()) {
    _load();
  }

  Future<void> _load() async {
    final prefs = await SharedPreferences.getInstance();
    final code = prefs.getString('app_language');
    if (code != null) {
      state = state.copyWith(language: AppLanguageX.fromCode(code));
    }
  }

  Future<void> setLanguage(AppLanguage lang) async {
    state = state.copyWith(language: lang);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('app_language', lang.code);
  }
}

final settingsProvider =
    StateNotifierProvider<SettingsNotifier, SettingsState>(
  (ref) => SettingsNotifier(),
);