// User / `me` response from GET /api/v1/me.

class AppUser {
  final String id;
  final String username;
  final String displayName;
  final String preferredLanguage;
  final String role; // 'customer' | 'admin'
  final String safetyLockState; // 'unlocked' | 'locked'
  final String? safetyLockReason;
  final int failedMfaCount;
  final String riskTolerance;
  final String horizon;

  const AppUser({
    required this.id,
    required this.username,
    required this.displayName,
    required this.preferredLanguage,
    required this.role,
    required this.safetyLockState,
    required this.failedMfaCount,
    required this.riskTolerance,
    required this.horizon,
    this.safetyLockReason,
  });

  bool get isAdmin => role == 'admin';
  bool get isLocked => safetyLockState == 'locked';

  factory AppUser.fromJson(Map<String, dynamic> j) => AppUser(
        id: j['id'] as String,
        username: j['username'] as String,
        displayName: j['display_name'] as String,
        preferredLanguage: j['preferred_language'] as String? ?? 'en',
        role: j['role'] as String? ?? 'customer',
        safetyLockState: j['safety_lock_state'] as String? ?? 'unlocked',
        safetyLockReason: j['safety_lock_reason'] as String?,
        failedMfaCount: (j['failed_mfa_count'] as num?)?.toInt() ?? 0,
        riskTolerance: j['risk_tolerance'] as String? ?? 'medium',
        horizon: j['horizon'] as String? ?? 'short',
      );
}
