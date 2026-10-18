// Wealth — catalog + holdings. Tapping a product opens a subscribe
// sheet; tapping a holding opens a redeem sheet.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api.dart';
import '../core/api_error.dart';
import '../core/i18n.dart';
import '../core/money.dart';
import '../models/wealth.dart';
import '../state/settings.dart';
import '../theme/app_theme.dart';
import '../widgets/empty_state.dart';
import '../widgets/glass_card.dart';

final wealthProductsProvider =
    FutureProvider.autoDispose<List<WealthProduct>>((ref) async {
  return ref.watch(apiProvider).wealthProducts();
});

final wealthHoldingsProvider =
    FutureProvider.autoDispose<List<WealthHolding>>((ref) async {
  return ref.watch(apiProvider).wealthHoldings();
});

class WealthScreen extends ConsumerWidget {
  const WealthScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    final asyncProducts = ref.watch(wealthProductsProvider);
    final asyncHoldings = ref.watch(wealthHoldingsProvider);

    return SafeArea(
      child: RefreshIndicator(
        onRefresh: () async {
          ref.invalidate(wealthProductsProvider);
          ref.invalidate(wealthHoldingsProvider);
          await Future.wait([
            ref.read(wealthProductsProvider.future),
            ref.read(wealthHoldingsProvider.future),
          ]);
        },
        child: SingleChildScrollView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 80),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                tr.t('wealth.title'),
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
              Text(
                tr.t('wealth.holdings'),
                style: const TextStyle(
                  fontSize: 13,
                  color: AppColors.textMuted,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 8),
              asyncHoldings.when(
                data: (list) {
                  if (list.isEmpty) {
                    return GlassCard(
                      child: SizedBox(
                        height: 80,
                        child: Center(
                          child: Text(
                            tr.t('wealth.empty.holdings'),
                            style: const TextStyle(
                              color: AppColors.textMuted,
                              fontSize: 13,
                            ),
                          ),
                        ),
                      ),
                    );
                  }
                  return Column(
                    children: [
                      for (final h in list)
                        Padding(
                          padding: const EdgeInsets.only(bottom: 8),
                          child: _HoldingTile(holding: h, tr: tr),
                        ),
                    ],
                  );
                },
                loading: () => const SizedBox(
                  height: 80,
                  child: Center(child: CircularProgressIndicator()),
                ),
                error: (e, _) => Text('$e',
                    style: const TextStyle(color: AppColors.tierRed)),
              ),
              const SizedBox(height: 18),
              Text(
                tr.t('wealth.products'),
                style: const TextStyle(
                  fontSize: 13,
                  color: AppColors.textMuted,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 8),
              asyncProducts.when(
                data: (list) {
                  if (list.isEmpty) {
                    return SizedBox(
                      height: 200,
                      child: EmptyState(
                        icon: Icons.savings_outlined,
                        label: tr.t('wealth.empty.products'),
                      ),
                    );
                  }
                  return Column(
                    children: [
                      for (final p in list)
                        Padding(
                          padding: const EdgeInsets.only(bottom: 8),
                          child: _ProductTile(product: p, tr: tr),
                        ),
                    ],
                  );
                },
                loading: () => const SizedBox(
                  height: 120,
                  child: Center(child: CircularProgressIndicator()),
                ),
                error: (e, _) => Text('$e',
                    style: const TextStyle(color: AppColors.tierRed)),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _ProductTile extends ConsumerWidget {
  final WealthProduct product;
  final Tr tr;
  const _ProductTile({required this.product, required this.tr});

  Color _riskColor(String r) => switch (r) {
        'high' => AppColors.tierRed,
        'medium' => AppColors.tierYellow,
        _ => AppColors.tierGreen,
      };

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return GlassCard(
      onTap: () => _showSubscribe(context, ref, product),
      child: Row(
        children: [
          Container(
            width: 38,
            height: 38,
            decoration: BoxDecoration(
              color: AppColors.surfaceHi,
              borderRadius: BorderRadius.circular(10),
            ),
            child: Icon(Icons.savings_outlined,
                color: _riskColor(product.riskLevel), size: 18),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  product.name,
                  style: const TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Text(
                  '${product.category}  ·  ${tr.t('wealth.expected_return')} ${product.expectedAnnualReturnPct.toStringAsFixed(2)}%',
                  style: const TextStyle(
                    fontSize: 11,
                    color: AppColors.textMuted,
                  ),
                ),
              ],
            ),
          ),
          Column(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              Container(
                padding: const EdgeInsets.symmetric(
                    horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: _riskColor(product.riskLevel).withValues(alpha: 0.15),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Text(
                  product.riskLevel,
                  style: TextStyle(
                    color: _riskColor(product.riskLevel),
                    fontSize: 10,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              const SizedBox(height: 4),
              Text(
                '${tr.t('wealth.min')} ${fenToCny(product.minSubscriptionFen)}',
                style: const TextStyle(
                  fontSize: 10,
                  color: AppColors.textMuted,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Future<void> _showSubscribe(
      BuildContext context, WidgetRef ref, WealthProduct product) async {
    final lang = ref.read(settingsProvider).language;
    final tr = Tr.of(lang);
    final ctrl = TextEditingController();
    bool busy = false;
    String? err;
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
                  product.name,
                  style: const TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: ctrl,
                  keyboardType:
                      const TextInputType.numberWithOptions(decimal: true),
                  decoration: InputDecoration(
                    labelText: tr.t('wealth.subscribe.amount'),
                    prefixText: '¥ ',
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
                          final fen = parseCnyToFen(ctrl.text);
                          if (fen == null || fen < product.minSubscriptionFen) {
                            setState(() => err = tr.t('below_minimum'));
                            return;
                          }
                          setState(() {
                            busy = true;
                            err = null;
                          });
                          try {
                            final res = await ref
                                .read(apiProvider)
                                .subscribeWealth(product.id, fen);
                            setState(() => busy = false);
                            if (ctx.mounted) Navigator.pop(ctx, true);
                            if (ctx.mounted) {
                              ScaffoldMessenger.of(ctx).showSnackBar(
                                SnackBar(
                                  content: Text(res.toString()),
                                ),
                              );
                            }
                          } catch (e) {
                            setState(() {
                              busy = false;
                              err = e is ApiError ? e.message : '$e';
                            });
                          }
                        },
                  child: Text(tr.t('wealth.subscribe')),
                ),
              ],
            ),
          );
        });
      },
    );
    if (ok == true) {
      ref.invalidate(wealthHoldingsProvider);
    }
  }
}

class _HoldingTile extends ConsumerWidget {
  final WealthHolding holding;
  final Tr tr;
  const _HoldingTile({required this.holding, required this.tr});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return GlassCard(
      padding: const EdgeInsets.all(12),
      child: Row(
        children: [
          Container(
            width: 32,
            height: 32,
            decoration: BoxDecoration(
              color: AppColors.surfaceHi,
              borderRadius: BorderRadius.circular(8),
            ),
            child: const Icon(Icons.savings_outlined,
                color: AppColors.accent2, size: 16),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  fenToCny(holding.amountFen),
                  style: const TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Text(
                  holding.subscribedAt.toString().substring(0, 10),
                  style: const TextStyle(
                    fontSize: 11,
                    color: AppColors.textMuted,
                  ),
                ),
              ],
            ),
          ),
          TextButton(
            onPressed: () async {
              try {
                await ref
                    .read(apiProvider)
                    .redeemWealth(holding.id);
                ref.invalidate(wealthHoldingsProvider);
              } catch (_) {}
            },
            child: Text(tr.t('wealth.redeem')),
          ),
        ],
      ),
    );
  }
}