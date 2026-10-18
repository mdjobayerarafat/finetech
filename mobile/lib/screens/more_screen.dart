// "More" — the catch-all surface for settings, subscriptions,
// audit, about, sign out. The mobile equivalent of the web sidebar.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api.dart';
import '../core/api_error.dart';
import '../core/i18n.dart';
import '../core/language.dart';
import '../core/money.dart';
import '../models/subscription.dart';
import '../state/auth.dart';
import '../state/settings.dart';
import '../theme/app_theme.dart';
import '../widgets/empty_state.dart';
import '../widgets/glass_card.dart';
import 'admin_screen.dart';
import 'budgets_screen.dart';
import 'forecast_screen.dart';
import 'goals_screen.dart';
import 'profile_screen.dart';
import 'receipt_screen.dart';

final subsProvider = FutureProvider.autoDispose<List<Subscription>>((ref) async {
  return ref.watch(apiProvider).subscriptions();
});

final auditProvider = FutureProvider.autoDispose((ref) async {
  final res = await ref.read(apiClientProvider).get('/audit?limit=50');
  return (res as List).cast<Map<String, dynamic>>();
});

class MoreScreen extends ConsumerWidget {
  const MoreScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    final user = ref.watch(currentUserProvider);
    return SafeArea(
      child: SingleChildScrollView(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 80),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              tr.t('more.title'),
              style: const TextStyle(
                fontSize: 22,
                fontWeight: FontWeight.w700,
              ),
            ),
            if (user != null) ...[
              const SizedBox(height: 8),
              Text(
                '${tr.t('shell.signed_in_as')} ${user.displayName}'
                '${user.isAdmin ? '  ·  ${tr.t('brand.admin')}' : ''}',
                style: const TextStyle(
                  color: AppColors.textMuted,
                  fontSize: 13,
                ),
              ),
            ],
            if (user?.isLocked == true) ...[
              const SizedBox(height: 12),
              GlassCard(
                border: Border.all(
                    color: AppColors.tierRed.withValues(alpha: 0.5)),
                child: Row(
                  children: [
                    const Icon(Icons.lock_outline,
                        color: AppColors.tierRed, size: 18),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        '${tr.t('shell.locked.title')} — ${tr.t('shell.locked.body')}',
                        style: const TextStyle(
                          color: AppColors.tierRed,
                          fontSize: 12,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ],
            const SizedBox(height: 14),
            _NavRow(
              icon: Icons.subscriptions_outlined,
              label: tr.t('more.subscriptions'),
              onTap: () => Navigator.of(context).push(
                MaterialPageRoute(
                  builder: (_) => const _SubscriptionsPage(),
                ),
              ),
            ),
            _NavRow(
              icon: Icons.account_balance_wallet_outlined,
              label: tr.t('more.budgets'),
              onTap: () => Navigator.of(context).push(
                MaterialPageRoute(builder: (_) => const BudgetsScreen()),
              ),
            ),
            _NavRow(
              icon: Icons.flag_outlined,
              label: 'Goals',
              onTap: () => Navigator.of(context).push(
                MaterialPageRoute(builder: (_) => const GoalsScreen()),
              ),
            ),
            _NavRow(
              icon: Icons.trending_up,
              label: tr.t('more.forecast'),
              onTap: () => Navigator.of(context).push(
                MaterialPageRoute(builder: (_) => const ForecastScreen()),
              ),
            ),
            _NavRow(
              icon: Icons.receipt,
              label: 'Receipt scan',
              onTap: () => Navigator.of(context).push(
                MaterialPageRoute(builder: (_) => const ReceiptScreen()),
              ),
            ),
            _NavRow(
              icon: Icons.receipt_long,
              label: tr.t('more.audit'),
              onTap: () => Navigator.of(context).push(
                MaterialPageRoute(builder: (_) => const _AuditPage()),
              ),
            ),
            if (user?.isAdmin == true)
              _NavRow(
                icon: Icons.admin_panel_settings,
                label: tr.t('brand.admin'),
                onTap: () => Navigator.of(context).push(
                  MaterialPageRoute(builder: (_) => const AdminScreen()),
                ),
              ),
            _NavRow(
              icon: Icons.person_outline,
              label: tr.t('more.about'),
              onTap: () => Navigator.of(context).push(
                MaterialPageRoute(builder: (_) => const ProfileScreen()),
              ),
            ),
            const SizedBox(height: 12),
            Text(
              tr.t('more.language'),
              style: const TextStyle(
                fontSize: 13,
                color: AppColors.textMuted,
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 6),
            _LangPicker(),
            const SizedBox(height: 18),
            _NavRow(
              icon: Icons.logout,
              label: tr.t('more.signout'),
              color: AppColors.tierRed,
              onTap: () async {
                await ref.read(authProvider.notifier).logout();
              },
            ),
            const SizedBox(height: 18),
            Center(
              child: Text(
                tr.t('more.sim_banner'),
                textAlign: TextAlign.center,
                style: const TextStyle(
                  fontSize: 11,
                  color: AppColors.textFaint,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _NavRow extends StatelessWidget {
  final IconData icon;
  final String label;
  final VoidCallback onTap;
  final Color? color;
  const _NavRow({
    required this.icon,
    required this.label,
    required this.onTap,
    this.color,
  });

  @override
  Widget build(BuildContext context) {
    final c = color ?? AppColors.text;
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: GlassCard(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        onTap: onTap,
        child: Row(
          children: [
            Icon(icon, color: c, size: 18),
            const SizedBox(width: 10),
            Expanded(
              child: Text(
                label,
                style: TextStyle(fontSize: 14, color: c),
              ),
            ),
            const Icon(Icons.arrow_forward_ios,
                size: 12, color: AppColors.textFaint),
          ],
        ),
      ),
    );
  }
}

class _LangPicker extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lang = ref.watch(settingsProvider).language;
    return Row(
      children: [
        Expanded(
          child: _LangButton(
            label: 'English',
            selected: lang == AppLanguage.en,
            onTap: () => ref
                .read(settingsProvider.notifier)
                .setLanguage(AppLanguage.en),
          ),
        ),
        const SizedBox(width: 8),
        Expanded(
          child: _LangButton(
            label: '中文',
            selected: lang == AppLanguage.zh,
            onTap: () => ref
                .read(settingsProvider.notifier)
                .setLanguage(AppLanguage.zh),
          ),
        ),
      ],
    );
  }
}

class _LangButton extends StatelessWidget {
  final String label;
  final bool selected;
  final VoidCallback onTap;
  const _LangButton({
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

class _SubscriptionsPage extends ConsumerWidget {
  const _SubscriptionsPage();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    final asyncSubs = ref.watch(subsProvider);
    return Scaffold(
      appBar: AppBar(title: Text(tr.t('subs.title'))),
      body: asyncSubs.when(
        data: (list) {
          if (list.isEmpty) {
            return EmptyState(
              icon: Icons.subscriptions_outlined,
              label: tr.t('subs.empty'),
            );
          }
          return ListView.separated(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
            itemCount: list.length,
            separatorBuilder: (_, __) => const SizedBox(height: 8),
            itemBuilder: (c, i) => _SubTile(sub: list[i], tr: tr),
          );
        },
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Padding(
          padding: const EdgeInsets.all(20),
          child: Text('$e',
              style: const TextStyle(color: AppColors.tierRed)),
        ),
      ),
    );
  }
}

class _SubTile extends ConsumerWidget {
  final Subscription sub;
  final Tr tr;
  const _SubTile({required this.sub, required this.tr});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return GlassCard(
      child: Row(
        children: [
          Container(
            width: 40,
            height: 40,
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(10),
              color: AppColors.surfaceHi,
            ),
            child: const Icon(Icons.receipt_long_outlined,
                color: AppColors.accent2, size: 18),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  sub.merchant,
                  style: const TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Text(
                  '${tr.t('subs.next')}: ${sub.nextExpectedAt.toString().substring(0, 10)}',
                  style: const TextStyle(
                    fontSize: 11,
                    color: AppColors.textMuted,
                  ),
                ),
              ],
            ),
          ),
          Text(
            fenToCny(sub.estimatedAmountFen),
            style: const TextStyle(
              fontSize: 14,
              fontWeight: FontWeight.w700,
            ),
          ),
          const SizedBox(width: 8),
          TextButton(
            onPressed: sub.cancellationStatus == 'cancelled'
                ? null
                : () => _confirmCancel(context, ref),
            child: Text(tr.t('subs.cancel')),
          ),
        ],
      ),
    );
  }

  Future<void> _confirmCancel(BuildContext context, WidgetRef ref) async {
    final lang = ref.read(settingsProvider).language;
    final tr = Tr.of(lang);
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(tr.t('subs.cancel')),
        content: Text(tr.t('subs.cancel.confirm')),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(tr.t('common.cancel')),
          ),
          ElevatedButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(tr.t('common.confirm')),
          ),
        ],
      ),
    );
    if (ok == true) {
      try {
        await ref.read(apiProvider).cancelSubscription(sub.id);
        ref.invalidate(subsProvider);
      } catch (e) {
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
                content: Text(e is ApiError ? e.message : '$e')),
          );
        }
      }
    }
  }
}

class _AuditPage extends ConsumerWidget {
  const _AuditPage();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    final async = ref.watch(auditProvider);
    return Scaffold(
      appBar: AppBar(title: Text(tr.t('audit.title'))),
      body: async.when(
        data: (events) {
          if (events.isEmpty) {
            return EmptyState(
              icon: Icons.history_edu_outlined,
              label: tr.t('audit.empty'),
            );
          }
          return ListView.separated(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
            itemCount: events.length,
            separatorBuilder: (_, __) => const SizedBox(height: 6),
            itemBuilder: (c, i) {
              final e = events[i];
              return GlassCard(
                padding: const EdgeInsets.all(12),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      '${e['action'] ?? '—'}',
                      style: const TextStyle(
                        fontSize: 13,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      '${e['tool'] ?? ''}',
                      style: const TextStyle(
                        fontSize: 11,
                        color: AppColors.textMuted,
                        fontFamily: 'monospace',
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      '${e['created_at'] ?? ''}',
                      style: const TextStyle(
                        fontSize: 10,
                        color: AppColors.textFaint,
                      ),
                    ),
                  ],
                ),
              );
            },
          );
        },
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Padding(
          padding: const EdgeInsets.all(20),
          child: Text('$e',
              style: const TextStyle(color: AppColors.tierRed)),
        ),
      ),
    );
  }
}