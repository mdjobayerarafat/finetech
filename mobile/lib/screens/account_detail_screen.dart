// Account detail — full transaction list for a single account.
// Reached by tapping an AccountCard on the dashboard.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../core/api.dart';
import '../core/api_error.dart';
import '../core/i18n.dart';
import '../core/money.dart';
import '../models/account.dart';
import '../state/settings.dart';
import '../theme/app_theme.dart';
import '../widgets/empty_state.dart';
import '../widgets/glass_card.dart';

class AccountDetailScreen extends ConsumerStatefulWidget {
  final Account account;
  const AccountDetailScreen({super.key, required this.account});

  @override
  ConsumerState<AccountDetailScreen> createState() =>
      _AccountDetailScreenState();
}

class _AccountDetailScreenState extends ConsumerState<AccountDetailScreen> {
  late Future<List<Txn>> _future;
  String? _category;
  DateTimeRange? _range;

  @override
  void initState() {
    super.initState();
    _future = _load();
  }

  Future<List<Txn>> _load() async {
    return ref.read(apiProvider).transactions(
          accountId: widget.account.id,
          limit: 100,
        );
  }

  @override
  Widget build(BuildContext context) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    final a = widget.account;
    return Scaffold(
      appBar: AppBar(title: Text(a.displayName)),
      body: RefreshIndicator(
        onRefresh: () async {
          setState(() => _future = _load());
          await _future;
        },
        child: ListView(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
          children: [
            GlassCard(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    tr.t('accounts.available'),
                    style: const TextStyle(
                      fontSize: 12,
                      color: AppColors.textMuted,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    fenToCny(a.availableBalanceFen),
                    style: const TextStyle(
                      fontSize: 28,
                      fontWeight: FontWeight.w800,
                    ),
                  ),
                  Text(
                    '${a.maskedNumber}  ·  ${a.currency}',
                    style: const TextStyle(
                      fontSize: 11,
                      color: AppColors.textFaint,
                    ),
                  ),
                  if (a.reservedBalanceFen > 0) ...[
                    const SizedBox(height: 8),
                    Text(
                      '${tr.t('accounts.reserved')} ${fenToCny(a.reservedBalanceFen)}',
                      style: const TextStyle(
                        fontSize: 12,
                        color: AppColors.tierYellow,
                      ),
                    ),
                  ],
                ],
              ),
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: _FilterChip(
                    label: _category ?? 'All categories',
                    onTap: _pickCategory,
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: _FilterChip(
                    label: _range == null
                        ? 'Any date'
                        : '${DateFormat.MMMd().format(_range!.start)} – ${DateFormat.MMMd().format(_range!.end)}',
                    onTap: _pickDateRange,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            FutureBuilder<List<Txn>>(
              future: _future,
              builder: (c, snap) {
                if (snap.connectionState == ConnectionState.waiting) {
                  return const Padding(
                    padding: EdgeInsets.all(20),
                    child: Center(child: CircularProgressIndicator()),
                  );
                }
                if (snap.hasError) {
                  return Text(
                    snap.error is ApiError
                        ? (snap.error as ApiError).message
                        : '${snap.error}',
                    style: const TextStyle(color: AppColors.tierRed),
                  );
                }
                var txns = snap.data ?? const <Txn>[];
                if (_category != null) {
                  txns = txns.where((t) => t.category == _category).toList();
                }
                if (_range != null) {
                  txns = txns
                      .where((t) =>
                          !t.occurredAt.isBefore(_range!.start) &&
                          !t.occurredAt.isAfter(
                              _range!.end.add(const Duration(days: 1))))
                      .toList();
                }
                if (txns.isEmpty) {
                  return SizedBox(
                    height: 240,
                    child: EmptyState(
                      icon: Icons.receipt_long_outlined,
                      label: tr.t('dashboard.no_transactions'),
                    ),
                  );
                }
                return Column(
                  children: [
                    for (final t in txns)
                      Padding(
                        padding: const EdgeInsets.only(bottom: 6),
                        child: Container(
                          padding: const EdgeInsets.symmetric(
                              horizontal: 12, vertical: 10),
                          decoration: BoxDecoration(
                            color: AppColors.surface,
                            borderRadius: BorderRadius.circular(12),
                            border: Border.all(color: AppColors.line),
                          ),
                          child: Row(
                            children: [
                              Expanded(
                                child: Column(
                                  crossAxisAlignment:
                                      CrossAxisAlignment.start,
                                  children: [
                                    Text(
                                      t.merchant.isNotEmpty
                                          ? t.merchant
                                          : t.category,
                                      style: const TextStyle(fontSize: 13),
                                    ),
                                    Text(
                                      '${t.category}  ·  ${DateFormat('MM-dd HH:mm').format(t.occurredAt)}',
                                      style: const TextStyle(
                                        fontSize: 11,
                                        color: AppColors.textMuted,
                                      ),
                                    ),
                                  ],
                                ),
                              ),
                              Text(
                                fenToCny(t.amountFen, signed: true),
                                style: TextStyle(
                                  fontSize: 13,
                                  fontWeight: FontWeight.w600,
                                  color: t.direction == 'credit'
                                      ? AppColors.tierGreen
                                      : AppColors.text,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                  ],
                );
              },
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _pickCategory() async {
    final cats = <String>{
      'dining',
      'groceries',
      'transport',
      'shopping',
      'subscription',
      'bills',
      'salary',
      'transfer',
    };
    final picked = await showModalBottomSheet<String?>(
      context: context,
      backgroundColor: AppColors.surface,
      builder: (ctx) => SafeArea(
        child: ListView(
          shrinkWrap: true,
          children: [
            ListTile(
              title: const Text('All'),
              onTap: () => Navigator.pop(ctx, null),
            ),
            for (final c in cats)
              ListTile(
                title: Text(c),
                onTap: () => Navigator.pop(ctx, c),
              ),
          ],
        ),
      ),
    );
    setState(() => _category = picked);
  }

  Future<void> _pickDateRange() async {
    final now = DateTime.now();
    final picked = await showDateRangePicker(
      context: context,
      firstDate: DateTime(now.year - 1),
      lastDate: now,
      initialDateRange: _range ??
          DateTimeRange(
            start: now.subtract(const Duration(days: 30)),
            end: now,
          ),
    );
    if (picked != null) setState(() => _range = picked);
  }
}

class _FilterChip extends StatelessWidget {
  final String label;
  final VoidCallback onTap;
  const _FilterChip({required this.label, required this.onTap});

  @override
  Widget build(BuildContext context) {
    return InkWell(
      borderRadius: BorderRadius.circular(10),
      onTap: onTap,
      child: Container(
        padding:
            const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        decoration: BoxDecoration(
          color: AppColors.surface,
          border: Border.all(color: AppColors.line),
          borderRadius: BorderRadius.circular(10),
        ),
        child: Row(
          children: [
            const Icon(Icons.tune, size: 14, color: AppColors.textMuted),
            const SizedBox(width: 6),
            Expanded(
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(fontSize: 12),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
