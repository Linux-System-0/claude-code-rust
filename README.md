# Claude Code Rust（CCB-Rust）

将 [claude-code-best/claude-code](https://github.com/claude-code-best/claude-code)（TypeScript / Bun 的
Claude Code 复原版）**按功能边界重写为 Rust** 的工程。

> 状态：**Phase 0 脚手架阶段**。目标是先产出可运行的 CLI 子集，再逐步补齐工具、
> 循环、权限与 TUI，而非最后一次性打通。当前仓库只有 workspace 骨架与工程基线。

---

## 设计决策（已确认）

| # | 决策 |
|---|---|
| D1 | **OAuth 全部砍掉**（Anthropic 账号 / Console / MCP） |
| D2 | **只保留两种协议**：Anthropic Messages 兼容 + OpenAI Chat Completions 兼容 |
| D3 | **无默认 API 端点**：不硬编码任何 `base_url`，缺失即报错 |
| D4 | 第三方 SDK / 服务型集成删除，改用原生 HTTP / 标准实现 |
| D5 | 用 `reqwest` 直连 Anthropic / OpenAI 兼容端点 |

完整裁剪范围见 [`Plan/`](./Plan/)：

- [`Plan/README.md`](./Plan/README.md) — 概览与决策
- [`Plan/01-trim-scope.md`](./Plan/01-trim-scope.md) — 保留 / 删除 / 替换对照
- [`Plan/02-architecture.md`](./Plan/02-architecture.md) — 目标架构与分层
- [`Plan/03-configuration.md`](./Plan/03-configuration.md) — 配置入口与错误提示
- [`Plan/04-task-list.md`](./Plan/04-task-list.md) — 分阶段任务列表
- [`Plan/05-migration-and-verification.md`](./Plan/05-migration-and-verification.md) — 迁移与验证

---

## 仓库结构

```
crates/
├── ccb-cli/                    # 入口：clap 解析 + fast-path（bin: ccb）
├── ccb-config/                 # settings.json / env / keychain
├── ccb-core/                   # 消息模型、query 循环、QueryEngine
├── ccb-auth/                   # API Key / apiKeyHelper（无 OAuth）
├── ccb-api/                    # 统一 API 抽象
│   ├── protocol-anthropic/     # Anthropic Messages 适配器（SSE）
│   └── protocol-openai/        # OpenAI Chat Completions 适配器（SSE）
├── ccb-tools/                  # Tool trait + registry + 核心工具
├── ccb-permissions/            # 权限模式与规则引擎
├── ccb-mcp/                    # MCP client/server（无 OAuth）
├── ccb-session/                # 会话持久化、压缩、文件历史
├── ccb-tui/                    # ratatui 界面
├── ccb-context/                # system prompt / CLAUDE.md / git 上下文
└── ccb-telemetry/              # tracing 日志
```

依赖方向严格自上而下（详见 `Plan/02-architecture.md` §2）。

---

## 构建

要求 Rust `1.75+`（`rust-toolchain.toml` 固定 `stable`，含 `rustfmt` / `clippy`）。

```bash
# 构建整个 workspace（默认启用两种协议）
cargo build --workspace

# 运行 CLI（当前为占位实现，clap 解析见 T1.1）
cargo run -p ccb-cli
```

### Cargo Feature（只有两个）

协议适配在编译期选择，**仅提供 `anthropic` 与 `openai`**，不存在其它 provider
的 feature（见 `Plan/01-trim-scope.md` §2）：

```bash
# 默认：两种协议都编入
cargo build -p ccb-cli

# 只编入 Anthropic 兼容
cargo build -p ccb-cli --no-default-features --features anthropic

# 只编入 OpenAI 兼容
cargo build -p ccb-cli --no-default-features --features openai
```

两个都不选会触发编译期错误（`ccb-core` 的 `compile_error!`），避免运行时才暴露
「未知 provider」。

---

## 配置（无默认端点）

`base_url` 与 key 必须显式提供，缺失即报错。支持三处入口，优先级：

```
CLI 参数 > 环境变量 > 项目 .claude/settings.json > 用户 ~/.claude/settings.json
```

示例（`~/.claude/settings.json`）：

```jsonc
// OpenAI 兼容
{ "modelType": "openai", "model": "deepseek-chat",
  "env": { "OPENAI_BASE_URL": "https://api.deepseek.com/v1", "OPENAI_API_KEY": "sk-xxxx" } }

// Anthropic 兼容
{ "modelType": "anthropic", "model": "claude-sonnet-4-5",
  "env": { "ANTHROPIC_BASE_URL": "https://my-gateway.example.com", "ANTHROPIC_AUTH_TOKEN": "sk-xxxx" } }
```

详见 [`Plan/03-configuration.md`](./Plan/03-configuration.md)。

---

## 开发与验收

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

CI（`.github/workflows/ci.yml`）在 Linux / macOS / Windows 上跑
`fmt` / `clippy -D warnings` / `test`，并覆盖协议 feature 矩阵。

TypeScript 原实现冻结为 tag `ts-baseline-2.8.4`（commit `77a7934e`），作为行为对拍
的参照，见 [`reference/README.md`](./reference/README.md)。

---

## 许可

本仓库的 Rust 重写代码以 [MIT](./LICENSE) 发布。冻结的 TypeScript 参照源码为逆向
工程产物、未附带许可证，仅作本地行为比对，不随本仓库分发。
