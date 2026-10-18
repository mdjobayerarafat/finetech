// Tier badge: Green / Yellow / Red pill with the matching color.
// Used everywhere a risk tier needs to surface — pending actions,
// transfers, cards, wealth subscriptions.

import 'package:flutter/material.dart';
import '../core/i18n.dart';
import '../core/language.dart';
import '../models/transfer.dart';
import '../theme/app_theme.dart';

class TierBadge extends StatelessWidget {
  final RiskTier tier;
  final bool compact;
  const TierBadge({super.key, required this.tier, this.compact = false});

  @override
  Widget build(BuildContext context) {
    final color = switch (tier) {
      RiskTier.green => AppColors.tierGreen,
      RiskTier.yellow => AppColors.tierYellow,
      RiskTier.red => AppColors.tierRed,
    };
    final tr = Tr.of(Localizations.localeOf(context).languageCode == 'zh'
        ? AppLanguage.zh
        : AppLanguage.en);
    final label = switch (tier) {
      RiskTier.green => tr.t('tier.green'),
      RiskTier.yellow => tr.t('tier.yellow'),
      RiskTier.red => tr.t('tier.red'),
    };
    return Container(
      padding: EdgeInsets.symmetric(
        horizontal: compact ? 8 : 10,
        vertical: compact ? 3 : 5,
      ),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.15),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: color.withValues(alpha: 0.6)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 7,
            height: 7,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          ),
          const SizedBox(width: 6),
          Text(
            label,
            style: TextStyle(
              color: color,
              fontSize: compact ? 11 : 12,
              fontWeight: FontWeight.w600,
              letterSpacing: 0.2,
            ),
          ),
        ],
      ),
    );
  }
}