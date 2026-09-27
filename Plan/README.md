# Rust 重写计划（Plan）

本目录记录将当前 TypeScript 版 Claude Code CLI 重写为 Rust 的**决策、裁剪范围、目标架构、配置设计与任务列表**。

> 状态：**Phase 1 基础设施已完成**（T1.1–T1.5）。CLI 解析、分层配置、缺失端点
> 报错、日志体系与平台/shell 检测均已落地；下一步进入 Phase 2 数据模型。

---

## 最终决策（已确认）

| # | 决策 | 说明 |
|---|---|---|
| D1 | **OAuth 全部砍掉** | 含 Anthropic 账号 OAuth、Console OAuth、**MCP OAuth** |
| D2 | **只保留两种协议格式** | Anthropic Messages 兼容 + OpenAI Chat Completions 兼容 |
| D3 | **默认 API 端点一个不留** | 不硬编码任何 base_url，全部由用户配置，缺失即报错 |
| D4 | 第三方 SDK 集成可删 | 改用原生 HTTP/标准实现；服务型集成（微信等）直接删 |
| D5 | 原生 HTTP 协议保留 | Anthropic / OpenAI 兼容端点用 `reqwest` 直连 |

---

## 文档索引

| 文件 | 内容 |
|---|---|
| [01-trim-scope.md](./01-trim-scope.md) | 保留 / 删除 / 替换 对照表（认证、Provider、Packages、Feature Flags、依赖、默认端点） |
| [02-architecture.md](./02-architecture.md) | 目标 Rust 架构与分层 |
| [03-configuration.md](./03-configuration.md) | 配置入口、优先级、示例、错误提示 |
| [04-task-list.md](./04-task-list.md) | 分阶段任务列表 |
| [05-migration-and-verification.md](./05-migration-and-verification.md) | 迁移与验证策略 |

---

## 现状规模（基线）

| 区域 | 文件数 | 代码行数 |
|---|---|---|
| `src/` (TS/TSX) | 2,372 | 557,516 |
| `packages/` (TS/TSX) | 852 | 155,138 |
| 测试 | 450 | ~64,893 |
| **合计** | **~3,120** | **~712,000** |

功能单元：64 个工具目录、144 个命令入口、151 个 UI 组件、96 个 hooks、80+ 个 feature flag。
