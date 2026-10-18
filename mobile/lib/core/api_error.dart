// Typed error class for API failures. Mirrors the `ApiError` in the web
// frontend's lib/api.ts so the same friendly-error mapping can be reused.

class ApiError implements Exception {
  final int status;
  final String code;
  final String message;
  final String traceId;
  final Map<String, dynamic> details;
  final Map<String, dynamic>? body;

  ApiError({
    required this.status,
    required this.code,
    required this.message,
    this.traceId = '',
    this.details = const {},
    this.body,
  });

  @override
  String toString() => 'ApiError($status $code: $message)';
}

/// Lightweight "what to do next" envelope so the chat surface can show a
/// short toast without parsing the raw message every time.
class FriendlyError {
  final String title;
  final String body;
  final String? actionLabel;
  final String? actionRoute;

  const FriendlyError({
    required this.title,
    required this.body,
    this.actionLabel,
    this.actionRoute,
  });
}

FriendlyError friendlyError(ApiError err) {
  final d = err.details;
  final whatNext = d['what_next'] is String ? d['what_next'] as String : null;
  final question = d['question'] is String ? d['question'] as String : null;
  switch (err.code) {
    case 'insufficient_funds':
      return FriendlyError(
        title: 'Not enough in this account',
        body: whatNext ?? 'Top up, pick a different source, or reduce the amount.',
        actionLabel: 'Open accounts',
      );
    case 'below_minimum':
      return FriendlyError(
        title: 'Amount is below the minimum',
        body: whatNext ?? 'Try a higher amount, or pick a different goal.',
      );
    case 'clarification_required':
      return FriendlyError(
        title: 'One more thing',
        body: question ?? 'The agent needs more information to continue.',
      );
    case 'forbidden':
      return FriendlyError(
        title: 'Blocked by an admin rule',
        body: whatNext ?? err.message,
        actionLabel: 'Open chat',
      );
    case 'schema_invalid':
      return FriendlyError(
        title: 'Check the input',
        body: whatNext ?? err.message,
      );
    case 'not_found':
      return FriendlyError(title: "Couldn't find it", body: err.message);
    case 'mfa_required':
    case 'mfa_failed':
      return FriendlyError(
        title: err.code == 'mfa_failed' ? "That code didn't match" : 'MFA required',
        body: err.message,
        actionLabel: 'Open the action',
      );
    case 'expired':
      return FriendlyError(
        title: 'This expired',
        body: 'Open the chat to start a new confirmation.',
      );
    case 'parameter_changed':
      return FriendlyError(
        title: 'The parameters changed',
        body: 'Please re-confirm with the new values.',
      );
    default:
      return FriendlyError(title: 'Something went wrong', body: err.message);
  }
}
