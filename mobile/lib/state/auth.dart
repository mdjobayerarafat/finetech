// Auth state: ApiClient instance, current user, login flow.

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api.dart';
import '../core/api_client.dart';
import '../core/api_error.dart';
import '../models/user.dart';

class AuthData {
  final AppUser? user;
  final bool initializing;
  final String? error;
  const AuthData({this.user, this.initializing = false, this.error});
}

class AuthNotifier extends StateNotifier<AuthData> {
  final Api _api;
  final ApiClient _client;
  AuthNotifier(this._api, this._client) : super(const AuthData());

  /// Try to restore a session. Returns true if a user is now signed in.
  Future<bool> bootstrap() async {
    state = const AuthData(initializing: true);
    if (!_client.isAuthenticated) {
      state = const AuthData();
      return false;
    }
    try {
      final body = await _client.get('/me') as Map<String, dynamic>;
      final user = AppUser.fromJson(body);
      state = AuthData(user: user);
      return true;
    } catch (e) {
      _client.clearSession();
      await _client.persistSession();
      state = AuthData(error: _describe(e));
      return false;
    }
  }

  Future<bool> login(String username, String password) async {
    state = const AuthData(initializing: true);
    try {
      await _api.login(username, password);
      final body = await _client.get('/me') as Map<String, dynamic>;
      state = AuthData(user: AppUser.fromJson(body));
      return true;
    } catch (e) {
      state = AuthData(error: _describe(e));
      return false;
    }
  }

  Future<void> logout() async {
    await _api.logout();
    state = const AuthData();
  }

  void updateUser(AppUser user) {
    state = AuthData(user: user);
  }

  String _describe(Object e) {
    if (e is ApiError) return e.message;
    return 'Network error.';
  }
}

final authProvider =
    StateNotifierProvider<AuthNotifier, AuthData>((ref) {
  return AuthNotifier(ref.watch(apiProvider), ref.watch(apiClientProvider));
});

/// Convenience: just the current user (null if not signed in).
final currentUserProvider = Provider<AppUser?>((ref) {
  return ref.watch(authProvider).user;
});