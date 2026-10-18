// Money helpers. All server amounts are integer fen (1/100 of CNY).
// We never use double for ledger math; the display layer formats only.

import 'package:intl/intl.dart';

class Money {
  final int fen;
  const Money(this.fen);

  bool get isNegative => fen < 0;
  bool get isPositive => fen > 0;

  Money operator +(Money other) => Money(fen + other.fen);
  Money operator -(Money other) => Money(fen - other.fen);
  Money operator -() => Money(-fen);

  @override
  String toString() => 'Money(fen: $fen)';
}

Money fen(int value) => Money(value);

/// Format integer fen as a localized CNY string. `signed` adds a leading
/// + or - for credits/debits.
String formatFen(int fen, {bool signed = false, String? locale}) {
  final cny = fen / 100.0;
  final fmt = NumberFormat.currency(
    locale: locale ?? 'zh_CN',
    symbol: '¥',
    decimalDigits: 2,
  );
  final body = fmt.format(cny.abs());
  if (!signed) return fen < 0 ? '-$body' : body;
  if (fen > 0) return '+$body';
  if (fen < 0) return '-$body';
  return body;
}

/// Shorthand. The web frontend uses `fenToCny` for this.
String fenToCny(int fen, {bool signed = false}) =>
    formatFen(fen, signed: signed);

/// Parse a user-typed CNY amount into fen. Accepts "12", "12.5", "12.34".
/// Returns null on invalid input.
int? parseCnyToFen(String input) {
  final trimmed = input.trim();
  if (trimmed.isEmpty) return null;
  final parsed = double.tryParse(trimmed);
  if (parsed == null || parsed.isNaN || parsed.isInfinite) return null;
  if (parsed < 0) return null;
  return (parsed * 100).round();
}
