// Frosted-glass container. Used as the basic surface for cards,
// sheets, and the shell. Backed by BackdropFilter so it picks up the
// aurora wash behind it.

import 'dart:ui';
import 'package:flutter/material.dart';
import '../theme/app_theme.dart';

class GlassCard extends StatelessWidget {
  final Widget child;
  final EdgeInsetsGeometry padding;
  final double radius;
  final Color? tint;
  final Border? border;
  final VoidCallback? onTap;

  const GlassCard({
    super.key,
    required this.child,
    this.padding = const EdgeInsets.all(16),
    this.radius = 18,
    this.tint,
    this.border,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final radius0 = BorderRadius.circular(radius);
    final card = ClipRRect(
      borderRadius: radius0,
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 18, sigmaY: 18),
        child: Container(
          padding: padding,
          decoration: BoxDecoration(
            gradient: LinearGradient(
              begin: Alignment.topLeft,
              end: Alignment.bottomRight,
              colors: [
                (tint ?? AppColors.surface).withValues(alpha: 0.55),
                (tint ?? AppColors.surface).withValues(alpha: 0.35),
              ],
            ),
            borderRadius: radius0,
            border: border ??
                Border.all(color: AppColors.lineHi.withValues(alpha: 0.6)),
          ),
          child: child,
        ),
      ),
    );
    if (onTap == null) return card;
    return InkWell(onTap: onTap, borderRadius: radius0, child: card);
  }
}