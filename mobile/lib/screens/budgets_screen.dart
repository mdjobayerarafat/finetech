// Budgets — list + summary, set a new category cap, delete a cap.
// Mirrors /budgets in the web app.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api.dart';
import '../core/api_error.dart';
import '../core/i18n.dart';
import '../core/money.dart';
import '../models/budget.dart';
import '../state/settings.dart';
import '../theme/app_theme.dart';
import '../widgets/empty_state.dart';
import '../widgets/glass_card.dart';

final budgetsListProvider =
    FutureProvider.autoDispose<List<Budget>>((ref) async {
  return ref.watch(apiProvider).budgets();
});

final budgetSummaryProvider =
    FutureProvider.autoDispose<BudgetSummary>((ref) async {
  return ref.watch(apiProvider).budgetSummary();
});

class BudgetsScreen extends ConsumerWidget {
  const BudgetsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    final asyncList = ref.watch(budgetsListProvider);
    final asyncSummary = ref.watch(budgetSummaryProvider);

    return Scaffold(
      appBar: AppBar(
        title: Text(tr.t('budgets.title')),
        actions: [
          IconButton(
            icon: const Icon(Icons.add),
            onPressed: () => _showSet(context, ref),
          ),
        ],
      ),
      body: RefreshIndicator(
        onRefresh: () async {
          ref.invalidate(budgetsListProvider);
          ref.invalidate(budgetSummaryProvider);
          await Future.wait([
            ref.read(budgetsListProvider.future),
            ref.read(budgetSummaryProvider.future),
          ]);
        },
        child: ListView(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
          children: [
            asyncSummary.when(
              data: (s) => _SummaryCard(summary: s, tr: tr),
              loading: () => const SizedBox(height: 80),
              error: (e, _) => Text('$e',
                  style: const TextStyle(color: AppColors.tierRed)),
            ),
            const SizedBox(height: 12),
            asyncList.when(
              data: (list) {
                if (list.isEmpty) {
                  return SizedBox(
                    height: 200,
                    child: EmptyState(
                      icon: Icons.savings_outlined,
                      label: tr.t('budgets.empty'),
                    ),
                  );
                }
                return Column(
                  children: [
                    for (final b in list)
                      Padding(
                        padding: const EdgeInsets.only(bottom: 8),
                        child: _BudgetTile(budget: b, tr: tr),
                      ),
                  ],
                );
              },
              loading: () => const Padding(
                padding: EdgeInsets.all(20),
                child: Center(child: CircularProgressIndicator()),
              ),
              error: (e, _) => Text('$e',
                  style: const TextStyle(color: AppColors.tierRed)),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _showSet(BuildContext context, WidgetRef ref) async {
    final lang = ref.read(settingsProvider).language;
    final tr = Tr.of(lang);
    final cat = TextEditingController();
    final amt = TextEditingController();
    final note = TextEditingController();
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(tr.t('budgets.title')),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: cat,
                decoration: const InputDecoration(
                  labelText: 'Category (e.g. dining, shopping)',
                ),
              ),
              const SizedBox(height: 8),
              TextField(
                controller: amt,
                keyboardType:
                    const TextInputType.numberWithOptions(decimal: true),
                decoration: const InputDecoration(
                  labelText: 'Monthly cap (CNY)',
                  prefixText: '¥ ',
                ),
              ),
              const SizedBox(height: 8),
              TextField(
                controller: note,
                decoration: InputDecoration(
                  labelText: 'Note (optional)',
                  helperText: tr.t('common.optional'),
                ),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(tr.t('common.cancel')),
          ),
          ElevatedButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(tr.t('common.save')),
          ),
        ],
      ),
    );
    if (ok == true) {
      final fen = parseCnyToFen(amt.text);
      if (cat.text.trim().isEmpty || fen == null || fen <= 0) return;
      try {
        await ref.read(apiProvider).setBudget(
              category: cat.text.trim(),
              amountFen: fen,
              note: note.text.trim().isEmpty ? null : note.text.trim(),
            );
        ref.invalidate(budgetsListProvider);
        ref.invalidate(budgetSummaryProvider);
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

class _SummaryCard extends StatelessWidget {
  final BudgetSummary summary;
  final Tr tr;
  const _SummaryCard({required this.summary, required this.tr});

  @override
  Widget build(BuildContext context) {
    return GlassCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            '${summary.budgetCount} ${tr.t('budgets.title').toLowerCase()}',
            style: const TextStyle(
              fontSize: 12,
              color: AppColors.textMuted,
            ),
          ),
          const SizedBox(height: 6),
          Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      fenToCny(summary.totalSpentFen),
                      style: const TextStyle(
                        fontSize: 22,
                        fontWeight: FontWeight.w800,
                      ),
                    ),
                    Text(
                      '${tr.t('budgets.spent')} / ${fenToCny(summary.totalLimitFen)}',
                      style: const TextStyle(
                        fontSize: 11,
                        color: AppColors.textMuted,
                      ),
                    ),
                  ],
                ),
              ),
              Container(
                width: 80,
                height: 80,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  color: AppColors.surfaceHi,
                  borderRadius: BorderRadius.circular(999),
                ),
                child: Text(
                  '${(summary.totalSpentFen * 100 ~/ (summary.totalLimitFen == 0 ? 1 : summary.totalLimitFen))}%',
                  style: const TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
            ],
          ),
          if (summary.breached.isNotEmpty) ...[
            const SizedBox(height: 8),
            Text(
              '⚠ ${summary.breached.length} breached',
              style: const TextStyle(
                color: AppColors.tierRed,
                fontSize: 12,
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _BudgetTile extends ConsumerWidget {
  final Budget budget;
  final Tr tr;
  const _BudgetTile({required this.budget, required this.tr});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final pct = budget.utilizationPct.clamp(0, 100) / 100.0;
    final color = budget.breached
        ? AppColors.tierRed
        : pct > 0.8
            ? AppColors.tierYellow
            : AppColors.tierGreen;
    return GlassCard(
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  budget.category,
                  style: const TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              Text(
                '${(pct * 100).toStringAsFixed(0)}%',
                style: TextStyle(
                  color: color,
                  fontSize: 12,
                  fontWeight: FontWeight.w700,
                ),
              ),
              const SizedBox(width: 4),
              IconButton(
                icon: const Icon(Icons.delete_outline, size: 18),
                color: AppColors.textFaint,
                onPressed: () async {
                  try {
                    await ref.read(apiProvider).deleteBudget(budget.id);
                    ref.invalidate(budgetsListProvider);
                    ref.invalidate(budgetSummaryProvider);
                  } catch (_) {}
                },
              ),
            ],
          ),
          const SizedBox(height: 6),
          ClipRRect(
            borderRadius: BorderRadius.circular(4),
            child: LinearProgressIndicator(
              value: pct.toDouble(),
              minHeight: 6,
              backgroundColor: AppColors.surfaceHi,
              color: color,
            ),
          ),
          const SizedBox(height: 6),
          Row(
            children: [
              Text(
                '${tr.t('budgets.spent')} ${fenToCny(budget.spentFen)}',
                style: const TextStyle(
                  fontSize: 11,
                  color: AppColors.textMuted,
                ),
              ),
              const Spacer(),
              Text(
                '${tr.t('budgets.limit')} ${fenToCny(budget.limitFen)}',
                style: const TextStyle(
                  fontSize: 11,
                  color: AppColors.textMuted,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
