// Account balance + transactions.

class Account {
  final String id;
  final String displayName;
  final String maskedNumber;
  final String currency;
  final int availableBalanceFen;
  final int reservedBalanceFen;

  const Account({
    required this.id,
    required this.displayName,
    required this.maskedNumber,
    required this.currency,
    required this.availableBalanceFen,
    required this.reservedBalanceFen,
  });

  int get totalBalanceFen => availableBalanceFen + reservedBalanceFen;

  factory Account.fromJson(Map<String, dynamic> j) => Account(
        id: j['id'] as String,
        displayName: j['display_name'] as String,
        maskedNumber: j['masked_number'] as String? ?? '',
        currency: j['currency'] as String? ?? 'CNY',
        availableBalanceFen: (j['available_balance_fen'] as num).toInt(),
        reservedBalanceFen: (j['reserved_balance_fen'] as num?)?.toInt() ?? 0,
      );
}

class Txn {
  final String id;
  final String accountId;
  final int amountFen;
  final String direction; // 'credit' | 'debit'
  final String category;
  final String merchant;
  final String memo;
  final DateTime occurredAt;
  final String status;

  const Txn({
    required this.id,
    required this.accountId,
    required this.amountFen,
    required this.direction,
    required this.category,
    required this.merchant,
    required this.memo,
    required this.occurredAt,
    required this.status,
  });

  factory Txn.fromJson(Map<String, dynamic> j) => Txn(
        id: j['id'] as String,
        accountId: j['account_id'] as String? ?? '',
        amountFen: (j['amount_fen'] as num).toInt(),
        direction: j['direction'] as String,
        category: j['category'] as String? ?? '',
        merchant: j['merchant'] as String? ?? '',
        memo: j['memo'] as String? ?? '',
        occurredAt: DateTime.parse(j['occurred_at'] as String).toLocal(),
        status: j['status'] as String? ?? 'posted',
      );
}

class SpendingItem {
  final String category;
  final int totalFen;
  final double sharePct;
  final int transactionCount;

  const SpendingItem({
    required this.category,
    required this.totalFen,
    required this.sharePct,
    required this.transactionCount,
  });

  factory SpendingItem.fromJson(Map<String, dynamic> j) => SpendingItem(
        category: j['category'] as String,
        totalFen: (j['total_fen'] as num).toInt(),
        sharePct: (j['share_pct'] as num).toDouble(),
        transactionCount: (j['transaction_count'] as num).toInt(),
      );
}

class SpendingReport {
  final DateTime fromDate;
  final DateTime toDate;
  final String groupBy;
  final int incomeFen;
  final int expenseFen;
  final List<SpendingItem> items;

  const SpendingReport({
    required this.fromDate,
    required this.toDate,
    required this.groupBy,
    required this.incomeFen,
    required this.expenseFen,
    required this.items,
  });

  factory SpendingReport.fromJson(Map<String, dynamic> j) => SpendingReport(
        fromDate: DateTime.parse(j['from_date'] as String).toLocal(),
        toDate: DateTime.parse(j['to_date'] as String).toLocal(),
        groupBy: j['group_by'] as String? ?? 'category',
        incomeFen: (j['income_fen'] as num?)?.toInt() ?? 0,
        expenseFen: (j['expense_fen'] as num?)?.toInt() ?? 0,
        items: ((j['items'] as List?) ?? const [])
            .cast<Map<String, dynamic>>()
            .map(SpendingItem.fromJson)
            .toList(),
      );
}
