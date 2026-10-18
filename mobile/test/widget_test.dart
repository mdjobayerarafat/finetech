// Smoke test: the app boots to the login screen. We can't go further
// without a live backend, but the boot path is the most fragile piece
// of code (provider wiring, theme, auth bootstrap) so it's worth a
// minimal check.

import 'package:fintech_pioneers/app.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('app boots to the login screen', (WidgetTester tester) async {
    await tester.pumpWidget(
      const ProviderScope(child: FinTechPioneersApp()),
    );
    await tester.pump();
    // The login title is rendered in both languages; pump once more so
    // the auth bootstrap has a chance to settle.
    await tester.pump(const Duration(milliseconds: 50));
    expect(find.text('Welcome back').evaluate().isNotEmpty ||
            find.text('欢迎回来').evaluate().isNotEmpty,
        isTrue,
        reason: 'Login screen title should render in either language.');
  });
}