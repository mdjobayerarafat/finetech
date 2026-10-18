// Reusable "nothing here" placeholder.

import 'package:flutter/material.dart';
import '../core/i18n.dart';
import '../core/language.dart';
import '../theme/app_theme.dart';

class EmptyState extends StatelessWidget {
  final IconData icon;
  final String label;
  const EmptyState({super.key, required this.icon, required this.label});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 56,
              height: 56,
              decoration: BoxDecoration(
                color: AppColors.surface,
                shape: BoxShape.circle,
                border: Border.all(color: AppColors.lineHi),
              ),
              child: Icon(icon, color: AppColors.textMuted),
            ),
            const SizedBox(height: 14),
            Text(
              label,
              textAlign: TextAlign.center,
              style: const TextStyle(color: AppColors.textMuted, fontSize: 14),
            ),
          ],
        ),
      ),
    );
  }
}

// Helper: pull translation lookup from any widget.
Tr trOf(BuildContext context) {
  final isZh =
      Localizations.localeOf(context).languageCode == 'zh';
  return Tr.of(isZh ? AppLanguage.zh : AppLanguage.en);
}