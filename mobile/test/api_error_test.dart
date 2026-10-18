// Friendly error mapper — make sure the common backend codes surface
// the right copy to the user.

import 'package:fintech_pioneers/core/api_error.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('insufficient_funds → "Not enough in this account"', () {
    final e = ApiError(
      status: 422,
      code: 'insufficient_funds',
      message: 'insufficient funds',
      details: const {'what_next': 'Try a smaller amount.'},
    );
    final f = friendlyError(e);
    expect(f.title, 'Not enough in this account');
    expect(f.body, 'Try a smaller amount.');
    expect(f.actionLabel, 'Open accounts');
  });

  test('clarification_required → "One more thing"', () {
    final e = ApiError(
      status: 400,
      code: 'clarification_required',
      message: 'needs info',
      details: const {'question': 'Who should I pay?'},
    );
    final f = friendlyError(e);
    expect(f.title, 'One more thing');
    expect(f.body, 'Who should I pay?');
  });

  test('mfa_failed uses "didn\'t match" copy', () {
    final e = ApiError(
      status: 403,
      code: 'mfa_failed',
      message: 'wrong code',
    );
    final f = friendlyError(e);
    expect(f.title, "That code didn't match");
  });

  test('unknown code falls back to the raw message', () {
    final e = ApiError(status: 500, code: 'weird', message: 'oops');
    final f = friendlyError(e);
    expect(f.title, 'Something went wrong');
    expect(f.body, 'oops');
  });
}
