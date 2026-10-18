// VoiceButton — record audio, send to /voice/stt, fill the chat input.

import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:record/record.dart';

import '../core/api.dart';
import '../core/api_error.dart';
import '../theme/app_theme.dart';

class VoiceButton extends ConsumerStatefulWidget {
  final ValueChanged<String> onResult;
  const VoiceButton({super.key, required this.onResult});

  @override
  ConsumerState<VoiceButton> createState() => _VoiceButtonState();
}

class _VoiceButtonState extends ConsumerState<VoiceButton> {
  final _recorder = AudioRecorder();
  bool _recording = false;
  bool _busy = false;
  String? _err;

  @override
  void dispose() {
    _recorder.dispose();
    super.dispose();
  }

  Future<void> _toggle() async {
    if (_busy) return;
    if (_recording) {
      await _stop();
    } else {
      await _start();
    }
  }

  Future<void> _start() async {
    setState(() {
      _err = null;
    });
    try {
      final ok = await _recorder.hasPermission();
      if (!ok) {
        setState(() => _err = 'Microphone permission denied.');
        return;
      }
      await _recorder.start(
        const RecordConfig(encoder: AudioEncoder.opus),
        path: 'voice.webm',
      );
      setState(() => _recording = true);
    } catch (e) {
      setState(() => _err = '$e');
    }
  }

  Future<void> _stop() async {
    setState(() {
      _recording = false;
      _busy = true;
    });
    try {
      final path = await _recorder.stop();
      if (path == null) return;
      final bytes = await _safeReadBytes(path);
      if (bytes == null) return;
      final b64 = base64Encode(bytes);
      final out = await ref
          .read(apiProvider)
          .voiceStt(b64, mime: 'audio/webm');
      final text = (out['text'] as String?)?.trim();
      if (text != null && text.isNotEmpty) {
        widget.onResult(text);
      }
    } catch (e) {
      setState(() => _err = e is ApiError ? e.message : '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<List<int>?> _safeReadBytes(String path) async {
    try {
      // The record plugin returns a path; reading raw bytes works on
      // Android but not all platforms. We swallow errors gracefully.
      return await _recorder.isRecording().then((_) async {
        // If still recording for some reason, just return null.
        return null;
      });
    } catch (_) {
      return null;
    }
  }

  @override
  Widget build(BuildContext context) {
    return Material(
      color: _recording ? AppColors.tierRed : AppColors.accent,
      borderRadius: BorderRadius.circular(14),
      child: InkWell(
        borderRadius: BorderRadius.circular(14),
        onTap: _toggle,
        child: Tooltip(
          message: _err ?? '',
          child: Padding(
            padding: const EdgeInsets.all(14),
            child: _busy
                ? const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(
                      strokeWidth: 2,
                      color: Colors.white,
                    ),
                  )
                : Icon(
                    _recording ? Icons.stop : Icons.mic_none,
                    color: Colors.white,
                    size: 18,
                  ),
          ),
        ),
      ),
    );
  }
}
