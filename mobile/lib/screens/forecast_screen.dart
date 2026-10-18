// Forecast / "Can I afford it?" — runs /forecast/simulate and shows
// the verdict + monthly series. Web equivalent: /forecast.

import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api.dart';
import '../core/api_error.dart';
import '../core/i18n.dart';
import '../core/money.dart';
import '../state/settings.dart';
import '../theme/app_theme.dart';
import '../widgets/glass_card.dart';

class ForecastScreen extends ConsumerStatefulWidget {
  const ForecastScreen({super.key});
  @override
  ConsumerState<ForecastScreen> createState() => _ForecastScreenState();
}

class _ForecastScreenState extends ConsumerState<ForecastScreen> {
  final _amount = TextEditingController(text: '500');
  Map<String, dynamic>? _result;
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    _amount.dispose();
    super.dispose();
  }

  Future<void> _run() async {
    final cny = double.tryParse(_amount.text.trim());
    if (cny == null || cny <= 0) {
      setState(() => _error = 'Enter an amount.');
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
      _result = null;
    });
    try {
      final r = await ref.read(apiProvider).forecast(cny);
      setState(() => _result = r);
    } catch (e) {
      setState(() => _error = e is ApiError ? e.message : '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Color _verdictColor(String? v) => switch (v) {
        'yes' => AppColors.tierGreen,
        'tight' => AppColors.tierYellow,
        _ => AppColors.tierRed,
      };

  String _verdictLabel(String? v, Tr tr) => switch (v) {
        'yes' => tr.t('forecast.verdict.yes'),
        'tight' => tr.t('forecast.verdict.tight'),
        _ => tr.t('forecast.verdict.no'),
      };

  @override
  Widget build(BuildContext context) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    return Scaffold(
      appBar: AppBar(title: Text(tr.t('forecast.title'))),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 32),
        children: [
          GlassCard(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  tr.t('forecast.amount'),
                  style: const TextStyle(
                    fontSize: 12,
                    color: AppColors.textMuted,
                  ),
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    Expanded(
                      child: TextField(
                        controller: _amount,
                        keyboardType:
                            const TextInputType.numberWithOptions(decimal: true),
                        style: const TextStyle(
                          fontSize: 22,
                          fontWeight: FontWeight.w700,
                        ),
                        decoration: const InputDecoration(prefixText: '¥ '),
                      ),
                    ),
                    const SizedBox(width: 10),
                    ElevatedButton(
                      onPressed: _busy ? null : _run,
                      child: _busy
                          ? const SizedBox(
                              width: 16,
                              height: 16,
                              child: CircularProgressIndicator(
                                strokeWidth: 2,
                                color: Colors.white,
                              ),
                            )
                          : Text(tr.t('forecast.run')),
                    ),
                  ],
                ),
                if (_error != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    _error!,
                    style: const TextStyle(
                      color: AppColors.tierRed,
                      fontSize: 12,
                    ),
                  ),
                ],
              ],
            ),
          ),
          if (_result != null) ...[
            const SizedBox(height: 16),
            _VerdictCard(
              result: _result!,
              color: _verdictColor(_result!['verdict'] as String?),
              label: _verdictLabel(_result!['verdict'] as String?, tr),
            ),
            const SizedBox(height: 16),
            _SeriesCard(result: _result!),
            const SizedBox(height: 16),
            if ((_result!['upcoming_bills'] as List?)?.isNotEmpty == true)
              _UpcomingCard(result: _result!),
          ],
        ],
      ),
    );
  }
}

class _VerdictCard extends StatelessWidget {
  final Map<String, dynamic> result;
  final Color color;
  final String label;
  const _VerdictCard({
    required this.result,
    required this.color,
    required this.label,
  });

  @override
  Widget build(BuildContext context) {
    return GlassCard(
      border: Border.all(color: color.withValues(alpha: 0.5)),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(
                result['verdict'] == 'yes'
                    ? Icons.check_circle_outline
                    : result['verdict'] == 'tight'
                        ? Icons.error_outline
                        : Icons.cancel_outlined,
                color: color,
                size: 22,
              ),
              const SizedBox(width: 8),
              Text(
                label,
                style: TextStyle(
                  color: color,
                  fontSize: 16,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ],
          ),
          if (result['narrative'] is String) ...[
            const SizedBox(height: 6),
            Text(
              result['narrative'] as String,
              style: const TextStyle(
                color: AppColors.textMuted,
                fontSize: 13,
                height: 1.45,
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _SeriesCard extends StatelessWidget {
  final Map<String, dynamic> result;
  const _SeriesCard({required this.result});

  @override
  Widget build(BuildContext context) {
    final series = (result['series'] as List? ?? const [])
        .cast<Map<String, dynamic>>();
    if (series.isEmpty) return const SizedBox.shrink();
    final spots = <FlSpot>[];
    final minY = series
        .map((p) => (p['projected_balance_fen'] as num).toDouble())
        .reduce((a, b) => a < b ? a : b);
    final maxY = series
        .map((p) => (p['projected_balance_fen'] as num).toDouble())
        .reduce((a, b) => a > b ? a : b);
    for (var i = 0; i < series.length; i++) {
      spots.add(FlSpot(
        i.toDouble(),
        (series[i]['projected_balance_fen'] as num).toDouble(),
      ));
    }
    return GlassCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Projected balance',
            style: TextStyle(
              fontSize: 12,
              color: AppColors.textMuted,
            ),
          ),
          const SizedBox(height: 8),
          SizedBox(
            height: 160,
            child: LineChart(
              LineChartData(
                gridData: const FlGridData(show: false),
                borderData: FlBorderData(show: false),
                titlesData: const FlTitlesData(show: false),
                minY: minY - 5000,
                maxY: maxY + 5000,
                lineBarsData: [
                  LineChartBarData(
                    spots: spots,
                    isCurved: true,
                    color: AppColors.accent2,
                    barWidth: 2.5,
                    dotData: const FlDotData(show: false),
                    belowBarData: BarAreaData(
                      show: true,
                      color: AppColors.accent2.withValues(alpha: 0.12),
                    ),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 6),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                fenToCny(minY.toInt()),
                style: const TextStyle(
                  fontSize: 11,
                  color: AppColors.textFaint,
                ),
              ),
              Text(
                fenToCny(maxY.toInt()),
                style: const TextStyle(
                  fontSize: 11,
                  color: AppColors.textFaint,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _UpcomingCard extends StatelessWidget {
  final Map<String, dynamic> result;
  const _UpcomingCard({required this.result});

  @override
  Widget build(BuildContext context) {
    final bills = (result['upcoming_bills'] as List? ?? const [])
        .cast<Map<String, dynamic>>();
    return GlassCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Upcoming bills',
            style: TextStyle(
              fontSize: 12,
              color: AppColors.textMuted,
            ),
          ),
          const SizedBox(height: 8),
          for (final b in bills)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: Row(
                children: [
                  Expanded(
                    child: Text(
                      b['subject'] as String? ?? '—',
                      style: const TextStyle(fontSize: 13),
                    ),
                  ),
                  Text(
                    b['due_at'] as String? ?? '',
                    style: const TextStyle(
                      fontSize: 11,
                      color: AppColors.textMuted,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Text(
                    fenToCny((b['amount_fen'] as num).toInt()),
                    style: const TextStyle(
                      fontSize: 13,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}
