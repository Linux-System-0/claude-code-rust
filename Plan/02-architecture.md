# 02 · 目标 Rust 架构

## 1. Workspace 布局

```
claude-code-rust/
├── Cargo.toml                      # workspace
├── crates/
│   ├── ccb-cli/                    # 入口：clap 解析 + fast-path
│   ├── ccb-config/                 # settings.json / env / keychain 配置
│   ├── ccb-core/                   # 消息模型、query 循环、QueryEngine
│   ├── ccb-auth/                   # API Key / apiKeyHelper（无 OAuth）
│   ├── ccb-api/                    # 统一 API 抽象层
│   │   ├── protocol-anthropic/     # Anthropic Messages 适配器
│   │   └── protocol-openai/        # OpenAI Chat Completions 适配器
│   ├── ccb-tools/                  # Tool trait + registry + 核心工具
│   ├── ccb-permissions/            # 权限模式与规则引擎
│   ├── ccb-mcp/                    # MCP client/server（无 OAuth）
│   ├── ccb-session/                # 会话持久化、压缩、文件历史
│   ├── ccb-tui/                    # ratatui 界面
│   ├── ccb-context/                # system prompt / CLAUDE.md / git 上下文
│   └── ccb-telemetry/              # 轻量日志（tracing）
└── Plan/                           # 本目录
```

## 2. 分层依赖方向

```
        ccb-cli
       /   |    \
   ccb-tui ccb-mcp ccb-context
       \   |    /
        ccb-core
       /    |    \
  ccb-api ccb-tools ccb-session
       \    |    /
    ccb-auth ccb-permissions
            |
       ccb-config
            |
        ccb-telemetry
```

## 3. 双协议适配层（核心）

```rust
// 统一内部事件，两个适配器都转换到此模型
pub enum StreamEvent {
    MessageStart { id: String, model: String, usage: Usage },
    ContentBlockStart { index: usize, block: ContentBlock },
    TextDelta { index: usize, text: String },
    ThinkingDelta { index: usize, text: String },
    ToolUseDelta { index: usize, partial_json: String },
    ContentBlockStop { index: usize },
    MessageDelta { stop_reason: StopReason, usage: Usage },
    MessageStop,
    Error(ApiError),
}

#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    fn name(&self) -> &'static str;               // "anthropic" | "openai"
    async fn stream(
        &self,
        req: ChatRequest,
        cfg: &EndpointConfig,
    ) -> Result<impl Stream<Item = Result<StreamEvent>>>;
}
```

两套适配器职责：
- **anthropic**：`/v1/messages`，SSE，`tool_use`/`thinking` 块
- **openai**：`/chat/completions`，SSE，`delta.tool_calls`、reasoning content

## 4. 关键设计决策

| 项 | 决策 |
|---|---|
| 异步运行时 | `tokio` |
| HTTP | `reqwest`（rustls，支持自定义 base_url/header）|
| 序列化 | `serde` + `serde_json`，事件用 tagged enum |
| 错误 | `thiserror`（库）/ `anyhow`（应用）|
| 配置 | `serde` + 自定义多源合并 |
| CLI | `clap`（derive）|
| TUI | `ratatui` + `crossterm` |
| 文件监听 | `notify` |
| 日志 | `tracing` + `tracing-subscriber` |
| 密钥 | `keyring` crate（macOS Keychain / 跨平台）|
| MCP | `rmcp` 或裸 JSON-RPC over stdio/SSE |

## 5. 与 TS 版的关键差异

| 维度 | TS 版 | Rust 版 |
|---|---|---|
| UI | React + 自研 Ink fork | `ratatui`（重做布局/焦点/按键模型）|
| 状态 | Zustand store | 事件驱动 + `Arc<Mutex>` / channel |
| 类型校验 | zod 运行时 | serde（编译期为主，运行时按需）|
| 并发 | 事件循环 + Promise | tokio task + channel |
| 功能开关 | `bun:bundle` feature() | Cargo feature（默认极少）|
| Provider | 7 个 | **2 个** |
| 认证 | OAuth + Key | **仅 Key** |
| 默认端点 | 多处 | **无** |
