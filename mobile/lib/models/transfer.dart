// Smart transfer / transfer-preview shapes.
// These come from /api/v1/transfers/preview, /transfers,
// /smart-transfers/preview, /smart-transfers/execute.

import 'package:flutter/foundation.dart';

@immutable
class Payee {
  final String id;
  final String displayName;
  final String? phoneTail;
  final String? mockDestination;
  final bool verified;

  const Payee({
    required this.id,
    required this.displayName,
    this.phoneTail,
    this.mockDestination,
    this.verified = false,
  });

  factory Payee.fromJson(Map<String, dynamic> j) => Payee(
        id: j['id'] as String,
        displayName: j['display_name'] as String,
        phoneTail: j['phone_tail'] as String?,
        mockDestination: j['mock_destination'] as String?,
        verified: j['verified'] as bool? ?? false,
      );
}

enum RiskTier { green, yellow, red }

extension RiskTierX on RiskTier {
  String get wire => switch (this) {
        RiskTier.green => 'Green',
        RiskTier.yellow => 'Yellow',
        RiskTier.red => 'Red',
      };
  static RiskTier parse(String? s) => switch (s) {
        'Yellow' => RiskTier.yellow,
        'Red' => RiskTier.red,
        _ => RiskTier.green,
      };
}

class TransferPreview {
  final String resolvedPayeeId;
  final String resolvedPayeeName;
  final String resolvedPayeeDestination;
  final int amountFen;
  final RiskTier riskTier;
  final String decisionReason;
  final int dailyRemainingFen;

  const TransferPreview({
    required this.resolvedPayeeId,
    required this.resolvedPayeeName,
    required this.resolvedPayeeDestination,
    required this.amountFen,
    required this.riskTier,
    required this.decisionReason,
    required this.dailyRemainingFen,
  });

  factory TransferPreview.fromJson(Map<String, dynamic> j) => TransferPreview(
        resolvedPayeeId: j['resolved_payee_id'] as String,
        resolvedPayeeName: j['resolved_payee_name'] as String,
        resolvedPayeeDestination:
            j['resolved_payee_destination'] as String? ?? '',
        amountFen: (j['amount_fen'] as num).toInt(),
        riskTier: RiskTierX.parse(j['risk_tier'] as String?),
        decisionReason: j['decision_reason'] as String? ?? '',
        dailyRemainingFen: (j['daily_remaining_fen'] as num?)?.toInt() ?? 0,
      );
}

class TransferCreateOut {
  final String? transferId;
  final String? pendingActionId;
  final RiskTier riskTier;

  const TransferCreateOut({
    this.transferId,
    this.pendingActionId,
    required this.riskTier,
  });

  factory TransferCreateOut.fromJson(Map<String, dynamic> j) =>
      TransferCreateOut(
        transferId: j['transfer_id'] as String?,
        pendingActionId: j['pending_action_id'] as String?,
        riskTier: RiskTierX.parse(j['risk_tier'] as String?),
      );
}
