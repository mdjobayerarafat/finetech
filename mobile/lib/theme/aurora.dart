// Aurora background — a slow, painterly wash of indigo/cyan/pink that
// gives the dark UI some life without getting in the way of text.

import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'app_theme.dart';

class AuroraBackdrop extends StatefulWidget {
  const AuroraBackdrop({super.key});
  @override
  State<AuroraBackdrop> createState() => _AuroraBackdropState();
}

class _AuroraBackdropState extends State<AuroraBackdrop>
    with SingleTickerProviderStateMixin {
  late final AnimationController _c =
      AnimationController(vsync: this, duration: const Duration(seconds: 22))
        ..repeat();

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _c,
      builder: (_, __) => CustomPaint(
        painter: _AuroraPainter(_c.value),
        child: const SizedBox.expand(),
      ),
    );
  }
}

class _AuroraPainter extends CustomPainter {
  final double t; // 0..1
  _AuroraPainter(this.t);

  @override
  void paint(Canvas canvas, Size size) {
    final rect = Offset.zero & size;
    // Base vertical wash
    final base = Paint()
      ..shader = const LinearGradient(
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
        colors: [Color(0xFF0A0E1A), Color(0xFF0F172A), Color(0xFF0A0E1A)],
      ).createShader(rect);
    canvas.drawRect(rect, base);

    // Three soft glows that drift in a circle.
    final cx = size.width / 2;
    final cy = size.height / 2;
    final r = math.max(size.width, size.height) * 0.7;
    final a = t * 2 * math.pi;
    void glow(Offset c, Color color, double radius) {
      final p = Paint()
        ..shader = RadialGradient(
          colors: [color.withValues(alpha: 0.32), color.withValues(alpha: 0)],
        ).createShader(Rect.fromCircle(center: c, radius: radius));
      canvas.drawCircle(c, radius, p);
    }

    glow(Offset(cx + r * math.cos(a), cy + r * math.sin(a)),
        AppColors.aurora1, r * 0.55);
    glow(Offset(cx + r * math.cos(a + 2.1), cy + r * math.sin(a + 2.1)),
        AppColors.aurora2, r * 0.5);
    glow(Offset(cx + r * math.cos(a + 4.2), cy + r * math.sin(a + 4.2)),
        AppColors.aurora3, r * 0.45);
  }

  @override
  bool shouldRepaint(_AuroraPainter old) => old.t != t;
}
