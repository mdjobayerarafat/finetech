// Budget shapes from /api/v1/budgets. Web's lib/api.ts already
// defines the full surface; we mirror enough for the mobile screens.

class Budget {
  final String id;
  final String category;
  final int limitFen;
  final int spentFen;
  final int remainingFen;
  final double utilizationPct;
  final bool breached;
  final String? note;
  final DateTime createdAt;
  final DateTime updatedAt;

  const Budget({
    required this.id,
    required this.category,
    required this.limitFen,
    required this.spentFen,
    required this.remainingFen,
    required this.utilizationPct,
    required this.breached,
    required this.createdAt,
    required this.updatedAt,
    this.note,
  });

  factory Budget.fromJson(Map<String, dynamic> j) => Budget(
        id: j['id'] as String,
        category: j['category'] as String,
        limitFen: (j['limit_fen'] as num).toInt(),
        spentFen: (j['spent_fen'] as num?)?.toInt() ?? 0,
        remainingFen: (j['remaining_fen'] as num?)?.toInt() ?? 0,
        utilizationPct: (j['utilization_pct'] as num?)?.toDouble() ?? 0,
        breached: j['breached'] as bool? ?? false,
        note: j['note'] as String?,
        createdAt: DateTime.parse(j['created_at'] as String).toLocal(),
        updatedAt: DateTime.parse(j['updated_at'] as String).toLocal(),
      );
}

class BudgetSummary {
  final int budgetCount;
  final int totalLimitFen;
  final int totalSpentFen;
  final int remainingFen;
  final List<BudgetBreachRow> breached;
  final List<BudgetBreachRow> nearLimit;

  const BudgetSummary({
    required this.budgetCount,
    required this.totalLimitFen,
    required this.totalSpentFen,
    required this.remainingFen,
    required this.breached,
    required this.nearLimit,
  });

  factory BudgetSummary.fromJson(Map<String, dynamic> j) => BudgetSummary(
        budgetCount: (j['budget_count'] as num?)?.toInt() ?? 0,
        totalLimitFen: (j['total_limit_fen'] as num?)?.toInt() ?? 0,
        totalSpentFen: (j['total_spent_fen'] as num?)?.toInt() ?? 0,
        remainingFen: (j['remaining_fen'] as num?)?.toInt() ?? 0,
        breached: ((j['breached'] as List?) ?? const [])
            .cast<Map<String, dynamic>>()
            .map(BudgetBreachRow.fromJson)
            .toList(),
        nearLimit: ((j['near_limit'] as List?) ?? const [])
            .cast<Map<String, dynamic>>()
            .map(BudgetBreachRow.fromJson)
            .toList(),
      );
}

class BudgetBreachRow {
  final String category;
  final int limitFen;
  final int spentFen;
  final double utilizationPct;
  const BudgetBreachRow({
    required this.category,
    required this.limitFen,
    required this.spentFen,
    required this.utilizationPct,
  });
  factory BudgetBreachRow.fromJson(Map<String, dynamic> j) => BudgetBreachRow(
        category: j['category'] as String,
        limitFen: (j['limit_fen'] as num).toInt(),
        spentFen: (j['spent_fen'] as num).toInt(),
        utilizationPct: (j['utilization_pct'] as num?)?.toDouble() ?? 0,
      );
}
