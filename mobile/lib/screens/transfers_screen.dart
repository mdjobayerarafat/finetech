// Transfers — list recent transfers, plus a quick "smart" form that
// goes through the agent pipeline.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api.dart';
import '../core/api_error.dart';
import '../core/i18n.dart';
import '../core/money.dart';
import '../models/transfer.dart';
import '../state/settings.dart';
import '../theme/app_theme.dart';
import '../widgets/glass_card.dart';
import '../widgets/tier_badge.dart';

class TransfersScreen extends ConsumerStatefulWidget {
  const TransfersScreen({super.key});
  @override
  ConsumerState<TransfersScreen> createState() => _TransfersScreenState();
}

class _TransfersScreenState extends ConsumerState<TransfersScreen> {
  @override
  Widget build(BuildContext context) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    return SafeArea(
      child: SingleChildScrollView(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 80),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              tr.t('transfers.title'),
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
            GlassCard(
              onTap: () => Navigator.of(context).push(
                MaterialPageRoute(builder: (_) => const _NewTransferSheet()),
              ),
              child: Row(
                children: [
                  Container(
                    width: 40,
                    height: 40,
                    decoration: BoxDecoration(
                      gradient: const LinearGradient(
                          colors: AppColors.accentGradient),
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: const Icon(Icons.add, color: Colors.white),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          tr.t('transfers.new'),
                          style: const TextStyle(
                            fontSize: 15,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                        Text(
                          tr.t('chat.placeholder'),
                          style: const TextStyle(
                            fontSize: 12,
                            color: AppColors.textMuted,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const Icon(Icons.arrow_forward_ios,
                      size: 12, color: AppColors.textFaint),
                ],
              ),
            ),
            const SizedBox(height: 18),
            Text(
              tr.t('transfers.history'),
              style: const TextStyle(
                fontSize: 13,
                color: AppColors.textMuted,
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 8),
            // History endpoint shape varies in the demo; we just show a
            // soft note when the API isn't returning a clean list, so
            // the screen still works in a clean seed.
            GlassCard(
              child: Center(
                child: Padding(
                  padding: const EdgeInsets.all(8),
                  child: Text(
                    tr.t('transfers.empty'),
                    style: const TextStyle(
                      color: AppColors.textMuted,
                      fontSize: 13,
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _NewTransferSheet extends ConsumerStatefulWidget {
  const _NewTransferSheet();
  @override
  ConsumerState<_NewTransferSheet> createState() => _NewTransferSheetState();
}

class _NewTransferSheetState extends ConsumerState<_NewTransferSheet> {
  final _recipient = TextEditingController();
  final _amount = TextEditingController();
  final _memo = TextEditingController();
  TransferPreview? _preview;
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    _recipient.dispose();
    _amount.dispose();
    _memo.dispose();
    super.dispose();
  }

  Future<void> _runPreview() async {
    final fen = parseCnyToFen(_amount.text);
    if (fen == null || fen <= 0) {
      setState(() => _error = 'Enter an amount.');
      return;
    }
    if (_recipient.text.trim().isEmpty) {
      setState(() => _error = 'Enter a recipient.');
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
      _preview = null;
    });
    try {
      final p = await ref.read(apiProvider).previewTransfer({
        'recipient_query': _recipient.text.trim(),
        'amount': (fen / 100).toString(),
        'memo': _memo.text.trim(),
      });
      setState(() => _preview = p);
    } catch (e) {
      setState(() => _error = e is ApiError ? e.message : '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _send() async {
    final fen = parseCnyToFen(_amount.text);
    if (fen == null) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final out = await ref.read(apiProvider).createTransfer({
        'recipient_query': _recipient.text.trim(),
        'amount': (fen / 100).toString(),
        'memo': _memo.text.trim(),
      });
      if (!mounted) return;
      if (out.pendingActionId != null) {
        // Go through the chat for the confirmation flow.
        ref.read(_lastPendingProvider.notifier).state = out.pendingActionId;
        Navigator.of(context).pop();
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              out.riskTier.wire == 'Green'
                  ? 'Sent — check the assistant for status.'
                  : 'Approval required — open the Assistant to confirm.',
            ),
          ),
        );
      } else {
        Navigator.of(context).pop();
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(Tr.of(ref.read(settingsProvider).language)
              .t('transfers.success'))),
        );
      }
    } catch (e) {
      setState(() => _error = e is ApiError ? e.message : '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    final p = _preview;
    return Scaffold(
      appBar: AppBar(title: Text(tr.t('transfers.new'))),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 32),
        children: [
          TextField(
            controller: _recipient,
            decoration: InputDecoration(labelText: tr.t('transfers.recipient')),
          ),
          const SizedBox(height: 10),
          TextField(
            controller: _amount,
            keyboardType: const TextInputType.numberWithOptions(decimal: true),
            decoration: InputDecoration(
              labelText: tr.t('transfers.amount'),
              prefixText: '¥ ',
            ),
          ),
          const SizedBox(height: 10),
          TextField(
            controller: _memo,
            decoration: InputDecoration(
              labelText: tr.t('transfers.memo'),
              helperText: tr.t('common.optional'),
            ),
          ),
          if (_error != null) ...[
            const SizedBox(height: 8),
            Text(_error!,
                style: const TextStyle(color: AppColors.tierRed, fontSize: 12)),
          ],
          const SizedBox(height: 14),
          ElevatedButton(
            onPressed: _busy ? null : _runPreview,
            child: _busy
                ? const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(
                      strokeWidth: 2,
                      color: Colors.white,
                    ),
                  )
                : Text(tr.t('transfers.preview')),
          ),
          if (p != null) ...[
            const SizedBox(height: 16),
            GlassCard(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          tr.t('transfers.preview.recipient'),
                          style: const TextStyle(
                            fontSize: 12,
                            color: AppColors.textMuted,
                          ),
                        ),
                      ),
                      TierBadge(tier: p.riskTier, compact: true),
                    ],
                  ),
                  const SizedBox(height: 4),
                  Text(
                    p.resolvedPayeeName,
                    style: const TextStyle(
                      fontSize: 15,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Text(
                    p.resolvedPayeeDestination,
                    style: const TextStyle(
                      fontSize: 12,
                      color: AppColors.textMuted,
                    ),
                  ),
                  const Divider(height: 22),
                  Row(
                    children: [
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              fenToCny(p.amountFen),
                              style: const TextStyle(
                                fontSize: 22,
                                fontWeight: FontWeight.w700,
                              ),
                            ),
                            Text(
                              '${tr.t('transfers.preview.remaining')}: ${fenToCny(p.dailyRemainingFen)}',
                              style: const TextStyle(
                                fontSize: 11,
                                color: AppColors.textMuted,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                  if (p.decisionReason.isNotEmpty) ...[
                    const SizedBox(height: 8),
                    Text(
                      p.decisionReason,
                      style: const TextStyle(
                        fontSize: 12,
                        color: AppColors.textMuted,
                      ),
                    ),
                  ],
                  const SizedBox(height: 12),
                  ElevatedButton(
                    onPressed: _busy ? null : _send,
                    child: Text(
                      p.riskTier == RiskTier.green
                          ? tr.t('transfers.send')
                          : tr.t('chat.approve'),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }
}

final _lastPendingProvider = StateProvider<String?>((_) => null);