// Profile — current user, language picker, sign out.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api.dart';
import '../core/i18n.dart';
import '../core/language.dart';
import '../state/auth.dart';
import '../state/settings.dart';
import '../theme/app_theme.dart';
import '../widgets/glass_card.dart';

class ProfileScreen extends ConsumerWidget {
  const ProfileScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    final user = ref.watch(currentUserProvider);
    return Scaffold(
      appBar: AppBar(title: Text(tr.t('more.about'))),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
        children: [
          GlassCard(
            child: Row(
              children: [
                Container(
                  width: 56,
                  height: 56,
                  decoration: const BoxDecoration(
                    shape: BoxShape.circle,
                    gradient: LinearGradient(
                      colors: AppColors.accentGradient,
                    ),
                  ),
                  child: Center(
                    child: Text(
                      (user?.displayName.isNotEmpty == true
                              ? user!.displayName[0]
                              : '?')
                          .toUpperCase(),
                      style: const TextStyle(
                        color: Colors.white,
                        fontSize: 22,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        user?.displayName ?? '—',
                        style: const TextStyle(
                          fontSize: 16,
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      Text(
                        '@${user?.username ?? ''}  ·  ${user?.role ?? ''}',
                        style: const TextStyle(
                          fontSize: 12,
                          color: AppColors.textMuted,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          GlassCard(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  tr.t('more.language'),
                  style: const TextStyle(
                    fontSize: 12,
                    color: AppColors.textMuted,
                  ),
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    Expanded(
                      child: _LangTile(
                        label: 'English',
                        selected: lang == AppLanguage.en,
                        onTap: () async {
                          await ref
                              .read(settingsProvider.notifier)
                              .setLanguage(AppLanguage.en);
                          if (user != null) {
                            try {
                              await ref
                                  .read(apiProvider)
                                  .setPreferredLanguage(AppLanguage.en);
                            } catch (_) {}
                          }
                        },
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: _LangTile(
                        label: '中文',
                        selected: lang == AppLanguage.zh,
                        onTap: () async {
                          await ref
                              .read(settingsProvider.notifier)
                              .setLanguage(AppLanguage.zh);
                          if (user != null) {
                            try {
                              await ref
                                  .read(apiProvider)
                                  .setPreferredLanguage(AppLanguage.zh);
                            } catch (_) {}
                          }
                        },
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
          if (user != null) ...[
            const SizedBox(height: 16),
            GlassCard(
              child: Column(
                children: [
                  _row('User ID', user.id),
                  _row('Role', user.role),
                  _row('Risk tolerance', user.riskTolerance),
                  _row('Horizon', user.horizon),
                  _row('Safety lock', user.safetyLockState),
                ],
              ),
            ),
          ],
          const SizedBox(height: 18),
          OutlinedButton.icon(
            onPressed: () async {
              await ref.read(authProvider.notifier).logout();
            },
            icon: const Icon(Icons.logout, size: 16),
            label: Text(tr.t('more.signout')),
            style: OutlinedButton.styleFrom(
              foregroundColor: AppColors.tierRed,
              side: const BorderSide(color: AppColors.tierRed),
            ),
          ),
          const SizedBox(height: 12),
          Text(
            tr.t('about.disclaimer'),
            textAlign: TextAlign.center,
            style: const TextStyle(
              fontSize: 11,
              color: AppColors.textFaint,
            ),
          ),
        ],
      ),
    );
  }

  Widget _row(String k, String v) => Padding(
        padding: const EdgeInsets.symmetric(vertical: 4),
        child: Row(
          children: [
            SizedBox(
              width: 120,
              child: Text(
                k,
                style: const TextStyle(
                  fontSize: 12,
                  color: AppColors.textMuted,
                ),
              ),
            ),
            Expanded(
              child: Text(
                v,
                style: const TextStyle(fontSize: 13),
              ),
            ),
          ],
        ),
      );
}

class _LangTile extends StatelessWidget {
  final String label;
  final bool selected;
  final VoidCallback onTap;
  const _LangTile({
    required this.label,
    required this.selected,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return InkWell(
      borderRadius: BorderRadius.circular(12),
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(vertical: 12),
        decoration: BoxDecoration(
          color: selected ? AppColors.accent : AppColors.surface,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(
            color: selected ? AppColors.accent : AppColors.line,
          ),
        ),
        alignment: Alignment.center,
        child: Text(
          label,
          style: TextStyle(
            color: selected ? Colors.white : AppColors.text,
            fontWeight: FontWeight.w600,
            fontSize: 14,
          ),
        ),
      ),
    );
  }
}
