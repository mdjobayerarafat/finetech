// ConfirmationCard — the single most important widget in the app.
//
// Mirrors the web's `ConfirmationCard`. Shows the proposed action's
// parameters, the risk tier, and offers two paths:
//   • Yellow → one-tap "Confirm"
//   • Red    → 6-digit mock-MFA entry, then verify
// Both call the agent namespace and surface the result inline.

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api.dart';
import '../core/i18n.dart';
import '../models/agent.dart';
import '../models/transfer.dart' show RiskTier;
import '../state/settings.dart';
import '../theme/app_theme.dart';
import 'glass_card.dart';
import 'tier_badge.dart';

class ConfirmationCard extends ConsumerStatefulWidget {
  final PendingAction pendingAction;
  final String? workflowId;
  final VoidCallback? onDone;
  const ConfirmationCard({
    super.key,
    required this.pendingAction,
    this.workflowId,
    this.onDone,
  });

  @override
  ConsumerState<ConfirmationCard> createState() => _ConfirmationCardState();
}

enum _Phase { idle, busy, awaitingMfa, done }

class _ConfirmationCardState extends ConsumerState<ConfirmationCard> {
  _Phase _phase = _Phase.idle;
  String? _error;
  final _mfa = TextEditingController();

  @override
  void dispose() {
    _mfa.dispose();
    super.dispose();
  }

  Future<void> _approve() async {
    setState(() {
      _phase = _Phase.busy;
      _error = null;
    });
    try {
      final api = ref.read(apiProvider);
      if (widget.pendingAction.riskTier == RiskTier.red) {
        setState(() => _phase = _Phase.awaitingMfa);
        return;
      }
      await api.confirmAction(widget.pendingAction.id);
      if (!mounted) return;
      setState(() => _phase = _Phase.done);
      widget.onDone?.call();
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _phase = _Phase.idle;
        _error = e.toString();
      });
    }
  }

  Future<void> _submitMfa() async {
    setState(() {
      _phase = _Phase.busy;
      _error = null;
    });
    try {
      final api = ref.read(apiProvider);
      final code = _mfa.text.trim();
      await api.verifyAction(widget.pendingAction.id, code);
      await api.confirmAction(widget.pendingAction.id);
      if (!mounted) return;
      setState(() => _phase = _Phase.done);
      widget.onDone?.call();
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _phase = _Phase.awaitingMfa;
        _error = e.toString();
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    final p = widget.pendingAction;
    final params = p.parameters ?? const {};
    final friendly = _friendlyParams(params);
    final isDone = _phase == _Phase.done;

    return GlassCard(
      border: Border.all(color: AppColors.accent.withValues(alpha: 0.4)),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(Icons.shield_outlined,
                  size: 18, color: AppColors.accent2),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  tr.t('chat.pending_action'),
                  style: const TextStyle(
                    fontSize: 13,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              TierBadge(tier: p.riskTier, compact: true),
            ],
          ),
          const SizedBox(height: 4),
          Text(
            p.tool,
            style: const TextStyle(
              fontSize: 11,
              color: AppColors.textMuted,
              fontFamily: 'monospace',
            ),
          ),
          const SizedBox(height: 12),
          if (friendly.isEmpty)
            Text(tr.t('chat.executed'),
                style: const TextStyle(color: AppColors.textMuted))
          else
            ...friendly.entries.map((e) => Padding(
                  padding: const EdgeInsets.symmetric(vertical: 3),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      SizedBox(
                        width: 110,
                        child: Text(
                          e.key,
                          style: const TextStyle(
                            fontSize: 12,
                            color: AppColors.textMuted,
                          ),
                        ),
                      ),
                      Expanded(
                        child: Text(
                          e.value,
                          style: const TextStyle(fontSize: 13),
                        ),
                      ),
                    ],
                  ),
                )),
          if (_error != null) ...[
            const SizedBox(height: 8),
            Text(
              _error!,
              style: const TextStyle(color: AppColors.tierRed, fontSize: 12),
            ),
          ],
          const SizedBox(height: 14),
          if (isDone)
            Row(
              children: [
                const Icon(Icons.check_circle_outline,
                    color: AppColors.tierGreen, size: 18),
                const SizedBox(width: 8),
                Text(
                  tr.t('chat.executed'),
                  style: const TextStyle(
                    color: AppColors.tierGreen,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            )
          else if (_phase == _Phase.awaitingMfa)
            _MfaBlock(
              controller: _mfa,
              tr: tr,
              onSubmit: _submitMfa,
              busy: _phase == _Phase.busy,
            )
          else
            Row(
              children: [
                Expanded(
                  child: ElevatedButton(
                    onPressed: _phase == _Phase.busy ? null : _approve,
                    child: Text(tr.t('chat.approve')),
                  ),
                ),
              ],
            ),
        ],
      ),
    );
  }

  Map<String, String> _friendlyParams(Map<String, dynamic> params) {
    final out = <String, String>{};
    const hidden = {'tier'};
    params.forEach((k, v) {
      if (hidden.contains(k)) return;
      out[_humanKey(k)] = _humanVal(k, v);
    });
    return out;
  }

  String _humanKey(String k) => switch (k) {
        'amount_fen' => 'Amount',
        'amount_cny' => 'Amount',
        'memo' => 'Memo',
        'account_id' => 'Account',
        'source_account_id' => 'From',
        'payee_id' => 'Payee',
        'recipient_query' => 'Recipient',
        'phone_tail' => 'Phone',
        'days_before' => 'Days before',
        'goal_id' => 'Goal',
        'product_id' => 'Product',
        'holding_id' => 'Holding',
        'merchant' => 'Merchant',
        'schedule' => 'Schedule',
        'display_name' => 'Name',
        'deliver_at' => 'Deliver at',
        'subscription_id' => 'Subscription',
        'subject' => 'Subject',
        'due_at' => 'Due',
        _ => k,
      };

  String _humanVal(String key, Object? v) {
    if (v == null) return '—';
    if (key == 'amount_fen') return '¥${(v as num).toInt() / 100}';
    if (key == 'amount_cny') return '¥$v';
    if (key == 'days_before') return '$v day(s)';
    return '$v';
  }
}

class _MfaBlock extends StatelessWidget {
  final TextEditingController controller;
  final Tr tr;
  final VoidCallback onSubmit;
  final bool busy;
  const _MfaBlock({
    required this.controller,
    required this.tr,
    required this.onSubmit,
    required this.busy,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(tr.t('chat.mfa.label'),
            style: const TextStyle(fontSize: 12, color: AppColors.textMuted)),
        const SizedBox(height: 8),
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: controller,
                keyboardType: TextInputType.number,
                maxLength: 6,
                style: const TextStyle(
                  fontSize: 18,
                  letterSpacing: 8,
                  fontFamily: 'monospace',
                ),
                inputFormatters: [FilteringTextInputFormatter.digitsOnly],
                decoration: InputDecoration(
                  counterText: '',
                  hintText: '••••••',
                  hintStyle: const TextStyle(
                    color: AppColors.textFaint,
                    letterSpacing: 8,
                  ),
                ),
              ),
            ),
            const SizedBox(width: 10),
            ElevatedButton(
              onPressed: busy ? null : onSubmit,
              child: Text(tr.t('chat.mfa.submit')),
            ),
          ],
        ),
        const SizedBox(height: 4),
        Text(tr.t('chat.mfa.hint'),
            style: const TextStyle(fontSize: 11, color: AppColors.textFaint)),
      ],
    );
  }
}
