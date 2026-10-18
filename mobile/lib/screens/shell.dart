// Main app shell — bottom-nav switching between the five primary
// surfaces (Dashboard, Assistant, Transfers, Cards, Wealth, More).

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/i18n.dart';
import '../state/settings.dart';
import '../theme/app_theme.dart';
import '../theme/aurora.dart';
import '../widgets/network_banner.dart';
import 'cards_screen.dart';
import 'chat_screen.dart';
import 'dashboard_screen.dart';
import 'more_screen.dart';
import 'transfers_screen.dart';
import 'wealth_screen.dart';

final shellTabProvider = StateProvider<int>((_) => 0);

class AppShell extends ConsumerWidget {
  const AppShell({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    final tab = ref.watch(shellTabProvider);

    const tabs = <_Tab>[
      _Tab(icon: Icons.dashboard_outlined, active: Icons.dashboard, key: 'dashboard'),
      _Tab(icon: Icons.chat_bubble_outline, active: Icons.chat_bubble, key: 'assistant'),
      _Tab(icon: Icons.swap_horiz_outlined, active: Icons.swap_horiz, key: 'transfers'),
      _Tab(icon: Icons.credit_card_outlined, active: Icons.credit_card, key: 'cards'),
      _Tab(icon: Icons.savings_outlined, active: Icons.savings, key: 'wealth'),
      _Tab(icon: Icons.menu, active: Icons.menu_open, key: 'more'),
    ];

    final pages = const [
      DashboardScreen(),
      ChatScreen(),
      TransfersScreen(),
      CardsScreen(),
      WealthScreen(),
      MoreScreen(),
    ];

    final labels = [
      tr.t('nav.dashboard'),
      tr.t('nav.assistant'),
      tr.t('nav.transfers'),
      tr.t('nav.cards'),
      tr.t('nav.wealth'),
      tr.t('nav.more'),
    ];

    return Scaffold(
      extendBody: true,
      body: NetworkBanner(
        child: Stack(
          children: [
            const Positioned.fill(child: AuroraBackdrop()),
            IndexedStack(
              index: tab,
              children: pages,
            ),
          ],
        ),
      ),
      bottomNavigationBar: Container(
        decoration: const BoxDecoration(
          color: AppColors.surface,
          border: Border(top: BorderSide(color: AppColors.line)),
        ),
        child: SafeArea(
          top: false,
          child: NavigationBar(
            backgroundColor: AppColors.surface,
            selectedIndex: tab,
            onDestinationSelected: (i) =>
                ref.read(shellTabProvider.notifier).state = i,
            destinations: [
              for (var i = 0; i < tabs.length; i++)
                NavigationDestination(
                  icon: Icon(tabs[i].icon),
                  selectedIcon: Icon(tabs[i].active, color: AppColors.accent2),
                  label: labels[i],
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class _Tab {
  final IconData icon;
  final IconData active;
  final String key;
  const _Tab({required this.icon, required this.active, required this.key});
}