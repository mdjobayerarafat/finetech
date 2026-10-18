// Money helpers — integer fen everywhere, double only at the display edge.

import 'package:fintech_pioneers/core/money.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('parseCnyToFen', () {
    test('parses plain integers', () {
      expect(parseCnyToFen('12'), 1200);
      expect(parseCnyToFen('0'), 0);
    });
    test('parses decimals correctly', () {
      expect(parseCnyToFen('12.34'), 1234);
      expect(parseCnyToFen('0.5'), 50);
      expect(parseCnyToFen('99.99'), 9999);
    });
    test('rejects garbage', () {
      expect(parseCnyToFen(''), isNull);
      expect(parseCnyToFen('abc'), isNull);
      expect(parseCnyToFen('-5'), isNull);
    });
  });

  group('formatFen', () {
    test('renders positive and negative', () {
      expect(formatFen(1234), '¥12.34');
      expect(formatFen(-1234), '-¥12.34');
    });
    test('signed mode adds leading +', () {
      expect(formatFen(1234, signed: true), '+¥12.34');
    });
  });

  test('Money arithmetic is integer-only', () {
    final a = Money(1234);
    final b = Money(56);
    expect((a + b).fen, 1290);
    expect((a - b).fen, 1178);
    expect((-a).fen, -1234);
  });
}
