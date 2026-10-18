// Dashboard — greeting, total balance across accounts, spending
// breakdown (pie), recent transactions.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:fl_chart/fl_chart.dart';

import '../core/api.dart';
import '../core/i18n.dart';
import '../core/money.dart';
import '../models/account.dart';
import '../state/auth.dart';
import '../state/settings.dart';
import '../theme/app_theme.dart';
import '../widgets/account_card.dart';
import '../widgets/digest_card.dart';
import '../widgets/glass_card.dart';
import 'account_detail_screen.dart';

final accountsProvider = FutureProvider.autoDispose<List<Account>>((ref) async {
  return ref.watch(apiProvider).accounts();
});

final transactionsProvider =
    FutureProvider.autoDispose<List<Txn>>((ref) async {
  return ref.watch(apiProvider).transactions(limit: 15);
});

final spendingProvider = FutureProvider.autoDispose((ref) async {
  return ref.watch(apiProvider).spending();
});

class DashboardScreen extends ConsumerWidget {
  const DashboardScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    final user = ref.watch(currentUserProvider);

    return SafeArea(
      child: RefreshIndicator(
        onRefresh: () async {
          ref.invalidate(accountsProvider);
          ref.invalidate(transactionsProvider);
          ref.invalidate(spendingProvider);
          await Future.wait([
            ref.read(accountsProvider.future),
            ref.read(transactionsProvider.future),
            ref.read(spendingProvider.future),
          ]);
        },
        child: SingleChildScrollView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 80),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                user == null
                    ? tr.t('dashboard.title')
                    : tr.t('dashboard.hello',
                        params: {'name': user.displayName}),
                style: const TextStyle(
                  fontSize: 22,
                  fontWeight: FontWeight.w700,
                ),
              ),
              const SizedBox(height: 4),
              Text(
                tr.t('brand.disclaimer'),
                style: const TextStyle(
                  fontSize: 11,
                  color: AppColors.textFaint,
                ),
              ),
              const SizedBox(height: 16),
              _TotalBalanceCard(),
              const SizedBox(height: 16),
              const DigestCard(),
              const SizedBox(height: 16),
              _SectionHeader(label: tr.t('dashboard.accounts')),
              const SizedBox(height: 8),
              _AccountsList(),
              const SizedBox(height: 18),
              _SectionHeader(label: tr.t('dashboard.spending')),
              const SizedBox(height: 8),
              _SpendingCard(),
              const SizedBox(height: 18),
              _SectionHeader(label: tr.t('dashboard.recent')),
              const SizedBox(height: 8),
              _RecentList(),
            ],
          ),
        ),
      ),
    );
  }
}

class _TotalBalanceCard extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final asyncAccounts = ref.watch(accountsProvider);
    return GlassCard(
      padding: const EdgeInsets.all(20),
      child: asyncAccounts.when(
        data: (list) {
          final total =
              list.fold<int>(0, (acc, a) => acc + a.totalBalanceFen);
          final lang = ref.watch(settingsProvider).language;
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                Tr.of(lang).t('dashboard.total_balance'),
                style: const TextStyle(
                  fontSize: 12,
                  color: AppColors.textMuted,
                ),
              ),
              const SizedBox(height: 6),
              Text(
                fenToCny(total),
                style: const TextStyle(
                  fontSize: 30,
                  fontWeight: FontWeight.w800,
                  letterSpacing: -0.5,
                ),
              ),
              const SizedBox(height: 6),
              Row(
                children: [
                  Container(
                    width: 6,
                    height: 6,
                    decoration: const BoxDecoration(
                      color: AppColors.tierGreen,
                      shape: BoxShape.circle,
                    ),
                  ),
                  const SizedBox(width: 6),
                  Text(
                    '${list.length} account${list.length == 1 ? '' : 's'}',
                    style: const TextStyle(
                      fontSize: 11,
                      color: AppColors.textMuted,
                    ),
                  ),
                ],
              ),
            ],
          );
        },
        loading: () => const SizedBox(
          height: 100,
          child: Center(
            child: SizedBox(
              width: 22,
              height: 22,
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
          ),
        ),
        error: (e, _) => Text(
          '$e',
          style: const TextStyle(color: AppColors.tierRed, fontSize: 12),
        ),
      ),
    );
  }
}

class _AccountsList extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final asyncAccounts = ref.watch(accountsProvider);
    return asyncAccounts.when(
      data: (list) => Column(
        children: [
          for (final a in list)
            Padding(
              padding: const EdgeInsets.only(bottom: 10),
              child: AccountCard(
                account: a,
                onTap: () => Navigator.of(context).push(
                  MaterialPageRoute(
                    builder: (_) => AccountDetailScreen(account: a),
                  ),
                ),
              ),
            ),
        ],
      ),
      loading: () => const SizedBox(height: 80),
      error: (e, _) => Text('$e',
          style: const TextStyle(color: AppColors.tierRed, fontSize: 12)),
    );
  }
}

class _SpendingCard extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final asyncSpending = ref.watch(spendingProvider);
    return GlassCard(
      padding: const EdgeInsets.all(14),
      child: asyncSpending.when(
        data: (report) {
          if (report.items.isEmpty) {
            return Text(
              Tr.of(ref.watch(settingsProvider).language)
                  .t('dashboard.no_transactions'),
              style: const TextStyle(
                color: AppColors.textMuted,
                fontSize: 13,
              ),
            );
          }
          final top = report.items.take(5).toList();
          final palette = [
            AppColors.accent,
            AppColors.accent2,
            AppColors.accent3,
            AppColors.tierYellow,
            AppColors.tierGreen,
          ];
          return Row(
            children: [
              SizedBox(
                width: 130,
                height: 130,
                child: PieChart(
                  PieChartData(
                    sections: [
                      for (var i = 0; i < top.length; i++)
                        PieChartSectionData(
                          color: palette[i % palette.length],
                          value: top[i].totalFen.toDouble(),
                          radius: 28,
                          showTitle: false,
                        ),
                    ],
                    centerSpaceRadius: 28,
                    sectionsSpace: 2,
                  ),
                ),
              ),
              const SizedBox(width: 14),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    for (var i = 0; i < top.length; i++)
                      Padding(
                        padding: const EdgeInsets.only(bottom: 6),
                        child: Row(
                          children: [
                            Container(
                              width: 8,
                              height: 8,
                              decoration: BoxDecoration(
                                color: palette[i % palette.length],
                                shape: BoxShape.circle,
                              ),
                            ),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text(
                                top[i].category,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: const TextStyle(fontSize: 12),
                              ),
                            ),
                            Text(
                              fenToCny(top[i].totalFen),
                              style: const TextStyle(
                                fontSize: 12,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                          ],
                        ),
                      ),
                  ],
                ),
              ),
            ],
          );
        },
        loading: () => const SizedBox(height: 130),
        error: (e, _) => Text('$e',
            style: const TextStyle(color: AppColors.tierRed, fontSize: 12)),
      ),
    );
  }
}

class _RecentList extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final asyncTxns = ref.watch(transactionsProvider);
    final lang = ref.watch(settingsProvider).language;
    return asyncTxns.when(
      data: (list) {
        if (list.isEmpty) {
          return GlassCard(
            child: Text(
              Tr.of(lang).t('dashboard.no_transactions'),
              style: const TextStyle(
                color: AppColors.textMuted,
                fontSize: 13,
              ),
            ),
          );
        }
        return Column(
          children: [
            for (final t in list)
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: TransactionTile(txn: t),
              ),
          ],
        );
      },
      loading: () => const SizedBox(height: 80),
      error: (e, _) => Text('$e',
          style: const TextStyle(color: AppColors.tierRed, fontSize: 12)),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  final String label;
  const _SectionHeader({required this.label});

  @override
  Widget build(BuildContext context) {
    return Text(
      label,
      style: const TextStyle(
        fontSize: 13,
        color: AppColors.textMuted,
        fontWeight: FontWeight.w600,
        letterSpacing: 0.4,
      ),
    );
  }
}