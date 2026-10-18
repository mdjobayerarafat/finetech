---
id: arch-home-zh
title: FinTech Pioneers 架构 (中文)
sidebar_label: 架构总览 (中文)
sidebar_position: 0
---

# FinTech Pioneers 架构 (Bilingual / 双语)

> English: [Architecture Home](../README.md)

本目录是 FinTech Pioneers 的**内部**设计参考。它与
[外部安全自评](../self-assessment/zh/README.md)互为补充，但
**不替代**单文件的[旧版架构说明](../ARCHITECTURE.md)与外部
自评文档。自评文档描述"哪些安全结论可以、哪些不可以对外
声称",而本目录描述"系统是如何搭建起来的"。

文档按阅读顺序排列。每一页都假设你已读过前一页。

| 编号 | 页面 | 内容 |
|---|---|---|
| 0 | [系统总览](./00-system-overview.md) | 组件图、部署拓扑、整栈 Mermaid `flowchart` 与部署视图。 |
| 1 | [请求生命周期](./01-request-lifecycle.md) | 每个 agent 请求的四个阶段(Plan → Validate → Authorise → Execute),一笔 Yellow 级转账的 `sequenceDiagram`,以及 LLM 工具循环的 `stateDiagram-v2`。 |
| 2 | [数据模型](./02-data-model.md) | 16 张生产表的 `erDiagram`,金额规则(`*_fen` BIGINT),以及 `audit_events` 哈希链。 |
| 3 | [核心算法](./03-algorithms.md) | 七个核心算法的伪代码与源码索引:策略分级、反诈卫士、智能转账解析、"我负担得起吗?"预测、收据 OCR、规划器 DAG、订阅识别。 |
| 4 | [前端](./04-frontend.md) | Next.js 14 App Router 树状结构、双语 i18n 实现、共享组件库、跨页面刷新的 `bus` 模式。 |
| 5 | [部署](./05-deployment.md) | `ops/dev.sh` 工作流、环境变量、构建/运行命令、`.run/` 下的日志布局。 |
| 6 | [安全设计](./06-security-design.md) | 八层纵深防御,每条结论都对应到具体源码文件;并交叉引用 [外部自评](../self-assessment/zh/README.md) 的风险与非声称页。 |

## 阅读对象

- **新加入的工程师** — 需要快速熟悉 Rust 后端或 Next.js 前端。
- **代码审查者 / 安全审计** — 需要把某条架构或安全声明对回具体代码。
- **演示操作者** — 想理解当聊天界面弹出 Yellow 级确认时,日志行分别意味着什么。

## 离线阅读

每一页都带有 YAML front-matter(`id`、`title`、`sidebar_label`、
`sidebar_position`),可以直接被 Docusaurus、Mintlify、Nextra
或 Docsify 导入。Mermaid 图全部使用内联 ` ```mermaid ` 代码块;
GitHub 原生渲染,Docusaurus 需要 `@docusaurus/theme-mermaid`。

## 本目录**不**包含的内容

- **单文件旧版架构说明** — [../ARCHITECTURE.md](../ARCHITECTURE.md),仅作历史保留,新内容请使用本目录。
- **外部安全自评** — [../self-assessment/zh/](../self-assessment/zh/README.md),面向审查者的双语文档,涵盖风险与非声称。
- **API 端点参考** — 见 [../API.md](../API.md)。
- **逐工具分级推理** — 见 [../POLICY.md](../POLICY.md) 以及自评的 [§1 权限分级](../self-assessment/zh/01-permission-tiers.md)。
