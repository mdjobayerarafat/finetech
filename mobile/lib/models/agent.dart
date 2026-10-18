// Agent message + workflow shapes from /api/v1/agent/messages, /workflows.

import 'package:flutter/foundation.dart';

import 'transfer.dart';

@immutable
class AgentNode {
  final String id;
  final int index;
  final String tool;
  final Map<String, dynamic> parameters;
  final RiskTier riskTier;
  final bool approvalRequired;
  final String status;
  final String? pendingActionId;
  final Map<String, dynamic>? result;

  const AgentNode({
    required this.id,
    required this.index,
    required this.tool,
    required this.parameters,
    required this.riskTier,
    required this.approvalRequired,
    required this.status,
    this.pendingActionId,
    this.result,
  });

  factory AgentNode.fromJson(Map<String, dynamic> j) => AgentNode(
        id: j['id'] as String,
        index: (j['index'] as num).toInt(),
        tool: j['tool'] as String,
        parameters: (j['parameters'] as Map?)?.cast<String, dynamic>() ?? const {},
        riskTier: RiskTierX.parse(j['risk_tier'] as String?),
        approvalRequired: j['approval_required'] as bool? ?? false,
        status: j['status'] as String? ?? 'pending',
        pendingActionId: j['pending_action_id'] as String?,
        result: (j['result'] as Map?)?.cast<String, dynamic>(),
      );
}

class AgentPlan {
  final String? workflowId;
  final String? status;
  final List<AgentNode> nodes;

  const AgentPlan({this.workflowId, this.status, required this.nodes});

  factory AgentPlan.fromJson(Map<String, dynamic> j) => AgentPlan(
        workflowId: j['workflow_id'] as String?,
        status: j['status'] as String?,
        nodes: ((j['nodes'] as List?) ?? const [])
            .cast<Map<String, dynamic>>()
            .map(AgentNode.fromJson)
            .toList(),
      );
}

class AgentMessageOut {
  final String intent;
  final String language;
  final Map<String, dynamic> entities;
  final bool clarificationRequired;
  final List<String> questions;
  final AgentPlan plan;
  final PendingAction? pendingAction;
  final String? proposalText;
  final String traceId;

  const AgentMessageOut({
    required this.intent,
    required this.language,
    required this.entities,
    required this.clarificationRequired,
    required this.questions,
    required this.plan,
    this.pendingAction,
    this.proposalText,
    required this.traceId,
  });

  factory AgentMessageOut.fromJson(Map<String, dynamic> j) => AgentMessageOut(
        intent: j['intent'] as String? ?? '',
        language: j['language'] as String? ?? 'en',
        entities: (j['entities'] as Map?)?.cast<String, dynamic>() ?? const {},
        clarificationRequired: j['clarification_required'] as bool? ?? false,
        questions: ((j['questions'] as List?) ?? const []).cast<String>(),
        plan: AgentPlan.fromJson(
            (j['plan'] as Map?)?.cast<String, dynamic>() ?? const {}),
        pendingAction: j['pending_action'] == null
            ? null
            : PendingAction.fromJson(
                (j['pending_action'] as Map).cast<String, dynamic>()),
        proposalText: j['proposal_text'] as String?,
        traceId: j['trace_id'] as String? ?? '',
      );
}

class PendingAction {
  final String id;
  final String tool;
  final String parameterHash;
  final RiskTier riskTier;
  final DateTime expiresAt;
  final String? confirmationStatus;
  final String? mfaStatus;
  final Map<String, dynamic>? parameters;

  const PendingAction({
    required this.id,
    required this.tool,
    required this.parameterHash,
    required this.riskTier,
    required this.expiresAt,
    this.confirmationStatus,
    this.mfaStatus,
    this.parameters,
  });

  factory PendingAction.fromJson(Map<String, dynamic> j) => PendingAction(
        id: j['id'] as String,
        tool: j['tool'] as String? ?? '',
        parameterHash: j['parameter_hash'] as String? ?? '',
        riskTier: RiskTierX.parse(j['risk_tier'] as String?),
        expiresAt: DateTime.parse(j['expires_at'] as String).toLocal(),
        confirmationStatus: j['confirmation_status'] as String?,
        mfaStatus: j['mfa_status'] as String?,
        parameters: (j['parameters'] as Map?)?.cast<String, dynamic>(),
      );
}

class ConfirmOut {
  final String pendingActionId;
  final String confirmationStatus;
  final String mfaStatus;
  final String? workflowId;
  final String? workflowNodeId;
  final bool executed;
  final Map<String, dynamic>? result;

  const ConfirmOut({
    required this.pendingActionId,
    required this.confirmationStatus,
    required this.mfaStatus,
    this.workflowId,
    this.workflowNodeId,
    required this.executed,
    this.result,
  });

  factory ConfirmOut.fromJson(Map<String, dynamic> j) => ConfirmOut(
        pendingActionId: j['pending_action_id'] as String,
        confirmationStatus: j['confirmation_status'] as String? ?? '',
        mfaStatus: j['mfa_status'] as String? ?? '',
        workflowId: j['workflow_id'] as String?,
        workflowNodeId: j['workflow_node_id'] as String?,
        executed: j['executed'] as bool? ?? false,
        result: (j['result'] as Map?)?.cast<String, dynamic>(),
      );
}
