// UI language. The web frontend uses 'en' and 'zh'. We mirror the same
// two values so the user's saved preference can round-trip with the
// backend and the web client.

enum AppLanguage { en, zh }

extension AppLanguageX on AppLanguage {
  String get code => this == AppLanguage.en ? 'en' : 'zh';
  String get display => this == AppLanguage.en ? 'English' : '中文';

  static AppLanguage fromCode(String? code) {
    if (code == 'zh') return AppLanguage.zh;
    return AppLanguage.en;
  }
}
