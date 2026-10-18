// Account summary card used on the dashboard.

import 'package:flutter/material.dart';
import '../core/i18n.dart';
import '../core/language.dart';
import '../core/money.dart';
import '../models/account.dart';
import '../theme/app_theme.dart';
import 'glass_card.dart';

class AccountCard extends StatelessWidget {
  final Account account;
  final bool expanded;
  final VoidCallback? onTap;
  const AccountCard({
    super.key,
    required this.account,
    this.expanded = false,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final tr = Tr.of(Localizations.localeOf(context).languageCode == 'zh'
        ? AppLanguage.zh
        : AppLanguage.en);
    return GlassCard(
      onTap: onTap,
      padding: const EdgeInsets.all(18),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
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
                  Icons.account_balance_wallet_outlined,
                  size: 20,
                  color: Colors.white,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      account.displayName,
                      style: const TextStyle(
                        fontSize: 15,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    Text(
                      account.maskedNumber,
                      style: const TextStyle(
                        fontSize: 12,
                        color: AppColors.textMuted,
                      ),
                    ),
                  ],
                ),
              ),
              Text(
                account.currency,
                style: const TextStyle(
                  fontSize: 11,
                  color: AppColors.textFaint,
                ),
              ),
            ],
          ),
          const SizedBox(height: 14),
          Text(
            fenToCny(account.availableBalanceFen),
            style: const TextStyle(
              fontSize: 26,
              fontWeight: FontWeight.w700,
              letterSpacing: -0.5,
            ),
          ),
          Text(
            '${tr.t('accounts.available')}'
            '${account.reservedBalanceFen > 0
                ? '  ·  ${tr.t('accounts.reserved')} ${fenToCny(account.reservedBalanceFen)}'
                : ''}',
            style: const TextStyle(
              fontSize: 12,
              color: AppColors.textMuted,
            ),
          ),
        ],
      ),
    );
  }
}

class TransactionTile extends StatelessWidget {
  final Txn txn;
  const TransactionTile({super.key, required this.txn});

  IconData _iconFor(String category) => switch (category) {
        'dining' => Icons.restaurant_outlined,
        'groceries' => Icons.shopping_basket_outlined,
        'transport' => Icons.directions_bus_outlined,
        'shopping' => Icons.shopping_bag_outlined,
        'subscription' => Icons.subscriptions_outlined,
        'bills' => Icons.receipt_long_outlined,
        'salary' => Icons.payments_outlined,
        'transfer' => Icons.swap_horiz,
        _ => Icons.attach_money,
      };

  @override
  Widget build(BuildContext context) {
    final isCredit = txn.direction == 'credit';
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      decoration: BoxDecoration(
        color: AppColors.surface,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: AppColors.line),
      ),
      child: Row(
        children: [
          Container(
            width: 36,
            height: 36,
            decoration: BoxDecoration(
              color: AppColors.surfaceHi,
              borderRadius: BorderRadius.circular(10),
            ),
            child: Icon(_iconFor(txn.category),
                size: 18, color: AppColors.textMuted),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  txn.merchant.isNotEmpty ? txn.merchant : txn.category,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(fontSize: 14),
                ),
                Text(
                  txn.memo.isNotEmpty
                      ? txn.memo
                      : _friendlyTime(txn.occurredAt),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    fontSize: 12,
                    color: AppColors.textMuted,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: 8),
          Text(
            fenToCny(txn.amountFen, signed: true),
            style: TextStyle(
              fontSize: 14,
              fontWeight: FontWeight.w600,
              color: isCredit ? AppColors.tierGreen : AppColors.text,
            ),
          ),
        ],
      ),
    );
  }

  String _friendlyTime(DateTime t) {
    final now = DateTime.now();
    final diff = now.difference(t);
    if (diff.inMinutes < 60) return '${diff.inMinutes}m';
    if (diff.inHours < 24) return '${diff.inHours}h';
    if (diff.inDays < 7) return '${diff.inDays}d';
    return '${t.year}-${t.month.toString().padLeft(2, '0')}-${t.day.toString().padLeft(2, '0')}';
  }
}