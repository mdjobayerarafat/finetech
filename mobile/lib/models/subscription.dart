// Subscription + reminder shapes from /api/v1/subscriptions, /reminders.

class Subscription {
  final String id;
  final String merchant;
  final int estimatedAmountFen;
  final int intervalDays;
  final DateTime nextExpectedAt;
  final DateTime? lastChargeAt;
  final String cancellationStatus;

  const Subscription({
    required this.id,
    required this.merchant,
    required this.estimatedAmountFen,
    required this.intervalDays,
    required this.nextExpectedAt,
    this.lastChargeAt,
    required this.cancellationStatus,
  });

  factory Subscription.fromJson(Map<String, dynamic> j) => Subscription(
        id: j['id'] as String,
        merchant: j['merchant'] as String,
        estimatedAmountFen: (j['estimated_amount_fen'] as num).toInt(),
        intervalDays: (j['interval_days'] as num?)?.toInt() ?? 30,
        nextExpectedAt:
            DateTime.parse(j['next_expected_at'] as String).toLocal(),
        lastChargeAt: j['last_charge_at'] == null
            ? null
            : DateTime.parse(j['last_charge_at'] as String).toLocal(),
        cancellationStatus: j['cancellation_status'] as String? ?? 'active',
      );
}
