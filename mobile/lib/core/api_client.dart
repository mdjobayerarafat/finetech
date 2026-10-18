// HTTP client for the FinTech Pioneers backend.
//
// The web app calls /api/v1/* with credentials and CSRF. On mobile we
// can't rely on browser cookies, so we explicitly attach the session
// token (read from shared_preferences) and the CSRF token (fetched from
// /auth/csrf on login) on every state-changing call. The backend stores
// the same session in an HttpOnly cookie AND in a bearer token; the
// token path is what mobile uses.
//
// For a localhost backend, pass --dart-define=API_BASE=http://10.0.2.2:8080
// (Android emulator) or --dart-define=API_BASE=http://localhost:8080
// (iOS sim or desktop). The default is the Android emulator loopback.

import 'dart:convert';
import 'dart:io';

import 'package:dio/dio.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'api_error.dart';

class ApiClient {
  final Dio _dio;
  String? _sessionCookie;
  String? _csrfToken;

  ApiClient({String? baseUrl})
      : _dio = Dio(
          BaseOptions(
            baseUrl: baseUrl ?? _defaultBase(),
            connectTimeout: const Duration(seconds: 12),
            sendTimeout: const Duration(seconds: 30),
            receiveTimeout: const Duration(seconds: 30),
            headers: {
              'Accept': 'application/json',
              'Content-Type': 'application/json',
            },
            // We handle non-2xx ourselves so we can wrap the body cleanly.
            validateStatus: (s) => s != null && s < 500,
          ),
        ) {
    _dio.interceptors.add(
      InterceptorsWrapper(
        onRequest: (options, handler) {
          if (_sessionCookie != null) {
            options.headers['Cookie'] = 'bp_session=$_sessionCookie';
          }
          final method = options.method.toUpperCase();
          if (method != 'GET' &&
              method != 'HEAD' &&
              method != 'OPTIONS' &&
              _csrfToken != null) {
            options.headers['X-CSRF-Token'] = _csrfToken;
          }
          handler.next(options);
        },
      ),
    );
  }

  static String _defaultBase() {
    const fromEnv = String.fromEnvironment('API_BASE');
    if (fromEnv.isNotEmpty) return fromEnv;
    // 10.0.2.2 is the Android emulator's loopback to the host machine.
    return 'http://10.0.2.2:8080/api/v1';
  }

  String get baseUrl => _dio.options.baseUrl;

  bool get isAuthenticated => _sessionCookie != null;

  void setSession({String? cookie, String? csrfToken}) {
    if (cookie != null) _sessionCookie = cookie;
    if (csrfToken != null) _csrfToken = csrfToken;
  }

  void clearSession() {
    _sessionCookie = null;
    _csrfToken = null;
  }

  Future<void> loadSession() async {
    final prefs = await SharedPreferences.getInstance();
    _sessionCookie = prefs.getString('bp_session');
    _csrfToken = prefs.getString('bp_csrf');
  }

  Future<void> persistSession() async {
    final prefs = await SharedPreferences.getInstance();
    if (_sessionCookie != null) {
      await prefs.setString('bp_session', _sessionCookie!);
    } else {
      await prefs.remove('bp_session');
    }
    if (_csrfToken != null) {
      await prefs.setString('bp_csrf', _csrfToken!);
    } else {
      await prefs.remove('bp_csrf');
    }
  }

  /// Pull the cookie + csrf token out of a login response, persist them,
  /// and update the in-memory state.
  void adoptAuthResponse(Response<dynamic> res) {
    final raw = res.headers[HttpHeaders.setCookieHeader];
    if (raw != null) {
      for (final c in raw) {
        final i = c.indexOf('=');
        if (i <= 0) continue;
        final name = c.substring(0, i);
        final value = c.substring(i + 1).split(';').first;
        if (name == 'bp_session') _sessionCookie = value;
        if (name == 'bp_csrf') _csrfToken = value;
      }
    }
    if (res.data is Map) {
      final body = res.data as Map;
      if (body['csrf_token'] is String) _csrfToken = body['csrf_token'] as String;
    }
  }

  Future<dynamic> get(String path) => _send(() => _dio.get(path));
  Future<dynamic> post(String path, {Object? body}) =>
      _send(() => _dio.post(path, data: body == null ? null : jsonEncode(body)));
  Future<dynamic> patch(String path, {Object? body}) =>
      _send(() => _dio.patch(path, data: body == null ? null : jsonEncode(body)));
  Future<dynamic> delete(String path) => _send(() => _dio.delete(path));

  /// Raw Response access — needed for adoptAuthResponse on login so we
  /// can read Set-Cookie headers.
  Future<Response<dynamic>> rawPost(String path, {Object? body}) {
    return _dio.post(path, data: body == null ? null : jsonEncode(body));
  }

  Future<dynamic> _send(Future<Response<dynamic>> Function() fn) async {
    final res = await fn();
    final status = res.statusCode ?? 0;
    final raw = res.data;
    Map<String, dynamic>? body;
    if (raw is Map<String, dynamic>) {
      body = raw;
    } else if (raw is String && raw.isNotEmpty) {
      try {
        final decoded = jsonDecode(raw);
        if (decoded is Map<String, dynamic>) body = decoded;
      } catch (_) {
        // Not JSON — treat as opaque.
      }
    }
    if (status >= 200 && status < 300) {
      if (status == 204) return null;
      return body ?? raw;
    }
    final err = body?['error'] as Map?;
    throw ApiError(
      status: status,
      code: (err?['code'] as String?) ?? 'http_$status',
      message: (err?['message'] as String?) ?? 'Request failed',
      traceId: (err?['trace_id'] as String?) ?? '',
      details: (err?['details'] as Map?)?.cast<String, dynamic>() ?? const {},
      body: body,
    );
  }
}
