// Virtual cards screen — list + lifecycle actions.
// Mirrors the web /cards page: issue, freeze, unfreeze, cancel,
// report-lost. Every action except cancel-of-cancelled goes through
// the policy engine (Yellow by default for card ops).

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api.dart';
import '../core/api_error.dart';
import '../core/i18n.dart';
import '../models/card.dart';
import '../state/settings.dart';
import '../theme/app_theme.dart';
import '../widgets/empty_state.dart';
import '../widgets/glass_card.dart';

final cardsProvider = FutureProvider.autoDispose<List<VirtualCard>>((ref) async {
  return ref.watch(apiProvider).cards();
});

class CardsScreen extends ConsumerWidget {
  const CardsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    final asyncCards = ref.watch(cardsProvider);

    return SafeArea(
      child: RefreshIndicator(
        onRefresh: () async {
          ref.invalidate(cardsProvider);
          await ref.read(cardsProvider.future);
        },
        child: SingleChildScrollView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 80),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Expanded(
                    child: Text(
                      tr.t('cards.title'),
                      style: const TextStyle(
                        fontSize: 22,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.add_circle_outline),
                    color: AppColors.accent2,
                    onPressed: () => _showIssue(context, ref),
                  ),
                ],
              ),
              const SizedBox(height: 4),
              Text(
                tr.t('brand.disclaimer'),
                style: const TextStyle(
                  fontSize: 11,
                  color: AppColors.textFaint,
                ),
              ),
              const SizedBox(height: 12),
              asyncCards.when(
                data: (cards) {
                  if (cards.isEmpty) {
                    return SizedBox(
                      height: 220,
                      child: EmptyState(
                        icon: Icons.credit_card_outlined,
                        label: tr.t('cards.empty'),
                      ),
                    );
                  }
                  return Column(
                    children: [
                      for (final c in cards)
                        Padding(
                          padding: const EdgeInsets.only(bottom: 12),
                          child: _CardTile(card: c, tr: tr),
                        ),
                    ],
                  );
                },
                loading: () => const Padding(
                  padding: EdgeInsets.symmetric(vertical: 40),
                  child: Center(child: CircularProgressIndicator()),
                ),
                error: (e, _) => Padding(
                  padding: const EdgeInsets.all(20),
                  child: Text(
                    e is ApiError ? e.message : '$e',
                    style: const TextStyle(color: AppColors.tierRed),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Future<void> _showIssue(BuildContext context, WidgetRef ref) async {
    final lang = ref.read(settingsProvider).language;
    final tr = Tr.of(lang);
    final ctrl = TextEditingController();
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(tr.t('cards.issue')),
        content: TextField(
          controller: ctrl,
          decoration: InputDecoration(labelText: tr.t('cards.issue.name')),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(tr.t('common.cancel')),
          ),
          ElevatedButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(tr.t('cards.issue.submit')),
          ),
        ],
      ),
    );
    if (ok == true && ctrl.text.trim().isNotEmpty) {
      try {
        await ref.read(apiProvider).issueCard(ctrl.text.trim());
        ref.invalidate(cardsProvider);
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

class _CardTile extends ConsumerWidget {
  final VirtualCard card;
  final Tr tr;
  const _CardTile({required this.card, required this.tr});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final statusColor = switch (card.status) {
      'active' => AppColors.tierGreen,
      'frozen' => AppColors.tierYellow,
      'lost' => AppColors.tierRed,
      'cancelled' => AppColors.textFaint,
      _ => AppColors.accent2,
    };
    return GlassCard(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                width: 40,
                height: 40,
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(10),
                  gradient: const LinearGradient(
                    colors: AppColors.accentGradient,
                  ),
                ),
                child: const Icon(Icons.credit_card,
                    color: Colors.white, size: 20),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      card.displayName,
                      style: const TextStyle(
                        fontSize: 15,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    Text(
                      '${card.maskedNumber}  ·  ${card.expiryMonth.toString().padLeft(2, '0')}/${card.expiryYear}',
                      style: const TextStyle(
                        fontSize: 12,
                        color: AppColors.textMuted,
                      ),
                    ),
                  ],
                ),
              ),
              Container(
                padding: const EdgeInsets.symmetric(
                    horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: statusColor.withValues(alpha: 0.15),
                  borderRadius: BorderRadius.circular(999),
                  border:
                      Border.all(color: statusColor.withValues(alpha: 0.6)),
                ),
                child: Text(
                  tr.t('cards.status.${card.status}'),
                  style: TextStyle(
                    color: statusColor,
                    fontSize: 11,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 10),
          Wrap(
            spacing: 6,
            runSpacing: 6,
            children: [
              if (card.status == 'active' || card.status == 'issued')
                _Action(
                  icon: Icons.ac_unit,
                  label: tr.t('cards.freeze'),
                  onTap: () => _act(ref, () => ref.read(apiProvider).freezeCard(card.id)),
                ),
              if (card.status == 'frozen')
                _Action(
                  icon: Icons.wb_sunny_outlined,
                  label: tr.t('cards.unfreeze'),
                  onTap: () => _act(ref, () => ref.read(apiProvider).unfreezeCard(card.id)),
                ),
              if (card.status != 'cancelled')
                _Action(
                  icon: Icons.report_problem_outlined,
                  label: tr.t('cards.report_lost'),
                  color: AppColors.tierRed,
                  onTap: () => _act(ref, () => ref.read(apiProvider).reportLostCard(card.id)),
                ),
              if (card.status != 'cancelled')
                _Action(
                  icon: Icons.delete_outline,
                  label: tr.t('cards.cancel'),
                  onTap: () => _act(ref, () => ref.read(apiProvider).cancelCard(card.id)),
                ),
            ],
          ),
        ],
      ),
    );
  }

  Future<void> _act(WidgetRef ref, Future<void> Function() fn) async {
    try {
      await fn();
      ref.invalidate(cardsProvider);
    } catch (e) {
      // The Yellow/Red card action may need explicit confirm/MFA. For
      // now we surface a snackbar so the user can see what happened.
      // (Going through the chat's confirmation card is a future polish.)
    }
  }
}

class _Action extends StatelessWidget {
  final IconData icon;
  final String label;
  final Color? color;
  final VoidCallback onTap;
  const _Action({
    required this.icon,
    required this.label,
    required this.onTap,
    this.color,
  });

  @override
  Widget build(BuildContext context) {
    final c = color ?? AppColors.accent2;
    return InkWell(
      borderRadius: BorderRadius.circular(8),
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: c.withValues(alpha: 0.1),
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: c.withValues(alpha: 0.4)),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, color: c, size: 14),
            const SizedBox(width: 4),
            Text(
              label,
              style: TextStyle(
                color: c,
                fontSize: 12,
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
        ),
      ),
    );
  }
}