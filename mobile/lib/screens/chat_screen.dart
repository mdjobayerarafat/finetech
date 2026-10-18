// Chat / Assistant — the marquee feature.
//
// Sends user text to /api/v1/agent/messages, renders the plan + any
// pending_action inline, and routes the user through Confirm / mock
// MFA via the ConfirmationCard widget.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api.dart';
import '../core/api_error.dart';
import '../core/i18n.dart';
import '../core/money.dart';
import '../models/agent.dart';
import '../state/settings.dart';
import '../theme/app_theme.dart';
import '../widgets/confirmation_card.dart';
import '../widgets/glass_card.dart';
import '../widgets/voice_button.dart';

class _ChatMessage {
  final String text;
  final bool fromUser;
  final AgentMessageOut? reply;
  final String? error;
  final DateTime at;
  _ChatMessage({
    required this.text,
    required this.fromUser,
    this.reply,
    this.error,
    DateTime? at,
  }) : at = at ?? DateTime.now();
}

class ChatScreen extends ConsumerStatefulWidget {
  const ChatScreen({super.key});
  @override
  ConsumerState<ChatScreen> createState() => _ChatScreenState();
}

class _ChatScreenState extends ConsumerState<ChatScreen> {
  final _input = TextEditingController();
  final _scroll = ScrollController();
  final List<_ChatMessage> _msgs = [];
  bool _busy = false;

  @override
  void dispose() {
    _input.dispose();
    _scroll.dispose();
    super.dispose();
  }

  Future<void> _send(String text) async {
    if (text.trim().isEmpty || _busy) return;
    final lang = ref.read(settingsProvider).language;
    setState(() {
      _msgs.add(_ChatMessage(text: text, fromUser: true));
      _busy = true;
    });
    _input.clear();
    _scrollDown();
    try {
      final out =
          await ref.read(apiProvider).sendAgentMessage(text, lang);
      setState(() => _msgs.add(_ChatMessage(text: text, fromUser: false, reply: out)));
    } catch (e) {
      final err = e is ApiError ? e.message : e.toString();
      setState(() =>
          _msgs.add(_ChatMessage(text: text, fromUser: false, error: err)));
    } finally {
      if (mounted) setState(() => _busy = false);
      _scrollDown();
    }
  }

  void _scrollDown() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_scroll.hasClients) return;
      _scroll.animateTo(
        _scroll.position.maxScrollExtent + 200,
        duration: const Duration(milliseconds: 300),
        curve: Curves.easeOut,
      );
    });
  }

  void _onActionDone() {
    // The workflow's already in flight. Just close the inline card.
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final lang = ref.watch(settingsProvider).language;
    final tr = Tr.of(lang);
    return SafeArea(
      child: Column(
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
            child: Row(
              children: [
                Text(
                  tr.t('chat.title'),
                  style: const TextStyle(
                    fontSize: 18,
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const Spacer(),
                if (_busy)
                  const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  ),
              ],
            ),
          ),
          Expanded(
            child: _msgs.isEmpty
                ? _ChatEmpty(tr: tr, onTap: (s) => _send(s))
                : ListView.builder(
                    controller: _scroll,
                    padding: const EdgeInsets.fromLTRB(12, 8, 12, 16),
                    itemCount: _msgs.length,
                    itemBuilder: (c, i) => _ChatBubble(
                      msg: _msgs[i],
                      tr: tr,
                      onActionDone: _onActionDone,
                    ),
                  ),
          ),
          _ChatInput(
            controller: _input,
            hint: tr.t('chat.placeholder'),
            sendLabel: tr.t('common.send'),
            onSubmit: _send,
            busy: _busy,
            voice: VoiceButton(
              onResult: (s) {
                _input.text = s;
                _send(s);
              },
            ),
          ),
        ],
      ),
    );
  }
}

class _ChatEmpty extends StatelessWidget {
  final Tr tr;
  final void Function(String) onTap;
  const _ChatEmpty({required this.tr, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final suggestions = [
      tr.t('chat.suggestion.transfer'),
      tr.t('chat.suggestion.bill'),
      tr.t('chat.suggestion.subscription'),
      tr.t('chat.suggestion.card'),
    ];
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(20, 8, 20, 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          GlassCard(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Container(
                      width: 32,
                      height: 32,
                      decoration: BoxDecoration(
                        borderRadius: BorderRadius.circular(10),
                        gradient: const LinearGradient(
                          colors: AppColors.accentGradient,
                        ),
                      ),
                      child: const Icon(
                        Icons.auto_awesome,
                        color: Colors.white,
                        size: 18,
                      ),
                    ),
                    const SizedBox(width: 10),
                    const Text(
                      'AI assistant',
                      style: TextStyle(fontWeight: FontWeight.w600),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                Text(
                  tr.t('chat.empty'),
                  style: const TextStyle(
                    color: AppColors.textMuted,
                    fontSize: 13,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 12),
          Text(
            tr.t('common.continue'),
            style: const TextStyle(
              fontSize: 11,
              color: AppColors.textFaint,
            ),
          ),
          const SizedBox(height: 6),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              for (final s in suggestions)
                ActionChip(
                  label: Text(s, style: const TextStyle(fontSize: 12)),
                  onPressed: () => onTap(s),
                  backgroundColor: AppColors.surface,
                  side: const BorderSide(color: AppColors.lineHi),
                ),
            ],
          ),
        ],
      ),
    );
  }
}

class _ChatBubble extends ConsumerWidget {
  final _ChatMessage msg;
  final Tr tr;
  final VoidCallback onActionDone;
  const _ChatBubble({
    required this.msg,
    required this.tr,
    required this.onActionDone,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    if (msg.fromUser) {
      return Align(
        alignment: Alignment.centerRight,
        child: Container(
          margin: const EdgeInsets.symmetric(vertical: 4, horizontal: 4),
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
          decoration: BoxDecoration(
            gradient: const LinearGradient(
              colors: AppColors.accentGradient,
            ),
            borderRadius: const BorderRadius.only(
              topLeft: Radius.circular(16),
              topRight: Radius.circular(16),
              bottomLeft: Radius.circular(16),
              bottomRight: Radius.circular(4),
            ),
          ),
          child: Text(
            msg.text,
            style: const TextStyle(color: Colors.white, fontSize: 14),
          ),
        ),
      );
    }
    final r = msg.reply;
    return Align(
      alignment: Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.symmetric(vertical: 4, horizontal: 4),
        constraints: const BoxConstraints(maxWidth: 460),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (msg.error != null)
              GlassCard(
                border: Border.all(
                    color: AppColors.tierRed.withValues(alpha: 0.5)),
                child: Row(
                  children: [
                    const Icon(Icons.error_outline,
                        color: AppColors.tierRed, size: 18),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        msg.error!,
                        style: const TextStyle(
                            color: AppColors.tierRed, fontSize: 13),
                      ),
                    ),
                  ],
                ),
              )
            else if (r != null) ...[
              if (r.proposalText != null && r.proposalText!.isNotEmpty)
                GlassCard(
                  child: Text(
                    r.proposalText!,
                    style: const TextStyle(fontSize: 14, height: 1.45),
                  ),
                ),
              if (r.clarificationRequired) ...[
                const SizedBox(height: 6),
                GlassCard(
                  border: Border.all(
                      color: AppColors.tierYellow.withValues(alpha: 0.5)),
                  child: Row(
                    children: [
                      const Icon(Icons.help_outline,
                          color: AppColors.tierYellow, size: 18),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          r.questions.isNotEmpty
                              ? r.questions.first
                              : tr.t('chat.clarification'),
                          style: const TextStyle(fontSize: 13),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
              if (r.plan.nodes.isNotEmpty) ...[
                const SizedBox(height: 6),
                for (final node in r.plan.nodes)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 6),
                    child: _PlanNodeTile(node: node, tr: tr),
                  ),
              ],
              if (r.pendingAction != null) ...[
                const SizedBox(height: 6),
                ConfirmationCard(
                  pendingAction: r.pendingAction!,
                  workflowId: r.plan.workflowId,
                  onDone: onActionDone,
                ),
              ],
            ],
          ],
        ),
      ),
    );
  }
}

class _PlanNodeTile extends StatelessWidget {
  final AgentNode node;
  final Tr tr;
  const _PlanNodeTile({required this.node, required this.tr});

  @override
  Widget build(BuildContext context) {
    return GlassCard(
      padding: const EdgeInsets.all(12),
      child: Row(
        children: [
          Container(
            width: 28,
            height: 28,
            decoration: BoxDecoration(
              color: AppColors.surfaceHi,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Center(
              child: Text(
                '${node.index + 1}',
                style: const TextStyle(
                  fontSize: 12,
                  color: AppColors.textMuted,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  node.tool,
                  style: const TextStyle(
                    fontSize: 12,
                    fontFamily: 'monospace',
                    color: AppColors.text,
                  ),
                ),
                if (_amount(node).isNotEmpty)
                  Text(
                    _amount(node),
                    style: const TextStyle(
                      fontSize: 11,
                      color: AppColors.textMuted,
                    ),
                  ),
              ],
            ),
          ),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
            decoration: BoxDecoration(
              color: AppColors.surfaceHi,
              borderRadius: BorderRadius.circular(6),
            ),
            child: Text(
              node.status,
              style: const TextStyle(fontSize: 10, color: AppColors.textMuted),
            ),
          ),
        ],
      ),
    );
  }

  String _amount(AgentNode n) {
    final amt = n.parameters['amount_fen'];
    if (amt is num) return fenToCny(amt.toInt());
    return '';
  }
}

class _ChatInput extends StatelessWidget {
  final TextEditingController controller;
  final String hint;
  final String sendLabel;
  final void Function(String) onSubmit;
  final bool busy;
  final Widget? voice;
  const _ChatInput({
    required this.controller,
    required this.hint,
    required this.sendLabel,
    required this.onSubmit,
    required this.busy,
    this.voice,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.fromLTRB(12, 8, 12, 14),
      decoration: const BoxDecoration(
        color: AppColors.surface,
        border: Border(top: BorderSide(color: AppColors.line)),
      ),
      child: SafeArea(
        top: false,
        child: Row(
          children: [
            Expanded(
              child: TextField(
                controller: controller,
                minLines: 1,
                maxLines: 4,
                enabled: !busy,
                textInputAction: TextInputAction.send,
                decoration: InputDecoration(
                  hintText: hint,
                  filled: true,
                  fillColor: AppColors.surfaceHi,
                ),
                onSubmitted: onSubmit,
              ),
            ),
            if (voice != null) ...[
              const SizedBox(width: 8),
              voice!,
            ],
            const SizedBox(width: 8),
            Material(
              color: AppColors.accent,
              borderRadius: BorderRadius.circular(14),
              child: InkWell(
                borderRadius: BorderRadius.circular(14),
                onTap: busy ? null : () => onSubmit(controller.text),
                child: Padding(
                  padding: const EdgeInsets.all(14),
                  child: busy
                      ? const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(
                            strokeWidth: 2,
                            color: Colors.white,
                          ),
                        )
                      : const Icon(Icons.send, color: Colors.white, size: 18),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}