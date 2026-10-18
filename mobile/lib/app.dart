// Top-level app widget. Decides between the splash/loading state,
// the login screen, and the main shell, and exposes a small toast
// surface for transient errors.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'core/language.dart';
import 'screens/login_screen.dart';
import 'screens/shell.dart';
import 'state/auth.dart';
import 'state/settings.dart';
import 'theme/app_theme.dart';

class FinTechPioneersApp extends ConsumerWidget {
  const FinTechPioneersApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final settings = ref.watch(settingsProvider);
    final auth = ref.watch(authProvider);
    final lang = settings.language;
    final isZh = lang == AppLanguage.zh;
    return MaterialApp(
      title: 'FinTech Pioneers',
      debugShowCheckedModeBanner: false,
      theme: buildAppTheme(),
      locale: Locale(isZh ? 'zh' : 'en'),
      supportedLocales: const [Locale('en'), Locale('zh')],
      localizationsDelegates: const [],
      home: _Gate(auth: auth),
    );
  }
}

class _Gate extends ConsumerStatefulWidget {
  final AuthData auth;
  const _Gate({required this.auth});
  @override
  ConsumerState<_Gate> createState() => _GateState();
}

class _GateState extends ConsumerState<_Gate> {
  bool _bootstrapped = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      if (!_bootstrapped) {
        _bootstrapped = true;
        await ref.read(authProvider.notifier).bootstrap();
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    if (widget.auth.initializing) {
      return const _Splash();
    }
    if (widget.auth.user == null) {
      return const LoginScreen();
    }
    return const AppShell();
  }
}

class _Splash extends StatelessWidget {
  const _Splash();
  @override
  Widget build(BuildContext context) {
    return const Scaffold(
      backgroundColor: AppColors.bg,
      body: Center(
        child: SizedBox(
          width: 32,
          height: 32,
          child: CircularProgressIndicator(strokeWidth: 2),
        ),
      ),
    );
  }
}