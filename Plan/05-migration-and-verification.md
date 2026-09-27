# 05 · 迁移与验证策略

## 1. 总体策略

**不是逐行翻译，而是按功能边界重写。**

- 保留 TS 版本作为**行为参照（oracle）**，用于对拍。
- 每个 Phase 产出可运行切片，逐步替换。
- 以现有 450 个测试 + 真实会话 transcript 作为验收依据。

## 2. 参照物管理

| 项 | 做法 |
|---|---|
| TS 源码 | 打 git tag / 复制到 `reference/`，重写期间只读 |
| 行为基线 | 录制真实会话的请求/响应（golden transcript）|
| 测试 | 逐步把核心测试翻译为 `cargo test` |

## 3. 对拍方法

1. 同一输入、同一配置（base_url + key）分别跑 TS 版与 Rust 版。
2. 对比：请求体（system prompt、tools 定义、消息序列）、流事件序列、最终输出、工具调用。
3. 差异即 bug，逐项收敛。

## 4. 验收标准（建议）

| 层级 | 标准 |
|---|---|
| M1 | 两个协议各成功完成一次流式对话 |
| M2 | 文件读写 / Search / Bash 工具链端到端可用，权限询问正确 |
| M3 | REPL 交互（输入、渲染、快捷键）与 TS 版体感一致 |
| M4 | 核心测试全绿；`clippy -D warnings` 通过 |

## 5. 兼容性范围（明确不做）

以下**不在**重写范围内（依据裁剪决策）：

- OAuth 登录（Anthropic 账号 / Console / MCP）
- Bedrock / Vertex / Foundry / Gemini native / Grok native
- 任何内置默认 API 端点
- 微信、Computer Use、Chrome MCP、Remote Control Server Web UI、Workflow Engine
- KAIROS 系列、语音、团队协作、Bridge、Daemon、Buddy 等 feature flag 功能
- 遥测（OTel / Langfuse / Sentry / GrowthBook）

## 6. 风险

| 风险 | 说明 | 缓解 |
|---|---|---|
| TUI 重写量大 | 7 万行组件 + 2 万行 hooks | 先做最小可用 REPL，逐步补齐 |
| Shell 解析器复杂 | bashParser/ast/权限校验约 9 千行 | 复用成熟 crate 或收敛功能 |
| MCP 协议细节 | 无 OAuth 后仍需完整 transport | 用 `rmcp` 或严格按协议自测 |
| 行为漂移 | 反编译代码存在隐性行为 | golden transcript 对拍 |
