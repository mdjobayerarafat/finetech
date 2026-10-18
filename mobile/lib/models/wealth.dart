// Wealth product + holding shapes from /api/v1/wealth.

class WealthProduct {
  final String id;
  final String name;
  final String category;
  final String riskLevel; // 'low' | 'medium' | 'high'
  final double expectedAnnualReturnPct;
  final int minSubscriptionFen;
  final String? suitabilityRequired;

  const WealthProduct({
    required this.id,
    required this.name,
    required this.category,
    required this.riskLevel,
    required this.expectedAnnualReturnPct,
    required this.minSubscriptionFen,
    this.suitabilityRequired,
  });

  factory WealthProduct.fromJson(Map<String, dynamic> j) => WealthProduct(
        id: j['id'] as String,
        name: j['name'] as String,
        category: j['category'] as String? ?? '',
        riskLevel: j['risk_level'] as String? ?? 'low',
        expectedAnnualReturnPct:
            (j['expected_annual_return_pct'] as num?)?.toDouble() ?? 0,
        minSubscriptionFen: (j['min_subscription_fen'] as num).toInt(),
        suitabilityRequired: j['suitability_required'] as String?,
      );
}

class WealthHolding {
  final String id;
  final String productId;
  final int amountFen;
  final DateTime subscribedAt;
  final String status;

  const WealthHolding({
    required this.id,
    required this.productId,
    required this.amountFen,
    required this.subscribedAt,
    required this.status,
  });

  factory WealthHolding.fromJson(Map<String, dynamic> j) => WealthHolding(
        id: j['id'] as String,
        productId: j['product_id'] as String,
        amountFen: (j['amount_fen'] as num).toInt(),
        subscribedAt: DateTime.parse(j['subscribed_at'] as String).toLocal(),
        status: j['status'] as String? ?? 'active',
      );
}
