# 01 · 裁剪范围（保留 / 删除 / 替换）

依据决策 D1–D5。所有 LOC 为粗估，部分文件与多功能交叉，实际按功能边界裁剪。

---

## 1. 认证层

**目标：只剩「显式 API Key + 显式 Base URL」一种模型。**

```
Anthropic 兼容:  ANTHROPIC_BASE_URL + ANTHROPIC_API_KEY / ANTHROPIC_AUTH_TOKEN
OpenAI  兼容:  OPENAI_BASE_URL    + OPENAI_API_KEY
```

### 删除
| 项 | 位置 | 约 LOC |
|---|---|---|
| OAuth 服务（授权码监听/加密/profile） | `src/services/oauth/` | 1,063 |
| 交互登录 UI | `src/components/ConsoleOAuthFlow.tsx` | 1,710 |
| OAuth 常量与 scope/端点 | `src/constants/oauth.ts` | ~236 |
| **MCP OAuth** | `src/services/mcp/auth.ts` | **2,465** |
| MCP OAuth 辅助 | `oauthPort.ts`、`xaa.ts`、`xaaIdpLogin.ts` | ~500 |
| MCP 代理/官方 registry OAuth | `claudeai.ts`、`officialRegistry.ts` | ~数百 |
| Auth host 白名单 | `src/services/auth/hostGuard.ts` | ~100 |
| `/login`、`/logout` | `src/commands/login/`、`logout/` | ~数百 |
| GitHub App OAuth 安装 | `src/commands/install-github-app/OAuthFlowStep.tsx` 等 | ~数百 |
| 订阅/等级门控 + OAuth token 存取 | `src/utils/auth.ts` | ~1,600 |
| 云凭据刷新（AWS/GCP） | `auth.ts` 中 `refreshAwsAuth`/`refreshGcpAuth` 等 | ~400 |

### 保留（`src/utils/auth.ts` 2,012 → ~400）
- `getAnthropicApiKey()` / `saveApiKey()` / `removeApiKey()`
- `getApiKeyFromApiKeyHelper()` + 缓存
- macOS Keychain / `~/.claude.json` 的 API Key 存取

---

## 2. Provider 层

**目标：两个协议适配器。**

```
                    ┌─ anthropic 适配器 ─ Anthropic Messages 协议
统一消息模型 ───────┤
                    └─ openai    适配器 ─ OpenAI Chat Completions 协议
```

| Provider | 处理 |
|---|---|
| firstParty → **anthropic 适配器** | ✅ 保留（`src/services/api/claude.ts`）|
| openai → **openai 适配器** | ✅ 保留（`src/services/api/openai/`，1,752 行）|
| bedrock / vertex / foundry / gemini / grok | ❌ 全删 |

覆盖场景：OpenAI / DeepSeek / Cerebras / Groq / 智谱 / 小米 / DashScope / vLLM / Ollama 等所有 OpenAI 兼容端点；任意 Anthropic 兼容网关。

### 删除
- `src/services/api/gemini/`（334）、`src/services/api/grok/`（321）
- `src/services/api/bedrockClient.ts`（65）、`src/utils/model/bedrock.ts`（265）、`src/utils/aws.ts`
- `src/services/providerRegistry/`（预设 cerebras/dashscope/deepseek 地址）
- `src/services/providerUsage/balance/deepseek.ts`
- `src/utils/chinaLlmProviders.ts`
- `src/services/api/openai/chatgptAuth.ts`（ChatGPT 账号认证，非 API Key）
- `providerClassification.ts`，`getAPIProvider()` 精简为 2 分支

---

## 3. Packages（整包）

| Package | LOC | 处理 |
|---|---|---|
| `packages/@ant/ink` | 27,827 | ❌ 不移植，换 `ratatui` |
| `packages/remote-control-server`（含 React Web UI） | 24,716 | ❌ 删（或仅保留 server，前端 TS sidecar）|
| `packages/@ant/computer-use-*`（mcp/swift/input） | 9,922 | ❌ 删 |
| `packages/workflow-engine` | 6,500 | ❌ 删 |
| `packages/@ant/claude-for-chrome-mcp` | 3,124 | ❌ 删 |
| `packages/weixin` | 2,265 | ❌ 删 |
| `packages/cloud-artifacts` | 223 | ❌ 删 |
| `packages/acp-link` | 4,586 | ⚠️ 视需要 |
| `packages/@ant/model-provider` | 5,193 | ⚠️ 视需要 |
| `packages/mcp-client` | 2,853 | ✅ 保留（去 OAuth）|
| `packages/builtin-tools` | 65,739 | ✅ 保留核心工具 |

---

## 4. Feature Flags（默认不实现）

代码已内置 80+ 个 `feature()` flag，多数默认关闭。Rust 侧不提供对应 Cargo feature 即可。

| Feature | 引用文件数 | 说明 |
|---|---|---|
| `KAIROS` + `KAIROS_BRIEF` + `KAIROS_CHANNELS` + `KAIROS_GITHUB_WEBHOOKS` + `KAIROS_PUSH_NOTIFICATION` | ~65 | 最大隐藏功能族 |
| `TRANSCRIPT_CLASSIFIER` | 95 | 转录分类 |
| `VOICE_MODE` | 49 | 语音模式 |
| `TEAMMEM` / `COORDINATOR_MODE` / `UDS_INBOX` / `LAN_PIPES` / `PROACTIVE` | 17/34/41/11/43 | 团队协作 + 多 agent 协调 |
| `BRIDGE_MODE` / `AGENT_TRIGGERS_REMOTE` | 35 | `src/bridge/`（13,470）|
| `CHICAGO_MCP` | 16 | Computer Use |
| `BG_SESSIONS` / `DAEMON` | 16/4 | `src/daemon/`（943）|
| `BUDDY` | 18 | `src/buddy/`（1,508）|
| `MCP_SKILLS` / `EXPERIMENTAL_SKILL_SEARCH` / `SKILL_LEARNING` | 9/23 | `src/skills/`（4,756）|
| `AWAY_SUMMARY`、`EXTRACT_MEMORIES`、`VERIFICATION_AGENT`、`ULTRAPLAN`、`SHOT_STATS`、`TOKEN_BUDGET`、`PROMPT_CACHE_BREAK_DETECTION`、`LODESTONE` | 各 3–10 | 单点功能 |

---

## 5. 第三方依赖替换表

| 现有 npm | Rust 替换 |
|---|---|
| `axios` / `undici` | `reqwest` |
| `execa` | `tokio::process` |
| `chokidar` | `notify` |
| `lodash-es` | 标准库 / `itertools` |
| `marked` / `highlight.js` / `turndown` | `pulldown-cmark` / `syntect` |
| `fuse.js` | 自研 fuzzy |
| `ignore` / `picomatch` | `ignore` / `globset` |
| `ajv` / `zod` | `serde` / `schemars` / `validator` |
| `xss` | `ammonia` |
| `semver` | `semver` |
| `plist` | `plist` |
| `qrcode` | `qrcode` |
| `sharp` | `image` |
| `@modelcontextprotocol/sdk` | `rmcp` 或裸 JSON-RPC |
| `@agentclientprotocol/sdk` | 裸 JSON-RPC / WebSocket |
| `@opentelemetry/*` / `@langfuse/*` / `@sentry/node` / `@growthbook/growthbook` | ❌ 删（或 `tracing`）|
| `@aws-sdk/*` / `@anthropic-ai/vertex-sdk` / `@anthropic-ai/foundry-sdk` / `@azure/identity` / `google-auth-library` / `@anthropic-ai/bedrock-sdk` / `openai` | ❌ 删，改原生 HTTP |

---

## 6. 默认端点删除清单（决策 D3）

| 位置 | 当前默认值 |
|---|---|
| `src/constants/product.ts` | `CLAUDE_AI_BASE_URL` 等 3 个 |
| `src/services/api/grok/client.ts` | `https://api.x.ai/v1` |
| `src/services/api/gemini/client.ts` | `https://generativelanguage.googleapis.com/v1beta` |
| `src/services/providerRegistry/loader.ts` | cerebras / dashscope / deepseek |
| `src/utils/chinaLlmProviders.ts` | deepseek / bigmodel / dashscope |
| `src/services/providerUsage/balance/deepseek.ts` | `https://api.deepseek.com` |
| `src/utils/sideQuery.ts` | gemini v1beta |
| `src/services/mcp/officialRegistry.ts` | mcp-registry anthropic |
| `src/components/FeedbackSurvey/submitTranscriptShare.ts` | 反馈上传端点 |
| `src/services/api/metricsOptOut.ts` | metrics 端点 |
| `src/utils/telemetry/bigqueryExporter.ts` | 默认 exporter 端点 |
| `src/services/analytics/growthbook.ts` | growthbook 端点 |
| `src/upstreamproxy/upstreamproxy.ts` | 上游默认 |
| `src/ssh/createSSHSession.ts` | 注入 `ANTHROPIC_BASE_URL`（ssh 已砍）|

→ 全部改为**由配置提供**，缺失即报错。

---

## 7. 收益估算

| | 初版 | 定稿 |
|---|---|---|
| 协议适配器 | 7 | **2** |
| 认证方式 | OAuth + API Key + 云凭据 | **仅显式 API Key** |
| 默认端点 | 多处内置 | **0** |
| OAuth 相关净删 | ~5,000 | **~8,000–9,000** |
| Provider 相关净删 | ~1,500 | **~2,500–3,500** |
| 第三方 npm 依赖 | ~90 | **~0（全部换 crate 或删）** |
