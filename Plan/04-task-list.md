# 04 · 任务列表

> 按依赖顺序。每个 Phase 结束应产出**可运行的 CLI 子集**，而非最后一次性打通。
> 状态标记：`[ ]` 待办 · `[~]` 进行中 · `[x]` 完成

---

## Phase 0 — 脚手架

- [x] T0.1 建立 Rust workspace（`Cargo.toml` + `crates/` 分层）
- [x] T0.2 Cargo feature 规划：仅 `anthropic` / `openai`，其余不提供
- [x] T0.3 冻结 TS 版本为参照（`reference/` 或 git tag），保留测试作验收基线
- [x] T0.4 CI：`cargo fmt` / `clippy -D warnings` / `cargo test` / 跨平台矩阵
- [x] T0.5 许可证与仓库元数据

## Phase 1 — 基础设施

- [x] T1.1 `clap` CLI 解析，复刻 `cli.tsx` 全部 fast-path（`--version` 等）
- [x] T1.2 配置系统：settings.json 多层加载 + env 合并 + Keychain
- [x] T1.3 **配置校验：base_url 必填，缺失给可操作报错**（见 03-configuration.md）
- [x] T1.4 日志/错误体系（`tracing` + `thiserror`）
- [x] T1.5 路径/平台工具、shell 检测

## Phase 2 — 数据模型

- [x] T2.1 Message 类型层级（`serde` tagged enum）
- [x] T2.2 Tool / Permission / Progress 类型契约
- [x] T2.3 schema 校验层
- [x] T2.4 IDs、会话级状态单例（`OnceLock` / context）

## Phase 3 — 双协议适配层

- [ ] T3.1 统一消息 / 流事件模型（`StreamEvent`）
- [ ] T3.2 **anthropic 适配器**（`ANTHROPIC_BASE_URL` + key，SSE）
- [ ] T3.3 **openai 适配器**（`OPENAI_BASE_URL` + key，SSE）
- [ ] T3.4 无默认端点：缺配置即报错
- [ ] T3.5 重试 / 超时 / 使用量统计
- [ ] T3.6 tool call 双向转换（anthropic blocks ↔ openai tool_calls）

## Phase 4 — 工具系统

- [ ] T4.1 Tool trait + registry + 分发
- [ ] T4.2 文件类：Read / Write / Edit / Glob / Grep
- [ ] T4.3 Shell 类：Bash / PowerShell（解析器 + 权限校验，最重）
- [ ] T4.4 其余核心工具（按裁剪后白名单）
- [ ] T4.5 工具结果存储与截断

## Phase 5 — 核心循环

- [ ] T5.1 query 单轮工具调用循环
- [ ] T5.2 QueryEngine 会话编排、turn 记账
- [ ] T5.3 上下文 / system prompt 构建（git、CLAUDE.md、memory）
- [ ] T5.4 压缩（auto compact / microcompact）

## Phase 6 — 权限系统

- [ ] T6.1 权限模式（default / acceptEdits / bypass / plan）
- [ ] T6.2 规则引擎（filesystem / bash 校验）
- [ ] T6.3 与 TUI 解耦的权限回调协议

## Phase 7 — MCP（去 OAuth 后）

- [ ] T7.1 MCP 客户端：stdio / SSE / 带 token 的 HTTP
- [ ] T7.2 MCP server 模式（`mcp serve`）
- [ ] ~~T7.3 MCP OAuth~~（已砍）

## Phase 8 — TUI

- [ ] T8.1 ratatui 基座 + 布局引擎
- [ ] T8.2 REPL 屏幕
- [ ] T8.3 PromptInput（多行 / typeahead / vim / 快捷键 / 粘贴）
- [ ] T8.4 消息渲染（markdown / 代码高亮 / diff）
- [ ] T8.5 权限对话框 / 设置面板 / 模型选择器 / Help
- [ ] T8.6 状态管理与事件流

## Phase 9 — 平台特性（可选）

- [ ] T9.1 原生能力（剪贴板 / 图像 / 修饰键）用 crate
- [ ] T9.2 会话持久化增强

## Phase 10 — 验证与交付

- [ ] T10.1 移植核心测试到 `cargo test` / 快照测试
- [ ] T10.2 与 TS 版行为对拍（golden transcript）
- [ ] T10.3 跨平台构建（macOS / Linux / Windows）+ 发布流水线
- [ ] T10.4 文档与迁移说明

---

## 里程碑

| 里程碑 | 内容 | 验收 |
|---|---|---|
| M1 | Phase 0–3 | 能发一条消息并流式打印（两个协议各验证一次）|
| M2 | Phase 4–6 | 能跑工具调用循环 + 权限询问 |
| M3 | Phase 7–8 | 交互式 REPL 可用 |
| M4 | Phase 10 | 核心测试通过、可发布 |
