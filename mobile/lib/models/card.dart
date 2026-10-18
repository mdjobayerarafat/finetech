// Virtual card shapes from /api/v1/cards.

class VirtualCard {
  final String id;
  final String displayName;
  final String maskedNumber;
  final int expiryMonth;
  final int expiryYear;
  final String status;
  final int? singleTxLimitFen;
  final int? monthlyLimitFen;

  const VirtualCard({
    required this.id,
    required this.displayName,
    required this.maskedNumber,
    required this.expiryMonth,
    required this.expiryYear,
    required this.status,
    this.singleTxLimitFen,
    this.monthlyLimitFen,
  });

  factory VirtualCard.fromJson(Map<String, dynamic> j) => VirtualCard(
        id: j['id'] as String,
        displayName: j['display_name'] as String,
        maskedNumber: j['masked_number'] as String? ?? '',
        expiryMonth: (j['expiry_month'] as num).toInt(),
        expiryYear: (j['expiry_year'] as num).toInt(),
        status: j['status'] as String,
        singleTxLimitFen: (j['single_tx_limit_fen'] as num?)?.toInt(),
        monthlyLimitFen: (j['monthly_limit_fen'] as num?)?.toInt(),
      );
}
