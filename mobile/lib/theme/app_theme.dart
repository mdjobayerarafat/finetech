// Visual language for FinTech Pioneers mobile.
//
// The web frontend leans on a dark, glassy "aurora" aesthetic with
// vivid accent gradients. We keep the same idea on mobile: a deep
// indigo/cyan backdrop, glass cards, and per-tier accent colors that
// echo the policy engine.

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:google_fonts/google_fonts.dart';

class AppColors {
  // Backdrop
  static const bg = Color(0xFF0A0E1A);
  static const surface = Color(0xFF111827);
  static const surfaceHi = Color(0xFF1F2937);

  // Foreground
  static const text = Color(0xFFE5E7EB);
  static const textMuted = Color(0xFF9CA3AF);
  static const textFaint = Color(0xFF6B7280);

  // Accent
  static const accent = Color(0xFF6366F1); // indigo
  static const accent2 = Color(0xFF22D3EE); // cyan
  static const accent3 = Color(0xFFF472B6); // pink
  static const accentGradient = [Color(0xFF818CF8), Color(0xFF22D3EE)];

  // Tier colors
  static const tierGreen = Color(0xFF10B981);
  static const tierYellow = Color(0xFFF59E0B);
  static const tierRed = Color(0xFFEF4444);

  // Lines
  static const line = Color(0xFF1F2937);
  static const lineHi = Color(0xFF334155);

  // Aurora glow tints used by the AnimatedBackground
  static const aurora1 = Color(0xFF4F46E5);
  static const aurora2 = Color(0xFF06B6D4);
  static const aurora3 = Color(0xFFEC4899);
}

ThemeData buildAppTheme() {
  final base = ThemeData.dark(useMaterial3: true);
  final textTheme = GoogleFonts.interTextTheme(base.textTheme).apply(
    bodyColor: AppColors.text,
    displayColor: AppColors.text,
  );

  return base.copyWith(
    scaffoldBackgroundColor: AppColors.bg,
    canvasColor: AppColors.bg,
    colorScheme: const ColorScheme.dark(
      surface: AppColors.surface,
      primary: AppColors.accent,
      secondary: AppColors.accent2,
      onPrimary: Colors.white,
      onSurface: AppColors.text,
      error: AppColors.tierRed,
    ),
    textTheme: textTheme,
    appBarTheme: AppBarTheme(
      backgroundColor: Colors.transparent,
      foregroundColor: AppColors.text,
      elevation: 0,
      scrolledUnderElevation: 0,
      systemOverlayStyle: const SystemUiOverlayStyle(
        statusBarColor: Colors.transparent,
        statusBarIconBrightness: Brightness.light,
        statusBarBrightness: Brightness.dark,
      ),
      titleTextStyle: GoogleFonts.spaceGrotesk(
        color: AppColors.text,
        fontSize: 18,
        fontWeight: FontWeight.w600,
      ),
    ),
    bottomNavigationBarTheme: const BottomNavigationBarThemeData(
      backgroundColor: AppColors.surface,
      selectedItemColor: AppColors.accent2,
      unselectedItemColor: AppColors.textMuted,
      showUnselectedLabels: true,
      type: BottomNavigationBarType.fixed,
    ),
    navigationBarTheme: NavigationBarThemeData(
      backgroundColor: AppColors.surface,
      indicatorColor: AppColors.accent.withValues(alpha: 0.20),
      labelTextStyle: WidgetStatePropertyAll(
        GoogleFonts.inter(fontSize: 12, color: AppColors.text),
      ),
      iconTheme: const WidgetStatePropertyAll(
        IconThemeData(color: AppColors.text),
      ),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: AppColors.surface,
      hintStyle: GoogleFonts.inter(color: AppColors.textFaint),
      labelStyle: GoogleFonts.inter(color: AppColors.textMuted),
      contentPadding: const EdgeInsets.symmetric(
        horizontal: 16,
        vertical: 14,
      ),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(14),
        borderSide: const BorderSide(color: AppColors.line),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(14),
        borderSide: const BorderSide(color: AppColors.line),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(14),
        borderSide: const BorderSide(color: AppColors.accent2, width: 1.4),
      ),
    ),
    cardTheme: const CardThemeData(
      color: AppColors.surface,
      elevation: 0,
      margin: EdgeInsets.zero,
    ),
    dividerColor: AppColors.line,
    iconTheme: const IconThemeData(color: AppColors.text),
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ElevatedButton.styleFrom(
        backgroundColor: AppColors.accent,
        foregroundColor: Colors.white,
        elevation: 0,
        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(14),
        ),
        textStyle: GoogleFonts.inter(
          fontWeight: FontWeight.w600,
          fontSize: 15,
        ),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: AppColors.text,
        side: const BorderSide(color: AppColors.lineHi),
        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(14),
        ),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(foregroundColor: AppColors.accent2),
    ),
    snackBarTheme: SnackBarThemeData(
      backgroundColor: AppColors.surfaceHi,
      contentTextStyle: GoogleFonts.inter(color: AppColors.text),
      behavior: SnackBarBehavior.floating,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
      ),
    ),
    dialogTheme: const DialogThemeData(
      backgroundColor: AppColors.surface,
      titleTextStyle: TextStyle(
        color: AppColors.text,
        fontSize: 18,
        fontWeight: FontWeight.w600,
      ),
      contentTextStyle: TextStyle(color: AppColors.text, fontSize: 15),
    ),
    chipTheme: ChipThemeData(
      backgroundColor: AppColors.surfaceHi,
      labelStyle: GoogleFonts.inter(color: AppColors.text, fontSize: 12),
      side: const BorderSide(color: AppColors.line),
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
    ),
  );
}
