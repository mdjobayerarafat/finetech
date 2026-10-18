---
id: ops-home-zh
title: 运维与测试 (中文)
sidebar_label: 运维与测试 (中文)
sidebar_position: 0
---

# 运维与测试

> English: [Operations & Testing](../README.md)

本目录是 FinTech Pioneers 的**"如何把系统跑起来"**参考。它与
[架构树](../architecture/zh/README.md)及
[外部安全自评](../self-assessment/zh/README.md)互为补充,但
不替代这两份文档。

| 编号 | 页面 | 内容 |
|---|---|---|
| 0 | [部署](./00-deployment.md) | 生产拓扑、环境变量、`systemd` 单元、反向代理与 TLS、Postgres 运维(迁移、备份、复位)、升级与回滚、日志轮转、可观测性。 |
| 1 | [故障排查](./01-troubleshooting.md) | 常见故障模式、每条日志行的含义、恢复命令、何时升级。 |
| 2 | [测试用例](./02-test-cases.md) | 全部 39 条单元测试、集成清单、red-team 场景、端到端 smoke 脚本,以及与 `SCENARIOS.md` 中 6 个用户场景的交叉引用。 |
| 3 | [持续集成](./03-ci.md) | 推荐的 CI 流水线(目前尚未配置)、GitHub Actions 模板、应 gate 的检查、可跳过的检查。 |

## 阅读对象

- **运维** — 负责生产环境搭建、迁移执行、密钥轮换、响应告警。
- **审查者** — 想看测试覆盖面,以及自评中的每条安全声明是如何被覆盖的。
- **演示操作者** — 需要一份确定性的快速启动指南,以及当 Next.js / Rust 进程中途崩溃时的恢复路径。

## 阅读方式

每页自成体系。最重要的跨链接是 **`02-test-cases.md` ↔ `../self-assessment/`** — 自评中的每一条声明都应能在这里的某条测试或清单项上找到对应。

## 本目录**不**包含的内容

- **架构树** — [../architecture/zh/](../architecture/zh/README.md)。如果还不了解系统如何搭建,请先读这一份。
- **外部安全自评** — [../self-assessment/zh/](../self-assessment/zh/README.md)。先读这一份以拿到风险与非声称清单。
- **演示脚本** — 6 个用户场景见 `../SCENARIOS.md`,build 起来后看应用内 `/docs` 页面。
- **单文件旧版架构说明** — [../ARCHITECTURE.md](../ARCHITECTURE.md),仅作历史保留。
