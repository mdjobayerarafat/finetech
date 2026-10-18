// Network banner — listens to connectivity_plus and shows a slim
// "you're offline" strip at the top of the shell when the device
// has no network. Auto-hides when connectivity returns.

import 'package:connectivity_plus/connectivity_plus.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../theme/app_theme.dart';

final _connectivityProvider =
    StreamProvider.autoDispose<List<ConnectivityResult>>((ref) {
  return Connectivity().onConnectivityChanged;
});

class NetworkBanner extends ConsumerWidget {
  final Widget child;
  const NetworkBanner({super.key, required this.child});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final asyncConn = ref.watch(_connectivityProvider);
    final initial = ConnectivityResult.values
        .toSet()
        .contains(ConnectivityResult.wifi);
    final online = asyncConn.maybeWhen(
      data: (list) => list.any((r) => r != ConnectivityResult.none),
      orElse: () => initial,
    );
    return Stack(
      children: [
        child,
        if (!online)
          Positioned(
            top: 0,
            left: 0,
            right: 0,
            child: SafeArea(
              bottom: false,
              child: Container(
                width: double.infinity,
                color: AppColors.tierRed.withValues(alpha: 0.92),
                padding:
                    const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                child: Row(
                  children: const [
                    Icon(Icons.wifi_off, color: Colors.white, size: 14),
                    SizedBox(width: 6),
                    Text(
                      "You're offline. Cached data shown.",
                      style: TextStyle(
                        color: Colors.white,
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
      ],
    );
  }
}