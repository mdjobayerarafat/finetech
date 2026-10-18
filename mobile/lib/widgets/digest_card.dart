// Daily digest card — shown on the dashboard for quick context.
// Pulls /insights/daily and shows renewals + goals + anomaly count.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../core/api.dart';
import '../core/money.dart';
import '../theme/app_theme.dart';
import 'glass_card.dart';

final digestProvider = FutureProvider.autoDispose((ref) async {
  return ref.watch(apiProvider).dailyDigest();
});

class DigestCard extends ConsumerWidget {
  const DigestCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final async = ref.watch(digestProvider);
    return async.when(
      data: (d) {
        final items = (d['items'] as Map?)?.cast<String, dynamic>() ?? const {};
        final renewals =
            ((items['renewals'] as List?) ?? const []).cast<Map>();
        final goals = ((items['goals'] as List?) ?? const []).cast<Map>();
        final anomalyCount = (items['anomaly_count_24h'] as num?)?.toInt() ?? 0;
        final spend = (items['spend_24h_fen'] as num?)?.toInt() ?? 0;
        if (renewals.isEmpty &&
            goals.isEmpty &&
            anomalyCount == 0 &&
            spend == 0) {
          return const SizedBox.shrink();
        }
        return GlassCard(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  const Icon(Icons.bolt, color: AppColors.accent2, size: 18),
                  const SizedBox(width: 6),
                  const Text(
                    'Today',
                    style: TextStyle(
                      fontSize: 13,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const Spacer(),
                  if (anomalyCount > 0)
                    Container(
                      padding: const EdgeInsets.symmetric(
                          horizontal: 8, vertical: 3),
                      decoration: BoxDecoration(
                        color: AppColors.tierYellow.withValues(alpha: 0.15),
                        borderRadius: BorderRadius.circular(999),
                      ),
                      child: Text(
                        '$anomalyCount anomalies',
                        style: const TextStyle(
                          color: AppColors.tierYellow,
                          fontSize: 11,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                ],
              ),
              const SizedBox(height: 4),
              Text(
                'Spent 24h: ${fenToCny(spend)}',
                style: const TextStyle(
                  fontSize: 11,
                  color: AppColors.textMuted,
                ),
              ),
              if (renewals.isNotEmpty) ...[
                const SizedBox(height: 8),
                for (final r in renewals.take(3))
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 2),
                    child: Row(
                      children: [
                        const Icon(Icons.autorenew,
                            size: 12, color: AppColors.textMuted),
                        const SizedBox(width: 4),
                        Expanded(
                          child: Text(
                            r['merchant'] as String? ?? '—',
                            style: const TextStyle(fontSize: 12),
                          ),
                        ),
                        Text(
                          fenToCny((r['amount_fen'] as num).toInt()),
                          style: const TextStyle(
                            fontSize: 12,
                            color: AppColors.textMuted,
                          ),
                        ),
                      ],
                    ),
                  ),
              ],
              if (goals.isNotEmpty) ...[
                const SizedBox(height: 8),
                for (final g in goals.take(3))
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 2),
                    child: Row(
                      children: [
                        const Icon(Icons.flag,
                            size: 12, color: AppColors.textMuted),
                        const SizedBox(width: 4),
                        Expanded(
                          child: Text(
                            g['name'] as String? ?? '—',
                            style: const TextStyle(fontSize: 12),
                          ),
                        ),
                        Text(
                          '${(g['progress_pct'] as num?)?.toInt() ?? 0}%',
                          style: const TextStyle(
                            fontSize: 12,
                            color: AppColors.textMuted,
                          ),
                        ),
                      ],
                    ),
                  ),
              ],
              if (d['generated_at'] is String) ...[
                const SizedBox(height: 6),
                Text(
                  'Generated ${_shortDateTime(d['generated_at'] as String)}',
                  style: const TextStyle(
                    fontSize: 10,
                    color: AppColors.textFaint,
                  ),
                ),
              ],
            ],
          ),
        );
      },
      loading: () => const SizedBox(height: 60),
      error: (_, __) => const SizedBox.shrink(),
    );
  }

  String _shortDateTime(String iso) {
    try {
      final t = DateTime.parse(iso).toLocal();
      return DateFormat('MM-dd HH:mm').format(t);
    } catch (_) {
      return iso;
    }
  }
}
