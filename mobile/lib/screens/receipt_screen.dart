// Receipt scan — pick an image, send to /receipts/scan, then jump to
// the chat with the extracted fields pre-filled.

import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:image_picker/image_picker.dart';

import '../core/api.dart';
import '../core/api_error.dart';
import '../core/i18n.dart';
import '../core/money.dart';
import '../state/settings.dart';
import '../theme/app_theme.dart';
import '../widgets/glass_card.dart';

class ReceiptScreen extends ConsumerStatefulWidget {
  const ReceiptScreen({super.key});
  @override
  ConsumerState<ReceiptScreen> createState() => _ReceiptScreenState();
}

class _ReceiptScreenState extends ConsumerState<ReceiptScreen> {
  final _picker = ImagePicker();
  XFile? _file;
  Map<String, dynamic>? _draft;
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    _picker.toString();
    super.dispose();
  }

  Future<void> _pick(ImageSource source) async {
    try {
      final f = await _picker.pickImage(
        source: source,
        maxWidth: 1280,
        imageQuality: 80,
      );
      if (f != null) setState(() => _file = f);
    } catch (e) {
      setState(() => _error = '$e');
    }
  }

  Future<void> _scan() async {
    if (_file == null) {
      setState(() => _error = 'Pick an image first.');
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
      _draft = null;
    });
    try {
      final bytes = await File(_file!.path).readAsBytes();
      final b64 = base64Encode(bytes);
      final draft = await ref.read(apiProvider).scanReceipt(
            filename: _file!.name,
            imageB64: b64,
          );
      setState(() => _draft = draft);
    } catch (e) {
      setState(() => _error = e is ApiError ? e.message : '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    return Scaffold(
      appBar: AppBar(title: const Text('Receipt scan')),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 32),
        children: [
          GlassCard(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text(
                  'Capture or upload a receipt',
                  style: TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 6),
                Text(
                  tr.t('brand.disclaimer'),
                  style: const TextStyle(
                    fontSize: 11,
                    color: AppColors.textFaint,
                  ),
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    Expanded(
                      child: OutlinedButton.icon(
                        icon: const Icon(Icons.camera_alt_outlined, size: 18),
                        label: const Text('Camera'),
                        onPressed: () => _pick(ImageSource.camera),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: OutlinedButton.icon(
                        icon: const Icon(Icons.photo_outlined, size: 18),
                        label: const Text('Gallery'),
                        onPressed: () => _pick(ImageSource.gallery),
                      ),
                    ),
                  ],
                ),
                if (_file != null) ...[
                  const SizedBox(height: 12),
                  ClipRRect(
                    borderRadius: BorderRadius.circular(10),
                    child: Image.file(
                      File(_file!.path),
                      height: 180,
                      width: double.infinity,
                      fit: BoxFit.cover,
                    ),
                  ),
                  const SizedBox(height: 12),
                  ElevatedButton(
                    onPressed: _busy ? null : _scan,
                    child: _busy
                        ? const SizedBox(
                            width: 16,
                            height: 16,
                            child: CircularProgressIndicator(
                              strokeWidth: 2,
                              color: Colors.white,
                            ),
                          )
                        : const Text('Scan'),
                  ),
                ],
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
          if (_draft != null) ...[
            const SizedBox(height: 16),
            GlassCard(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text(
                    'Extracted draft',
                    style: TextStyle(
                      fontSize: 14,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 8),
                  _row('Merchant', _draft!['payee_hint']?.toString() ?? '—'),
                  _row('Amount',
                      fenToCny((_draft!['amount_fen'] as num?)?.toInt() ?? 0)),
                  _row('Currency',
                      _draft!['currency']?.toString() ?? 'CNY'),
                  _row('Confidence',
                      '${((_draft!['confidence'] as num?)?.toDouble() ?? 0 * 100).toStringAsFixed(0)}%'),
                  _row('Source', _draft!['source']?.toString() ?? '—'),
                  if ((_draft!['notes'] as List?)?.isNotEmpty == true) ...[
                    const SizedBox(height: 6),
                    Text(
                      'Notes',
                      style: TextStyle(
                        fontSize: 11,
                        color: AppColors.textMuted,
                      ),
                    ),
                    for (final n in (_draft!['notes'] as List))
                      Text('• $n',
                          style: const TextStyle(
                            fontSize: 12,
                            color: AppColors.textMuted,
                          )),
                  ],
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }

  Widget _row(String label, String value) => Padding(
        padding: const EdgeInsets.symmetric(vertical: 3),
        child: Row(
          children: [
            SizedBox(
              width: 100,
              child: Text(
                label,
                style: const TextStyle(
                  fontSize: 12,
                  color: AppColors.textMuted,
                ),
              ),
            ),
            Expanded(
              child: Text(
                value,
                style: const TextStyle(fontSize: 13),
              ),
            ),
          ],
        ),
      );
}
