// Higher-level API surface built on top of ApiClient.
// Mirrors the namespaces in frontend/src/lib/api.ts (auth, accounts,
// transactions, transfers, cards, wealth, agent, subscriptions).

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/account.dart';
import '../models/agent.dart';
import '../models/budget.dart';
import '../models/card.dart';
import '../models/subscription.dart';
import '../models/transfer.dart';
import '../models/wealth.dart';
import 'api_client.dart';
import 'language.dart';

class Api {
  final ApiClient client;
  Api(this.client);

  // --- Auth ----------------------------------------------------------------

  Future<({String userId, String role, String? csrfToken})> login(
      String username, String password) async {
    final res = await client.rawPost('/auth/login',
        body: {'username': username, 'password': password});
    client.adoptAuthResponse(res);
    await client.persistSession();
    final body = res.data as Map<String, dynamic>;
    return (
      userId: body['user_id'] as String,
      role: body['role'] as String,
      csrfToken: body['csrf_token'] as String?,
    );
  }

  Future<void> logout() async {
    try {
      await client.post('/auth/logout');
    } catch (_) {/* best-effort */}
    client.clearSession();
    await client.persistSession();
  }

  Future<({String userId, String role, AppLanguage language})> me() async {
    final body = await client.get('/me') as Map<String, dynamic>;
    return (
      userId: body['id'] as String,
      role: body['role'] as String,
      language: AppLanguageX.fromCode(body['preferred_language'] as String?),
    );
  }

  Future<void> setPreferredLanguage(AppLanguage lang) async {
    await client.post('/me/language', body: {'language': lang.code});
  }

  // --- Accounts / reports --------------------------------------------------

  Future<List<Account>> accounts() async {
    final list = await client.get('/accounts') as List;
    return list.cast<Map<String, dynamic>>().map(Account.fromJson).toList();
  }

  Future<List<Txn>> transactions({String? accountId, int limit = 20}) async {
    final q = <String, String>{'limit': '$limit'};
    if (accountId != null) q['account_id'] = accountId;
    final list = await client
        .get('/transactions?${Uri(queryParameters: q).query}') as List;
    return list.cast<Map<String, dynamic>>().map(Txn.fromJson).toList();
  }

  Future<SpendingReport> spending() async {
    final body = await client.get('/reports/spending') as Map<String, dynamic>;
    return SpendingReport.fromJson(body);
  }

  // --- Transfers -----------------------------------------------------------

  Future<List<Payee>> payees({String? query}) async {
    final qs = query == null || query.isEmpty
        ? ''
        : '?q=${Uri.encodeQueryComponent(query)}';
    final list = await client.get('/smart-transfers/payees$qs') as List;
    return list.cast<Map<String, dynamic>>().map(Payee.fromJson).toList();
  }

  Future<TransferPreview> previewTransfer(Map<String, dynamic> body) async {
    final res = await client.post('/transfers/preview', body: body)
        as Map<String, dynamic>;
    return TransferPreview.fromJson(res);
  }

  Future<TransferCreateOut> createTransfer(Map<String, dynamic> body) async {
    final res = await client.post('/transfers', body: body)
        as Map<String, dynamic>;
    return TransferCreateOut.fromJson(res);
  }

  // --- Cards ---------------------------------------------------------------

  Future<List<VirtualCard>> cards() async {
    final list = await client.get('/cards') as List;
    return list.cast<Map<String, dynamic>>().map(VirtualCard.fromJson).toList();
  }

  Future<VirtualCard> issueCard(String displayName) async {
    final res = await client.post('/cards', body: {'display_name': displayName})
        as Map<String, dynamic>;
    return VirtualCard.fromJson(res);
  }

  Future<VirtualCard> freezeCard(String id) async {
    final res = await client.post('/cards/$id/freeze')
        as Map<String, dynamic>;
    return VirtualCard.fromJson(res);
  }

  Future<VirtualCard> unfreezeCard(String id) async {
    final res = await client.post('/cards/$id/unfreeze')
        as Map<String, dynamic>;
    return VirtualCard.fromJson(res);
  }

  Future<VirtualCard> cancelCard(String id) async {
    final res = await client.post('/cards/$id/cancel')
        as Map<String, dynamic>;
    return VirtualCard.fromJson(res);
  }

  Future<VirtualCard> reportLostCard(String id) async {
    final res = await client.post('/cards/$id/report-lost')
        as Map<String, dynamic>;
    return VirtualCard.fromJson(res);
  }

  // --- Wealth --------------------------------------------------------------

  Future<List<WealthProduct>> wealthProducts() async {
    final list = await client.get('/wealth/products') as List;
    return list
        .cast<Map<String, dynamic>>()
        .map(WealthProduct.fromJson)
        .toList();
  }

  Future<List<WealthHolding>> wealthHoldings() async {
    final list = await client.get('/wealth/holdings') as List;
    return list
        .cast<Map<String, dynamic>>()
        .map(WealthHolding.fromJson)
        .toList();
  }

  Future<Map<String, dynamic>> subscribeWealth(
      String productId, int amountFen) async {
    final res = await client.post('/wealth/subscribe',
        body: {'product_id': productId, 'amount': (amountFen / 100).toString()});
    return (res as Map).cast<String, dynamic>();
  }

  Future<Map<String, dynamic>> redeemWealth(String holdingId) async {
    final res = await client.post('/wealth/redeem',
        body: {'holding_id': holdingId});
    return (res as Map).cast<String, dynamic>();
  }

  // --- Subscriptions -------------------------------------------------------

  Future<List<Subscription>> subscriptions() async {
    final list = await client.get('/subscriptions') as List;
    return list
        .cast<Map<String, dynamic>>()
        .map(Subscription.fromJson)
        .toList();
  }

  Future<Map<String, dynamic>> cancelSubscription(String id) async {
    final res = await client.post('/subscriptions/$id/cancel')
        as Map<String, dynamic>;
    return res;
  }

  // --- Agent ---------------------------------------------------------------

  Future<AgentMessageOut> sendAgentMessage(String text, AppLanguage lang) async {
    final res = await client.post('/agent/messages',
        body: {'text': text, 'language': lang.code})
        as Map<String, dynamic>;
    return AgentMessageOut.fromJson(res);
  }

  Future<PendingAction> getAction(String id) async {
    final res = await client.get('/actions/$id') as Map<String, dynamic>;
    return PendingAction.fromJson(res);
  }

  Future<ConfirmOut> confirmAction(String id) async {
    final res = await client.post('/actions/$id/confirm')
        as Map<String, dynamic>;
    return ConfirmOut.fromJson(res);
  }

  Future<ConfirmOut> verifyAction(String id, String code) async {
    final res = await client.post('/actions/$id/verify',
        body: {'code': code}) as Map<String, dynamic>;
    return ConfirmOut.fromJson(res);
  }

  // --- Forecast (can-i-afford-it?) ----------------------------------------

  Future<Map<String, dynamic>> forecast(double amountCny) async {
    final res = await client.post('/forecast/simulate',
        body: {'amount_cny': amountCny}) as Map<String, dynamic>;
    return res;
  }

  // --- Budgets ------------------------------------------------------------

  Future<List<Budget>> budgets() async {
    final res = await client.get('/budgets') as Map<String, dynamic>;
    final list = (res['budgets'] as List? ?? const []);
    return list.cast<Map<String, dynamic>>().map(Budget.fromJson).toList();
  }

  Future<BudgetSummary> budgetSummary() async {
    final res = await client.get('/budgets/summary') as Map<String, dynamic>;
    return BudgetSummary.fromJson(res);
  }

  Future<Budget> setBudget({
    required String category,
    required int amountFen,
    String? note,
  }) async {
    final res = await client.post('/budgets', body: {
      'category': category,
      'amount': (amountFen / 100).toString(),
      if (note != null) 'note': note,
    }) as Map<String, dynamic>;
    return Budget.fromJson(res);
  }

  Future<void> deleteBudget(String id) async {
    await client.delete('/budgets/$id');
  }

  // --- Goals --------------------------------------------------------------

  Future<List<Map<String, dynamic>>> goals() async {
    final res = await client.get('/goals') as Map<String, dynamic>;
    return ((res['items'] as List?) ?? const [])
        .cast<Map<String, dynamic>>();
  }

  Future<Map<String, dynamic>> contributeToGoal(
    String id,
    int amountFen,
    String sourceAccountId, {
    String memo = '',
  }) async {
    final res = await client.post('/goals/$id/contribute', body: {
      'amount_fen': amountFen,
      'source_account_id': sourceAccountId,
      'memo': memo,
    }) as Map<String, dynamic>;
    return res;
  }

  // --- Daily digest -------------------------------------------------------

  Future<Map<String, dynamic>> dailyDigest() async {
    final res = await client.get('/insights/daily') as Map<String, dynamic>;
    return res;
  }

  // --- Pending workflows (cross-session resume) ---------------------------

  Future<List<Map<String, dynamic>>> pendingWorkflows() async {
    final res = await client
        .get('/workflows-pending') as Map<String, dynamic>;
    return ((res['items'] as List?) ?? const [])
        .cast<Map<String, dynamic>>();
  }

  // --- Receipt scan -------------------------------------------------------

  Future<Map<String, dynamic>> scanReceipt({
    String? merchantHint,
    double? amountCny,
    String? filename,
    String? imageB64,
  }) async {
    final res = await client.post('/receipts/scan', body: {
      if (merchantHint != null) 'merchant_hint': merchantHint,
      if (amountCny != null) 'amount_cny': amountCny,
      if (filename != null) 'filename': filename,
      if (imageB64 != null) 'image_b64': imageB64,
    }) as Map<String, dynamic>;
    return res;
  }

  // --- Voice --------------------------------------------------------------

  Future<Map<String, dynamic>> voiceStt(String audioB64,
      {String mime = 'audio/webm'}) async {
    final res = await client.post('/voice/stt',
        body: {'audio_b64': audioB64, 'mime': mime})
        as Map<String, dynamic>;
    return res;
  }

  Future<Map<String, dynamic>> voiceTts(String text, AppLanguage lang) async {
    final res = await client.post('/voice/tts',
        body: {'text': text, 'language': lang.code})
        as Map<String, dynamic>;
    return res;
  }
}

extension on ApiClient {
  // rawPost moved to ApiClient itself.
}

// ---------------------------------------------------------------------------
// Riverpod providers for the API surface.
// ---------------------------------------------------------------------------

final apiClientProvider = Provider<ApiClient>((ref) {
  final client = ApiClient();
  // Fire-and-forget restore of the saved session so the very first
  // /me call (driven by AuthNotifier.bootstrap) doesn't 401.
  client.loadSession();
  return client;
});

final apiProvider = Provider<Api>((ref) => Api(ref.watch(apiClientProvider)));
