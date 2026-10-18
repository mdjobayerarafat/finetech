// Login screen — bilingual, with quick-fill demo accounts.
// The web frontend pre-fills demo credentials from a card; we do the
// same so reviewers can sign in with one tap.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/i18n.dart';
import '../core/language.dart';
import '../state/auth.dart';
import '../state/settings.dart';
import '../theme/app_theme.dart';
import '../theme/aurora.dart';
import '../widgets/glass_card.dart';

class LoginScreen extends ConsumerStatefulWidget {
  const LoginScreen({super.key});
  @override
  ConsumerState<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends ConsumerState<LoginScreen> {
  final _user = TextEditingController();
  final _pass = TextEditingController();
  final _userFocus = FocusNode();
  final _passFocus = FocusNode();
  bool _obscure = true;
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    _user.dispose();
    _pass.dispose();
    _userFocus.dispose();
    _passFocus.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final u = _user.text.trim();
    final p = _pass.text;
    if (u.isEmpty || p.isEmpty) {
      setState(() => _error = 'Fill both fields.');
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    final ok = await ref.read(authProvider.notifier).login(u, p);
    if (!mounted) return;
    setState(() {
      _busy = false;
      _error = ok ? null : ref.read(authProvider).error;
    });
  }

  void _fillDemo(String username, String password) {
    setState(() {
      _user.text = username;
      _pass.text = password;
      _error = null;
    });
    _userFocus.unfocus();
    _passFocus.unfocus();
  }

  @override
  Widget build(BuildContext context) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    return Scaffold(
      body: Stack(
        children: [
          const Positioned.fill(child: AuroraBackdrop()),
          SafeArea(
            child: SingleChildScrollView(
              padding: const EdgeInsets.fromLTRB(20, 24, 20, 32),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Row(
                    children: [
                      Container(
                        width: 36,
                        height: 36,
                        decoration: BoxDecoration(
                          borderRadius: BorderRadius.circular(10),
                          gradient: const LinearGradient(
                            colors: AppColors.accentGradient,
                          ),
                        ),
                        child: const Icon(
                          Icons.auto_awesome,
                          color: Colors.white,
                          size: 20,
                        ),
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: Text(
                          tr.t('brand.name'),
                          style: const TextStyle(
                            fontSize: 18,
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                      ),
                      _LangToggle(
                        current: lang,
                        onChange: (l) =>
                            ref.read(settingsProvider.notifier).setLanguage(l),
                      ),
                    ],
                  ),
                  const SizedBox(height: 24),
                  GlassCard(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          tr.t('login.title'),
                          style: const TextStyle(
                            fontSize: 22,
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                        const SizedBox(height: 4),
                        Text(
                          tr.t('login.subtitle'),
                          style: const TextStyle(
                            color: AppColors.textMuted,
                            fontSize: 13,
                          ),
                        ),
                        const SizedBox(height: 18),
                        TextField(
                          controller: _user,
                          focusNode: _userFocus,
                          textInputAction: TextInputAction.next,
                          autocorrect: false,
                          enableSuggestions: false,
                          decoration: InputDecoration(
                            labelText: tr.t('login.username'),
                          ),
                          onSubmitted: (_) => _passFocus.requestFocus(),
                        ),
                        const SizedBox(height: 10),
                        TextField(
                          controller: _pass,
                          focusNode: _passFocus,
                          obscureText: _obscure,
                          textInputAction: TextInputAction.done,
                          decoration: InputDecoration(
                            labelText: tr.t('login.password'),
                            suffixIcon: IconButton(
                              icon: Icon(_obscure
                                  ? Icons.visibility_outlined
                                  : Icons.visibility_off_outlined),
                              onPressed: () =>
                                  setState(() => _obscure = !_obscure),
                            ),
                          ),
                          onSubmitted: (_) => _submit(),
                        ),
                        if (_error != null) ...[
                          const SizedBox(height: 10),
                          Text(
                            '${tr.t('login.signInFailed')}: $_error',
                            style: const TextStyle(
                              color: AppColors.tierRed,
                              fontSize: 12,
                            ),
                          ),
                        ],
                        const SizedBox(height: 14),
                        ElevatedButton(
                          onPressed: _busy ? null : _submit,
                          child: _busy
                              ? SizedBox(
                                  height: 18,
                                  width: 18,
                                  child: const CircularProgressIndicator(
                                    strokeWidth: 2,
                                    color: Colors.white,
                                  ),
                                )
                              : Text(tr.t('login.submit')),
                        ),
                        const SizedBox(height: 10),
                        Text(
                          tr.t('login.hero.body'),
                          style: const TextStyle(
                            fontSize: 12,
                            color: AppColors.textMuted,
                            height: 1.4,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: 18),
                  Text(
                    tr.t('login.demo.title'),
                    style: const TextStyle(
                      fontSize: 13,
                      fontWeight: FontWeight.w600,
                      color: AppColors.textMuted,
                    ),
                  ),
                  const SizedBox(height: 8),
                  Text(
                    tr.t('login.demo.hint'),
                    style: const TextStyle(
                      fontSize: 11,
                      color: AppColors.textFaint,
                    ),
                  ),
                  const SizedBox(height: 10),
                  ..._demoAccounts(tr, lang).map(
                    (d) => Padding(
                      padding: const EdgeInsets.only(bottom: 8),
                      child: _DemoRow(d: d, onTap: _fillDemo),
                    ),
                  ),
                  const SizedBox(height: 16),
                  Text(
                    tr.t('brand.disclaimer'),
                    textAlign: TextAlign.center,
                    style: const TextStyle(
                      fontSize: 11,
                      color: AppColors.textFaint,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  List<_DemoAccount> _demoAccounts(Tr tr, AppLanguage lang) {
    return [
      _DemoAccount(
        username: 'demo',
        password: 'demo1234',
        label: tr.t('demo.customer.alex.label'),
        sub: tr.t('demo.customer.alex.sub'),
        icon: Icons.person_outline,
      ),
      _DemoAccount(
        username: 'intl_student',
        password: 'demo1234',
        label: tr.t('demo.customer.mei.label'),
        sub: tr.t('demo.customer.mei.sub'),
        icon: Icons.school_outlined,
      ),
      _DemoAccount(
        username: 'admin',
        password: 'admin1234',
        label: tr.t('demo.admin.label'),
        sub: tr.t('demo.admin.sub'),
        icon: Icons.admin_panel_settings_outlined,
      ),
    ];
  }
}

class _DemoAccount {
  final String username;
  final String password;
  final String label;
  final String sub;
  final IconData icon;
  const _DemoAccount({
    required this.username,
    required this.password,
    required this.label,
    required this.sub,
    required this.icon,
  });
}

class _DemoRow extends StatelessWidget {
  final _DemoAccount d;
  final void Function(String, String) onTap;
  const _DemoRow({required this.d, required this.onTap});

  @override
  Widget build(BuildContext context) {
    return GlassCard(
      padding: const EdgeInsets.all(12),
      onTap: () => onTap(d.username, d.password),
      child: Row(
        children: [
          Container(
            width: 32,
            height: 32,
            decoration: BoxDecoration(
              color: AppColors.surfaceHi,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Icon(d.icon, size: 16, color: AppColors.accent2),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  d.label,
                  style: const TextStyle(
                    fontSize: 13,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Text(
                  d.sub,
                  style: const TextStyle(
                    fontSize: 11,
                    color: AppColors.textMuted,
                  ),
                ),
              ],
            ),
          ),
          const Icon(Icons.arrow_forward_ios, size: 12, color: AppColors.textFaint),
        ],
      ),
    );
  }
}

class _LangToggle extends StatelessWidget {
  final AppLanguage current;
  final ValueChanged<AppLanguage> onChange;
  const _LangToggle({required this.current, required this.onChange});

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        color: AppColors.surface,
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: AppColors.line),
      ),
      padding: const EdgeInsets.all(2),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          _pill('EN', current == AppLanguage.en,
              () => onChange(AppLanguage.en)),
          _pill('中', current == AppLanguage.zh,
              () => onChange(AppLanguage.zh)),
        ],
      ),
    );
  }

  Widget _pill(String label, bool active, VoidCallback onTap) {
    return InkWell(
      borderRadius: BorderRadius.circular(999),
      onTap: onTap,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 180),
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        decoration: BoxDecoration(
          color: active ? AppColors.accent : Colors.transparent,
          borderRadius: BorderRadius.circular(999),
        ),
        child: Text(
          label,
          style: TextStyle(
            color: active ? Colors.white : AppColors.textMuted,
            fontSize: 12,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
    );
  }
}