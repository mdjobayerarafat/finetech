// Goals — list savings goals with progress + contribute flow.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api.dart';
import '../core/api_error.dart';
import '../core/i18n.dart';
import '../core/money.dart';
import '../state/settings.dart';
import '../theme/app_theme.dart';
import '../widgets/empty_state.dart';
import '../widgets/glass_card.dart';

final goalsProvider = FutureProvider.autoDispose((ref) async {
  return ref.watch(apiProvider).goals();
});

final accountsForGoalProvider = FutureProvider.autoDispose((ref) async {
  return ref.watch(apiProvider).accounts();
});

class GoalsScreen extends ConsumerWidget {
  const GoalsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    final asyncGoals = ref.watch(goalsProvider);

    return Scaffold(
      appBar: AppBar(title: Text('Goals')),
      body: RefreshIndicator(
        onRefresh: () async {
          ref.invalidate(goalsProvider);
          await ref.read(goalsProvider.future);
        },
        child: asyncGoals.when(
          data: (list) {
            if (list.isEmpty) {
              return SizedBox(
                height: 300,
                child: EmptyState(
                  icon: Icons.flag_outlined,
                  label: 'No goals yet.',
                ),
              );
            }
            return ListView.separated(
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
              itemCount: list.length,
              separatorBuilder: (_, __) => const SizedBox(height: 8),
              itemBuilder: (c, i) {
                final g = list[i];
                return _GoalTile(goal: g, tr: tr);
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
      ),
    );
  }
}

class _GoalTile extends ConsumerWidget {
  final Map<String, dynamic> goal;
  final Tr tr;
  const _GoalTile({required this.goal, required this.tr});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final target = (goal['target_fen'] as num?)?.toInt() ?? 0;
    final saved = (goal['saved_fen'] as num?)?.toInt() ?? 0;
    final pct = target == 0 ? 0.0 : (saved / target).clamp(0, 1).toDouble();
    final color = pct >= 1
        ? AppColors.tierGreen
        : pct > 0.5
            ? AppColors.accent2
            : AppColors.accent;
    return GlassCard(
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                width: 32,
                height: 32,
                decoration: BoxDecoration(
                  color: color.withValues(alpha: 0.15),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Icon(Icons.flag, color: color, size: 16),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  goal['name'] as String? ?? '—',
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
            ],
          ),
          const SizedBox(height: 8),
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
                fenToCny(saved),
                style: const TextStyle(
                  fontSize: 12,
                  color: AppColors.textMuted,
                ),
              ),
              const Spacer(),
              Text(
                fenToCny(target),
                style: const TextStyle(
                  fontSize: 12,
                  color: AppColors.textMuted,
                ),
              ),
            ],
          ),
          if ((goal['days_to_goal'] as num?) != null) ...[
            const SizedBox(height: 4),
            Text(
              '${goal['days_to_goal']} days to goal',
              style: const TextStyle(
                fontSize: 11,
                color: AppColors.textFaint,
              ),
            ),
          ],
          const SizedBox(height: 8),
          Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              TextButton.icon(
                onPressed: () => _showContribute(context, ref, goal['id'] as String),
                icon: const Icon(Icons.add, size: 14),
                label: const Text('Contribute'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Future<void> _showContribute(
      BuildContext context, WidgetRef ref, String goalId) async {
    final asyncAccounts = ref.read(accountsForGoalProvider);
    final amt = TextEditingController();
    final memo = TextEditingController();
    String? sourceId;
    String? err;
    bool busy = false;
    final lang = ref.read(settingsProvider).language;
    final tr = Tr.of(lang);
    final ok = await showModalBottomSheet<bool>(
      context: context,
      isScrollControlled: true,
      backgroundColor: AppColors.surface,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (ctx) {
        return StatefulBuilder(builder: (ctx, setState) {
          return Padding(
            padding: EdgeInsets.fromLTRB(
              16,
              16,
              16,
              MediaQuery.of(ctx).viewInsets.bottom + 16,
            ),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  tr.t('common.amount'),
                  style: const TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 12),
                asyncAccounts.when(
                  data: (list) {
                    sourceId ??= list.isNotEmpty ? list.first.id : null;
                    return DropdownButtonFormField<String>(
                      initialValue: sourceId,
                      decoration: const InputDecoration(labelText: 'From'),
                      items: [
                        for (final a in list)
                          DropdownMenuItem(
                            value: a.id,
                            child: Text(a.displayName),
                          ),
                      ],
                      onChanged: (v) => setState(() => sourceId = v),
                    );
                  },
                  loading: () => const LinearProgressIndicator(),
                  error: (e, _) => Text('$e',
                      style: const TextStyle(color: AppColors.tierRed)),
                ),
                const SizedBox(height: 10),
                TextField(
                  controller: amt,
                  keyboardType:
                      const TextInputType.numberWithOptions(decimal: true),
                  decoration: const InputDecoration(
                    labelText: 'Amount (CNY)',
                    prefixText: '¥ ',
                  ),
                ),
                const SizedBox(height: 10),
                TextField(
                  controller: memo,
                  decoration: InputDecoration(
                    labelText: 'Memo',
                    helperText: tr.t('common.optional'),
                  ),
                ),
                if (err != null) ...[
                  const SizedBox(height: 8),
                  Text(err!,
                      style: const TextStyle(
                          color: AppColors.tierRed, fontSize: 12)),
                ],
                const SizedBox(height: 12),
                ElevatedButton(
                  onPressed: busy
                      ? null
                      : () async {
                          final fen = parseCnyToFen(amt.text);
                          if (fen == null || fen <= 0 || sourceId == null) {
                            setState(() => err = 'Missing fields.');
                            return;
                          }
                          setState(() {
                            busy = true;
                            err = null;
                          });
                          try {
                            await ref
                                .read(apiProvider)
                                .contributeToGoal(
                                  goalId,
                                  fen,
                                  sourceId!,
                                  memo: memo.text,
                                );
                            if (ctx.mounted) Navigator.pop(ctx, true);
                          } catch (e) {
                            setState(() {
                              busy = false;
                              err = e is ApiError ? e.message : '$e';
                            });
                          }
                        },
                  child: Text(tr.t('common.confirm')),
                ),
              ],
            ),
          );
        });
      },
    );
    if (ok == true) {
      ref.invalidate(goalsProvider);
    }
  }
}
