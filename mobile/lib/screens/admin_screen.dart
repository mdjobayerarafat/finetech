// Admin console — overview, user list, safety events, audit chain
// verify, safety-lock unlock. Visible to admin role only.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api.dart';
import '../core/i18n.dart';
import '../state/auth.dart';
import '../state/settings.dart';
import '../theme/app_theme.dart';
import '../widgets/glass_card.dart';

final adminOverviewProvider = FutureProvider.autoDispose((ref) async {
  return ref.watch(apiProvider).client.get('/admin/overview')
      as Map<String, dynamic>;
});

final adminUsersProvider = FutureProvider.autoDispose((ref) async {
  final res = await ref.watch(apiProvider).client.get('/admin/users')
      as List;
  return res.cast<Map<String, dynamic>>();
});

final safetyEventsProvider = FutureProvider.autoDispose((ref) async {
  final res = await ref.watch(apiProvider).client.get('/admin/safety-events')
      as List;
  return res.cast<Map<String, dynamic>>();
});

final chainVerifyProvider = FutureProvider.autoDispose((ref) async {
  return ref.watch(apiProvider).client.get('/audit/verify-chain')
      as Map<String, dynamic>;
});

class AdminScreen extends ConsumerWidget {
  const AdminScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    final user = ref.watch(currentUserProvider);
    if (user == null || !user.isAdmin) {
      return Scaffold(
        appBar: AppBar(title: Text(tr.t('brand.admin'))),
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Text(
              'Admin only.',
              style: const TextStyle(color: AppColors.textMuted),
            ),
          ),
        ),
      );
    }

    return Scaffold(
      appBar: AppBar(title: Text(tr.t('brand.admin'))),
      body: RefreshIndicator(
        onRefresh: () async {
          ref.invalidate(adminOverviewProvider);
          ref.invalidate(adminUsersProvider);
          ref.invalidate(safetyEventsProvider);
          ref.invalidate(chainVerifyProvider);
          await Future.wait([
            ref.read(adminOverviewProvider.future),
            ref.read(adminUsersProvider.future),
            ref.read(safetyEventsProvider.future),
            ref.read(chainVerifyProvider.future),
          ]);
        },
        child: ListView(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
          children: [
            _OverviewSection(),
            const SizedBox(height: 16),
            _ChainCard(),
            const SizedBox(height: 16),
            _UsersSection(),
            const SizedBox(height: 16),
            _SafetySection(),
          ],
        ),
      ),
    );
  }
}

class _OverviewSection extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final async = ref.watch(adminOverviewProvider);
    return async.when(
      data: (data) {
        final counts = (data['counts'] as Map?)?.cast<String, dynamic>() ?? {};
        Widget tile(String label, String value, {Color? color}) => Expanded(
              child: Container(
                margin: const EdgeInsets.only(right: 6),
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: AppColors.surface,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: AppColors.line),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      label,
                      style: const TextStyle(
                        fontSize: 11,
                        color: AppColors.textMuted,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      value,
                      style: TextStyle(
                        fontSize: 18,
                        fontWeight: FontWeight.w700,
                        color: color,
                      ),
                    ),
                  ],
                ),
              ),
            );
        return GlassCard(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text(
                'Overview',
                style: TextStyle(
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 8),
              Row(
                children: [
                  tile('Users', '${counts['users'] ?? 0}'),
                  tile('Locked', '${counts['locked'] ?? 0}',
                      color: (counts['locked'] ?? 0) > 0
                          ? AppColors.tierRed
                          : null),
                ],
              ),
              const SizedBox(height: 6),
              Row(
                children: [
                  tile('Transactions', '${counts['transactions'] ?? 0}'),
                  tile('Safety events', '${counts['safety_events'] ?? 0}'),
                ],
              ),
              const SizedBox(height: 6),
              Row(
                children: [
                  tile('Audit events', '${counts['audit_events'] ?? 0}'),
                  tile('Admins', '${counts['admins'] ?? 0}'),
                ],
              ),
            ],
          ),
        );
      },
      loading: () => const SizedBox(height: 100),
      error: (e, _) => Text('$e',
          style: const TextStyle(color: AppColors.tierRed)),
    );
  }
}

class _ChainCard extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final async = ref.watch(chainVerifyProvider);
    return async.when(
      data: (data) {
        final ok = data['ok'] as bool? ?? false;
        final events = (data['events'] as num?)?.toInt() ?? 0;
        return GlassCard(
          border: Border.all(
            color: ok
                ? AppColors.tierGreen.withValues(alpha: 0.4)
                : AppColors.tierRed.withValues(alpha: 0.4),
          ),
          child: Row(
            children: [
              Icon(
                ok ? Icons.verified_outlined : Icons.broken_image_outlined,
                color: ok ? AppColors.tierGreen : AppColors.tierRed,
                size: 24,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      ok ? 'Audit chain OK' : 'Audit chain BROKEN',
                      style: TextStyle(
                        color: ok ? AppColors.tierGreen : AppColors.tierRed,
                        fontSize: 14,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    Text(
                      '$events events walked',
                      style: const TextStyle(
                        fontSize: 11,
                        color: AppColors.textMuted,
                      ),
                    ),
                    if (!ok && data['break_at'] is String)
                      Text(
                        'break at: ${data['break_at']}',
                        style: const TextStyle(
                          fontSize: 10,
                          color: AppColors.tierRed,
                        ),
                      ),
                  ],
                ),
              ),
            ],
          ),
        );
      },
      loading: () => const SizedBox(height: 60),
      error: (e, _) => Text('$e',
          style: const TextStyle(color: AppColors.tierRed)),
    );
  }
}

class _UsersSection extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final async = ref.watch(adminUsersProvider);
    return async.when(
      data: (list) => GlassCard(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              'Users',
              style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 8),
            for (final u in list)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Row(
                  children: [
                    Container(
                      width: 8,
                      height: 8,
                      decoration: BoxDecoration(
                        color: u['safety_lock_state'] == 'locked'
                            ? AppColors.tierRed
                            : AppColors.tierGreen,
                        shape: BoxShape.circle,
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        '${u['display_name']}  ·  @${u['username']}',
                        style: const TextStyle(fontSize: 13),
                      ),
                    ),
                    Text(
                      u['role'] as String,
                      style: const TextStyle(
                        fontSize: 11,
                        color: AppColors.textMuted,
                      ),
                    ),
                    if (u['safety_lock_state'] == 'locked')
                      TextButton(
                        onPressed: () => _unlock(context, ref, u['id'] as String),
                        child: const Text('Unlock'),
                      ),
                  ],
                ),
              ),
          ],
        ),
      ),
      loading: () => const SizedBox(height: 60),
      error: (e, _) => Text('$e',
          style: const TextStyle(color: AppColors.tierRed)),
    );
  }

  Future<void> _unlock(BuildContext context, WidgetRef ref, String userId) async {
    final reason = TextEditingController(text: 'Admin override');
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Unlock user'),
        content: TextField(
          controller: reason,
          decoration: const InputDecoration(labelText: 'Reason'),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Unlock'),
          ),
        ],
      ),
    );
    if (ok == true) {
      try {
        await ref.read(apiProvider).client.post(
              '/admin/safety-lock/$userId/unlock',
              body: {'reason': reason.text},
            );
        ref.invalidate(adminUsersProvider);
      } catch (e) {
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text('$e')),
          );
        }
      }
    }
  }
}

class _SafetySection extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final async = ref.watch(safetyEventsProvider);
    return async.when(
      data: (list) {
        if (list.isEmpty) {
          return GlassCard(
            child: SizedBox(
              height: 80,
              child: Center(
                child: Text(
                  'No safety events.',
                  style: const TextStyle(
                    color: AppColors.textMuted,
                    fontSize: 13,
                  ),
                ),
              ),
            ),
          );
        }
        return GlassCard(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text(
                'Safety events',
                style: TextStyle(
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 8),
              for (final e in list.take(20))
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 4),
                  child: Row(
                    children: [
                      const Icon(
                        Icons.warning_amber_outlined,
                        color: AppColors.tierYellow,
                        size: 14,
                      ),
                      const SizedBox(width: 6),
                      Expanded(
                        child: Text(
                          e['kind'] as String? ?? '—',
                          style: const TextStyle(fontSize: 12),
                        ),
                      ),
                      Text(
                        (e['created_at'] as String? ?? '').substring(0, 10),
                        style: const TextStyle(
                          fontSize: 10,
                          color: AppColors.textFaint,
                        ),
                      ),
                    ],
                  ),
                ),
            ],
          ),
        );
      },
      loading: () => const SizedBox(height: 80),
      error: (e, _) => Text('$e',
          style: const TextStyle(color: AppColors.tierRed)),
    );
  }
}
