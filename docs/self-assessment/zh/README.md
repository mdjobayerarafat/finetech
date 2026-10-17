---
id: index-zh
title: 安全自评估(总览)
sidebar_label: 总览
sidebar_position: 0
---

# 安全自评估(总览)

> **Phase 7 交付物。** 本文档是面向审核者与评委的*外部*自评估。
> 内部设计文档——威胁模型、分层防御、"AI 提议,软件决策"原则——在
> [`../../SECURITY.md`](../../SECURITY.md)。建议先读那份看技术细节,
> 再读这份看"系统今天能做什么、承载真金白银之前还要加固什么"的
> 诚实清单。
>
> 双语: 英文版在 [../README.md](../README.md),每页末尾有中文附录。
> 两版内容一致,只是举例用了符合中文场景的术语。

## 内容

七节,涵盖:

1. **权限分级** 的实现(绿 / 黄 / 红)。
2. **审计与可观测性**(哈希链、日志内容、红队套件)。
3. **大模型安全边界**(架构规则,保证模型不是权威)。
4. **15 条已知风险**(R-1 到 R-15),每条给出"演示可接受 /
   生产不可接受"的判定与具体加固路径。
5. 我们**未声称**的事。
6. **代码索引**——每条主张对应到具体文件 / 函数。

## 页面

| # | 主题 | 中文 | EN |
|---|---|---|---|
| 0 | 范围说明 | [00-scope.md](./00-scope.md) | [00-scope.md](../00-scope.md) |
| 1 | 权限分级 | [01-permission-tiers.md](./01-permission-tiers.md) | [01-permission-tiers.md](../01-permission-tiers.md) |
| 2 | 审计与可观测性 | [02-audit.md](./02-audit.md) | [02-audit.md](../02-audit.md) |
| 3 | 大模型安全边界 | [03-llm-safety.md](./03-llm-safety.md) | [03-llm-safety.md](../03-llm-safety.md) |
| 4 | 已知风险 (R-1…R-15) | [04-known-risks.md](./04-known-risks.md) | [04-known-risks.md](../04-known-risks.md) |
| 5 | 未声称的事 | [05-non-claims.md](./05-non-claims.md) | [05-non-claims.md](../05-non-claims.md) |
| 6 | 代码索引 | [06-code-map.md](./06-code-map.md) | [06-code-map.md](../06-code-map.md) |

## 评审者快速路径

- **五分钟:** 读 [§0](./00-scope.md),扫一遍
  [§4](./04-known-risks.md) 的 R-1…R-15,再看
  [代码索引](./06-code-map.md)。
- **十五分钟:** 加 [§1](./01-permission-tiers.md) 与
  [§2](./02-audit.md)——这两节是"AI 提议,软件决策"主张的依据。
- **深入阅读:** [§3](./03-llm-safety.md) 的 LLM 边界与红队套件,
  然后读 [§4](./04-known-risks.md) 的每条加固路径。

## 相关文档

- [`../../SECURITY.md`](../../SECURITY.md) — 内部设计、威胁模型、
  分层防御。
- [`../../THREAT_MODEL.md`](../../THREAT_MODEL.md) — STRIDE 分析
  (在本文中引用;本构建版本尚未撰写)。
- [`../../POLICY.md`](../../POLICY.md) — 绿 / 黄 / 红 引擎细节。
- [`../../AGENT_PIPELINE.md`](../../AGENT_PIPELINE.md) — 意图如何
  变成计划(在本文中引用)。
